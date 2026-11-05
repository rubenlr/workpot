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
use crate::infra::remote_provider::{GhGlabProvider, RemoteApi};
use crate::infra::store;
use crate::services::{
    catalog, excludes, git_state, local_catalog_sync, org, remote_catalog, remotes, repo_convert,
    roots, sync,
};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

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
    remote_api: Arc<dyn RemoteApi>,
}

/// Back-compat alias; prefer [`AppState`].
pub type AppContext = AppState;

impl AppState {
    /// Lazy bootstrap using macOS default paths.
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
            remote_api: Arc::new(GhGlabProvider::system()),
        })
    }

    /// Replace the remote provider (tests inject a fake instead of spawning `gh` / `glab`).
    pub fn with_remote_api(mut self, api: Arc<dyn RemoteApi>) -> Self {
        self.remote_api = api;
        self
    }

    pub fn remote_api(&self) -> &Arc<dyn RemoteApi> {
        &self.remote_api
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
        let config = self.config()?;
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

    pub fn catalog_launch_path(&self, path: &Path) -> Result<PathBuf> {
        self.db
            .with_read(|conn| catalog::catalog_launch_path(conn, path))
    }

    pub fn remove_repo(&self, path: &Path) -> Result<()> {
        let mut config = self.config_mut()?;
        self.db.with_write(|conn| {
            catalog::remove_repo_with_exclude(conn, &self.config_path, &mut config, path)
        })
    }

    pub fn excludes_list(&self) -> Result<Vec<String>> {
        Ok(excludes::list_excludes(&*self.config()?))
    }

    pub fn excludes_remove(&self, glob: &str) -> Result<()> {
        let mut config = self.config_mut()?;
        excludes::remove_exclude(&self.config_path, &mut config, glob)
    }

    /// Full sync: remote catalog index, then local catalog sync. Returns the local summary;
    /// use [`Self::run_sync_full`] for the remote counts too. Always lists remotes.
    pub fn run_sync(&self) -> Result<local_catalog_sync::LocalCatalogSyncSummary> {
        self.run_sync_full(true).map(|s| s.local)
    }

    /// Full sync with remote + local summaries. `force_remote = false` honours
    /// `remote_sync_ttl_minutes` (used by tray panel open).
    pub fn run_sync_full(&self, force_remote: bool) -> Result<sync::FullSyncSummary> {
        let config = self.config()?;
        sync::run_full(&self.db, &config, self.remote_api.as_ref(), force_remote)
    }

    /// Remote catalog pass only (`workpot sync remote`).
    pub fn run_remote_catalog_sync(
        &self,
        force: bool,
    ) -> Result<crate::services::remote_catalog_sync::RemoteSyncSummary> {
        let config = self.config()?;
        Ok(sync::run_remote(
            &self.db,
            &config,
            self.remote_api.as_ref(),
            force,
        ))
    }

    /// Indexed remote repos that are not cloned (present, configured source).
    pub fn list_remote_only(&self) -> Result<Vec<remote_catalog::RemoteRepoRecord>> {
        let config = self.config()?;
        self.db
            .with_read(|conn| remote_catalog::list_remote_only(conn, &config))
    }

    /// Local clone paths whose upstream repo no longer exists.
    pub fn removed_upstream_paths(&self) -> Result<std::collections::HashSet<String>> {
        self.db.with_read(remote_catalog::removed_upstream_paths)
    }

    pub fn remote_source_statuses(&self) -> Result<Vec<remote_catalog::RemoteSourceStatus>> {
        let config = self.config()?;
        self.db
            .with_read(|conn| remote_catalog::source_statuses(conn, &config))
    }

    /// Resolve a user identifier (planned clone path, `source:full_path`, full path, or unique
    /// repo name) to an indexed remote repo that is **not cloned**.
    pub fn resolve_remote_only(
        &self,
        identifier: &str,
    ) -> Result<Option<remote_catalog::RemoteRepoRecord>> {
        self.db.with_read(|conn| {
            match remote_catalog::resolve_remote_identifier(conn, identifier)? {
                Some(record) if !remote_catalog::is_cloned(conn, &record)? => Ok(Some(record)),
                _ => Ok(None),
            }
        })
    }

    /// Remote-only (indexed, not cloned) record whose planned clone path is `path`.
    pub(crate) fn remote_only_record(
        &self,
        path: &Path,
    ) -> Result<Option<remote_catalog::RemoteRepoRecord>> {
        self.db.with_read(
            |conn| match remote_catalog::find_by_clone_path(conn, path)? {
                Some(record) if !remote_catalog::is_cloned(conn, &record)? => Ok(Some(record)),
                _ => Ok(None),
            },
        )
    }

    /// Clone an indexed remote repo (`identifier`: planned clone path, `source:full_path`,
    /// full path, or unique repo name). Allowed under every clone policy.
    pub fn clone_remote(
        &self,
        identifier: &str,
    ) -> Result<crate::services::remote_clone::CloneOutcome> {
        let config = self.config()?;
        crate::services::remote_clone::clone_by_identifier(
            &self.db,
            &config,
            self.remote_api.as_ref(),
            identifier,
        )
    }

    pub fn remotes_add(&self, source: crate::domain::config::RemoteSource) -> Result<()> {
        remotes::add_remote(self, source)
    }

    pub fn remotes_set(&self, name: &str, patch: remotes::RemotePatch) -> Result<()> {
        remotes::set_remote(self, name, patch)
    }

    pub fn remotes_remove(&self, name: &str) -> Result<()> {
        remotes::remove_remote(self, name)
    }

    /// Local catalog sync (watch-root rescan + git refresh).
    pub fn run_local_catalog_sync(&self) -> Result<local_catalog_sync::LocalCatalogSyncSummary> {
        let config = self.config()?;
        local_catalog_sync::run_phased(&self.db, &config)
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

    /// Refresh git state for a single repository. Public API for the tray.
    ///
    /// Read-only: does not persist to SQLite. Use [`Self::refresh_and_persist_git_state`]
    /// when the DB row must be updated (clears stale `git_state_error` on success).
    pub fn refresh_git_state(&self, path: &Path) -> Result<GitState> {
        git_state::refresh_git_state(path)
    }

    /// Refresh git state for a single repository and persist the result to SQLite.
    pub fn refresh_and_persist_git_state(&self, path: &Path) -> Result<GitState> {
        self.db
            .with_write(|conn| git_state::refresh_and_persist(conn, path))
    }

    /// Paths of non-excluded repos for batch git refresh (read connection only).
    pub fn git_refresh_paths(&self) -> Result<Vec<PathBuf>> {
        self.db
            .with_read(local_catalog_sync::local_catalog_git_paths)
    }

    /// Persist batch git refresh results and return summary (`any_dirty` from DB).
    pub fn persist_git_refresh_results(
        &self,
        git_results: Vec<git_state::GitRefreshResult>,
    ) -> Result<GitRefreshSummary> {
        let mut refreshed = 0u32;
        let mut errors = 0u32;

        self.db.with_write(|conn| {
            let tx = conn.unchecked_transaction()?;
            for r in &git_results {
                let path_exists = Path::new(&r.path).exists();
                match (r.state.error.is_some(), path_exists) {
                    (false, _) => refreshed += 1,
                    (true, true) => errors += 1,
                    (true, false) => {}
                }
                if path_exists {
                    persist_refresh_result(&tx, r)?;
                }
            }
            catalog::prune_missing_repos(&tx)?;
            let config = self.config()?;
            repo_convert::persist_all_structural_preflight(&tx, &config)?;
            tx.commit()?;
            Ok(())
        })?;

        let any_dirty: bool = self.db.with_read(|conn| {
            conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM locations WHERE excluded = 0 AND is_dirty = 1)",
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
        let launch_path = self.catalog_launch_path(catalog_path)?;
        crate::services::branch_checkout::checkout_repo_branch(&launch_path, branch)?;
        self.db.with_write(|conn| {
            git_state::refresh_and_persist_catalog_entry(conn, catalog_path, &launch_path)
                .map(|_| ())
        })
    }

    /// Refresh git state for all non-excluded repos (rayon batch, then single tx persist).
    pub fn refresh_all_git_state(&self) -> Result<GitRefreshSummary> {
        let paths = self.git_refresh_paths()?;
        let fetch_cmd = self.config()?.fetch.clone();
        let git_results = git_state::refresh_all(paths, &fetch_cmd);
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
        let max_pinned = self.config()?.max_pinned;
        self.db
            .with_write(|conn| org::set_pin(conn, path, pinned, max_pinned))
    }

    pub fn set_pin_order(&self, items: &[(&str, i64)]) -> Result<()> {
        self.db.with_write(|conn| org::set_pin_order(conn, items))
    }

    pub fn list_hidden_branches(&self, path: &str) -> Result<Vec<String>> {
        self.db
            .with_read(|conn| org::list_hidden_branches(conn, path))
    }

    /// Persisted branch refs for a catalog location. Empty means never synced — tray falls back to git2.
    pub fn list_location_branches(&self, path: &str) -> Result<Vec<crate::infra::git::BranchRef>> {
        self.db
            .with_read(|conn| catalog::list_location_branches(conn, path))
    }

    pub fn set_branch_hidden(&self, path: &str, branch: &str, hidden: bool) -> Result<()> {
        self.db
            .with_write(|conn| org::set_branch_hidden(conn, path, branch, hidden))
    }

    pub fn convert_repo(
        &self,
        path: &Path,
        target: repo_convert::ConvertTarget,
        dry_run: bool,
    ) -> Result<repo_convert::ConvertResult> {
        let config = self.config()?;
        self.db
            .with_write(|conn| repo_convert::convert_repo(conn, &config, path, target, dry_run))
    }
}

/// Persist one batch-refresh result: a hard failure only records the error, anything else
/// stores the full git state.
fn persist_refresh_result(conn: &Connection, r: &git_state::GitRefreshResult) -> Result<()> {
    if git_state::is_hard_refresh_failure(&r.state) {
        let err = r.state.error.as_deref().unwrap_or("unknown");
        git_state::persist_git_state_error_only(conn, &r.path, err)?;
    } else {
        git_state::persist_git_state(conn, &r.path, &r.state)?;
    }
    Ok(())
}

/// Sibling temp file `write_atomic` stages `path` in (also what crash cleanup removes).
fn config_temp_path(path: &Path) -> PathBuf {
    path.with_extension("tmp")
}

/// Drop orphaned `config.toml.tmp` left by a crash between write and rename.
fn remove_stale_config_temp(path: &Path) {
    let tmp = config_temp_path(path);
    if tmp.is_file()
        && let Err(e) = fs::remove_file(&tmp)
    {
        log::warn!("could not remove stale config temp {}: {e}", tmp.display());
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
    parse_config(&raw)?;
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
    parse_config(&fs::read_to_string(path)?)
}

/// Parse `config.toml` text and validate the result.
fn parse_config(raw: &str) -> Result<Config> {
    let config: Config = toml::from_str(raw).map_err(|e| WorkpotError::Config(e.to_string()))?;
    config.validate().map_err(WorkpotError::Config)?;
    Ok(config)
}

/// Persist config to disk, preserving inline documentation comments.
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
    let tmp = config_temp_path(path);
    fs::write(&tmp, contents)?;
    #[cfg(unix)]
    {
        let file = std::fs::OpenOptions::new().write(true).open(&tmp)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}
