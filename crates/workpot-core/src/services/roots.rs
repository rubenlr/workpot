use crate::AppState;
use crate::domain::SOURCE_SCAN;
use crate::error::{Result, WorkpotError};
use crate::save_config;
use crate::services::{index, paths};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};

pub fn add_root(state: &AppState, path: &Path) -> Result<()> {
    let canonical = canonicalize_watch_root(path)?;

    {
        let config = state.config()?;
        if config
            .watch_roots
            .iter()
            .any(|r| roots_equal(r, &canonical))
        {
            return Err(WorkpotError::WatchRootAlreadyExists(
                canonical.display().to_string(),
            ));
        }

        if config.watch_roots.len() >= config.limits.max_watch_roots as usize {
            return Err(WorkpotError::LimitsExceeded(format!(
                "watch root count would exceed max_watch_roots {}",
                config.limits.max_watch_roots
            )));
        }
    }

    {
        let mut config = state.config_mut()?;
        config.watch_roots.push(canonical.clone());
        if let Err(e) = save_config(state.config_path(), &config) {
            config.watch_roots.pop();
            return Err(e);
        }
    }

    let index_result = {
        let config = state.config()?;
        index::run_full(&state.db, &config)
    };
    match index_result {
        Ok(_) => Ok(()),
        Err(e) => {
            let mut config = state.config_mut()?;
            config.watch_roots.pop();
            save_config(state.config_path(), &config)?;
            prune_scan_repos_under_root(state, &canonical)?;
            Err(e)
        }
    }
}

pub fn list_roots(state: &AppState) -> Vec<PathBuf> {
    state
        .config()
        .map(|config| config.watch_roots.clone())
        .unwrap_or_default()
}

pub fn remove_root(state: &AppState, path: &Path, skip_prune: bool) -> Result<()> {
    let canonical = canonicalize_watch_root(path)?;
    let config_path = state.config_path().to_path_buf();
    let pos = {
        let config = state.config()?;
        config
            .watch_roots
            .iter()
            .position(|r| roots_equal(r, &canonical))
            .ok_or_else(|| WorkpotError::WatchRootNotFound(canonical.display().to_string()))?
    };

    let removed = {
        let mut config = state.config_mut()?;
        config.watch_roots.remove(pos)
    };

    let save_result = {
        let config = state.config()?;
        save_config(&config_path, &config)
    };
    if let Err(e) = save_result {
        let mut config = state.config_mut()?;
        config.watch_roots.insert(pos, removed);
        return Err(e);
    }

    if skip_prune {
        return Ok(());
    }

    match state.with_write_connection(|conn| prune_scan_repos_under_root_conn(conn, &canonical)) {
        Ok(_) => Ok(()),
        Err(e) => {
            let mut config = state.config_mut()?;
            config.watch_roots.insert(pos, removed);
            save_config(&config_path, &config)?;
            Err(e)
        }
    }
}

/// Reload config from disk (D-19).
pub fn reload_config(state: &AppState) -> Result<()> {
    let config = crate::load_config(state.config_path())?;
    *state.config_mut()? = config;
    Ok(())
}

fn prune_scan_repos_under_root(state: &AppState, root: &Path) -> Result<u32> {
    state.with_write_connection(|conn| prune_scan_repos_under_root_conn(conn, root))
}

/// Delete scan-sourced repos whose canonical path is under `root` (D-21). Prefix match in Rust only.
fn prune_scan_repos_under_root_conn(conn: &Connection, root: &Path) -> Result<u32> {
    let root_canon = root
        .canonicalize()
        .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", root.display())))?;

    let mut stmt = conn.prepare("SELECT path FROM repos WHERE source = ?1")?;
    let paths: Vec<String> = stmt
        .query_map(params![SOURCE_SCAN], |row| row.get(0))?
        .collect::<std::result::Result<_, _>>()?;

    let mut removed = 0u32;
    for path_key in paths {
        let repo_path = Path::new(&path_key);
        if paths::path_under_root(repo_path, &root_canon) {
            conn.execute("DELETE FROM repos WHERE path = ?1", params![path_key])?;
            removed += 1;
        }
    }
    Ok(removed)
}

fn canonicalize_watch_root(path: &Path) -> Result<PathBuf> {
    if !path.exists() {
        return Err(WorkpotError::InvalidPath(format!(
            "path does not exist: {}",
            path.display()
        )));
    }
    if !path.is_dir() {
        return Err(WorkpotError::InvalidPath(format!(
            "not a directory: {}",
            path.display()
        )));
    }
    path.canonicalize()
        .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", path.display())))
}

fn roots_equal(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(ac), Ok(bc)) => ac == bc,
        _ => a == b,
    }
}
