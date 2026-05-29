use crate::domain::SOURCE_SCAN;
use crate::error::{Result, WorkpotError};
use crate::save_config;
use crate::services::{index, paths};
use crate::AppContext;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

pub fn add_root(ctx: &mut AppContext, path: &Path) -> Result<()> {
    let canonical = canonicalize_watch_root(path)?;

    if ctx.config().watch_roots.iter().any(|r| roots_equal(r, &canonical)) {
        return Err(WorkpotError::WatchRootAlreadyExists(
            canonical.display().to_string(),
        ));
    }

    if ctx.config().watch_roots.len() >= ctx.config().limits.max_watch_roots as usize {
        return Err(WorkpotError::LimitsExceeded(format!(
            "watch root count would exceed max_watch_roots {}",
            ctx.config().limits.max_watch_roots
        )));
    }

    ctx.config_mut().watch_roots.push(canonical.clone());
    if let Err(e) = save_config(ctx.config_path(), ctx.config()) {
        ctx.config_mut().watch_roots.pop();
        return Err(e);
    }

    match index::run_full(ctx.connection(), ctx.config()) {
        Ok(_) => Ok(()),
        Err(e) => {
            ctx.config_mut().watch_roots.pop();
            save_config(ctx.config_path(), ctx.config())?;
            prune_scan_repos_under_root(ctx.connection(), &canonical)?;
            Err(e)
        }
    }
}

pub fn list_roots(ctx: &AppContext) -> Vec<PathBuf> {
    ctx.config().watch_roots.clone()
}

pub fn remove_root(ctx: &mut AppContext, path: &Path, skip_prune: bool) -> Result<()> {
    let canonical = canonicalize_watch_root(path)?;
    let config_path = ctx.config_path().to_path_buf();
    let pos = ctx
        .config()
        .watch_roots
        .iter()
        .position(|r| roots_equal(r, &canonical))
        .ok_or_else(|| WorkpotError::WatchRootNotFound(canonical.display().to_string()))?;
    let removed = ctx.config_mut().watch_roots.remove(pos);

    if let Err(e) = save_config(&config_path, ctx.config()) {
        ctx.config_mut().watch_roots.insert(pos, removed);
        return Err(e);
    }

    if skip_prune {
        return Ok(());
    }

    match prune_scan_repos_under_root(ctx.connection(), &canonical) {
        Ok(_) => Ok(()),
        Err(e) => {
            ctx.config_mut().watch_roots.insert(pos, removed);
            save_config(&config_path, ctx.config())?;
            Err(e)
        }
    }
}

/// Reload config from disk (D-19).
pub fn reload_config(ctx: &mut AppContext) -> Result<()> {
    let config = crate::load_config(ctx.config_path())?;
    *ctx.config_mut() = config;
    Ok(())
}

/// Delete scan-sourced repos whose canonical path is under `root` (D-21). Prefix match in Rust only.
fn prune_scan_repos_under_root(conn: &Connection, root: &Path) -> Result<u32> {
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
        if repo_under_root(repo_path, &root_canon)? {
            conn.execute("DELETE FROM repos WHERE path = ?1", params![path_key])?;
            removed += 1;
        }
    }
    Ok(removed)
}

fn repo_under_root(repo_path: &Path, root_canon: &Path) -> Result<bool> {
    Ok(paths::path_under_root(repo_path, root_canon))
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
