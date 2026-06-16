use crate::domain::{Config, SOURCE_MANUAL, SOURCE_SCAN};
use crate::error::{Result, WorkpotError};
use crate::infra::git::resolve_git_common_dir;
use crate::services::{catalog, discovery, git_state, paths};
use rusqlite::{Connection, Transaction, params};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndexSummary {
    pub added: u32,
    pub removed: u32,
    pub skipped: u32,
    pub git_refreshed: u32,
    pub git_errors: u32,
}

#[derive(Debug)]
struct ChangeEntry {
    path: String,
    action: &'static str,
}

/// Discovery output for phased indexing (filesystem scan + read-only catalog queries).
#[derive(Debug)]
pub struct DiscoveryPlan {
    pub started_at: i64,
    pub upserts: Vec<(PathBuf, String)>,
    pub removes: Vec<String>,
    pub pre_skipped: u32,
    changelog: Vec<ChangeEntry>,
    pub scan_candidate_count: usize,
}

/// Phase 1: scan watch roots and plan catalog changes (read connection only).
pub fn discover_phase(conn: &Connection, config: &Config) -> Result<DiscoveryPlan> {
    let started_at = crate::services::git_state::unix_now_secs();
    let exclude_set = discovery::build_exclude_set(config)?;
    let watch_roots = canonical_watch_roots(config);
    let configured_roots = &config.watch_roots;
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

    let scan_candidate_count = scan_candidates.len();
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

    let mut removes = collect_stale_scan_paths(conn, configured_roots, &watch_roots, &seen_paths)?;
    removes.extend(collect_orphan_scan_paths(conn, configured_roots)?);
    removes.extend(catalog::missing_repo_paths(conn)?);
    validate_manual_outside_roots(conn, configured_roots, &mut removes)?;
    removes.sort();
    removes.dedup();

    log::debug!(
        "index discovery: scan_candidates={} upserts={} removes={}",
        scan_candidate_count,
        upserts.len(),
        removes.len()
    );

    Ok(DiscoveryPlan {
        started_at,
        upserts,
        removes,
        pre_skipped,
        changelog,
        scan_candidate_count,
    })
}

/// Phase 2: merge discovery plan into the catalog (write connection, one transaction).
pub fn merge_catalog_phase(
    conn: &Connection,
    config: &Config,
    plan: DiscoveryPlan,
) -> Result<IndexSummary> {
    let max_repos = config.limits.max_repos;
    let projected = projected_repo_count(conn, &plan.removes, &plan.upserts)?;
    if projected > i64::from(max_repos) {
        let projected_u32 = u32::try_from(projected).unwrap_or(u32::MAX);
        if let Err(e) = record_cap_exceeded_run(conn, plan.started_at, projected, max_repos) {
            log::warn!("failed to record cap-exceeded audit row: {e}");
        }
        return Err(WorkpotError::IndexCapExceeded {
            projected: projected_u32,
            max: max_repos,
        });
    }

    let mut summary = IndexSummary {
        skipped: plan.pre_skipped,
        ..IndexSummary::default()
    };

    let mut changelog = plan.changelog;

    let tx = conn.unchecked_transaction()?;
    let run_id = insert_index_run(&tx, plan.started_at)?;

    let backfill_skipped_tx = backfill_empty_git_common_dir(&tx, &mut changelog)?;
    summary.skipped += backfill_skipped_tx;

    for (path, git_common_dir) in &plan.upserts {
        let path_key = path.display().to_string();
        if catalog::upsert_scan(&tx, path, git_common_dir)? {
            summary.added += 1;
            changelog.push(ChangeEntry {
                path: path_key,
                action: "added",
            });
        }
    }

    for path_key in &plan.removes {
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

    finish_index_run(&tx, run_id, "ok", &summary, None)?;
    tx.commit()?;
    Ok(summary)
}

/// Phase 3 prep: paths for git refresh (read connection).
pub fn index_git_paths(conn: &Connection) -> Result<Vec<PathBuf>> {
    let mut stmt = conn.prepare("SELECT path FROM repos WHERE excluded = 0")?;
    Ok(stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .map(PathBuf::from)
        .collect())
}

/// Phase 4: persist rayon git refresh results (write connection, one transaction).
pub fn persist_index_git_phase(
    conn: &Connection,
    summary: &mut IndexSummary,
    git_results: Vec<crate::services::git_state::GitRefreshResult>,
) -> Result<()> {
    for r in &git_results {
        if r.state.error.is_some() {
            summary.git_errors += 1;
        } else {
            summary.git_refreshed += 1;
        }
    }

    let git_tx = conn.unchecked_transaction()?;
    let refresh_time = crate::services::git_state::unix_now_secs();
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
    if let Err(e) = git_tx.commit() {
        log::warn!("git refresh commit failed after successful index merge: {e}");
        summary.git_errors = summary.git_errors.saturating_add(summary.git_refreshed);
        summary.git_refreshed = 0;
    }
    Ok(())
}

/// Phased index: release locks between discovery, merge, git refresh, and persist.
pub fn run_phased(pool: &crate::infra::db::DbPool, config: &Config) -> Result<IndexSummary> {
    let started_at = crate::services::git_state::unix_now_secs();
    log::debug!("index run_phased: start");
    match run_phased_inner(pool, config, started_at) {
        Ok(summary) => {
            log::debug!(
                "index run_phased: complete added={} removed={} skipped={} git_refreshed={} git_errors={}",
                summary.added,
                summary.removed,
                summary.skipped,
                summary.git_refreshed,
                summary.git_errors
            );
            Ok(summary)
        }
        Err(WorkpotError::IndexCapExceeded { projected, max }) => {
            Err(WorkpotError::IndexCapExceeded { projected, max })
        }
        Err(e) => {
            if let Err(audit_err) = pool.with_write(|conn| record_error_run(conn, started_at, &e)) {
                log::warn!("failed to record error audit row: {audit_err}");
            }
            Err(e)
        }
    }
}

fn run_phased_inner(
    pool: &crate::infra::db::DbPool,
    config: &Config,
    _started_at: i64,
) -> Result<IndexSummary> {
    test_index_delay();

    let plan = pool.with_read(|conn| discover_phase(conn, config))?;
    let mut summary = pool.with_write(|conn| merge_catalog_phase(conn, config, plan))?;

    let all_paths = pool.with_read(index_git_paths)?;
    log::debug!("index git second pass: start repos={}", all_paths.len());
    let git_pass_started = std::time::Instant::now();
    let git_results = git_state::refresh_all(all_paths);
    log::debug!(
        "index git second pass: refresh_all elapsed_ms={}",
        git_pass_started.elapsed().as_millis()
    );

    test_index_delay();

    pool.with_write(|conn| persist_index_git_phase(conn, &mut summary, git_results))?;
    Ok(summary)
}

fn test_index_delay() {
    if let Ok(ms) = std::env::var("WORKPOT_TEST_INDEX_DELAY_MS")
        && let Ok(ms) = ms.parse::<u64>()
    {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }
}

/// Full watch-root rescan with transactional merge, caps, and audit history (D-07, D-14–D-18).
pub fn run_full(pool: &crate::infra::db::DbPool, config: &Config) -> Result<IndexSummary> {
    run_phased(pool, config)
}

/// Single-connection variant for unit tests.
pub fn run_full_connection(conn: &Connection, config: &Config) -> Result<IndexSummary> {
    let started_at = crate::services::git_state::unix_now_secs();
    log::debug!("index run_full: start");
    match run_full_inner(conn, config, started_at) {
        Ok(summary) => {
            log::debug!(
                "index run_full: complete added={} removed={} skipped={} git_refreshed={} git_errors={}",
                summary.added,
                summary.removed,
                summary.skipped,
                summary.git_refreshed,
                summary.git_errors
            );
            Ok(summary)
        }
        Err(WorkpotError::IndexCapExceeded { projected, max }) => {
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

fn run_full_inner(conn: &Connection, config: &Config, _started_at: i64) -> Result<IndexSummary> {
    test_index_delay();

    let plan = discover_phase(conn, config)?;
    let mut summary = merge_catalog_phase(conn, config, plan)?;

    let all_paths = index_git_paths(conn)?;
    log::debug!("index git second pass: start repos={}", all_paths.len());
    let git_pass_started = std::time::Instant::now();
    let git_results = git_state::refresh_all(all_paths);
    log::debug!(
        "index git second pass: refresh_all elapsed_ms={}",
        git_pass_started.elapsed().as_millis()
    );

    test_index_delay();
    persist_index_git_phase(conn, &mut summary, git_results)?;
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
    let mut stmt =
        conn.prepare("SELECT path FROM repos WHERE git_common_dir = '' OR git_common_dir IS NULL")?;
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

fn scan_paths_by_source(conn: &Connection, source: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT path FROM repos WHERE source = ?1 AND excluded = 0")?;
    stmt.query_map(params![source], |row| row.get(0))?
        .collect::<std::result::Result<_, _>>()
        .map_err(WorkpotError::Database)
}

fn collect_stale_scan_paths(
    conn: &Connection,
    configured_roots: &[PathBuf],
    scan_roots: &[PathBuf],
    seen: &HashSet<String>,
) -> Result<Vec<String>> {
    let paths = scan_paths_by_source(conn, SOURCE_SCAN)?;

    let mut stale = Vec::new();
    for path_key in paths {
        let path = Path::new(&path_key);
        if !configured_roots
            .iter()
            .any(|root| paths::path_under_root(path, root))
        {
            continue;
        }
        // Root still configured but not scannable this run — preserve indexed repos.
        if !scan_roots
            .iter()
            .any(|root| paths::path_under_root(path, root))
        {
            continue;
        }
        if !seen.contains(&path_key) {
            stale.push(path_key);
        }
    }
    Ok(stale)
}

/// Scan repos not under any configured watch root (orphans after config edits or partial failures).
fn collect_orphan_scan_paths(
    conn: &Connection,
    configured_roots: &[PathBuf],
) -> Result<Vec<String>> {
    let paths = scan_paths_by_source(conn, SOURCE_SCAN)?;

    Ok(paths
        .into_iter()
        .filter(|path_key| {
            let path = Path::new(path_key);
            !configured_roots
                .iter()
                .any(|root| paths::path_under_root(path, root))
        })
        .collect())
}

fn validate_manual_outside_roots(
    conn: &Connection,
    configured_roots: &[PathBuf],
    removes: &mut Vec<String>,
) -> Result<()> {
    let paths = scan_paths_by_source(conn, SOURCE_MANUAL)?;

    for path_key in paths {
        let path = Path::new(&path_key);
        if configured_roots
            .iter()
            .any(|root| paths::path_under_root(path, root))
        {
            continue;
        }
        if !path.exists() || (!catalog::is_git_worktree(path) && !catalog::is_bare_repo(path)) {
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
    let finished_at = crate::services::git_state::unix_now_secs();
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
    let finished_at = crate::services::git_state::unix_now_secs();
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
    let finished_at = crate::services::git_state::unix_now_secs();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::store;

    fn conn_with_scan_path(path_key: &str) -> rusqlite::Connection {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("workpot.db");
        let conn = store::open_connection(&db_path).expect("open db");
        conn.execute(
            "INSERT INTO repos (path, name, registered_at, source, git_common_dir, excluded)
             VALUES (?1, 'demo', 0, ?2, '', 0)",
            params![path_key, SOURCE_SCAN],
        )
        .expect("insert scan row");
        std::mem::forget(dir);
        conn
    }

    #[test]
    fn collect_orphan_scan_paths_honors_configured_roots_without_canonicalization() {
        let configured = PathBuf::from("/tmp/workpot-nonexistent-root-demo");
        let repo_key = format!("{}/myrepo", configured.display());
        let conn = conn_with_scan_path(&repo_key);

        let orphans = collect_orphan_scan_paths(&conn, std::slice::from_ref(&configured))
            .expect("collect orphans");
        assert!(
            orphans.is_empty(),
            "repos under a configured root must not be purged when the root is absent from the canonical set"
        );

        let orphans_without_configured = collect_orphan_scan_paths(&conn, &[])
            .expect("collect orphans without configured roots");
        assert_eq!(orphans_without_configured, vec![repo_key]);
    }

    #[test]
    fn canonical_watch_roots_omits_nonexistent_paths() {
        let mut config = Config::default();
        config
            .watch_roots
            .push(PathBuf::from("/tmp/workpot-missing-watch-root-nope-xyz"));
        let temp = std::env::temp_dir();
        config.watch_roots.push(temp.clone());

        let roots = canonical_watch_roots(&config);
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], temp.canonicalize().expect("temp canon"));
    }

    #[test]
    fn projected_repo_count_applies_removes_and_upserts() {
        let existing = "/tmp/workpot-projected-count-existing";
        let conn = conn_with_scan_path(existing);
        let replacement = PathBuf::from("/tmp/workpot-projected-count-replacement");

        let count = projected_repo_count(
            &conn,
            &[existing.to_string()],
            &[(replacement, String::new())],
        )
        .expect("projected count");
        assert_eq!(count, 1);
    }

    #[test]
    fn collect_stale_scan_paths_skips_repos_when_configured_root_not_scanned() {
        let configured = PathBuf::from("/tmp/workpot-nonexistent-root-demo");
        let repo_key = format!("{}/myrepo", configured.display());
        let conn = conn_with_scan_path(&repo_key);
        let seen = HashSet::new();

        let stale = collect_stale_scan_paths(&conn, std::slice::from_ref(&configured), &[], &seen)
            .expect("collect stale");
        assert!(
            stale.is_empty(),
            "repos under an unscannable configured root must not be marked stale"
        );
    }
}
