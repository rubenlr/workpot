use crate::domain::Config;
use crate::error::{Result, WorkpotError};
use crate::save_config;
use std::path::Path;

pub fn list_excludes(config: &Config) -> Vec<String> {
    config.excludes.clone()
}

pub fn remove_exclude(config_path: &Path, config: &mut Config, glob: &str) -> Result<()> {
    let before = config.excludes.len();
    config.excludes.retain(|e| e != glob);
    if config.excludes.len() == before {
        return Err(WorkpotError::NotFound(glob.to_string()));
    }
    save_config(config_path, config)
}
