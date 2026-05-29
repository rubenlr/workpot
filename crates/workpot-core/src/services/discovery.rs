use crate::domain::Config;
use crate::error::{Result, WorkpotError};
use crate::infra::git::list_worktree_paths;
use crate::services::catalog::{is_bare_repo, is_git_worktree};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Built-in exclude globs applied in addition to user config (D-09).
pub fn built_in_defaults() -> &'static [&'static str] {
    &[
        "**/node_modules/**",
        "**/target/**",
        "**/.Trash/**",
        "**/.Trash-*/**",
        "**/build/**",
        "**/dist/**",
        "**/.build/**",
        "**/.git/modules/**",
        "**/DerivedData/**",
        "**/Library/Caches/**",
    ]
}

/// Union of built-in defaults and `config.excludes` (D-08, D-09). Globs use `/`; match canonical paths when possible.
pub fn build_exclude_set(config: &Config) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pat in built_in_defaults()
        .iter()
        .copied()
        .chain(config.excludes.iter().map(String::as_str))
    {
        builder.add(
            Glob::new(pat)
                .map_err(|e| WorkpotError::Config(format!("invalid exclude glob: {e}")))?,
        );
    }
    builder
        .build()
        .map_err(|e| WorkpotError::Config(format!("exclude glob set: {e}")))
}

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
                let match_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                if exclude_set.is_match(&match_path) {
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
                            list.push(canon.clone());
                            if is_bare_repo(&canon) {
                                if let Ok(linked) = list_worktree_paths(&canon) {
                                    list.extend(linked);
                                }
                            }
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
