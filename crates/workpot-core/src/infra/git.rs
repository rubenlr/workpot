use crate::domain::GitState;
use crate::error::{Result, WorkpotError};
use git2::{ErrorCode, Repository, Status, StatusOptions};
use std::path::{Path, PathBuf};

/// Canonical absolute path to the shared git directory (D-05).
pub fn resolve_git_common_dir(path: &Path) -> Result<PathBuf> {
    // Canonicalize first (T-03-01: path traversal mitigation)
    let canonical = path
        .canonicalize()
        .map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))?;

    let repo = Repository::open(&canonical)
        .map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))?;

    let common = repo.commondir();

    // commondir() may return a relative path — join to canonical if needed
    let resolved = if common.is_absolute() {
        common.to_path_buf()
    } else {
        canonical.join(common)
    };

    std::fs::canonicalize(&resolved).map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))
}

/// Linked worktree paths for a bare repository (D-04). Omits the bare worktree entry itself.
pub fn list_worktree_paths(repo_path: &Path) -> Result<Vec<PathBuf>> {
    let repo = Repository::open(repo_path)
        .map_err(|_| WorkpotError::GitUnavailable(repo_path.to_path_buf()))?;

    let names = repo
        .worktrees()
        .map_err(|_| WorkpotError::GitUnavailable(repo_path.to_path_buf()))?;

    let mut paths = Vec::new();
    for item in names.iter() {
        // iter yields Result<Option<&str>, Error>; skip errors and None (non-UTF8)
        let Some(name) = item.ok().flatten() else {
            continue;
        };
        let Ok(wt) = repo.find_worktree(name) else {
            continue;
        };
        let wt_path = wt.path();
        match std::fs::canonicalize(wt_path) {
            Ok(canon) => {
                paths.push(canon);
            }
            Err(e) => {
                eprintln!("warning: skip worktree {}: {e}", wt_path.display());
            }
        }
    }

    Ok(paths)
}

/// Open a repository at `path` and query branch, dirty flag, and ahead/behind.
///
/// `path` must already be absolute and canonical (see `refresh_git_state`, T-03-01 / T-03-04).
/// All git2 errors map to WorkpotError::GitUnavailable — git2::Error is never exposed.
pub fn open_and_query(path: &Path) -> Result<GitState> {
    debug_assert!(
        path.is_absolute(),
        "open_and_query requires an absolute canonical path"
    );

    let repo = Repository::open(path)
        .map_err(|e| WorkpotError::GitUnavailable(format!("{}: {e}", path.display()).into()))?;

    // D-13: bare repos skip dirty check; return branch if readable, is_dirty=None
    if repo.is_bare() {
        let branch = head_name(&repo).ok();
        return Ok(GitState {
            branch,
            is_dirty: None,
            ahead: None,
            behind: None,
            error: None,
        });
    }

    let branch = head_name(&repo).ok();

    let is_dirty = match detect_dirty(&repo) {
        Ok(dirty) => Some(dirty),
        Err(e) => {
            return Ok(GitState {
                branch,
                is_dirty: None,
                ahead: None,
                behind: None,
                error: Some(e.to_string()),
            });
        }
    };

    let (ahead, behind) = detect_ahead_behind(&repo).unwrap_or((None, None));

    Ok(GitState {
        branch,
        is_dirty,
        ahead,
        behind,
        error: None,
    })
}

/// Return the branch name (shorthand) or a 7-char OID for detached HEAD.
/// Returns "unborn" if HEAD points to an unborn branch (empty repo, no commits).
fn head_name(repo: &Repository) -> std::result::Result<String, git2::Error> {
    let head = match repo.head() {
        Ok(h) => h,
        Err(ref e) if e.code() == ErrorCode::UnbornBranch => {
            return Ok("unborn".to_string());
        }
        Err(e) => return Err(e),
    };

    if head.is_branch() {
        Ok(head.shorthand().unwrap_or("HEAD").to_string())
    } else {
        // Detached HEAD: store first 7 chars of OID hex (D-01)
        let oid = head
            .target()
            .ok_or_else(|| git2::Error::from_str("no HEAD target"))?;
        let hex = oid.to_string();
        Ok(hex[..7.min(hex.len())].to_string())
    }
}

/// Return true if the repo has staged or unstaged changes to tracked files.
/// Untracked files, ignored files, and submodule changes are excluded (D-10, D-11, D-12).
fn detect_dirty(repo: &Repository) -> std::result::Result<bool, git2::Error> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(false)
        .recurse_untracked_dirs(false)
        .exclude_submodules(true);

    let statuses = repo.statuses(Some(&mut opts))?;

    let dirty_flags = Status::INDEX_NEW
        | Status::INDEX_MODIFIED
        | Status::INDEX_DELETED
        | Status::INDEX_RENAMED
        | Status::INDEX_TYPECHANGE
        | Status::WT_MODIFIED
        | Status::WT_DELETED
        | Status::WT_RENAMED
        | Status::WT_TYPECHANGE;

    Ok(statuses.iter().any(|e| e.status().intersects(dirty_flags)))
}

/// Return (ahead, behind) counts relative to the configured upstream branch.
/// Returns (None, None) if HEAD is detached or no upstream is configured (D-04).
fn detect_ahead_behind(
    repo: &Repository,
) -> std::result::Result<(Option<i64>, Option<i64>), git2::Error> {
    let head = repo.head()?;
    if !head.is_branch() {
        return Ok((None, None)); // detached HEAD has no upstream
    }

    let branch_name = head.shorthand().unwrap_or("");
    let branch = repo.find_branch(branch_name, git2::BranchType::Local)?;

    let upstream = match branch.upstream() {
        Ok(u) => u,
        Err(_) => return Ok((None, None)), // no upstream configured (D-04)
    };

    let local_oid = head
        .target()
        .ok_or_else(|| git2::Error::from_str("no local OID"))?;
    let upstream_oid = upstream
        .get()
        .target()
        .ok_or_else(|| git2::Error::from_str("no upstream OID"))?;

    let (ahead, behind) = repo.graph_ahead_behind(local_oid, upstream_oid)?;
    Ok((
        Some(i64::try_from(ahead).unwrap_or(i64::MAX)),
        Some(i64::try_from(behind).unwrap_or(i64::MAX)),
    ))
}
