use crate::domain::Config;
use crate::error::{Result, WorkpotError};
use crate::infra::git::resolve_git_common_dir;
use crate::services::{catalog, discovery};
use globset::{Glob, GlobSet, GlobSetBuilder};
use rusqlite::{params, Connection};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndexSummary {
    pub added: u32,
    pub removed: u32,
    pub skipped: u32,
}

/// Full watch-root rescan: discover candidates, resolve `git_common_dir`, merge into SQLite.
pub fn run_full(conn: &Connection, config: &Config) -> Result<IndexSummary> {
    let exclude_set = build_exclude_set(&config.excludes)?;
    let mut summary = IndexSummary::default();
    let mut seen_paths = HashSet::new();

    for root in &config.watch_roots {
        let root_canon = match root.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("warning: skip watch root {}: {e}", root.display());
                continue;
            }
        };

        let candidates = discovery::scan_root(&root_canon, &exclude_set)?;
        for path in candidates {
            let path_key = path.display().to_string();
            seen_paths.insert(path_key);

            match resolve_git_common_dir(&path) {
                Ok(common) => {
                    let common_str = common.display().to_string();
                    if catalog::upsert_scan(conn, &path, &common_str)? {
                        summary.added += 1;
                    }
                }
                Err(_) => {
                    eprintln!(
                        "warning: skip {}: git unavailable",
                        path.display()
                    );
                    summary.skipped += 1;
                }
            }
        }

        summary.removed += prune_stale_scan_under_root(conn, &root_canon, &seen_paths)?;
    }

    Ok(summary)
}

fn build_exclude_set(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pat in patterns {
        builder.add(
            Glob::new(pat).map_err(|e| WorkpotError::Config(format!("invalid exclude glob: {e}")))?,
        );
    }
    builder
        .build()
        .map_err(|e| WorkpotError::Config(format!("exclude glob set: {e}")))
}

fn prune_stale_scan_under_root(
    conn: &Connection,
    root: &Path,
    seen: &HashSet<String>,
) -> Result<u32> {
    let mut stmt = conn.prepare(
        "SELECT path FROM repos WHERE source = 'scan' AND excluded = 0",
    )?;
    let paths: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<std::result::Result<_, _>>()?;

    let mut removed = 0u32;
    for path_key in paths {
        if !path_under_root(Path::new(&path_key), root) {
            continue;
        }
        if !seen.contains(&path_key) {
            conn.execute("DELETE FROM repos WHERE path = ?1", params![path_key])?;
            removed += 1;
        }
    }
    Ok(removed)
}

fn path_under_root(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}
