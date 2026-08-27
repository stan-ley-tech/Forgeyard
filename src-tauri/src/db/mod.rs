mod schema;

use std::path::Path;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::error::AppResult;

pub type DbPool = Pool<SqliteConnectionManager>;

/// Opens (creating if needed) the app's SQLite database under the given
/// app-data directory and applies the schema. Safe to call on every launch:
/// every statement is `IF NOT EXISTS`.
pub fn init_pool(app_data_dir: &Path) -> AppResult<DbPool> {
    std::fs::create_dir_all(app_data_dir)?;
    let db_path = app_data_dir.join("forgeyard.db");

    let manager = SqliteConnectionManager::file(db_path).with_init(|conn| {
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
    });
    let pool = Pool::builder().max_size(4).build(manager)?;

    let conn = pool.get()?;
    conn.execute_batch(schema::SCHEMA)?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_pool_creates_database_and_tables() {
        let dir = tempfile::tempdir().unwrap();
        let pool = init_pool(dir.path()).expect("pool should initialize");
        let conn = pool.get().unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'projects'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
