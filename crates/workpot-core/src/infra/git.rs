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
