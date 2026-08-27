use std::path::PathBuf;

use tauri::State;

use crate::error::AppResult;
use crate::git::{self, BranchInfo, CommitInfo, GitStatus};
use crate::project;
use crate::state::AppState;

fn project_path(state: &AppState, project_id: &str) -> AppResult<PathBuf> {
    Ok(PathBuf::from(project::find(&state.db, project_id)?.path))
}

#[tauri::command]
pub fn git_status(state: State<AppState>, project_id: String) -> AppResult<GitStatus> {
    git::status(&project_path(&state, &project_id)?)
}

#[tauri::command]
pub fn git_branches(state: State<AppState>, project_id: String) -> AppResult<Vec<BranchInfo>> {
    git::branches(&project_path(&state, &project_id)?)
}

#[tauri::command]
pub fn git_checkout(
    state: State<AppState>,
    project_id: String,
    branch: String,
    create: bool,
) -> AppResult<()> {
    git::checkout(&project_path(&state, &project_id)?, &branch, create)
}

#[tauri::command]
pub fn git_stage(state: State<AppState>, project_id: String, files: Vec<String>) -> AppResult<()> {
    git::stage(&project_path(&state, &project_id)?, &files)
}

#[tauri::command]
pub fn git_unstage(
    state: State<AppState>,
    project_id: String,
    files: Vec<String>,
) -> AppResult<()> {
    git::unstage(&project_path(&state, &project_id)?, &files)
}

#[tauri::command]
pub fn git_commit(state: State<AppState>, project_id: String, message: String) -> AppResult<()> {
    git::commit(&project_path(&state, &project_id)?, &message)
}

#[tauri::command]
pub fn git_diff(
    state: State<AppState>,
    project_id: String,
    file: Option<String>,
    staged: bool,
) -> AppResult<String> {
    git::diff(&project_path(&state, &project_id)?, file.as_deref(), staged)
}

#[tauri::command]
pub fn git_log(
    state: State<AppState>,
    project_id: String,
    limit: u32,
) -> AppResult<Vec<CommitInfo>> {
    git::log(&project_path(&state, &project_id)?, limit)
}

#[tauri::command]
pub fn git_stash(
    state: State<AppState>,
    project_id: String,
    message: Option<String>,
) -> AppResult<()> {
    git::stash(&project_path(&state, &project_id)?, message.as_deref())
}

#[tauri::command]
pub fn git_stash_pop(state: State<AppState>, project_id: String) -> AppResult<()> {
    git::stash_pop(&project_path(&state, &project_id)?)
}

#[tauri::command]
pub fn git_merge(state: State<AppState>, project_id: String, branch: String) -> AppResult<()> {
    git::merge(&project_path(&state, &project_id)?, &branch)
}
