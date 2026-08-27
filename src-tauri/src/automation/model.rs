use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::DbPool;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRef {
    pub group: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AutomationRule {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub watch_globs: Vec<String>,
    pub tasks: Vec<TaskRef>,
    pub enabled: bool,
    pub created_at: String,
}

pub fn create(
    pool: &DbPool,
    project_id: &str,
    name: &str,
    watch_globs: Vec<String>,
    tasks: Vec<TaskRef>,
) -> AppResult<AutomationRule> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    let watch_globs_json = serde_json::to_string(&watch_globs).map_err(|e| AppError::Other(e.to_string()))?;
    let tasks_json = serde_json::to_string(&tasks).map_err(|e| AppError::Other(e.to_string()))?;

    let conn = pool.get()?;
    conn.execute(
        "INSERT INTO automation_rules (id, project_id, name, watch_globs, tasks, enabled, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)",
        params![id, project_id, name, watch_globs_json, tasks_json, created_at],
    )?;

    Ok(AutomationRule {
        id,
        project_id: project_id.to_string(),
        name: name.to_string(),
        watch_globs,
        tasks,
        enabled: true,
        created_at,
    })
}

pub fn list(pool: &DbPool, project_id: &str) -> AppResult<Vec<AutomationRule>> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, project_id, name, watch_globs, tasks, enabled, created_at
         FROM automation_rules WHERE project_id = ?1 ORDER BY created_at",
    )?;
    let rows = stmt
        .query_map(params![project_id], row_to_rule)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn find(pool: &DbPool, id: &str) -> AppResult<AutomationRule> {
    let conn = pool.get()?;
    conn.query_row(
        "SELECT id, project_id, name, watch_globs, tasks, enabled, created_at
         FROM automation_rules WHERE id = ?1",
        params![id],
        row_to_rule,
    )
    .optional()?
    .ok_or_else(|| AppError::Other(format!("automation rule not found: {id}")))
}

pub fn delete(pool: &DbPool, id: &str) -> AppResult<()> {
    let conn = pool.get()?;
    let affected = conn.execute("DELETE FROM automation_rules WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(AppError::Other(format!("automation rule not found: {id}")));
    }
    Ok(())
}

pub fn set_enabled(pool: &DbPool, id: &str, enabled: bool) -> AppResult<()> {
    let conn = pool.get()?;
    let affected = conn.execute(
        "UPDATE automation_rules SET enabled = ?2 WHERE id = ?1",
        params![id, enabled as i64],
    )?;
    if affected == 0 {
        return Err(AppError::Other(format!("automation rule not found: {id}")));
    }
    Ok(())
}

fn row_to_rule(row: &rusqlite::Row) -> rusqlite::Result<AutomationRule> {
    let watch_globs_json: String = row.get(3)?;
    let tasks_json: String = row.get(4)?;
    let enabled: i64 = row.get(5)?;
    Ok(AutomationRule {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        watch_globs: serde_json::from_str(&watch_globs_json).unwrap_or_default(),
        tasks: serde_json::from_str(&tasks_json).unwrap_or_default(),
        enabled: enabled != 0,
        created_at: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn pool() -> DbPool {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::init_pool(dir.path()).unwrap();
        // automation_rules.project_id has a foreign key into projects, so a
        // row needs to exist there first.
        pool.get()
            .unwrap()
            .execute(
                "INSERT INTO projects (id, name, path, created_at, updated_at) VALUES ('proj', 'proj', '/tmp/proj', '', '')",
                [],
            )
            .unwrap();
        pool
    }

    fn sample_tasks() -> Vec<TaskRef> {
        vec![TaskRef { group: "Development".into(), name: "Test".into() }]
    }

    #[test]
    fn create_and_list_round_trip() {
        let pool = pool();
        create(&pool, "proj", "on save", vec!["*.rs".into()], sample_tasks()).unwrap();

        let rules = list(&pool, "proj").unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name, "on save");
        assert_eq!(rules[0].watch_globs, vec!["*.rs".to_string()]);
        assert!(rules[0].enabled);
    }

    #[test]
    fn set_enabled_toggles_the_flag() {
        let pool = pool();
        let rule = create(&pool, "proj", "on save", vec![], sample_tasks()).unwrap();

        set_enabled(&pool, &rule.id, false).unwrap();

        assert!(!find(&pool, &rule.id).unwrap().enabled);
    }

    #[test]
    fn delete_removes_the_rule() {
        let pool = pool();
        let rule = create(&pool, "proj", "on save", vec![], sample_tasks()).unwrap();

        delete(&pool, &rule.id).unwrap();

        assert!(matches!(find(&pool, &rule.id), Err(AppError::Other(_))));
    }
}
