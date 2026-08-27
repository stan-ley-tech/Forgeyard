use std::path::Path;

use tauri::State;

use crate::detect::{self, ProjectHealth};
use crate::error::AppResult;
use crate::project;
use crate::state::AppState;

#[tauri::command]
pub fn get_project_health(state: State<AppState>, id: String) -> AppResult<ProjectHealth> {
    let record = project::find(&state.db, &id)?;
    Ok(detect::scan(Path::new(&record.path)))
}
