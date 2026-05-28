//! Workpot shared core — persistence, catalog, and path resolution.

pub mod domain;
pub mod error;
pub mod infra;
pub mod services;

use crate::domain::{Config, RepoRecord};
use crate::error::Result;
use crate::infra::paths;
use crate::infra::store;
use crate::services::catalog;
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

pub use crate::error::WorkpotError;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Application context: config + SQLite connection. Open via [`AppContext::open`] in production.
pub struct AppContext {
    config_path: PathBuf,
    db_path: PathBuf,
    config: Config,
    conn: Connection,
}

impl AppContext {
    /// Lazy bootstrap using macOS default paths (D-01, D-02, D-04).
    pub fn open() -> Result<Self> {
        let config_path = paths::config_file()?;
        let db_path = paths::database_file()?;
        Self::open_with_paths(config_path, db_path)
    }

    /// Open with explicit paths — intended for integration tests; production CLI uses [`Self::open`].
    pub fn open_with_paths(config_path: PathBuf, db_path: PathBuf) -> Result<Self> {
        ensure_default_config(&config_path)?;
        let config = load_config(&config_path)?;
        let conn = store::open_connection(&db_path)?;
        Ok(Self {
            config_path,
            db_path,
            config,
            conn,
        })
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn database_path(&self) -> &Path {
        &self.db_path
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn register_manual(&self, path: &Path) -> Result<RepoRecord> {
        catalog::register_manual(&self.conn, path)
    }

    pub fn list_repos(&self) -> Result<Vec<RepoRecord>> {
        catalog::list_repos(&self.conn)
    }

    pub fn remove_repo(&self, path: &Path) -> Result<()> {
        catalog::remove_repo(&self.conn, path)
    }
}

fn ensure_default_config(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() {
        return Ok(());
    }
    let default = Config::default();
    let contents = toml::to_string_pretty(&default)
        .map_err(|e| crate::error::WorkpotError::Config(e.to_string()))?;
    fs::write(path, contents)?;
    Ok(())
}

fn load_config(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }
    let contents = fs::read_to_string(path)?;
    toml::from_str(&contents).map_err(|e| WorkpotError::Config(e.to_string()))
}
