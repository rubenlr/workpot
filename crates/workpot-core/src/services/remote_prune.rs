//! `prune_removed`: safe-only deletion of local clones whose upstream repo no longer exists.
//!
//! A clone is deleted only when **all** hold; otherwise it is kept and shown as
//! "removed upstream" in the UI:
//! - the repo is absent from a complete listing **and** a direct per-repo lookup returns 404
//! - the clone is a plain (non-bare) checkout under the source's `clone_root` with no linked
//!   worktrees
//! - no tracked changes, no untracked files, no stash, and no merge/rebase/cherry-pick/revert in
//!   progress or conflicted files
//! - every local branch has an upstream and nothing ahead of it
//! - HEAD is not detached at a commit missing from every remote-tracking ref, and no local tag
//!   points at such a commit
//! - the repo has no submodules (conservative: `.gitmodules` or a non-empty `.git/modules`)
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
    let worktrees = git::list_worktrees(canonical)?;
    if !worktrees.paths.is_empty() {
        return Ok(Some("has linked worktrees".into()));
    }
    if worktrees.unreadable > 0 {
        log::warn!(
            "prune: {} linked worktree entr(ies) of {} could not be read; keeping",
            worktrees.unreadable,
            canonical.display()
        );
        return Ok(Some("could not read linked worktree entries".into()));
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
    match git::unfinished_operation_or_conflicts(&repo) {
        Ok(None) => {}
        Ok(Some(reason)) => return Ok(Some(reason)),
        Err(e) => {
            log::warn!(
                "prune: operation/conflict check failed for {}: {e}; keeping",
                canonical.display()
            );
            return Ok(Some(
                "could not verify no merge/rebase is in progress".into(),
            ));
        }
    }
    drop(repo);
    if git::has_stash(canonical)? {
        return Ok(Some("stash entries".into()));
    }
    if let Some(reason) = branch_sync_blocker(canonical)? {
        return Ok(Some(reason));
    }
    Ok(unpushed_refs_blocker(canonical).or_else(|| submodule_blocker(canonical)))
}

fn branch_sync_blocker(canonical: &Path) -> Result<Option<String>> {
    Ok(match git::check_all_branches_synced(canonical)? {
        None => None,
        Some(git::SyncBlocker::NoUpstream { branch }) => {
            Some(format!("branch {branch} has no upstream"))
        }
        Some(git::SyncBlocker::UnpushedCommits { branch, count }) => {
            Some(format!("branch {branch} has {count} unpushed commit(s)"))
        }
        Some(git::SyncBlocker::NonUtf8BranchName) => Some("non-UTF-8 branch name".into()),
    })
}

/// Detached-HEAD commits and local tags not contained in any remote-tracking ref. Failing to
/// determine either keeps the clone.
fn unpushed_refs_blocker(canonical: &Path) -> Option<String> {
    match git::has_unpushed_detached_head(canonical) {
        Ok(true) => return Some("detached HEAD at a commit not on any remote branch".into()),
        Ok(false) => {}
        Err(e) => {
            log::warn!(
                "prune: detached-HEAD check failed for {}: {e}; keeping",
                canonical.display()
            );
            return Some("could not verify detached HEAD is pushed".into());
        }
    }
    match git::find_unpushed_tag(canonical) {
        Ok(None) => None,
        Ok(Some(tag)) => Some(format!("local tag {tag} is not on any remote branch")),
        Err(e) => {
            log::warn!(
                "prune: tag check failed for {}: {e}; keeping",
                canonical.display()
            );
            Some("could not verify local tags are pushed".into())
        }
    }
}

fn submodule_blocker(canonical: &Path) -> Option<String> {
    match git::has_submodules(canonical) {
        Ok(false) => None,
        Ok(true) => Some("has submodules".into()),
        Err(e) => {
            log::warn!(
                "prune: submodule check failed for {}: {e}; keeping",
                canonical.display()
            );
            Some("could not verify submodules".into())
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::git_cmd;
    use std::path::PathBuf;

    fn git(dir: &Path, args: &[&str]) {
        let output = git_cmd()
            .args(["-c", "user.name=T", "-c", "user.email=t@example.com"])
            .args(args)
            .current_dir(dir)
            .output()
            .expect("git");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// Canonical temp dir (macOS `/var` is a symlink) with a `root` subdirectory.
    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path().canonicalize().expect("canonicalize");
        let root = base.join("root");
        std::fs::create_dir_all(&root).expect("root");
        (dir, base, root)
    }

    fn init_repo(path: &Path) {
        std::fs::create_dir_all(path).expect("repo dir");
        git(path, &["init", "-q", "-b", "main"]);
        git(path, &["commit", "--allow-empty", "-q", "-m", "seed"]);
    }

    fn reason(canonical: &Path, root: &Path) -> Option<String> {
        placement_blocker(canonical, root).expect("placement")
    }

    #[test]
    fn placement_accepts_a_standalone_clone_under_the_root() {
        let (_dir, _base, root) = fixture();
        let repo = root.join("web");
        init_repo(&repo);
        assert_eq!(reason(&repo, &root), None);
    }

    #[test]
    fn placement_rejects_the_root_itself_and_paths_outside_it() {
        let (_dir, base, root) = fixture();
        init_repo(&root);
        let elsewhere = base.join("elsewhere");
        init_repo(&elsewhere);
        // A sibling whose name merely starts with the root's name is outside, too.
        let sibling = base.join("root-sibling");
        init_repo(&sibling);

        for outside in [&root, &elsewhere, &sibling] {
            assert_eq!(
                reason(outside, &root).as_deref(),
                Some("outside the source clone_root"),
                "{}",
                outside.display()
            );
        }
    }

    #[test]
    fn placement_rejects_bare_repos_and_plain_directories() {
        let (_dir, _base, root) = fixture();
        let bare = root.join("web.git");
        std::fs::create_dir_all(&bare).expect("bare dir");
        git(&bare, &["init", "--bare", "-q"]);
        let plain = root.join("not-a-repo");
        std::fs::create_dir_all(&plain).expect("plain dir");

        for path in [&bare, &plain] {
            assert_eq!(
                reason(path, &root).as_deref(),
                Some("not a plain checkout (bare layout)"),
                "{}",
                path.display()
            );
        }
    }

    #[test]
    fn placement_rejects_a_linked_worktree_checkout() {
        let (_dir, base, root) = fixture();
        let main = base.join("main-repo");
        init_repo(&main);
        let linked = root.join("wt");
        git(
            &main,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "feat",
                &linked.display().to_string(),
            ],
        );
        assert!(linked.join(".git").is_file());
        assert_eq!(
            reason(&linked, &root).as_deref(),
            Some("linked worktree, not a standalone clone")
        );
    }

    #[test]
    fn placement_rejects_a_clone_that_has_linked_worktrees() {
        let (_dir, base, root) = fixture();
        let repo = root.join("web");
        init_repo(&repo);
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "feat",
                &base.join("wt").display().to_string(),
            ],
        );
        assert_eq!(
            reason(&repo, &root).as_deref(),
            Some("has linked worktrees")
        );
    }

    #[test]
    fn deletion_blocker_reports_unreadable_paths_instead_of_deleting() {
        let (_dir, base, root) = fixture();
        let err = deletion_blocker(&base.join("missing"), &root).expect_err("missing path");
        assert!(matches!(err, WorkpotError::InvalidPath(_)), "{err:?}");
    }
}
