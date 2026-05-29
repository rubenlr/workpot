use crate::domain::GitState;
use crate::error::Result;
use rayon::prelude::*;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Result of a single-repo git refresh, returned from batch refresh_all.
pub struct GitRefreshResult {
    pub path: String,
    pub state: GitState,
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Write git state fields back to the repos row for `path_key`.
pub fn persist_git_state(conn: &Connection, path_key: &str, state: &GitState) -> Result<()> {
    conn.execute(
        "UPDATE repos SET branch=?1, is_dirty=?2, ahead=?3, behind=?4,
                          git_refreshed_at=?5, git_state_error=?6
         WHERE path=?7",
        params![
            state.branch,
            state.is_dirty.map(|b| i64::from(b)),
            state.ahead,
            state.behind,
            now_secs(),
            state.error,
            path_key,
        ],
    )?;
    Ok(())
}

/// Refresh git state for a single repo and persist the result to SQLite.
pub fn refresh_and_persist(conn: &Connection, path: &Path) -> Result<GitState> {
    let canonical = path
        .canonicalize()
        .map_err(|_| crate::error::WorkpotError::GitUnavailable(path.to_path_buf()))?;
    let path_key = canonical.display().to_string();
    let state = crate::infra::git::open_and_query(&canonical)?;
    persist_git_state(conn, &path_key, &state)?;
    Ok(state)
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
