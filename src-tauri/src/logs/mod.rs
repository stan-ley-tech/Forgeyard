use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::db::DbPool;
use crate::error::AppResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub id: i64,
    pub project_id: String,
    pub service: String,
    pub level: LogLevel,
    pub message: String,
    pub created_at: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct LogFilter {
    pub service: Option<String>,
    pub level: Option<LogLevel>,
    pub search: Option<String>,
    pub limit: Option<u32>,
}

/// Cheap heuristic so raw stdout/stderr text gets a useful severity without
/// requiring the underlying process to emit structured logs.
pub fn infer_level(stream: &str, message: &str) -> LogLevel {
    let lower = message.to_ascii_lowercase();
    if stream == "stderr" && ["error", "panic", "exception", "traceback"].iter().any(|m| lower.contains(m)) {
        LogLevel::Error
    } else if lower.contains("warn") {
        LogLevel::Warn
    } else if stream == "stderr" {
        LogLevel::Warn
    } else {
        LogLevel::Info
    }
}

/// Appends one line to the unified log and trims the project's history to
/// the most recent entries, so a long-running dev session doesn't grow the
/// database without bound.
pub fn record(pool: &DbPool, project_id: &str, service: &str, level: LogLevel, message: &str) -> AppResult<()> {
    const RETENTION_PER_PROJECT: i64 = 5000;

    let conn = pool.get()?;
    conn.execute(
        "INSERT INTO log_entries (project_id, service, level, message, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![project_id, service, level_to_str(level), message, chrono::Utc::now().to_rfc3339()],
    )?;
    conn.execute(
        "DELETE FROM log_entries WHERE project_id = ?1 AND id NOT IN (
            SELECT id FROM log_entries WHERE project_id = ?1 ORDER BY id DESC LIMIT ?2
        )",
        params![project_id, RETENTION_PER_PROJECT],
    )?;
    Ok(())
}

pub fn query(pool: &DbPool, project_id: &str, filter: &LogFilter) -> AppResult<Vec<LogEntry>> {
    let conn = pool.get()?;
    let level_str = filter.level.map(level_to_str);
    let limit = filter.limit.unwrap_or(200) as i64;

    let mut stmt = conn.prepare(
        "SELECT id, project_id, service, level, message, created_at
         FROM log_entries
         WHERE project_id = ?1
           AND (?2 IS NULL OR service = ?2)
           AND (?3 IS NULL OR level = ?3)
           AND (?4 IS NULL OR message LIKE '%' || ?4 || '%')
         ORDER BY id DESC
         LIMIT ?5",
    )?;

    let mut rows = stmt
        .query_map(params![project_id, filter.service, level_str, filter.search, limit], |row| {
            Ok(LogEntry {
                id: row.get(0)?,
                project_id: row.get(1)?,
                service: row.get(2)?,
                level: str_to_level(&row.get::<_, String>(3)?),
                message: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    rows.reverse(); // oldest first, matching reading order in the UI
    Ok(rows)
}

fn level_to_str(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Info => "info",
        LogLevel::Warn => "warn",
        LogLevel::Error => "error",
    }
}

fn str_to_level(s: &str) -> LogLevel {
    match s {
        "warn" => LogLevel::Warn,
        "error" => LogLevel::Error,
        _ => LogLevel::Info,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn pool() -> DbPool {
        let dir = tempfile::tempdir().unwrap();
        let pool = db::init_pool(dir.path()).unwrap();
        // log_entries.project_id has a foreign key into projects, so a row
        // needs to exist there first.
        pool.get()
            .unwrap()
            .execute(
                "INSERT INTO projects (id, name, path, created_at, updated_at) VALUES ('proj', 'proj', '/tmp/proj', '', '')",
                [],
            )
            .unwrap();
        pool
    }

    #[test]
    fn record_and_query_round_trip() {
        let pool = pool();
        record(&pool, "proj", "api", LogLevel::Info, "GET /users 200").unwrap();
        record(&pool, "proj", "worker", LogLevel::Error, "job failed").unwrap();

        let all = query(&pool, "proj", &LogFilter::default()).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].message, "GET /users 200");
    }

    #[test]
    fn query_filters_by_service_and_level() {
        let pool = pool();
        record(&pool, "proj", "api", LogLevel::Info, "ok").unwrap();
        record(&pool, "proj", "worker", LogLevel::Error, "boom").unwrap();

        let filtered = query(
            &pool,
            "proj",
            &LogFilter { service: Some("worker".into()), ..Default::default() },
        )
        .unwrap();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].message, "boom");

        let errors_only = query(
            &pool,
            "proj",
            &LogFilter { level: Some(LogLevel::Error), ..Default::default() },
        )
        .unwrap();
        assert_eq!(errors_only.len(), 1);
    }

    #[test]
    fn query_filters_by_search_text() {
        let pool = pool();
        record(&pool, "proj", "api", LogLevel::Info, "GET /users 200").unwrap();
        record(&pool, "proj", "api", LogLevel::Info, "GET /projects 500").unwrap();

        let matches = query(
            &pool,
            "proj",
            &LogFilter { search: Some("500".into()), ..Default::default() },
        )
        .unwrap();

        assert_eq!(matches.len(), 1);
        assert!(matches[0].message.contains("500"));
    }

    #[test]
    fn infer_level_flags_stderr_error_markers() {
        assert_eq!(infer_level("stderr", "Unhandled exception at line 4"), LogLevel::Error);
        assert_eq!(infer_level("stderr", "deprecation warning: foo"), LogLevel::Warn);
        assert_eq!(infer_level("stdout", "server listening on :3000"), LogLevel::Info);
    }
}
