use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::db::DbPool;
use crate::error::AppResult;
use crate::event_sink::EventSink;
use crate::project;

use super::model::{self, AutomationRule};
use super::runner;

/// Debounce window: filesystem events arrive in bursts (a save can emit
/// several write events for one file), so events within this window of the
/// last accepted one are dropped rather than re-running the whole chain.
const DEBOUNCE_WINDOW: Duration = Duration::from_millis(300);

struct WatchHandle {
    _watcher: RecommendedWatcher,
}

/// Owns one live filesystem watcher per enabled automation rule. Dropping
/// a rule's `RecommendedWatcher` (via `unwatch`) stops that subscription;
/// nothing else needs to be torn down.
pub struct AutomationEngine {
    app: Arc<dyn EventSink>,
    db: DbPool,
    active: Mutex<HashMap<String, WatchHandle>>,
}

impl AutomationEngine {
    pub fn new(app: impl EventSink + 'static, db: DbPool) -> Self {
        Self { app: Arc::new(app), db, active: Mutex::new(HashMap::new()) }
    }

    /// Starts watching every enabled rule across all registered projects.
    /// Called once at app startup.
    pub fn start_all(&self) -> AppResult<()> {
        for record in project::list(&self.db)? {
            for rule in model::list(&self.db, &record.id)? {
                if rule.enabled {
                    self.watch(rule, PathBuf::from(&record.path));
                }
            }
        }
        Ok(())
    }

    pub fn watch(&self, rule: AutomationRule, project_path: PathBuf) {
        self.unwatch(&rule.id);

        let app = self.app.clone();
        let db = self.db.clone();
        let rule_id = rule.id.clone();
        let watch_globs = rule.watch_globs.clone();
        let tasks_to_run = rule.tasks.clone();
        let last_run = Arc::new(Mutex::new(Instant::now() - Duration::from_secs(3600)));

        let watch_project_path = project_path.clone();
        let mut watcher = match notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            let Ok(event) = res else { return };
            if !(event.kind.is_create() || event.kind.is_modify() || event.kind.is_remove()) {
                return;
            }
            let Some(path) = event.paths.first() else { return };
            if !matches_any_glob(path, &watch_globs) {
                return;
            }

            {
                let mut last = last_run.lock().unwrap();
                if last.elapsed() < DEBOUNCE_WINDOW {
                    return;
                }
                *last = Instant::now();
            }

            let app = app.clone();
            let db = db.clone();
            let rule_id = rule_id.clone();
            let tasks_to_run = tasks_to_run.clone();
            let project_path = watch_project_path.clone();
            let trigger_path = path.display().to_string();

            tauri::async_runtime::spawn(async move {
                let _ =
                    runner::run_now(app.as_ref(), &db, &rule_id, &project_path, &tasks_to_run, Some(&trigger_path))
                        .await;
            });
        }) {
            Ok(w) => w,
            Err(_) => return,
        };

        if watcher.watch(&project_path, RecursiveMode::Recursive).is_err() {
            return;
        }

        self.active.lock().unwrap().insert(rule.id.clone(), WatchHandle { _watcher: watcher });
    }

    pub fn unwatch(&self, rule_id: &str) {
        self.active.lock().unwrap().remove(rule_id);
    }
}

/// Hand-rolled instead of pulling in a glob crate: rules only need `*` as
/// a wildcard (e.g. `*.rs`, `src/*.ts`), which a short recursive matcher
/// covers without adding a dependency for the whole glob spec.
fn matches_any_glob(path: &std::path::Path, globs: &[String]) -> bool {
    if globs.is_empty() {
        return true;
    }
    let path_str = path.to_string_lossy().replace('\\', "/");
    globs.iter().any(|pattern| glob_match(pattern, &path_str))
}

fn glob_match(pattern: &str, text: &str) -> bool {
    fn helper(p: &[u8], t: &[u8]) -> bool {
        match (p.first(), t.first()) {
            (None, None) => true,
            (Some(b'*'), _) => helper(&p[1..], t) || (!t.is_empty() && helper(p, &t[1..])),
            (Some(pc), Some(tc)) if pc == tc => helper(&p[1..], &t[1..]),
            _ => false,
        }
    }
    helper(pattern.as_bytes(), text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_matches_any_suffix() {
        assert!(glob_match("*.rs", "src/main.rs"));
        assert!(!glob_match("*.rs", "src/main.ts"));
    }

    #[test]
    fn wildcard_matches_a_path_segment() {
        assert!(glob_match("src/*.ts", "src/app.ts"));
        assert!(!glob_match("src/*.ts", "lib/app.ts"));
    }

    #[test]
    fn empty_glob_list_matches_everything() {
        assert!(matches_any_glob(std::path::Path::new("anything.txt"), &[]));
    }
}
