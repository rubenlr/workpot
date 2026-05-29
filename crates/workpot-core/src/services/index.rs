use crate::domain::{Config, SOURCE_MANUAL, SOURCE_SCAN};
use crate::error::{Result, WorkpotError};
use crate::infra::git::resolve_git_common_dir;
use crate::services::{catalog, discovery, git_state};
use rusqlite::{params, Connection, Transaction};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndexSummary {
    pub added: u32,
    pub removed: u32,
    pub skipped: u32,
    pub git_refreshed: u32,
    pub git_errors: u32,
}

struct ChangeEntry {
    path: String,
    action: &'static str,
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Full watch-root rescan with transactional merge, caps, and audit history (D-07, D-14–D-18).
pub fn run_full(conn: &Connection, config: &Config) -> Result<IndexSummary> {
    let started_at = now_secs();
    match run_full_inner(conn, config, started_at) {
        Ok(summary) => Ok(summary),
        Err(WorkpotError::IndexCapExceeded { projected, max }) => {
            if let Err(e) = record_cap_exceeded_run(conn, started_at, i64::from(projected), max) {
                log::warn!("failed to record cap-exceeded audit row: {e}");
            }
            Err(WorkpotError::IndexCapExceeded { projected, max })
        }
        Err(e) => {
            if let Err(audit_err) = record_error_run(conn, started_at, &e) {
                log::warn!("failed to record error audit row: {audit_err}");
            }
            Err(e)
        }
    }
}

fn run_full_inner(conn: &Connection, config: &Config, started_at: i64) -> Result<IndexSummary> {
    let exclude_set = discovery::build_exclude_set(config)?;
    let max_repos = config.limits.max_repos;

    let watch_roots = canonical_watch_roots(config);
    let mut changelog: Vec<ChangeEntry> = Vec::new();
    let mut pre_skipped = 0u32;

    let mut seen_paths: HashSet<String> = HashSet::new();
    let mut scan_candidates: Vec<PathBuf> = Vec::new();

    for root in &watch_roots {
        let candidates = discovery::scan_root(root, &exclude_set)?;
        for path in candidates {
            let path_key = path.display().to_string();
            if seen_paths.insert(path_key) {
                scan_candidates.push(path);
            }
        }
    }

    let mut upserts: Vec<(PathBuf, String)> = Vec::new();
    for path in scan_candidates {
        let path_key = path.display().to_string();
        match resolve_git_common_dir(&path) {
            Ok(common) => {
                upserts.push((path, common.display().to_string()));
            }
            Err(_) => {
                log::warn!("skip {}: git unavailable", path_key);
                pre_skipped += 1;
                changelog.push(ChangeEntry {
                    path: path_key,
                    action: "skipped",
                });
            }
        }
    }

    let mut removes = collect_stale_scan_paths(conn, &watch_roots, &seen_paths)?;
    removes.extend(collect_missing_paths(conn)?);
    validate_manual_outside_roots(conn, &watch_roots, &mut removes)?;
    removes.sort();
    removes.dedup();

    let projected = projected_repo_count(conn, &removes, &upserts)?;
    if projected > i64::from(max_repos) {
        let projected_u32 = u32::try_from(projected).unwrap_or(u32::MAX);
        return Err(WorkpotError::IndexCapExceeded {
            projected: projected_u32,
            max: max_repos,
        });
    }

    let mut summary = IndexSummary {
        skipped: pre_skipped,
        ..IndexSummary::default()
    };

    let tx = conn.unchecked_transaction()?;
    let run_id = insert_index_run(&tx, started_at)?;

    let backfill_skipped_tx = backfill_empty_git_common_dir(&tx, &mut changelog)?;
    summary.skipped += backfill_skipped_tx;

    for (path, git_common_dir) in &upserts {
        let path_key = path.display().to_string();
        if catalog::upsert_scan(&tx, path, git_common_dir)? {
            summary.added += 1;
            changelog.push(ChangeEntry {
                path: path_key,
                action: "added",
            });
        }
    }

    for path_key in &removes {
        let deleted = tx.execute("DELETE FROM repos WHERE path = ?1", params![path_key])?;
        if deleted > 0 {
            summary.removed += 1;
            changelog.push(ChangeEntry {
                path: path_key.clone(),
                action: "removed",
            });
        }
    }

    for entry in &changelog {
        tx.execute(
            "INSERT INTO index_changes (run_id, path, action) VALUES (?1, ?2, ?3)",
            params![run_id, entry.path, entry.action],
        )?;
    }

    finish_index_run(
        &tx,
        run_id,
        "ok",
        &summary,
        None,
    )?;
    tx.commit()?;

    // Second pass: git state refresh (separate transaction per Pitfall 6)
    let all_paths: Vec<PathBuf> = {
        let mut stmt = conn.prepare("SELECT path FROM repos WHERE excluded = 0")?;
        stmt.query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .map(PathBuf::from)
            .collect()
    };

    // rayon parallel refresh must complete before opening any DB transaction
    let git_results = git_state::refresh_all(all_paths);

    for r in &git_results {
        if r.state.error.is_some() {
            summary.git_errors += 1;
        } else {
            summary.git_refreshed += 1;
        }
    }

    let git_tx = conn.unchecked_transaction()?;
    let refresh_time = now_secs();
    for r in &git_results {
        let updated = git_tx.execute(
            "UPDATE repos SET branch=?1, is_dirty=?2, ahead=?3, behind=?4,
                              git_refreshed_at=?5, git_state_error=?6
             WHERE path=?7",
            rusqlite::params![
                r.state.branch,
                r.state.is_dirty.map(|b| b as i64),
                r.state.ahead,
                r.state.behind,
                refresh_time,
                r.state.error,
                r.path,
            ],
        )?;
        if updated == 0 {
            log::warn!("git refresh: no repo row matched path {}", r.path);
            if r.state.error.is_none() {
                summary.git_refreshed = summary.git_refreshed.saturating_sub(1);
                summary.git_errors += 1;
            }
        }
    }
    git_tx.commit()?;

    Ok(summary)
}

fn canonical_watch_roots(config: &Config) -> Vec<PathBuf> {
    config
        .watch_roots
        .iter()
        .filter_map(|root| match root.canonicalize() {
            Ok(p) => Some(p),
            Err(e) => {
                log::warn!("skip watch root {}: {e}", root.display());
                None
            }
        })
        .collect()
}

fn backfill_empty_git_common_dir(
    conn: &Connection,
    changelog: &mut Vec<ChangeEntry>,
) -> Result<u32> {
    let mut stmt = conn.prepare(
        "SELECT path FROM repos WHERE git_common_dir = '' OR git_common_dir IS NULL",
    )?;
    let paths: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<std::result::Result<_, _>>()?;

    let mut skipped = 0u32;
    for path_key in paths {
        let path = Path::new(&path_key);
        if !path.exists() {
            continue;
        }
        if !catalog::is_git_worktree(path) && !catalog::is_bare_repo(path) {
            continue;
        }
        match resolve_git_common_dir(path) {
            Ok(common) => {
                let common_str = common.display().to_string();
                conn.execute(
                    "UPDATE repos SET git_common_dir = ?1 WHERE path = ?2",
                    params![common_str, path_key],
                )?;
            }
            Err(_) => {
                log::warn!("skip backfill {}: git unavailable", path.display());
                skipped += 1;
                changelog.push(ChangeEntry {
                    path: path_key,
                    action: "skipped",
                });
            }
        }
    }
    Ok(skipped)
}

fn collect_stale_scan_paths(
    conn: &Connection,
    watch_roots: &[PathBuf],
    seen: &HashSet<String>,
) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT path FROM repos WHERE source = ?1 AND excluded = 0",
    )?;
    let paths: Vec<String> = stmt
        .query_map(params![SOURCE_SCAN], |row| row.get(0))?
        .collect::<std::result::Result<_, _>>()?;

    let mut stale = Vec::new();
    for path_key in paths {
        let path = Path::new(&path_key);
        if !watch_roots.iter().any(|root| path_under_root(path, root)) {
            continue;
        }
        if !seen.contains(&path_key) {
            stale.push(path_key);
        }
    }
    Ok(stale)
}

fn collect_missing_paths(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT path FROM repos WHERE excluded = 0")?;
    let paths: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<std::result::Result<_, _>>()?;

    Ok(paths
        .into_iter()
        .filter(|path_key| !Path::new(path_key).exists())
        .collect())
}

fn validate_manual_outside_roots(
    conn: &Connection,
    watch_roots: &[PathBuf],
    removes: &mut Vec<String>,
) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT path FROM repos WHERE source = ?1 AND excluded = 0",
    )?;
    let paths: Vec<String> = stmt
        .query_map(params![SOURCE_MANUAL], |row| row.get(0))?
        .collect::<std::result::Result<_, _>>()?;

    for path_key in paths {
        let path = Path::new(&path_key);
        if watch_roots.iter().any(|root| path_under_root(path, root)) {
            continue;
        }
        if !path.exists()
            || (!catalog::is_git_worktree(path) && !catalog::is_bare_repo(path))
        {
            removes.push(path_key);
        }
    }
    Ok(())
}

fn projected_repo_count(
    conn: &Connection,
    removes: &[String],
    upserts: &[(PathBuf, String)],
) -> Result<i64> {
    let mut paths: HashSet<String> = HashSet::new();
    let mut stmt = conn.prepare("SELECT path FROM repos WHERE excluded = 0")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    for row in rows {
        paths.insert(row?);
    }
    for key in removes {
        paths.remove(key);
    }
    for (path, _) in upserts {
        paths.insert(path.display().to_string());
    }
    Ok(i64::try_from(paths.len()).unwrap_or(i64::MAX))
}

fn record_error_run(conn: &Connection, started_at: i64, err: &WorkpotError) -> Result<()> {
    let finished_at = now_secs();
    let message = err.to_string();
    conn.execute(
        "INSERT INTO index_runs (started_at, finished_at, status, added_count, removed_count, skipped_count, message)
         VALUES (?1, ?2, 'error', 0, 0, 0, ?3)",
        params![started_at, finished_at, message],
    )?;
    Ok(())
}

fn record_cap_exceeded_run(
    conn: &Connection,
    started_at: i64,
    projected: i64,
    max: u32,
) -> Result<()> {
    let finished_at = now_secs();
    let message = format!("projected {projected} repos exceeds max {max}");
    conn.execute(
        "INSERT INTO index_runs (started_at, finished_at, status, added_count, removed_count, skipped_count, message)
         VALUES (?1, ?2, 'cap_exceeded', 0, 0, 0, ?3)",
        params![started_at, finished_at, message],
    )?;
    Ok(())
}

fn insert_index_run(tx: &Transaction<'_>, started_at: i64) -> Result<i64> {
    tx.execute(
        "INSERT INTO index_runs (started_at, status) VALUES (?1, 'ok')",
        params![started_at],
    )?;
    Ok(tx.last_insert_rowid())
}

fn finish_index_run(
    tx: &Transaction<'_>,
    run_id: i64,
    status: &str,
    summary: &IndexSummary,
    message: Option<&str>,
) -> Result<()> {
    let finished_at = now_secs();
    tx.execute(
        "UPDATE index_runs SET finished_at = ?1, status = ?2, added_count = ?3, removed_count = ?4, skipped_count = ?5, message = ?6
         WHERE id = ?7",
        params![
            finished_at,
            status,
            summary.added,
            summary.removed,
            summary.skipped,
            message,
            run_id,
        ],
    )?;
    Ok(())
}

fn path_under_root(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}
