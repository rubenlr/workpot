use crate::error::{Result, WorkpotError};
use directories::BaseDirs;
use std::path::PathBuf;

pub fn config_file() -> Result<PathBuf> {
    let base = BaseDirs::new().ok_or(WorkpotError::PathsUnavailable)?;
    #[cfg(target_os = "macos")]
    let config_root = base.home_dir().join(".config");
    #[cfg(not(target_os = "macos"))]
    let config_root = base.config_dir().to_path_buf();
    Ok(config_root.join("workpot").join("config.toml"))
}

pub fn database_file() -> Result<PathBuf> {
    BaseDirs::new()
        .map(|b| b.data_dir().join("workpot").join("workpot.db"))
        .ok_or(WorkpotError::PathsUnavailable)
}
