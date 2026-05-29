use crate::domain::GitState;
use crate::error::Result;
use rayon::prelude::*;
use std::path::{Path, PathBuf};

/// Result of a single-repo git refresh, returned from batch refresh_all.
pub struct GitRefreshResult {
    pub path: String,
    pub state: GitState,
}

/// Public API: refresh git state for a single repository at `path`.
///
/// Canonicalizes path before delegating to infra (T-03-04 path traversal mitigation).
/// This is the per-repo API for Phase 4 tray (D-18).
pub fn refresh_git_state(path: &Path) -> Result<GitState> {
    // T-03-04: canonicalize path before passing to Repository::open
    let canonical = path
        .canonicalize()
        .map_err(|_| crate::error::WorkpotError::GitUnavailable(path.to_path_buf()))?;
    crate::infra::git::open_and_query(&canonical)
}

/// Refresh git state for all provided paths in parallel using rayon.
///
/// Never aborts on individual failure — embeds error string in GitState.error (D-16).
/// Each rayon thread opens its own Repository via open_and_query (Repository is Send not Sync).
pub fn refresh_all(paths: Vec<PathBuf>) -> Vec<GitRefreshResult> {
    paths
        .into_par_iter()
        .map(|path| {
            let state = refresh_git_state(&path).unwrap_or_else(|e| GitState {
                branch: None,
                is_dirty: None,
                ahead: None,
                behind: None,
                error: Some(e.to_string()),
            });
            GitRefreshResult {
                path: path.display().to_string(),
                state,
            }
        })
        .collect()
}
