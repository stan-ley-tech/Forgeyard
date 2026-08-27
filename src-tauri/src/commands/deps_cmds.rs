use std::path::Path;

use tauri::State;

use crate::deps::{self, DependencyReport};
use crate::error::AppResult;
use crate::project;
use crate::state::AppState;

#[tauri::command]
pub fn list_dependencies(state: State<AppState>, id: String) -> AppResult<DependencyReport> {
    let record = project::find(&state.db, &id)?;
    Ok(deps::scan(Path::new(&record.path)))
}
