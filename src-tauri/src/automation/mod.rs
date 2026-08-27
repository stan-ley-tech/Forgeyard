mod model;
mod runner;
mod watcher;

pub use model::{AutomationRule, TaskRef};
#[allow(unused_imports)] // StageStatus rides along inside StageResult for the frontend
pub use runner::{run_now, AutomationRunRecord, StageResult, StageStatus};
pub use watcher::AutomationEngine;

use crate::db::DbPool;
use crate::error::AppResult;

pub fn create_rule(
    pool: &DbPool,
    project_id: &str,
    name: &str,
    watch_globs: Vec<String>,
    tasks: Vec<TaskRef>,
) -> AppResult<AutomationRule> {
    model::create(pool, project_id, name, watch_globs, tasks)
}

pub fn list_rules(pool: &DbPool, project_id: &str) -> AppResult<Vec<AutomationRule>> {
    model::list(pool, project_id)
}

pub fn find_rule(pool: &DbPool, id: &str) -> AppResult<AutomationRule> {
    model::find(pool, id)
}

pub fn delete_rule(pool: &DbPool, id: &str) -> AppResult<()> {
    model::delete(pool, id)
}

pub fn set_rule_enabled(pool: &DbPool, id: &str, enabled: bool) -> AppResult<()> {
    model::set_enabled(pool, id, enabled)
}

pub fn list_runs(pool: &DbPool, rule_id: &str) -> AppResult<Vec<AutomationRunRecord>> {
    runner::list_runs(pool, rule_id)
}
