pub mod model;

use std::path::Path;

use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use crate::db::DbPool;
use crate::error::{AppError, AppResult};

pub use model::ProjectRecord;

/// Registers a project by absolute path. `name` defaults to the directory's
/// file name when not provided.
pub fn add(pool: &DbPool, path: &str, name: Option<String>) -> AppResult<ProjectRecord> {
    let path = Path::new(path);
    let canonical = path
        .canonicalize()
        .map_err(|_| AppError::InvalidPath(path.display().to_string()))?;

    if !canonical.is_dir() {
        return Err(AppError::InvalidPath(canonical.display().to_string()));
    }

    let path_str = normalize(&canonical);
    let name = name.unwrap_or_else(|| {
        canonical
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path_str.clone())
    });

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let conn = pool.get()?;
    let result = conn.execute(
        "INSERT INTO projects (id, name, path, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
        params![id, name, path_str, now],
    );

    match result {
        Ok(_) => Ok(ProjectRecord {
            id,
            name,
            path: path_str,
            created_at: now.clone(),
            updated_at: now,
        }),
        Err(rusqlite::Error::SqliteFailure(err, _))
            if err.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            Err(AppError::DuplicateProject(path_str))
        }
        Err(e) => Err(e.into()),
    }
}

pub fn list(pool: &DbPool) -> AppResult<Vec<ProjectRecord>> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, name, path, created_at, updated_at FROM projects ORDER BY name COLLATE NOCASE",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn find(pool: &DbPool, id: &str) -> AppResult<ProjectRecord> {
    let conn = pool.get()?;
    conn.query_row(
        "SELECT id, name, path, created_at, updated_at FROM projects WHERE id = ?1",
        params![id],
        |row| {
            Ok(ProjectRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| AppError::ProjectNotFound(id.to_string()))
}

pub fn remove(pool: &DbPool, id: &str) -> AppResult<()> {
    let conn = pool.get()?;
    let affected = conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(AppError::ProjectNotFound(id.to_string()));
    }
    Ok(())
}

/// Strips the `\\?\` verbatim prefix Windows adds during canonicalization so
/// stored paths stay readable and match what the user typed.
fn normalize(path: &Path) -> String {
    let raw = path.display().to_string();
    raw.strip_prefix(r"\\?\").unwrap_or(&raw).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn setup() -> (tempfile::TempDir, DbPool) {
        let data_dir = tempfile::tempdir().unwrap();
        let pool = db::init_pool(data_dir.path()).unwrap();
        (data_dir, pool)
    }

    #[test]
    fn add_registers_a_project_with_defaulted_name() {
        let (_data_dir, pool) = setup();
        let project_dir = tempfile::tempdir().unwrap();

        let record = add(&pool, project_dir.path().to_str().unwrap(), None).unwrap();

        assert_eq!(
            record.name,
            project_dir.path().file_name().unwrap().to_string_lossy()
        );
        assert_eq!(list(&pool).unwrap().len(), 1);
    }

    #[test]
    fn add_rejects_duplicate_paths() {
        let (_data_dir, pool) = setup();
        let project_dir = tempfile::tempdir().unwrap();
        let path = project_dir.path().to_str().unwrap();

        add(&pool, path, None).unwrap();
        let second = add(&pool, path, None);

        assert!(matches!(second, Err(AppError::DuplicateProject(_))));
    }

    #[test]
    fn add_rejects_missing_paths() {
        let (_data_dir, pool) = setup();
        let result = add(&pool, "Z:/definitely/not/a/real/path", None);
        assert!(matches!(result, Err(AppError::InvalidPath(_))));
    }

    #[test]
    fn remove_deletes_a_registered_project() {
        let (_data_dir, pool) = setup();
        let project_dir = tempfile::tempdir().unwrap();
        let record = add(&pool, project_dir.path().to_str().unwrap(), None).unwrap();

        remove(&pool, &record.id).unwrap();

        assert!(list(&pool).unwrap().is_empty());
        assert!(matches!(
            find(&pool, &record.id),
            Err(AppError::ProjectNotFound(_))
        ));
    }
}
