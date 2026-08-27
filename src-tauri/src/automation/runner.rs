use std::path::Path;
use std::process::Stdio;

use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use uuid::Uuid;

use crate::db::DbPool;
use crate::error::AppResult;
use crate::event_sink::{to_json, EventSink};
use crate::tasks;

use super::model::TaskRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StageStatus {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub group: String,
    pub name: String,
    pub status: StageStatus,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AutomationRunRecord {
    pub id: String,
    pub rule_id: String,
    pub trigger_path: Option<String>,
    pub stages: Vec<StageResult>,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct AutomationRunEvent {
    rule_id: String,
    run_id: String,
    stages: Vec<StageResult>,
    finished: bool,
}

/// Runs every task in a rule sequentially, stopping at the first failure
/// (remaining stages are recorded as `Skipped` rather than silently
/// dropped), and persists the outcome. Each stage's completion is emitted
/// live so the UI can show progress rather than only a final result.
pub async fn run_now(
    app: &dyn EventSink,
    db: &DbPool,
    rule_id: &str,
    project_path: &Path,
    tasks_to_run: &[TaskRef],
    trigger_path: Option<&str>,
) -> AppResult<Vec<StageResult>> {
    let run_id = Uuid::new_v4().to_string();
    let started_at = Utc::now().to_rfc3339();
    let groups = tasks::load(project_path);

    let mut stages = Vec::with_capacity(tasks_to_run.len());
    let mut failed = false;

    for task_ref in tasks_to_run {
        let status_and_code = if failed {
            (StageStatus::Skipped, None)
        } else {
            match resolve_command(&groups, task_ref) {
                Some(command) => execute_stage(project_path, &command).await,
                None => (StageStatus::Failed, None),
            }
        };

        if status_and_code.0 == StageStatus::Failed {
            failed = true;
        }

        stages.push(StageResult {
            group: task_ref.group.clone(),
            name: task_ref.name.clone(),
            status: status_and_code.0,
            exit_code: status_and_code.1,
        });

        emit_progress(app, rule_id, &run_id, &stages, false);
    }

    emit_progress(app, rule_id, &run_id, &stages, true);
    persist_run(db, &run_id, rule_id, trigger_path, &stages, !failed, &started_at)?;

    Ok(stages)
}

fn resolve_command(groups: &[tasks::TaskGroup], task_ref: &TaskRef) -> Option<String> {
    groups
        .iter()
        .find(|g| g.name == task_ref.group)
        .and_then(|g| g.tasks.iter().find(|t| t.name == task_ref.name))
        .map(|t| t.command.clone())
}

async fn execute_stage(cwd: &Path, command: &str) -> (StageStatus, Option<i32>) {
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", command]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", command]);
        c
    };
    cmd.current_dir(cwd).stdout(Stdio::null()).stderr(Stdio::null());

    match cmd.status().await {
        Ok(status) if status.success() => (StageStatus::Passed, status.code()),
        Ok(status) => (StageStatus::Failed, status.code()),
        Err(_) => (StageStatus::Failed, None),
    }
}

fn emit_progress(app: &dyn EventSink, rule_id: &str, run_id: &str, stages: &[StageResult], finished: bool) {
    app.emit(
        "automation:run",
        to_json(&AutomationRunEvent {
            rule_id: rule_id.to_string(),
            run_id: run_id.to_string(),
            stages: stages.to_vec(),
            finished,
        }),
    );
}

fn persist_run(
    db: &DbPool,
    run_id: &str,
    rule_id: &str,
    trigger_path: Option<&str>,
    stages: &[StageResult],
    passed: bool,
    started_at: &str,
) -> AppResult<()> {
    let conn = db.get()?;
    let stages_json = serde_json::to_string(stages).unwrap_or_default();
    conn.execute(
        "INSERT INTO automation_runs (id, rule_id, trigger_path, stages, status, started_at, finished_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            run_id,
            rule_id,
            trigger_path,
            stages_json,
            if passed { "passed" } else { "failed" },
            started_at,
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub fn list_runs(db: &DbPool, rule_id: &str) -> AppResult<Vec<AutomationRunRecord>> {
    let conn = db.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, rule_id, trigger_path, stages, status, started_at, finished_at
         FROM automation_runs WHERE rule_id = ?1 ORDER BY started_at DESC LIMIT 50",
    )?;
    let rows = stmt
        .query_map(params![rule_id], |row| {
            let stages_json: String = row.get(3)?;
            Ok(AutomationRunRecord {
                id: row.get(0)?,
                rule_id: row.get(1)?,
                trigger_path: row.get(2)?,
                stages: serde_json::from_str(&stages_json).unwrap_or_default(),
                status: row.get(4)?,
                started_at: row.get(5)?,
                finished_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::event_sink::NoopSink;
    use std::fs;

    fn pool() -> DbPool {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::init_pool(dir.path()).unwrap();
        // automation_rules.project_id has a foreign key into projects, so a
        // row needs to exist there first.
        pool.get()
            .unwrap()
            .execute(
                "INSERT INTO projects (id, name, path, created_at, updated_at) VALUES ('proj', 'proj', '/tmp/proj', '', '')",
                [],
            )
            .unwrap();
        pool
    }

    /// automation_runs.rule_id has a foreign key into automation_rules, so
    /// tests exercise `run_now` against a real rule rather than a made-up id.
    fn test_rule(db: &DbPool) -> String {
        super::super::model::create(db, "proj", "test rule", vec![], vec![])
            .unwrap()
            .id
    }

    fn project_with_tasks() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("forgeyard.toml"),
            r#"
[[groups]]
name = "Development"
tasks = [
  { name = "Pass", command = "exit 0" },
  { name = "Fail", command = "exit 1" },
  { name = "ShouldSkip", command = "exit 0" },
]
"#,
        )
        .unwrap();
        dir
    }

    #[tokio::test]
    async fn all_stages_pass_when_every_task_succeeds() {
        let db = pool();
        let rule_id = test_rule(&db);
        let project = project_with_tasks();
        let tasks_to_run = vec![TaskRef { group: "Development".into(), name: "Pass".into() }];

        let stages = run_now(&NoopSink, &db, &rule_id, project.path(), &tasks_to_run, None)
            .await
            .unwrap();

        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0].status, StageStatus::Passed);

        let runs = list_runs(&db, &rule_id).unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].status, "passed");
    }

    #[tokio::test]
    async fn a_failing_stage_skips_the_remaining_ones() {
        let db = pool();
        let rule_id = test_rule(&db);
        let project = project_with_tasks();
        let tasks_to_run = vec![
            TaskRef { group: "Development".into(), name: "Pass".into() },
            TaskRef { group: "Development".into(), name: "Fail".into() },
            TaskRef { group: "Development".into(), name: "ShouldSkip".into() },
        ];

        let stages = run_now(&NoopSink, &db, &rule_id, project.path(), &tasks_to_run, None)
            .await
            .unwrap();

        assert_eq!(stages[0].status, StageStatus::Passed);
        assert_eq!(stages[1].status, StageStatus::Failed);
        assert_eq!(stages[2].status, StageStatus::Skipped);

        let runs = list_runs(&db, &rule_id).unwrap();
        assert_eq!(runs[0].status, "failed");
    }

    #[tokio::test]
    async fn an_undefined_task_reference_is_reported_as_failed() {
        let db = pool();
        let rule_id = test_rule(&db);
        let project = project_with_tasks();
        let tasks_to_run = vec![TaskRef { group: "Development".into(), name: "Nope".into() }];

        let stages = run_now(&NoopSink, &db, &rule_id, project.path(), &tasks_to_run, None)
            .await
            .unwrap();

        assert_eq!(stages[0].status, StageStatus::Failed);
    }
}
