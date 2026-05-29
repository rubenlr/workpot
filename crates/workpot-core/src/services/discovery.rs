use crate::error::{Result, WorkpotError};
use crate::services::catalog::{is_bare_repo, is_git_worktree};
use globset::GlobSet;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Walk a watch root and return canonical candidate repo paths (D-01, D-02).
pub fn scan_root(root: &Path, exclude_set: &GlobSet) -> Result<Vec<PathBuf>> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let root_canon = root
        .canonicalize()
        .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", root.display())))?;

    let candidates = Arc::new(Mutex::new(Vec::new()));
    let exclude_set = exclude_set.clone();

    let walker = WalkBuilder::new(&root_canon)
        .follow_links(false)
        .standard_filters(false)
        .filter_entry({
            let candidates = Arc::clone(&candidates);
            move |entry| {
                let path = entry.path();
                if exclude_set.is_match(path) {
                    if entry.file_type().is_some_and(|t| t.is_dir()) {
                        return false;
                    }
                    return true;
                }
                if entry.file_type().is_some_and(|t| t.is_dir())
                    && (is_git_worktree(path) || is_bare_repo(path))
                {
                    if let Ok(canon) = path.canonicalize() {
                        if let Ok(mut list) = candidates.lock() {
                            list.push(canon);
                        }
                    }
                    return false;
                }
                true
            }
        })
        .build();

    for result in walker {
        result.map_err(|e| WorkpotError::InvalidPath(format!("discovery walk: {e}")))?;
    }

    let candidates = Arc::try_unwrap(candidates)
        .map_err(|_| WorkpotError::InvalidPath("discovery walk: lock still held".into()))?
        .into_inner()
        .map_err(|_| WorkpotError::InvalidPath("discovery walk: lock poisoned".into()))?;

    Ok(candidates)
}
