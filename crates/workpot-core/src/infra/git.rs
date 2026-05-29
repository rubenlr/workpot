use crate::error::{Result, WorkpotError};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Canonical absolute path to the shared git directory (D-05).
pub fn resolve_git_common_dir(path: &Path) -> Result<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--git-common-dir"])
        .output()
        .map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))?;

    if !output.status.success() {
        return Err(WorkpotError::GitUnavailable(path.to_path_buf()));
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if raw.is_empty() {
        return Err(WorkpotError::GitUnavailable(path.to_path_buf()));
    }

    let common = PathBuf::from(&raw);
    let resolved = if common.is_absolute() {
        common
    } else {
        path.join(common)
    };

    std::fs::canonicalize(&resolved).map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))
}

/// Linked worktree paths for a bare repository (D-04). Omits the bare worktree entry itself.
pub fn list_worktree_paths(repo_path: &Path) -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["worktree", "list", "--porcelain"])
        .output()
        .map_err(|_| WorkpotError::GitUnavailable(repo_path.to_path_buf()))?;

    if !output.status.success() {
        return Err(WorkpotError::GitUnavailable(repo_path.to_path_buf()));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut linked = Vec::new();
    let mut lines = text.lines().peekable();

    while let Some(line) = lines.next() {
        let Some(path_str) = line.strip_prefix("worktree ") else {
            continue;
        };
        let mut is_bare_entry = false;
        while let Some(&next) = lines.peek() {
            if next.starts_with("worktree ") {
                break;
            }
            if next == "bare" {
                is_bare_entry = true;
            }
            lines.next();
        }
        if !is_bare_entry {
            let path = PathBuf::from(path_str);
            let resolved = if path.is_absolute() {
                path
            } else {
                repo_path.join(path)
            };
            match std::fs::canonicalize(&resolved) {
                Ok(canon) => linked.push(canon),
                Err(e) => {
                    eprintln!(
                        "warning: skip worktree {}: {e}",
                        resolved.display()
                    );
                }
            }
        }
    }

    Ok(linked)
}
