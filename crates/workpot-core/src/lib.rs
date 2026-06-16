//! Workpot shared core — persistence, catalog, and path resolution.

#![deny(clippy::disallowed_methods)]
#![cfg_attr(test, allow(clippy::disallowed_methods))]

pub mod domain;
pub mod error;
pub mod infra;
pub mod services;

#[cfg(test)]
pub mod testing;

use crate::domain::Config;
use crate::infra::config_doc;
use crate::infra::db::DbPool;
use crate::infra::paths;
use crate::infra::store;
use crate::services::{catalog, excludes, index, org, roots};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub use crate::domain::GitState;
pub use crate::domain::RepoRecord;
pub use crate::error::{Result, WorkpotError};
pub use crate::services::git_state::GitRefreshSummary;
pub use crate::services::repo_priority::{
    SectionedRepos, flat_tray_ordered, flat_tray_ordered_repos, section_sort,
};
pub use crate::services::repo_sync::{SyncDirection, SyncFailure, run_repo_sync};
pub use crate::services::stale_dirty::has_stale_dirty;

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

fn lock_poison<T>(_: PoisonError<T>) -> WorkpotError {
    WorkpotError::Config("application state lock poisoned".to_string())
}

/// Application state: config + read/write SQLite pool. Open via [`AppState::open`] in production.
pub struct AppState {
    config_path: PathBuf,
    db_path: PathBuf,
    config: RwLock<Config>,
    db: DbPool,
}

/// Back-compat alias; prefer [`AppState`].
pub type AppContext = AppState;

impl AppState {
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
        let db = store::open_pool(&db_path)?;
        Ok(Self {
            config_path,
            db_path,
            config: RwLock::new(config),
            db,
        })
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn database_path(&self) -> &Path {
        &self.db_path
    }

    pub fn config(&self) -> Result<RwLockReadGuard<'_, Config>> {
        self.config.read().map_err(lock_poison)
    }

    pub fn db(&self) -> &DbPool {
        &self.db
    }

    pub fn register_manual(&self, path: &Path) -> Result<RepoRecord> {
        let config = self.config.read().map_err(lock_poison)?;
        self.db
            .with_write(|conn| catalog::register_manual(conn, &config, path))
    }

    pub fn list_repos(&self) -> Result<Vec<RepoRecord>> {
        self.db.with_read(catalog::list_repos)
    }

    pub fn touch_last_opened_at(&self, path: &Path) -> Result<()> {
        self.db
            .with_write(|conn| catalog::touch_last_opened_at(conn, path))
    }

    pub fn indexed_launch_path(&self, path: &Path) -> Result<PathBuf> {
        self.db
            .with_read(|conn| catalog::indexed_launch_path(conn, path))
    }

    pub fn remove_repo(&self, path: &Path) -> Result<()> {
        let mut config = self.config.write().map_err(lock_poison)?;
        self.db.with_write(|conn| {
            catalog::remove_repo_with_exclude(conn, &self.config_path, &mut config, path)
        })
    }

    pub fn excludes_list(&self) -> Result<Vec<String>> {
        Ok(excludes::list_excludes(&*self.config()?))
    }

    pub fn excludes_remove(&self, glob: &str) -> Result<()> {
        let mut config = self.config.write().map_err(lock_poison)?;
        excludes::remove_exclude(&self.config_path, &mut config, glob)
    }

    pub fn run_index(&self) -> Result<index::IndexSummary> {
        let config = self.config.read().map_err(lock_poison)?;
        index::run_phased(&self.db, &config)
    }

    pub fn run_index_phased(&self) -> Result<index::IndexSummary> {
        self.run_index()
    }

    pub fn config_mut(&self) -> Result<RwLockWriteGuard<'_, Config>> {
        self.config.write().map_err(lock_poison)
    }

    pub(crate) fn with_write_connection<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        self.db.with_write(f)
    }

    pub fn reload_config(&self) -> Result<()> {
        roots::reload_config(self)
    }

    pub fn roots_add(&self, path: &Path) -> Result<()> {
        roots::add_root(self, path)
    }

    pub fn roots_list(&self) -> Result<Vec<PathBuf>> {
        Ok(roots::list_roots(self))
    }

    pub fn roots_remove(&self, path: &Path, skip_prune: bool) -> Result<()> {
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
        self.db
            .with_write(|conn| crate::services::git_state::refresh_and_persist(conn, path))
    }

    /// Paths of non-excluded repos for batch git refresh (read connection only).
    pub fn git_refresh_paths(&self) -> Result<Vec<PathBuf>> {
        self.db.with_read(|conn| {
            let mut stmt = conn.prepare("SELECT path FROM repos WHERE excluded = 0")?;
            let paths = stmt
                .query_map([], |row| row.get::<_, String>(0))?
                .filter_map(|r| r.ok())
                .map(PathBuf::from)
                .collect();
            Ok(paths)
        })
    }

    /// Persist batch git refresh results and return summary (`any_dirty` from DB).
    pub fn persist_git_refresh_results(
        &self,
        git_results: Vec<crate::services::git_state::GitRefreshResult>,
    ) -> Result<GitRefreshSummary> {
        let mut refreshed = 0u32;
        let mut errors = 0u32;

        self.db.with_write(|conn| {
            let tx = conn.unchecked_transaction()?;
            for r in &git_results {
                let path_exists = Path::new(&r.path).exists();
                if r.state.error.is_some() {
                    if path_exists {
                        errors += 1;
                    }
                } else {
                    refreshed += 1;
                }
                if !path_exists {
                    continue;
                }
                if crate::services::git_state::is_hard_refresh_failure(&r.state) {
                    let err = r.state.error.as_deref().unwrap_or("unknown");
                    crate::services::git_state::persist_git_state_error_only(&tx, &r.path, err)?;
                } else {
                    crate::services::git_state::persist_git_state(&tx, &r.path, &r.state)?;
                }
            }
            catalog::prune_missing_repos(&tx)?;
            tx.commit()?;
            Ok(())
        })?;

        let any_dirty: bool = self.db.with_read(|conn| {
            conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM repos WHERE excluded = 0 AND is_dirty = 1)",
                [],
                |row| row.get(0),
            )
            .map_err(WorkpotError::from)
        })?;

        Ok(GitRefreshSummary {
            refreshed,
            errors,
            any_dirty,
        })
    }

    /// Checkout a branch in an indexed repo and persist updated git state.
    pub fn checkout_repo_branch(&self, catalog_path: &Path, branch: &str) -> Result<()> {
        let launch_path = self.indexed_launch_path(catalog_path)?;
        crate::services::branch_checkout::checkout_repo_branch(&launch_path, branch)?;
        self.db.with_write(|conn| {
            crate::services::git_state::refresh_and_persist_catalog_entry(
                conn,
                catalog_path,
                &launch_path,
            )
            .map(|_| ())
        })
    }

    /// Refresh git state for all non-excluded repos (rayon batch, then single tx persist).
    pub fn refresh_all_git_state(&self) -> Result<GitRefreshSummary> {
        let paths = self.git_refresh_paths()?;
        let git_results = crate::services::git_state::refresh_all(paths);
        self.persist_git_refresh_results(git_results)
    }

    pub fn set_tags(&self, path: &str, tags: &[&str]) -> Result<()> {
        self.db.with_write(|conn| org::set_tags(conn, path, tags))
    }

    pub fn add_tag(&self, path: &str, tag: &str) -> Result<()> {
        self.db.with_write(|conn| org::add_tag(conn, path, tag))
    }

    pub fn remove_tag(&self, path: &str, tag: &str) -> Result<()> {
        self.db.with_write(|conn| org::remove_tag(conn, path, tag))
    }

    pub fn list_tags_for_repo(&self, path: &str) -> Result<Vec<String>> {
        self.db
            .with_read(|conn| org::list_tags_for_repo(conn, path))
    }

    pub fn list_all_tags(&self) -> Result<Vec<String>> {
        self.db.with_read(org::list_all_tags)
    }

    pub fn set_notes(&self, path: &str, notes: Option<&str>) -> Result<()> {
        self.db.with_write(|conn| org::set_notes(conn, path, notes))
    }

    pub fn set_alias(&self, repo_path: &str, alias: Option<&str>) -> Result<()> {
        self.db
            .with_write(|conn| org::set_alias(conn, repo_path, alias))
    }

    pub fn set_pin(&self, path: &str, pinned: bool) -> Result<()> {
        let max_pinned = self.config.read().map_err(lock_poison)?.max_pinned;
        self.db
            .with_write(|conn| org::set_pin(conn, path, pinned, max_pinned))
    }

    pub fn set_pin_order(&self, items: &[(&str, i64)]) -> Result<()> {
        self.db.with_write(|conn| org::set_pin_order(conn, items))
    }

    pub fn convert_repo(
        &self,
        path: &Path,
        target: crate::services::repo_convert::ConvertTarget,
        dry_run: bool,
    ) -> Result<crate::services::repo_convert::ConvertResult> {
        let config = self.config.read().map_err(lock_poison)?;
        self.db.with_write(|conn| {
            crate::services::repo_convert::convert_repo(conn, &config, path, target, dry_run)
        })
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
    let contents = config_doc::render_init_config(&default);
    write_atomic(path, &contents)?;
    Ok(())
}

/// Write a documented default `config.toml` (explicit bootstrap / `workpot settings init`).
pub fn init_config_file(path: &Path, home: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        return Err(WorkpotError::Config(
            "config already exists; pass --force to overwrite".to_string(),
        ));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = config_doc::render_init_config(&default_config(home));
    write_atomic(path, &contents)?;
    Ok(())
}

/// Backfill missing documentation comments in an existing config file.
pub fn annotate_config_comments(path: &Path) -> Result<usize> {
    let raw = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&raw).map_err(|e| WorkpotError::Config(e.to_string()))?;
    config.validate().map_err(WorkpotError::Config)?;
    let mut doc = raw
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| WorkpotError::Config(e.to_string()))?;
    let added = config_doc::add_missing_comments(&mut doc);
    config_doc::write_document(path, &doc)?;
    Ok(added)
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

/// Persist config to disk (D-19), preserving inline documentation comments.
pub fn save_config(config_path: &Path, config: &Config) -> Result<()> {
    config.validate().map_err(WorkpotError::Config)?;
    let mut doc = config_doc::load_document(config_path)?;
    config_doc::apply_config_to_document(&mut doc, config);
    config_doc::write_document(config_path, &doc)?;
    Ok(())
}

/// Write `contents` to `path` atomically via temp file + fsync + rename.
pub(crate) fn write_atomic(path: &Path, contents: &str) -> Result<()> {
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
