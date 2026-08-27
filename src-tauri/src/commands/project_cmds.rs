use tauri::State;

use crate::error::AppResult;
use crate::project::{self, ProjectRecord};
use crate::state::AppState;

#[tauri::command]
pub fn list_projects(state: State<AppState>) -> AppResult<Vec<ProjectRecord>> {
    project::list(&state.db)
}

#[tauri::command]
pub fn add_project(
    state: State<AppState>,
    path: String,
    name: Option<String>,
) -> AppResult<ProjectRecord> {
    project::add(&state.db, &path, name)
}

#[tauri::command]
pub fn remove_project(state: State<AppState>, id: String) -> AppResult<()> {
    project::remove(&state.db, &id)
}
