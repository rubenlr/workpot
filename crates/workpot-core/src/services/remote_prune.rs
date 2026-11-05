//! `prune_removed`: safe-only deletion of local clones whose upstream repo no longer exists.
//!
//! A clone is deleted only when **all** hold; otherwise it is kept and shown as
//! "removed upstream" in the UI:
//! - the repo is absent from a complete listing **and** a direct per-repo lookup returns 404
//! - the clone is a plain (non-bare) checkout under the source's `clone_root` with no linked
//!   worktrees
//! - no tracked changes, no untracked files, no stash
//! - every local branch has an upstream and nothing ahead of it
//!
//! Git-ignored files (build output, local `.env`) are removed along with the clone.

use crate::domain::config::RemoteSource;
use crate::error::{Result, WorkpotError};
use crate::infra::db::DbPool;
use crate::infra::git;
use crate::infra::remote_provider::RemoteApi;
use crate::services::catalog;
use crate::services::remote_catalog::{self, RemoteRepoRecord};
use crate::services::remote_catalog_store::mark_present;
use git2::Repository;
use rusqlite::params;
use std::path::Path;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PruneReport {
    pub pruned: u32,
    /// `(local path, reason it was kept)`.
    pub kept: Vec<(String, String)>,
}

/// Why `local` may NOT be deleted, or `None` when it is safe.
fn deletion_blocker(local: &Path, clone_root: &Path) -> Result<Option<String>> {
    let canonical = local
        .canonicalize()
        .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", local.display())))?;
    let root = clone_root
        .canonicalize()
        .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", clone_root.display())))?;
    if let Some(reason) = placement_blocker(&canonical, &root)? {
        return Ok(Some(reason));
    }
    repo_state_blocker(&canonical)
}

/// Layout checks: the clone must be a plain, standalone checkout strictly under `root`.
fn placement_blocker(canonical: &Path, root: &Path) -> Result<Option<String>> {
    if canonical == root || !canonical.starts_with(root) {
        return Ok(Some("outside the source clone_root".into()));
    }
    if catalog::is_bare_repo(canonical) || !catalog::is_git_worktree(canonical) {
        return Ok(Some("not a plain checkout (bare layout)".into()));
    }
    if !canonical.join(".git").is_dir() {
        return Ok(Some("linked worktree, not a standalone clone".into()));
    }
    if !git::list_worktree_paths(canonical)?.is_empty() {
        return Ok(Some("has linked worktrees".into()));
    }
    Ok(None)
}

/// Content checks: nothing uncommitted, untracked, stashed, or unpushed would be lost.
fn repo_state_blocker(canonical: &Path) -> Result<Option<String>> {
    let repo = Repository::open(canonical)
        .map_err(|_| WorkpotError::GitUnavailable(canonical.to_path_buf()))?;
    let dirty = git::detect_dirty(&repo).unwrap_or_else(|e| {
        log::warn!(
            "prune: dirty check failed for {}: {e}; treating as dirty",
            canonical.display()
        );
        true
    });
    if dirty {
        return Ok(Some("uncommitted changes".into()));
    }
    let untracked = git::has_untracked(&repo).unwrap_or_else(|e| {
        log::warn!(
            "prune: untracked check failed for {}: {e}; treating as untracked",
            canonical.display()
        );
        true
    });
    if untracked {
        return Ok(Some("untracked files".into()));
    }
    drop(repo);
    if git::has_stash(canonical)? {
        return Ok(Some("stash entries".into()));
    }
    match git::check_all_branches_synced(canonical)? {
        None => Ok(None),
        Some(git::SyncBlocker::NoUpstream { branch }) => {
            Ok(Some(format!("branch {branch} has no upstream")))
        }
        Some(git::SyncBlocker::UnpushedCommits { branch, count }) => Ok(Some(format!(
            "branch {branch} has {count} unpushed commit(s)"
        ))),
        Some(git::SyncBlocker::NonUtf8BranchName) => Ok(Some("non-UTF-8 branch name".into())),
    }
}

/// Prune one source (no-op unless `prune_removed`).
pub fn prune_source(pool: &DbPool, api: &dyn RemoteApi, source: &RemoteSource) -> PruneReport {
    let mut report = PruneReport::default();
    if !source.prune_removed {
        return report;
    }
    let candidates = match pool.with_read(remote_catalog::list_gone_cloned) {
        Ok(rows) => rows,
        Err(e) => {
            log::warn!("prune {}: {e}", source.name);
            return report;
        }
    };
    for (record, local) in candidates
        .into_iter()
        .filter(|(r, _)| r.source_id == source.name)
    {
        match prune_one(pool, api, source, &record, &local) {
            Ok(None) => report.pruned += 1,
            Ok(Some(reason)) => {
                log::info!("prune {}: keeping {local}: {reason}", source.name);
                report.kept.push((local, reason));
            }
            Err(e) => {
                log::warn!("prune {}: keeping {local}: {e}", source.name);
                report.kept.push((local, e.to_string()));
            }
        }
    }
    report
}

/// `Ok(None)` = deleted; `Ok(Some(reason))` = kept.
fn prune_one(
    pool: &DbPool,
    api: &dyn RemoteApi,
    source: &RemoteSource,
    record: &RemoteRepoRecord,
    local: &str,
) -> Result<Option<String>> {
    // Confirm with the host; a transient/API error must never lead to deletion.
    if api.repo_exists(source, &record.full_path)? {
        pool.with_write(|conn| mark_present(conn, &record.source_id, &record.full_path))?;
        return Ok(Some("still exists upstream".into()));
    }

    let local_path = Path::new(local);
    if !local_path.exists() {
        // Already gone from disk; just drop the catalog row.
        delete_location(pool, local)?;
        return Ok(None);
    }
    if let Some(reason) = deletion_blocker(local_path, &source.resolved_clone_root())? {
        return Ok(Some(reason));
    }
    std::fs::remove_dir_all(local_path)?;
    delete_location(pool, local)?;
    log::info!("pruned {local} (removed upstream: {})", record.full_path);
    Ok(None)
}

fn delete_location(pool: &DbPool, local: &str) -> Result<()> {
    pool.with_write(|conn| {
        conn.execute("DELETE FROM locations WHERE path = ?1", params![local])?;
        Ok(())
    })
}
