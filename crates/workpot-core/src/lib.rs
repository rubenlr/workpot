//! Workpot shared core — persistence, catalog, and path resolution.

pub mod domain;
pub mod error;
pub mod infra;
pub mod services;

use crate::domain::{Config, RepoRecord};
use crate::error::Result;
use crate::infra::paths;
use crate::infra::store;
use crate::services::{catalog, excludes, index, roots};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

pub use crate::error::WorkpotError;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// First-run config: empty `excludes`; `watch_roots` seeded with existing `~/code` and `~/dev`.
pub fn default_config(home: &Path) -> Config {
    let mut config = Config::default();
    for name in ["code", "dev"] {
        let candidate = home.join(name);
        if candidate.is_dir() {
            config.watch_roots.push(candidate);
        }
    }
    config
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

    pub fn remove_repo(&mut self, path: &Path) -> Result<()> {
        catalog::remove_repo_with_exclude(&self.conn, &self.config_path, &mut self.config, path)
    }

    pub fn excludes_list(&self) -> Vec<String> {
        excludes::list_excludes(&self.config)
    }

    pub fn excludes_remove(&mut self, glob: &str) -> Result<()> {
        excludes::remove_exclude(&self.config_path, &mut self.config, glob)
    }

    pub fn run_index(&self) -> Result<index::IndexSummary> {
        index::run_full(&self.conn, &self.config)
    }

    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn reload_config(&mut self) -> Result<()> {
        roots::reload_config(self)
    }

    pub fn roots_add(&mut self, path: &Path) -> Result<()> {
        roots::add_root(self, path)
    }

    pub fn roots_list(&self) -> Vec<PathBuf> {
        roots::list_roots(self)
    }

    pub fn roots_remove(&mut self, path: &Path, skip_prune: bool) -> Result<()> {
        roots::remove_root(self, path, skip_prune)
    }
}

fn ensure_default_config(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() {
        return Ok(());
    }
    let home = directories::BaseDirs::new()
        .map(|b| b.home_dir().to_path_buf())
        .ok_or(WorkpotError::PathsUnavailable)?;
    let default = default_config(&home);
    let contents = toml::to_string_pretty(&default)
        .map_err(|e| crate::error::WorkpotError::Config(e.to_string()))?;
    fs::write(path, contents)?;
    Ok(())
}

pub(crate) fn load_config(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }
    let contents = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&contents).map_err(|e| WorkpotError::Config(e.to_string()))?;
    config
        .validate()
        .map_err(WorkpotError::LimitsExceeded)?;
    Ok(config)
}

/// Persist config to disk (D-19).
pub fn save_config(config_path: &Path, config: &Config) -> Result<()> {
    config
        .validate()
        .map_err(WorkpotError::LimitsExceeded)?;
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = toml::to_string_pretty(config)
        .map_err(|e| WorkpotError::Config(e.to_string()))?;
    fs::write(config_path, contents)?;
    Ok(())
}
