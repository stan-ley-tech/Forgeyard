use std::path::PathBuf;

use tauri::State;

use crate::automation::{self, AutomationRule, AutomationRunRecord, StageResult, TaskRef};
use crate::error::AppResult;
use crate::project;
use crate::state::AppState;

#[tauri::command]
pub fn list_automation_rules(state: State<AppState>, project_id: String) -> AppResult<Vec<AutomationRule>> {
    automation::list_rules(&state.db, &project_id)
}

#[tauri::command]
pub fn create_automation_rule(
    state: State<AppState>,
    project_id: String,
    name: String,
    watch_globs: Vec<String>,
    tasks: Vec<TaskRef>,
) -> AppResult<AutomationRule> {
    let record = project::find(&state.db, &project_id)?;
    let rule = automation::create_rule(&state.db, &project_id, &name, watch_globs, tasks)?;
    state.automation.watch(rule.clone(), PathBuf::from(record.path));
    Ok(rule)
}

#[tauri::command]
pub fn delete_automation_rule(state: State<AppState>, id: String) -> AppResult<()> {
    state.automation.unwatch(&id);
    automation::delete_rule(&state.db, &id)
}

#[tauri::command]
pub fn toggle_automation_rule(state: State<AppState>, id: String, enabled: bool) -> AppResult<()> {
    automation::set_rule_enabled(&state.db, &id, enabled)?;
    let rule = automation::find_rule(&state.db, &id)?;

    if enabled {
        let record = project::find(&state.db, &rule.project_id)?;
        state.automation.watch(rule, PathBuf::from(record.path));
    } else {
        state.automation.unwatch(&id);
    }
    Ok(())
}

#[tauri::command]
pub async fn run_automation_rule_now(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Vec<StageResult>> {
    let rule = automation::find_rule(&state.db, &id)?;
    let record = project::find(&state.db, &rule.project_id)?;
    automation::run_now(&app, &state.db, &rule.id, &PathBuf::from(record.path), &rule.tasks, None).await
}

#[tauri::command]
pub fn list_automation_runs(state: State<AppState>, rule_id: String) -> AppResult<Vec<AutomationRunRecord>> {
    automation::list_runs(&state.db, &rule_id)
}
