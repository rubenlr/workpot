use crate::error::{Result, WorkpotError};
use directories::BaseDirs;
use std::path::PathBuf;

pub fn config_file() -> Result<PathBuf> {
    BaseDirs::new()
        .map(|b| b.config_dir().join("workpot").join("config.toml"))
        .ok_or(WorkpotError::PathsUnavailable)
}

pub fn database_file() -> Result<PathBuf> {
    BaseDirs::new()
        .map(|b| b.data_dir().join("workpot").join("workpot.db"))
        .ok_or(WorkpotError::PathsUnavailable)
}
