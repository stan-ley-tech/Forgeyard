use tauri::State;

use crate::error::AppResult;
use crate::logs::{self, LogEntry, LogFilter};
use crate::state::AppState;

#[tauri::command]
pub fn get_logs(
    state: State<AppState>,
    project_id: String,
    filter: Option<LogFilter>,
) -> AppResult<Vec<LogEntry>> {
    logs::query(&state.db, &project_id, &filter.unwrap_or_default())
}
