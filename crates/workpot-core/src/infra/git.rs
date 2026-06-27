use crate::domain::{BRANCH_UNBORN, GitState};
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
                log::warn!("skip worktree {}: {e}", wt_path.display());
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

    let (ahead, behind) = match detect_ahead_behind(&repo) {
        Ok(pair) => pair,
        Err(e) => {
            return Ok(GitState {
                branch,
                is_dirty,
                ahead: None,
                behind: None,
                error: Some(e.to_string()),
            });
        }
    };

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
            return Ok(BRANCH_UNBORN.to_string());
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
pub(crate) fn detect_dirty(repo: &Repository) -> std::result::Result<bool, git2::Error> {
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

/// Return true when the repo has untracked (non-ignored) files in the working tree.
pub(crate) fn has_untracked(repo: &Repository) -> std::result::Result<bool, git2::Error> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .exclude_submodules(true);

    let statuses = repo.statuses(Some(&mut opts))?;
    Ok(statuses
        .iter()
        .any(|e| e.status().intersects(Status::WT_NEW)))
}

/// Return the first path under `repo_path` with untracked files, if any.
pub fn first_untracked_worktree(repo_path: &Path) -> Result<Option<PathBuf>> {
    let paths = if Repository::open(repo_path)
        .map_err(|_| WorkpotError::GitUnavailable(repo_path.to_path_buf()))?
        .is_bare()
    {
        list_worktree_paths(repo_path)?
    } else {
        vec![
            repo_path
                .canonicalize()
                .map_err(|_| WorkpotError::GitUnavailable(repo_path.to_path_buf()))?,
        ]
    };

    for wt_path in paths {
        let repo = Repository::open(&wt_path)
            .map_err(|_| WorkpotError::GitUnavailable(wt_path.clone()))?;
        if has_untracked(&repo).map_err(|_| WorkpotError::GitUnavailable(wt_path.clone()))? {
            return Ok(Some(wt_path));
        }
    }
    Ok(None)
}

/// Return true when HEAD points at a commit rather than a named branch.
pub fn is_detached_head(repo_path: &Path) -> Result<bool> {
    let repo = Repository::open(repo_path)
        .map_err(|_| WorkpotError::GitUnavailable(repo_path.to_path_buf()))?;
    let head = match repo.head() {
        Ok(h) => h,
        Err(ref e) if e.code() == ErrorCode::UnbornBranch => return Ok(false),
        Err(_) => return Err(WorkpotError::GitUnavailable(repo_path.to_path_buf())),
    };
    Ok(!head.is_branch())
}

/// Return (ahead, behind) counts relative to the configured upstream branch.
/// Returns (None, None) if HEAD is detached or no upstream is configured (D-04).
fn detect_ahead_behind(
    repo: &Repository,
) -> std::result::Result<(Option<i64>, Option<i64>), git2::Error> {
    let head = match repo.head() {
        Ok(h) => h,
        Err(ref e) if e.code() == ErrorCode::UnbornBranch => {
            return Ok((None, None));
        }
        Err(e) => return Err(e),
    };
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

/// Branch sync issues discovered during conversion preflight.
#[derive(Debug, PartialEq, Eq)]
pub enum SyncBlocker {
    NoUpstream { branch: String },
    UnpushedCommits { branch: String, count: usize },
    NonUtf8BranchName,
}

/// Return true when the repository has any stash entries.
pub fn has_stash(repo_path: &Path) -> Result<bool> {
    let mut repo = Repository::open(repo_path)
        .map_err(|_| WorkpotError::GitUnavailable(repo_path.to_path_buf()))?;
    let mut found = false;
    repo.stash_foreach(|_i, _msg, _oid| {
        found = true;
        false
    })
    .map_err(|_| WorkpotError::GitUnavailable(repo_path.to_path_buf()))?;
    Ok(found)
}

/// Check every local branch has an upstream with zero commits ahead.
pub fn check_all_branches_synced(repo_path: &Path) -> Result<Option<SyncBlocker>> {
    let repo = Repository::open(repo_path)
        .map_err(|_| WorkpotError::GitUnavailable(repo_path.to_path_buf()))?;

    let branches = repo
        .branches(Some(git2::BranchType::Local))
        .map_err(|_| WorkpotError::GitUnavailable(repo_path.to_path_buf()))?;

    for item in branches {
        let (branch, _) =
            item.map_err(|_| WorkpotError::GitUnavailable(repo_path.to_path_buf()))?;
        let name = match branch.name() {
            Ok(Some(n)) => n.to_string(),
            Ok(None) => return Ok(Some(SyncBlocker::NonUtf8BranchName)),
            Err(_) => return Ok(Some(SyncBlocker::NonUtf8BranchName)),
        };

        let upstream = match branch.upstream() {
            Ok(u) => u,
            Err(_) => {
                return Ok(Some(SyncBlocker::NoUpstream { branch: name }));
            }
        };

        let local_oid = branch
            .get()
            .target()
            .ok_or_else(|| WorkpotError::GitUnavailable(repo_path.to_path_buf()))?;
        let upstream_oid = upstream
            .get()
            .target()
            .ok_or_else(|| WorkpotError::GitUnavailable(repo_path.to_path_buf()))?;

        let (ahead, _behind) = repo
            .graph_ahead_behind(local_oid, upstream_oid)
            .map_err(|_| WorkpotError::GitUnavailable(repo_path.to_path_buf()))?;

        if ahead > 0 {
            return Ok(Some(SyncBlocker::UnpushedCommits {
                branch: name,
                count: ahead,
            }));
        }
    }

    Ok(None)
}

fn detect_default_branch(repo: &Repository) -> std::result::Result<String, git2::Error> {
    if let Ok(head_ref) = repo.head()
        && let Ok(name) = head_ref.shorthand()
    {
        return Ok(name.to_string());
    }

    let branches = repo.branches(Some(git2::BranchType::Local))?;
    let mut names: Vec<String> = Vec::new();
    for item in branches {
        let (branch, _) = item?;
        if let Ok(Some(name)) = branch.name() {
            names.push(name.to_string());
        }
    }
    names.sort();
    names
        .into_iter()
        .next()
        .ok_or_else(|| git2::Error::from_str("no branches found"))
}

/// Snapshot of a git remote's fetch and push URLs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteConfig {
    pub name: String,
    pub url: String,
    pub push_url: Option<String>,
}

/// List configured remotes for a repository.
pub fn list_remotes(path: &Path) -> Result<Vec<RemoteConfig>> {
    let repo =
        Repository::open(path).map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))?;

    let names = repo
        .remotes()
        .map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))?;

    let mut remotes = Vec::new();
    for i in 0..names.len() {
        let Ok(Some(name)) = names.get(i) else {
            continue;
        };
        let remote = repo
            .find_remote(name)
            .map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))?;
        let url = remote
            .url()
            .map(String::from)
            .map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))?;
        let push_url = remote.pushurl().ok().flatten().map(String::from);
        remotes.push(RemoteConfig {
            name: name.to_string(),
            url,
            push_url,
        });
    }
    Ok(remotes)
}

/// Sync target remotes to match `remotes` exactly (add/update/delete as needed).
pub fn apply_remotes(path: &Path, remotes: &[RemoteConfig]) -> Result<()> {
    let repo =
        Repository::open(path).map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))?;

    let current_names: Vec<String> = {
        let names = repo
            .remotes()
            .map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))?;
        (0..names.len())
            .filter_map(|i| names.get(i).ok().flatten().map(String::from))
            .collect()
    };

    let snapshot_names: std::collections::HashSet<&str> =
        remotes.iter().map(|r| r.name.as_str()).collect();

    for name in &current_names {
        if !snapshot_names.contains(name.as_str()) {
            repo.remote_delete(name)
                .map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))?;
        }
    }

    for remote_cfg in remotes {
        let current_push_url = repo
            .find_remote(&remote_cfg.name)
            .ok()
            .and_then(|remote| remote.pushurl().ok().flatten().map(String::from));

        if current_names.contains(&remote_cfg.name) {
            repo.remote_set_url(&remote_cfg.name, &remote_cfg.url)
                .map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))?;
            match (&remote_cfg.push_url, &current_push_url) {
                (Some(url), _) => {
                    repo.remote_set_pushurl(&remote_cfg.name, Some(url))
                        .map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))?;
                }
                (None, Some(_)) => {
                    repo.remote_set_pushurl(&remote_cfg.name, None)
                        .map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))?;
                }
                (None, None) => {}
            }
        } else {
            repo.remote(&remote_cfg.name, &remote_cfg.url)
                .map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))?;
            if let Some(push_url) = &remote_cfg.push_url {
                repo.remote_set_pushurl(&remote_cfg.name, Some(push_url))
                    .map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))?;
            }
        }
    }

    Ok(())
}

/// Default branch for bare→normal conversion (HEAD symbolic target, else first local branch).
pub(crate) fn detect_default_branch_for_path(repo_path: &Path) -> Result<String> {
    let repo = Repository::open(repo_path)
        .map_err(|_| WorkpotError::GitUnavailable(repo_path.to_path_buf()))?;
    detect_default_branch(&repo).map_err(|_| {
        WorkpotError::InvalidPath("no branches found — cannot determine default branch".into())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;

    #[test]
    fn open_and_query_bare_repo_skips_dirty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().canonicalize().expect("canonicalize");
        Repository::init_bare(&path).expect("init bare");
        let state = open_and_query(&path).expect("query");
        assert_eq!(state.is_dirty, None);
    }

    #[test]
    fn open_and_query_unborn_branch() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().canonicalize().expect("canonicalize");
        Repository::init(&path).expect("init");
        let state = open_and_query(&path).expect("query");
        assert_eq!(state.branch.as_deref(), Some(crate::domain::BRANCH_UNBORN));
        assert_eq!(state.is_dirty, Some(false));
    }

    #[test]
    fn open_and_query_non_git_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().canonicalize().expect("canonicalize");
        let err = open_and_query(&path).unwrap_err();
        assert!(matches!(err, WorkpotError::GitUnavailable(_)));
    }

    #[test]
    fn open_and_query_reports_branch_and_clean_after_commit() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().canonicalize().expect("canonicalize");
        let repo = Repository::init(&path).expect("init");
        std::fs::write(path.join("README"), "hello").expect("write");
        let mut index = repo.index().expect("index");
        index.add_path(std::path::Path::new("README")).expect("add");
        index.write().expect("write index");
        let tree_id = index.write_tree().expect("tree");
        let tree = repo.find_tree(tree_id).expect("tree");
        let sig = git2::Signature::now("test", "test@example.com").expect("sig");
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .expect("commit");

        let state = open_and_query(&path).expect("query");
        assert!(state.branch.is_some(), "branch should be set after commit");
        assert_eq!(state.is_dirty, Some(false));
        assert!(state.error.is_none());
    }

    #[test]
    fn open_and_query_detects_dirty_worktree() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().canonicalize().expect("canonicalize");
        let repo = Repository::init(&path).expect("init");
        std::fs::write(path.join("README"), "hello").expect("write");
        let mut index = repo.index().expect("index");
        index.add_path(std::path::Path::new("README")).expect("add");
        index.write().expect("write index");
        let tree_id = index.write_tree().expect("tree");
        let tree = repo.find_tree(tree_id).expect("tree");
        let sig = git2::Signature::now("test", "test@example.com").expect("sig");
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .expect("commit");

        std::fs::write(path.join("README"), "dirty").expect("dirty");

        let state = open_and_query(&path).expect("query");
        assert_eq!(state.is_dirty, Some(true));
    }

    #[test]
    fn list_remotes_round_trip_via_apply_remotes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().canonicalize().expect("canonicalize");
        Repository::init(&path).expect("init");
        let repo = Repository::open(&path).expect("open");
        repo.remote("origin", "https://example.com/foo.git")
            .expect("add origin");

        let snapshot = list_remotes(&path).expect("list");
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].name, "origin");
        assert_eq!(snapshot[0].url, "https://example.com/foo.git");

        repo.remote_set_url("origin", "https://example.com/bar.git")
            .expect("mutate url");

        apply_remotes(&path, &snapshot).expect("apply");
        let restored = list_remotes(&path).expect("list again");
        assert_eq!(restored, snapshot);
    }

    #[test]
    fn apply_remotes_empty_removes_clone_injected_origin() {
        let dir = tempfile::tempdir().expect("tempdir");
        let source = dir.path().join("source");
        let clone_path = dir.path().join("clone");
        Repository::init(&source).expect("init source");
        let sig = git2::Signature::now("test", "test@example.com").expect("sig");
        std::fs::write(source.join("README"), "hi").expect("write");
        let repo = Repository::open(&source).expect("open source");
        let mut index = repo.index().expect("index");
        index.add_path(std::path::Path::new("README")).expect("add");
        index.write().expect("write");
        let tree_id = index.write_tree().expect("tree");
        let tree = repo.find_tree(tree_id).expect("tree");
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .expect("commit");

        let status = crate::testing::git_cmd()
            .args([
                "clone",
                "-q",
                source.to_str().expect("utf8"),
                clone_path.to_str().expect("utf8"),
            ])
            .status()
            .expect("clone");
        assert!(status.success());

        assert!(!list_remotes(&clone_path).expect("list").is_empty());
        apply_remotes(&clone_path, &[]).expect("clear remotes");
        assert!(list_remotes(&clone_path).expect("list after").is_empty());
    }

    #[test]
    fn resolve_git_common_dir_returns_absolute_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().canonicalize().expect("canonicalize");
        Repository::init(&path).expect("init");
        let common = resolve_git_common_dir(&path).expect("common dir");
        assert!(
            common.is_absolute(),
            "common dir must be absolute: {common:?}"
        );
        assert!(common.ends_with(".git"), "expected .git dir: {common:?}");
    }
}
