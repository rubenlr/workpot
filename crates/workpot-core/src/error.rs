use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkpotError {
    #[error("config error: {0}")]
    Config(String),

    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("migration error: {0}")]
    Migration(#[from] rusqlite_migration::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("could not resolve platform config/data directories")]
    PathsUnavailable,

    #[error("path is not a git repository: {0}")]
    NotGitRepo(PathBuf),

    #[error("repository already registered: {0}")]
    AlreadyRegistered(String),

    #[error("repository not found: {0}")]
    NotFound(String),

    #[error("invalid path: {0}")]
    InvalidPath(String),
}

pub type Result<T> = std::result::Result<T, WorkpotError>;
