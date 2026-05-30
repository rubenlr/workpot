//! Workpot shared core — persistence, catalog, and path resolution.

#![deny(clippy::disallowed_methods)]
#![cfg_attr(test, allow(clippy::disallowed_methods))]

pub mod domain;
pub mod error;
pub mod infra;
pub mod services;

use crate::domain::Config;
use crate::error::Result;
use crate::infra::paths;
use crate::infra::store;
use crate::services::{catalog, excludes, index, roots};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

pub use crate::domain::GitState;
pub use crate::domain::RepoRecord;
pub use crate::error::WorkpotError;
pub use crate::services::git_state::GitRefreshSummary;

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
        remove_stale_config_temp(&config_path);
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
        catalog::register_manual(&self.conn, &self.config, path)
    }

    pub fn list_repos(&self) -> Result<Vec<RepoRecord>> {
        catalog::list_repos(&self.conn)
    }

    pub fn touch_last_opened_at(&self, path: &Path) -> Result<()> {
        catalog::touch_last_opened_at(&self.conn, path)
    }

    pub fn indexed_launch_path(&self, path: &Path) -> Result<PathBuf> {
        catalog::indexed_launch_path(&self.conn, path)
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

    pub(crate) fn connection(&self) -> &Connection {
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

    /// Refresh git state for a single repository. Public API for Phase 4 tray (D-18).
    ///
    /// Read-only: does not persist to SQLite. Use [`Self::refresh_and_persist_git_state`]
    /// when the DB row must be updated (clears stale `git_state_error` on success).
    pub fn refresh_git_state(
        &self,
        path: &std::path::Path,
    ) -> crate::error::Result<crate::domain::GitState> {
        crate::services::git_state::refresh_git_state(path)
    }

    /// Refresh git state for a single repository and persist the result to SQLite.
    pub fn refresh_and_persist_git_state(
        &self,
        path: &std::path::Path,
    ) -> crate::error::Result<crate::domain::GitState> {
        crate::services::git_state::refresh_and_persist(&self.conn, path)
    }

    /// Paths of non-excluded repos for batch git refresh (short lock in tray layer).
    pub fn git_refresh_paths(&self) -> Result<Vec<PathBuf>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path FROM repos WHERE excluded = 0")?;
        let paths = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .map(PathBuf::from)
            .collect();
        Ok(paths)
    }

    /// Persist batch git refresh results and return summary (`any_dirty` from DB).
    pub fn persist_git_refresh_results(
        &self,
        git_results: Vec<crate::services::git_state::GitRefreshResult>,
    ) -> Result<GitRefreshSummary> {
        let mut refreshed = 0u32;
        let mut errors = 0u32;

        let tx = self.conn.unchecked_transaction()?;
        for r in &git_results {
            if r.state.error.is_some() {
                errors += 1;
            } else {
                refreshed += 1;
            }
            if crate::services::git_state::is_hard_refresh_failure(&r.state) {
                let err = r.state.error.as_deref().unwrap_or("unknown");
                crate::services::git_state::persist_git_state_error_only(&tx, &r.path, err)?;
            } else {
                crate::services::git_state::persist_git_state(&tx, &r.path, &r.state)?;
            }
        }
        tx.commit()?;

        let any_dirty: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM repos WHERE excluded = 0 AND is_dirty = 1)",
            [],
            |row| row.get(0),
        )?;

        Ok(GitRefreshSummary {
            refreshed,
            errors,
            any_dirty,
        })
    }

    /// Refresh git state for all non-excluded repos (rayon batch, then single tx persist).
    pub fn refresh_all_git_state(&self) -> Result<GitRefreshSummary> {
        let paths = self.git_refresh_paths()?;
        let git_results = crate::services::git_state::refresh_all(paths);
        self.persist_git_refresh_results(git_results)
    }
}

/// Drop orphaned `config.toml.tmp` left by a crash between write and rename.
fn remove_stale_config_temp(path: &Path) {
    let tmp = path.with_extension("tmp");
    if tmp.is_file() {
        let _ = fs::remove_file(&tmp);
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
    write_atomic(path, &contents)?;
    Ok(())
}

pub(crate) fn load_config(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }
    let contents = fs::read_to_string(path)?;
    let config: Config =
        toml::from_str(&contents).map_err(|e| WorkpotError::Config(e.to_string()))?;
    config.validate().map_err(WorkpotError::Config)?;
    Ok(config)
}

/// Persist config to disk (D-19).
pub fn save_config(config_path: &Path, config: &Config) -> Result<()> {
    config.validate().map_err(WorkpotError::Config)?;
    let contents =
        toml::to_string_pretty(config).map_err(|e| WorkpotError::Config(e.to_string()))?;
    write_atomic(config_path, &contents)?;
    Ok(())
}

/// Write `contents` to `path` atomically via temp file + fsync + rename.
fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, contents)?;
    #[cfg(unix)]
    {
        let file = std::fs::OpenOptions::new().write(true).open(&tmp)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}
