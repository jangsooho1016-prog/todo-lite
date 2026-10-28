use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("title is empty")]
    EmptyTitle,
    #[error("task not found: {0}")]
    NotFound(Uuid),
}

pub type Result<T> = std::result::Result<T, Error>;
