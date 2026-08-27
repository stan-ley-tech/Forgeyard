use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("connection pool error: {0}")]
    Pool(#[from] r2d2::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

// Tauri serializes command `Err` values back to the frontend, so the error
// needs to carry a plain message rather than its Rust representation.
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
