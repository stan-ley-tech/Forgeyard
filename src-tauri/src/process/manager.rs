use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use chrono::Utc;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::event_sink::{to_json, EventSink};
use crate::logs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessStatus {
    Running,
    Stopping,
    Stopped,
    Crashed,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    pub project_id: String,
    pub name: String,
    pub command: String,
    pub status: ProcessStatus,
    pub pid: Option<u32>,
    pub started_at: String,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
struct LogEvent {
    project_id: String,
    name: String,
    stream: &'static str,
    line: String,
    timestamp: String,
}

enum Signal {
    Graceful,
    Force,
}

struct ServiceEntry {
    info: ProcessInfo,
    expected_stop: Arc<AtomicBool>,
}

struct Inner {
    app: Arc<dyn EventSink>,
    db: DbPool,
    services: Mutex<HashMap<String, ServiceEntry>>,
}

/// Tracks every service Forgeyard has started, across all open projects.
///
/// Each running child is owned exclusively by the task spawned to await
/// its exit (see `spawn_wait_task`) — nothing else touches the `Child`
/// handle. `stop`/`kill` act on the OS pid directly instead, which avoids
/// needing a lock that both the exit-watcher and the stop/kill caller
/// would otherwise have to share across an `.await`.
#[derive(Clone)]
pub struct ProcessManager {
    inner: Arc<Inner>,
}

impl ProcessManager {
    pub fn new(app: impl EventSink + 'static, db: DbPool) -> Self {
        Self {
            inner: Arc::new(Inner {
                app: Arc::new(app),
                db,
                services: Mutex::new(HashMap::new()),
            }),
        }
    }

    pub fn start(
        &self,
        project_id: String,
        name: String,
        command: String,
        cwd: std::path::PathBuf,
    ) -> AppResult<ProcessInfo> {
        let key = service_key(&project_id, &name);

        {
            let services = self.inner.services.lock().unwrap();
            if let Some(existing) = services.get(&key) {
                if matches!(
                    existing.info.status,
                    ProcessStatus::Running | ProcessStatus::Stopping
                ) {
                    return Err(AppError::Process(format!("{name} is already running")));
                }
            }
        }

        let mut cmd = shell_command(&command);
        cmd.current_dir(&cwd);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .map_err(|e| AppError::Process(format!("failed to start {name}: {e}")))?;

        let pid = child.id();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let info = ProcessInfo {
            project_id: project_id.clone(),
            name: name.clone(),
            command,
            status: ProcessStatus::Running,
            pid,
            started_at: Utc::now().to_rfc3339(),
            exit_code: None,
        };
        let expected_stop = Arc::new(AtomicBool::new(false));

        self.inner.services.lock().unwrap().insert(
            key.clone(),
            ServiceEntry {
                info: info.clone(),
                expected_stop: expected_stop.clone(),
            },
        );

        if let Some(stdout) = stdout {
            self.spawn_log_reader(project_id.clone(), name.clone(), "stdout", stdout);
        }
        if let Some(stderr) = stderr {
            self.spawn_log_reader(project_id.clone(), name.clone(), "stderr", stderr);
        }
        self.spawn_wait_task(key, child, expected_stop);

        self.emit_status(&info);
        Ok(info)
    }

    pub fn stop(&self, project_id: &str, name: &str) -> AppResult<()> {
        self.signal(project_id, name, Signal::Graceful)
    }

    pub fn kill(&self, project_id: &str, name: &str) -> AppResult<()> {
        self.signal(project_id, name, Signal::Force)
    }

    /// Stops the current run (if any) and starts a fresh one with the given
    /// command, waiting for the old process to actually exit first so the
    /// replacement doesn't collide with it (e.g. over the same port).
    pub async fn restart(
        &self,
        project_id: String,
        name: String,
        command: String,
        cwd: std::path::PathBuf,
    ) -> AppResult<ProcessInfo> {
        let key = service_key(&project_id, &name);
        let is_active = {
            let services = self.inner.services.lock().unwrap();
            services
                .get(&key)
                .map(|e| {
                    matches!(
                        e.info.status,
                        ProcessStatus::Running | ProcessStatus::Stopping
                    )
                })
                .unwrap_or(false)
        };

        if is_active {
            self.stop(&project_id, &name)?;

            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                let still_active = {
                    let services = self.inner.services.lock().unwrap();
                    services
                        .get(&key)
                        .map(|e| {
                            matches!(
                                e.info.status,
                                ProcessStatus::Running | ProcessStatus::Stopping
                            )
                        })
                        .unwrap_or(false)
                };
                if !still_active {
                    break;
                }
                if tokio::time::Instant::now() >= deadline {
                    self.kill(&project_id, &name)?;
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        self.start(project_id, name, command, cwd)
    }

    pub fn list(&self, project_id: &str) -> Vec<ProcessInfo> {
        let services = self.inner.services.lock().unwrap();
        services
            .values()
            .filter(|e| e.info.project_id == project_id)
            .map(|e| e.info.clone())
            .collect()
    }

    fn signal(&self, project_id: &str, name: &str, signal: Signal) -> AppResult<()> {
        let key = service_key(project_id, name);

        let (pid, expected_stop, updated_info) = {
            let mut services = self.inner.services.lock().unwrap();
            let entry = services
                .get_mut(&key)
                .ok_or_else(|| AppError::Process(format!("{name} is not running")))?;
            let pid = entry
                .info
                .pid
                .ok_or_else(|| AppError::Process(format!("{name} is not running")))?;
            entry.info.status = ProcessStatus::Stopping;
            (pid, entry.expected_stop.clone(), entry.info.clone())
        };

        expected_stop.store(true, Ordering::SeqCst);
        self.emit_status(&updated_info);
        terminate(pid, signal)
    }

    fn spawn_log_reader<Reader>(
        &self,
        project_id: String,
        name: String,
        stream: &'static str,
        reader: Reader,
    ) where
        Reader: tokio::io::AsyncRead + Unpin + Send + 'static,
    {
        let app = self.inner.app.clone();
        let db = self.inner.db.clone();
        tauri::async_runtime::spawn(async move {
            let mut lines = BufReader::new(reader).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let level = logs::infer_level(stream, &line);
                let _ = logs::record(&db, &project_id, &name, level, &line);

                app.emit(
                    "process:log",
                    to_json(&LogEvent {
                        project_id: project_id.clone(),
                        name: name.clone(),
                        stream,
                        line,
                        timestamp: Utc::now().to_rfc3339(),
                    }),
                );
            }
        });
    }

    fn spawn_wait_task(&self, key: String, mut child: Child, expected_stop: Arc<AtomicBool>) {
        let inner = self.inner.clone();
        tauri::async_runtime::spawn(async move {
            let exit = child.wait().await;

            let (status, exit_code) = match exit {
                Ok(status) => {
                    let code = status.code();
                    let expected = expected_stop.load(Ordering::SeqCst);
                    let status = if expected || code == Some(0) {
                        ProcessStatus::Stopped
                    } else {
                        ProcessStatus::Crashed
                    };
                    (status, code)
                }
                Err(_) => (ProcessStatus::Crashed, None),
            };

            let updated = {
                let mut services = inner.services.lock().unwrap();
                services.get_mut(&key).map(|entry| {
                    entry.info.status = status;
                    entry.info.exit_code = exit_code;
                    entry.info.pid = None;
                    entry.info.clone()
                })
            };

            if let Some(info) = updated {
                inner.app.emit("process:status", to_json(&info));
            }
        });
    }

    fn emit_status(&self, info: &ProcessInfo) {
        self.inner.app.emit("process:status", to_json(info));
    }
}

fn service_key(project_id: &str, name: &str) -> String {
    format!("{project_id}::{name}")
}

fn shell_command(command: &str) -> Command {
    if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", command]);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", command]);
        cmd
    }
}

#[cfg(unix)]
fn terminate(pid: u32, signal: Signal) -> AppResult<()> {
    let sig = match signal {
        Signal::Graceful => libc::SIGTERM,
        Signal::Force => libc::SIGKILL,
    };
    // SAFETY: `pid` came from `Child::id()` for a process we spawned and is
    // signaled with a plain, non-reentrant libc call.
    let result = unsafe { libc::kill(pid as libc::pid_t, sig) };
    if result != 0 {
        return Err(AppError::Process(format!(
            "failed to signal pid {pid}: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn terminate(pid: u32, _signal: Signal) -> AppResult<()> {
    // Windows has no POSIX-style signal for arbitrary processes, so both
    // stop and kill hard-terminate the process tree via taskkill.
    let output = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .output()
        .map_err(|e| AppError::Process(format!("failed to run taskkill: {e}")))?;

    if !output.status.success() {
        return Err(AppError::Process(format!(
            "taskkill failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_sink::NoopSink;

    fn quick_exit(code: i32) -> String {
        format!("exit {code}")
    }

    fn long_running() -> &'static str {
        if cfg!(windows) {
            "ping -n 5 127.0.0.1 > NUL"
        } else {
            "sleep 5"
        }
    }

    async fn wait_for<F>(mut predicate: F) -> bool
    where
        F: FnMut() -> bool,
    {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        while tokio::time::Instant::now() < deadline {
            if predicate() {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        false
    }

    fn test_manager() -> ProcessManager {
        let dir = tempfile::tempdir().unwrap();
        // Leaked deliberately: the pool must outlive the temp dir for the
        // lifetime of the test, which is simplest to guarantee this way in
        // a short-lived test process.
        let dir = Box::leak(Box::new(dir));
        let db = crate::db::init_pool(dir.path()).unwrap();
        ProcessManager::new(NoopSink, db)
    }

    #[tokio::test]
    async fn start_reports_running_with_a_pid() {
        let manager = test_manager();
        let cwd = std::env::current_dir().unwrap();

        let info = manager
            .start("p1".into(), "web".into(), long_running().into(), cwd)
            .unwrap();

        assert_eq!(info.status, ProcessStatus::Running);
        assert!(info.pid.is_some());

        manager.kill("p1", "web").unwrap();
    }

    #[tokio::test]
    async fn clean_exit_is_reported_as_stopped_not_crashed() {
        let manager = test_manager();
        let cwd = std::env::current_dir().unwrap();

        manager
            .start("p2".into(), "job".into(), quick_exit(0), cwd)
            .unwrap();

        let stopped = wait_for(|| {
            manager
                .list("p2")
                .iter()
                .any(|p| p.name == "job" && p.status == ProcessStatus::Stopped)
        })
        .await;

        assert!(stopped, "expected job to finish as Stopped");
    }

    #[tokio::test]
    async fn nonzero_exit_is_reported_as_crashed() {
        let manager = test_manager();
        let cwd = std::env::current_dir().unwrap();

        manager
            .start("p3".into(), "job".into(), quick_exit(7), cwd)
            .unwrap();

        let crashed = wait_for(|| {
            manager
                .list("p3")
                .iter()
                .any(|p| p.name == "job" && p.status == ProcessStatus::Crashed)
        })
        .await;

        assert!(crashed, "expected job to finish as Crashed");
        let info = manager
            .list("p3")
            .into_iter()
            .find(|p| p.name == "job")
            .unwrap();
        assert_eq!(info.exit_code, Some(7));
    }

    #[tokio::test]
    async fn kill_is_reported_as_stopped_because_it_was_expected() {
        let manager = test_manager();
        let cwd = std::env::current_dir().unwrap();

        manager
            .start("p4".into(), "web".into(), long_running().into(), cwd)
            .unwrap();
        manager.kill("p4", "web").unwrap();

        let stopped = wait_for(|| {
            manager
                .list("p4")
                .iter()
                .any(|p| p.name == "web" && p.status == ProcessStatus::Stopped)
        })
        .await;

        assert!(
            stopped,
            "an intentional kill should not be reported as a crash"
        );
    }

    #[tokio::test]
    async fn starting_twice_while_running_is_rejected() {
        let manager = test_manager();
        let cwd = std::env::current_dir().unwrap();

        manager
            .start(
                "p5".into(),
                "web".into(),
                long_running().into(),
                cwd.clone(),
            )
            .unwrap();
        let second = manager.start("p5".into(), "web".into(), long_running().into(), cwd);

        assert!(second.is_err());
        manager.kill("p5", "web").unwrap();
    }
}
