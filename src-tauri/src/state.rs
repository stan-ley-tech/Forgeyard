use crate::db::DbPool;
use crate::process::ProcessManager;

/// Shared application state handed to every command via `tauri::State`.
pub struct AppState {
    pub db: DbPool,
    pub processes: ProcessManager,
}
