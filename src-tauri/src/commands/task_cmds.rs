use std::path::{Path, PathBuf};

use tauri::State;

use crate::error::{AppError, AppResult};
use crate::process::ProcessInfo;
use crate::project;
use crate::state::AppState;
use crate::tasks::{self, TaskGroup};

#[tauri::command]
pub fn list_tasks(state: State<AppState>, project_id: String) -> AppResult<Vec<TaskGroup>> {
    let record = project::find(&state.db, &project_id)?;
    Ok(tasks::load(Path::new(&record.path)))
}

#[tauri::command]
pub fn run_task(
    state: State<AppState>,
    project_id: String,
    group: String,
    name: String,
) -> AppResult<ProcessInfo> {
    let record = project::find(&state.db, &project_id)?;
    let groups = tasks::load(Path::new(&record.path));

    let command = groups
        .iter()
        .find(|g| g.name == group)
        .and_then(|g| g.tasks.iter().find(|t| t.name == name))
        .map(|t| t.command.clone())
        .ok_or_else(|| AppError::Task(format!("no task named '{name}' in group '{group}'")))?;

    let service_name = format!("{group}:{name}");
    state
        .processes
        .start(project_id, service_name, command, PathBuf::from(record.path))
}
