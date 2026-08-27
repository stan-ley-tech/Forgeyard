use std::path::PathBuf;

use tauri::State;

use crate::error::AppResult;
use crate::process::ProcessInfo;
use crate::project;
use crate::state::AppState;

#[tauri::command]
pub fn start_process(
    state: State<AppState>,
    project_id: String,
    name: String,
    command: String,
) -> AppResult<ProcessInfo> {
    let record = project::find(&state.db, &project_id)?;
    state
        .processes
        .start(project_id, name, command, PathBuf::from(record.path))
}

#[tauri::command]
pub fn stop_process(state: State<AppState>, project_id: String, name: String) -> AppResult<()> {
    state.processes.stop(&project_id, &name)
}

#[tauri::command]
pub fn kill_process(state: State<AppState>, project_id: String, name: String) -> AppResult<()> {
    state.processes.kill(&project_id, &name)
}

#[tauri::command]
pub async fn restart_process(
    state: State<'_, AppState>,
    project_id: String,
    name: String,
    command: String,
) -> AppResult<ProcessInfo> {
    let record = project::find(&state.db, &project_id)?;
    state
        .processes
        .restart(project_id, name, command, PathBuf::from(record.path))
        .await
}

#[tauri::command]
pub fn list_processes(state: State<AppState>, project_id: String) -> AppResult<Vec<ProcessInfo>> {
    Ok(state.processes.list(&project_id))
}
