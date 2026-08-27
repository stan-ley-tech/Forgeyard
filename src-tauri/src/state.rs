use crate::db::DbPool;

/// Shared application state handed to every command via `tauri::State`.
pub struct AppState {
    pub db: DbPool,
}
