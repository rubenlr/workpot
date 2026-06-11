use crate::domain::GitState;
use crate::error::Result;
use rayon::prelude::*;
use rusqlite::{Connection, params};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Summary returned after batch git refresh (tray background refresh).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GitRefreshSummary {
    pub refreshed: u32,
    pub errors: u32,
    pub any_dirty: bool,
}

/// Result of a single-repo git refresh, returned from batch refresh_all.
pub struct GitRefreshResult {
    pub path: String,
    pub state: GitState,
}

pub(crate) fn unix_now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Hard failure from `refresh_git_state` `Err` path — no git fields, only error message.
pub fn is_hard_refresh_failure(state: &GitState) -> bool {
    state.error.is_some() && state.branch.is_none() && state.is_dirty.is_none()
}

/// Update only error + timestamp; preserve prior branch/dirty/ahead/behind (CR-01).
pub fn persist_git_state_error_only(conn: &Connection, path_key: &str, error: &str) -> Result<()> {
    conn.execute(
        "UPDATE repos SET git_state_error=?1, git_refreshed_at=?2 WHERE path=?3",
        params![error, unix_now_secs(), path_key],
    )?;
    Ok(())
}

/// Write git state fields back to the repos row for `path_key`.
pub fn persist_git_state(conn: &Connection, path_key: &str, state: &GitState) -> Result<()> {
    conn.execute(
        "UPDATE repos SET branch=?1, is_dirty=?2, ahead=?3, behind=?4,
                          git_refreshed_at=?5, git_state_error=?6
         WHERE path=?7",
        params![
            state.branch,
            state.is_dirty.map(i64::from),
            state.ahead,
            state.behind,
            unix_now_secs(),
            state.error,
            path_key,
        ],
    )?;
    Ok(())
}

/// Refresh git state from `query_path` and persist under the catalog `persist_path` key.
pub fn refresh_and_persist_catalog_entry(
    conn: &Connection,
    persist_path: &Path,
    query_path: &Path,
) -> Result<GitState> {
    let path_key = crate::services::catalog::repo_path_key(conn, persist_path)?;
    let state = refresh_git_state(query_path)?;
    persist_git_state(conn, &path_key, &state)?;
    Ok(state)
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
    let repo_count = paths.len();
    let batch_started = std::time::Instant::now();
    log::debug!("git refresh_all: batch start repos={repo_count}");
    let results: Vec<GitRefreshResult> = paths
        .into_par_iter()
        .map(|path| {
            let path_key = path.display().to_string();
            let repo_started = std::time::Instant::now();
            let state = refresh_git_state(&path).unwrap_or_else(|e| {
                log::debug!("git refresh {path_key}: error {e}");
                GitState {
                    branch: None,
                    is_dirty: None,
                    ahead: None,
                    behind: None,
                    error: Some(e.to_string()),
                }
            });
            log::debug!(
                "git refresh {path_key}: elapsed_ms={}",
                repo_started.elapsed().as_millis()
            );
            GitRefreshResult {
                path: path_key,
                state,
            }
        })
        .collect();
    log::debug!(
        "git refresh_all: batch complete repos={repo_count} elapsed_ms={}",
        batch_started.elapsed().as_millis()
    );
    results
}

#[cfg(test)]
mod tests {
    use super::is_hard_refresh_failure;
    use crate::domain::GitState;

    #[test]
    fn hard_failure_when_error_without_git_fields() {
        let state = GitState {
            branch: None,
            is_dirty: None,
            ahead: None,
            behind: None,
            error: Some("git unavailable".to_string()),
        };
        assert!(is_hard_refresh_failure(&state));
    }

    #[test]
    fn not_hard_failure_when_branch_present_with_error() {
        let state = GitState {
            branch: Some("main".to_string()),
            is_dirty: Some(true),
            ahead: None,
            behind: None,
            error: Some("stale".to_string()),
        };
        assert!(!is_hard_refresh_failure(&state));
    }

    #[test]
    fn not_hard_failure_on_success() {
        let state = GitState {
            branch: Some("main".to_string()),
            is_dirty: Some(false),
            ahead: Some(0),
            behind: Some(0),
            error: None,
        };
        assert!(!is_hard_refresh_failure(&state));
    }
}
