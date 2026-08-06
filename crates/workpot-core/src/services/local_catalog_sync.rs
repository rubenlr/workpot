use crate::domain::{
    AttachResult, Config, MergeAction, ProjectSnapshot, RemoteRef, SOURCE_MANUAL, SOURCE_SCAN,
    attach_location, merge_overlapping_projects, normalize_remote_url,
};
use crate::error::{Result, WorkpotError};
use crate::infra::git::{self, resolve_git_common_dir};
use crate::services::{catalog, discovery, git_state, paths};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalCatalogSyncSummary {
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

/// Discovery output for phased local catalog sync (filesystem scan + read-only catalog queries).
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
        match try_resolve_git_common_dir(&path, &path_key) {
            Ok(common) => upserts.push((path, common)),
            Err(()) => {
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
        "local catalog sync discovery: scan_candidates={} upserts={} removes={}",
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
) -> Result<LocalCatalogSyncSummary> {
    let max_repos = config.limits.max_repos;
    let projected = projected_repo_count(conn, &plan.removes, &plan.upserts)?;
    if projected > i64::from(max_repos) {
        let projected_u32 = u32::try_from(projected).unwrap_or(u32::MAX);
        if let Err(e) = record_cap_exceeded_run(conn, plan.started_at, projected, max_repos) {
            log::warn!("failed to record cap-exceeded audit row: {e}");
        }
        return Err(WorkpotError::LocalCatalogSyncCapExceeded {
            projected: projected_u32,
            max: max_repos,
        });
    }

    let mut summary = LocalCatalogSyncSummary {
        skipped: plan.pre_skipped,
        ..LocalCatalogSyncSummary::default()
    };

    let mut changelog = plan.changelog;

    let tx = conn.unchecked_transaction()?;
    let run_id = insert_sync_run(&tx, plan.started_at)?;

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
        let deleted = tx.execute("DELETE FROM locations WHERE path = ?1", params![path_key])?;
        if deleted > 0 {
            summary.removed += 1;
            changelog.push(ChangeEntry {
                path: path_key.clone(),
                action: "removed",
            });
        }
    }

    // Wave 3: attach locations → projects, then merge overlapping remote sets.
    sync_project_identity(&tx)?;

    for entry in &changelog {
        tx.execute(
            "INSERT INTO local_catalog_sync_changes (run_id, path, action) VALUES (?1, ?2, ?3)",
            params![run_id, entry.path, entry.action],
        )?;
    }

    finish_sync_run(&tx, run_id, "ok", &summary, None)?;
    tx.commit()?;
    Ok(summary)
}

/// Phase 3 prep: paths for git refresh (read connection).
pub fn local_catalog_git_paths(conn: &Connection) -> Result<Vec<PathBuf>> {
    let mut stmt = conn.prepare("SELECT path FROM locations WHERE excluded = 0")?;
    Ok(stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .map(PathBuf::from)
        .collect())
}

/// Phase 4: persist rayon git refresh results (write connection, one transaction).
pub fn persist_local_catalog_git_phase(
    conn: &Connection,
    config: &Config,
    summary: &mut LocalCatalogSyncSummary,
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
            "UPDATE locations SET branch=?1, is_dirty=?2, ahead=?3, behind=?4,
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
    crate::services::repo_convert::persist_all_structural_preflight(&git_tx, config)?;
    // Wave 3: replace worktrees/branches after fetch+HEAD query; prune empty projects.
    persist_location_git_graph(&git_tx)?;
    prune_empty_projects(&git_tx)?;
    if let Err(e) = git_tx.commit() {
        log::warn!("git refresh commit failed after successful local catalog sync merge: {e}");
        summary.git_errors = summary.git_errors.saturating_add(summary.git_refreshed);
        summary.git_refreshed = 0;
    }
    Ok(())
}

/// Phased local catalog sync: release locks between discovery, merge, git refresh, and persist.
pub fn run_phased(
    pool: &crate::infra::db::DbPool,
    config: &Config,
) -> Result<LocalCatalogSyncSummary> {
    let started_at = crate::services::git_state::unix_now_secs();
    run_local_catalog_sync_with_audit(
        "run_phased",
        started_at,
        || {
            run_local_catalog_sync_pipeline(
                &config.fetch,
                || pool.with_read(|conn| discover_phase(conn, config)),
                |plan| pool.with_write(|conn| merge_catalog_phase(conn, config, plan)),
                || pool.with_read(local_catalog_git_paths),
                |summary, git_results| {
                    pool.with_write(|conn| {
                        persist_local_catalog_git_phase(conn, config, summary, git_results)
                    })
                },
            )
        },
        |e| pool.with_write(|conn| record_error_run(conn, started_at, e)),
    )
}

fn test_local_catalog_sync_delay() {
    if let Ok(ms) = std::env::var("WORKPOT_TEST_LOCAL_CATALOG_SYNC_DELAY_MS")
        && let Ok(ms) = ms.parse::<u64>()
    {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }
}

/// Full watch-root rescan with transactional merge, caps, and audit history (D-07, D-14–D-18).
pub fn run_full(
    pool: &crate::infra::db::DbPool,
    config: &Config,
) -> Result<LocalCatalogSyncSummary> {
    run_phased(pool, config)
}

/// Single-connection variant for unit tests.
pub fn run_full_connection(conn: &Connection, config: &Config) -> Result<LocalCatalogSyncSummary> {
    let started_at = crate::services::git_state::unix_now_secs();
    run_local_catalog_sync_with_audit(
        "run_full",
        started_at,
        || {
            run_local_catalog_sync_pipeline(
                &config.fetch,
                || discover_phase(conn, config),
                |plan| merge_catalog_phase(conn, config, plan),
                || local_catalog_git_paths(conn),
                |summary, git_results| {
                    persist_local_catalog_git_phase(conn, config, summary, git_results)
                },
            )
        },
        |e| record_error_run(conn, started_at, e),
    )
}

fn run_local_catalog_sync_with_audit(
    label: &str,
    _started_at: i64,
    run: impl FnOnce() -> Result<LocalCatalogSyncSummary>,
    record_error: impl FnOnce(&WorkpotError) -> Result<()>,
) -> Result<LocalCatalogSyncSummary> {
    log::debug!("local catalog sync {label}: start");
    match run() {
        Ok(summary) => {
            log::debug!(
                "local catalog sync {label}: complete added={} removed={} skipped={} git_refreshed={} git_errors={}",
                summary.added,
                summary.removed,
                summary.skipped,
                summary.git_refreshed,
                summary.git_errors
            );
            Ok(summary)
        }
        Err(WorkpotError::LocalCatalogSyncCapExceeded { projected, max }) => {
            Err(WorkpotError::LocalCatalogSyncCapExceeded { projected, max })
        }
        Err(e) => {
            if let Err(audit_err) = record_error(&e) {
                log::warn!("failed to record error audit row: {audit_err}");
            }
            Err(e)
        }
    }
}

fn run_local_catalog_sync_pipeline(
    fetch_cmd: &str,
    discover: impl FnOnce() -> Result<DiscoveryPlan>,
    merge: impl FnOnce(DiscoveryPlan) -> Result<LocalCatalogSyncSummary>,
    git_paths: impl FnOnce() -> Result<Vec<PathBuf>>,
    persist: impl FnOnce(
        &mut LocalCatalogSyncSummary,
        Vec<crate::services::git_state::GitRefreshResult>,
    ) -> Result<()>,
) -> Result<LocalCatalogSyncSummary> {
    test_local_catalog_sync_delay();
    let plan = discover()?;
    let mut summary = merge(plan)?;
    let all_paths = git_paths()?;
    let git_results = refresh_git_states(all_paths, fetch_cmd);
    test_local_catalog_sync_delay();
    persist(&mut summary, git_results)?;
    Ok(summary)
}

fn refresh_git_states(
    all_paths: Vec<PathBuf>,
    fetch_cmd: &str,
) -> Vec<crate::services::git_state::GitRefreshResult> {
    log::debug!(
        "local catalog sync git second pass: start repos={}",
        all_paths.len()
    );
    let git_pass_started = std::time::Instant::now();
    let git_results = git_state::refresh_all(all_paths, fetch_cmd);
    log::debug!(
        "local catalog sync git second pass: refresh_all elapsed_ms={}",
        git_pass_started.elapsed().as_millis()
    );
    git_results
}

fn try_resolve_git_common_dir(path: &Path, path_key: &str) -> std::result::Result<String, ()> {
    match resolve_git_common_dir(path) {
        Ok(common) => Ok(common.display().to_string()),
        Err(_) => {
            log::warn!("skip {path_key}: git unavailable");
            Err(())
        }
    }
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
        "SELECT path FROM locations WHERE git_common_dir = '' OR git_common_dir IS NULL",
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
        match try_resolve_git_common_dir(path, &path_key) {
            Ok(common_str) => {
                conn.execute(
                    "UPDATE locations SET git_common_dir = ?1 WHERE path = ?2",
                    params![common_str, path_key],
                )?;
            }
            Err(()) => {
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
    let mut stmt = conn.prepare("SELECT path FROM locations WHERE source = ?1 AND excluded = 0")?;
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
        // Root still configured but not scannable this run — preserve cataloged repos.
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
    let mut stmt = conn.prepare("SELECT path FROM locations WHERE excluded = 0")?;
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
        "INSERT INTO local_catalog_sync_runs (started_at, finished_at, status, added_count, removed_count, skipped_count, message)
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
        "INSERT INTO local_catalog_sync_runs (started_at, finished_at, status, added_count, removed_count, skipped_count, message)
         VALUES (?1, ?2, 'cap_exceeded', 0, 0, 0, ?3)",
        params![started_at, finished_at, message],
    )?;
    Ok(())
}

fn insert_sync_run(tx: &Transaction<'_>, started_at: i64) -> Result<i64> {
    tx.execute(
        "INSERT INTO local_catalog_sync_runs (started_at, status) VALUES (?1, 'ok')",
        params![started_at],
    )?;
    Ok(tx.last_insert_rowid())
}

fn finish_sync_run(
    tx: &Transaction<'_>,
    run_id: i64,
    status: &str,
    summary: &LocalCatalogSyncSummary,
    message: Option<&str>,
) -> Result<()> {
    let finished_at = crate::services::git_state::unix_now_secs();
    tx.execute(
        "UPDATE local_catalog_sync_runs SET finished_at = ?1, status = ?2, added_count = ?3, removed_count = ?4, skipped_count = ?5, message = ?6
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

/// Attach every location to a project (create/attach/conflict), then apply overlap merges (4A).
fn sync_project_identity(tx: &Transaction<'_>) -> Result<()> {
    let locations = load_location_paths(tx)?;
    for (path_key, git_common_dir) in &locations {
        if let Err(e) = attach_one_location(tx, path_key, git_common_dir) {
            log::warn!("project attach failed for {path_key}: {e}");
        }
    }

    let snapshots = load_project_snapshots(tx)?;
    for action in merge_overlapping_projects(&snapshots) {
        if let Err(e) = apply_merge_action(tx, &action) {
            log::warn!("project merge failed survivor={}: {e}", action.survivor_id);
        }
    }

    prune_empty_projects(tx)?;
    Ok(())
}

fn load_location_paths(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT path, git_common_dir FROM locations WHERE excluded = 0")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn remotes_for_path(path_key: &str) -> Vec<RemoteRef> {
    match git::list_remotes(Path::new(path_key)) {
        Ok(remotes) => remotes
            .into_iter()
            .map(|r| RemoteRef {
                name: r.name,
                url_raw: r.url,
            })
            .collect(),
        Err(e) => {
            log::warn!("list_remotes {path_key}: {e}");
            Vec::new()
        }
    }
}

fn attach_one_location(tx: &Transaction<'_>, path_key: &str, git_common_dir: &str) -> Result<()> {
    let remotes = remotes_for_path(path_key);
    let snapshots = load_project_snapshots(tx)?;
    let result = attach_location(&remotes, git_common_dir, &snapshots);

    let project_id = match result {
        AttachResult::Create {
            project_id,
            root_remote_normalized,
            root_remote_raw,
            root_remote_name,
            remotes_raw,
            ..
        } => {
            insert_project(
                tx,
                &project_id,
                &root_remote_normalized,
                &root_remote_raw,
                path_key,
            )?;
            insert_project_remotes_on_create(
                tx,
                &project_id,
                &root_remote_normalized,
                &root_remote_raw,
                &root_remote_name,
                &remotes_raw,
            )?;
            project_id
        }
        AttachResult::Attach { project_id } => {
            upsert_fork_remotes(tx, &project_id, &remotes)?;
            project_id
        }
        AttachResult::Conflict {
            winner_id,
            candidates,
        } => {
            log::warn!(
                "project attach conflict for {path_key}: winner={winner_id} candidates={candidates:?} (5A, no merge)"
            );
            upsert_fork_remotes(tx, &winner_id, &remotes)?;
            winner_id
        }
    };

    tx.execute(
        "UPDATE locations SET project_id = ?1 WHERE path = ?2",
        params![project_id, path_key],
    )?;
    Ok(())
}

fn insert_project(
    tx: &Transaction<'_>,
    project_id: &str,
    root_remote_normalized: &str,
    root_remote_raw: &str,
    location_path: &str,
) -> Result<()> {
    let name = Path::new(location_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string();
    let created_at = crate::services::git_state::unix_now_secs();
    tx.execute(
        "INSERT OR IGNORE INTO projects (id, root_remote_normalized, root_remote_raw, name, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            project_id,
            root_remote_normalized,
            root_remote_raw,
            name,
            created_at
        ],
    )?;
    Ok(())
}

fn insert_project_remotes_on_create(
    tx: &Transaction<'_>,
    project_id: &str,
    root_normalized: &str,
    root_raw: &str,
    root_remote_name: &str,
    remotes_raw: &[(String, String)],
) -> Result<()> {
    tx.execute(
        "INSERT OR IGNORE INTO project_remotes (project_id, url_normalized, url_raw, role, remote_name)
         VALUES (?1, ?2, ?3, 'root', ?4)",
        params![project_id, root_normalized, root_raw, root_remote_name],
    )?;

    for (name, url_raw) in remotes_raw {
        let Some(normalized) = normalize_remote_url(url_raw) else {
            continue;
        };
        if normalized == root_normalized {
            // Already inserted as root; keep remote_name if this is the elected remote.
            continue;
        }
        tx.execute(
            "INSERT OR IGNORE INTO project_remotes (project_id, url_normalized, url_raw, role, remote_name)
             VALUES (?1, ?2, ?3, 'fork', ?4)",
            params![project_id, normalized, url_raw, name],
        )?;
    }
    Ok(())
}

fn upsert_fork_remotes(
    tx: &Transaction<'_>,
    project_id: &str,
    remotes: &[RemoteRef],
) -> Result<()> {
    let root_normalized: Option<String> = tx
        .query_row(
            "SELECT root_remote_normalized FROM projects WHERE id = ?1",
            params![project_id],
            |row| row.get(0),
        )
        .optional()?;

    let Some(root_normalized) = root_normalized else {
        log::warn!("upsert_fork_remotes: missing project {project_id}");
        return Ok(());
    };

    for remote in remotes {
        let Some(normalized) = normalize_remote_url(&remote.url_raw) else {
            continue;
        };
        if normalized == root_normalized {
            // Do not demote or duplicate root; ensure a root row exists.
            tx.execute(
                "INSERT OR IGNORE INTO project_remotes (project_id, url_normalized, url_raw, role, remote_name)
                 VALUES (?1, ?2, ?3, 'root', ?4)",
                params![project_id, normalized, remote.url_raw, remote.name],
            )?;
            continue;
        }
        tx.execute(
            "INSERT OR IGNORE INTO project_remotes (project_id, url_normalized, url_raw, role, remote_name)
             VALUES (?1, ?2, ?3, 'fork', ?4)",
            params![project_id, normalized, remote.url_raw, remote.name],
        )?;
    }
    Ok(())
}

fn load_project_snapshots(conn: &Connection) -> Result<Vec<ProjectSnapshot>> {
    let mut projects_stmt = conn.prepare(
        "SELECT id, root_remote_normalized, created_at FROM projects ORDER BY created_at, id",
    )?;
    let project_rows: Vec<(String, String, i64)> = projects_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<std::result::Result<_, _>>()?;

    let mut remotes_by_project: HashMap<String, HashSet<String>> = HashMap::new();
    let mut root_via_upstream: HashMap<String, bool> = HashMap::new();
    let mut remotes_stmt =
        conn.prepare("SELECT project_id, url_normalized, role, remote_name FROM project_remotes")?;
    let remote_rows = remotes_stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;
    for row in remote_rows {
        let (project_id, url_normalized, role, remote_name) = row?;
        remotes_by_project
            .entry(project_id.clone())
            .or_default()
            .insert(url_normalized);
        if role == "root" {
            let via = remote_name.as_deref() == Some("upstream");
            root_via_upstream.insert(project_id, via);
        }
    }

    let mut location_counts: HashMap<String, usize> = HashMap::new();
    let mut count_stmt = conn.prepare(
        "SELECT project_id, COUNT(*) FROM locations
         WHERE project_id IS NOT NULL AND excluded = 0
         GROUP BY project_id",
    )?;
    let count_rows = count_stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    for row in count_rows {
        let (project_id, count) = row?;
        location_counts.insert(project_id, usize::try_from(count).unwrap_or(usize::MAX));
    }

    let mut snapshots = Vec::with_capacity(project_rows.len());
    for (id, root_remote_normalized, created_at) in project_rows {
        let mut remotes_normalized = remotes_by_project.remove(&id).unwrap_or_default();
        remotes_normalized.insert(root_remote_normalized.clone());
        let via = root_via_upstream.get(&id).copied().unwrap_or(false);
        let location_count = location_counts.get(&id).copied().unwrap_or(0);
        snapshots.push(ProjectSnapshot {
            id,
            root_remote_normalized,
            remotes_normalized,
            location_count,
            created_at,
            root_via_upstream: via,
        });
    }
    Ok(snapshots)
}

fn apply_merge_action(tx: &Transaction<'_>, action: &MergeAction) -> Result<()> {
    ensure_survivor_row(tx, action)?;

    let mut source_ids = action.absorbed_ids.clone();
    if action.prior_survivor_id != action.survivor_id {
        source_ids.push(action.prior_survivor_id.clone());
    }

    for src in &source_ids {
        if src == &action.survivor_id {
            continue;
        }
        migrate_project_remotes(tx, src, &action.survivor_id)?;
        tx.execute(
            "UPDATE locations SET project_id = ?1 WHERE project_id = ?2",
            params![action.survivor_id, src],
        )?;
        tx.execute("DELETE FROM projects WHERE id = ?1", params![src])?;
    }

    // Keep survivor root metadata sticky to the elected survivor root.
    tx.execute(
        "UPDATE projects SET root_remote_normalized = ?1 WHERE id = ?2",
        params![action.root_remote_normalized, action.survivor_id],
    )?;
    Ok(())
}

fn ensure_survivor_row(tx: &Transaction<'_>, action: &MergeAction) -> Result<()> {
    let exists: Option<i64> = tx
        .query_row(
            "SELECT 1 FROM projects WHERE id = ?1",
            params![action.survivor_id],
            |row| row.get(0),
        )
        .optional()?;
    if exists.is_some() {
        return Ok(());
    }

    // Id rewrite: clone prior survivor under the canonical project_id_for_root id.
    let prior = &action.prior_survivor_id;
    let (root_raw, name): (Option<String>, Option<String>) = tx
        .query_row(
            "SELECT root_remote_raw, name FROM projects WHERE id = ?1",
            params![prior],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .unwrap_or((None, None));

    tx.execute(
        "INSERT INTO projects (id, root_remote_normalized, root_remote_raw, name, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            action.survivor_id,
            action.root_remote_normalized,
            root_raw,
            name.unwrap_or_else(|| "project".to_string()),
            action.created_at
        ],
    )?;
    Ok(())
}

fn migrate_project_remotes(tx: &Transaction<'_>, from_id: &str, to_id: &str) -> Result<()> {
    // Drop remotes on the source that would collide on (project_id, url_raw) after move.
    tx.execute(
        "DELETE FROM project_remotes
         WHERE project_id = ?1
           AND url_raw IN (SELECT url_raw FROM project_remotes WHERE project_id = ?2)",
        params![from_id, to_id],
    )?;
    // Source root remotes become forks on the survivor unless they match survivor root.
    let survivor_root: String = tx.query_row(
        "SELECT root_remote_normalized FROM projects WHERE id = ?1",
        params![to_id],
        |row| row.get(0),
    )?;
    tx.execute(
        "UPDATE project_remotes
         SET role = CASE WHEN url_normalized = ?1 THEN 'root' ELSE 'fork' END
         WHERE project_id = ?2",
        params![survivor_root, from_id],
    )?;
    tx.execute(
        "UPDATE project_remotes SET project_id = ?1 WHERE project_id = ?2",
        params![to_id, from_id],
    )?;
    Ok(())
}

fn prune_empty_projects(conn: &Connection) -> Result<()> {
    conn.execute(
        "DELETE FROM projects
         WHERE id NOT IN (
           SELECT DISTINCT project_id FROM locations
           WHERE project_id IS NOT NULL
         )",
        [],
    )?;
    Ok(())
}

fn persist_location_git_graph(tx: &Transaction<'_>) -> Result<()> {
    let paths = load_location_paths(tx)?;
    for (path_key, _) in paths {
        if let Err(e) = replace_worktrees(tx, &path_key) {
            log::warn!("replace worktrees for {path_key}: {e}");
        }
        if let Err(e) = replace_branches(tx, &path_key) {
            log::warn!("replace branches for {path_key}: {e}");
        }
    }
    Ok(())
}

fn replace_worktrees(tx: &Transaction<'_>, location_path: &str) -> Result<()> {
    tx.execute(
        "DELETE FROM worktrees WHERE location_path = ?1",
        params![location_path],
    )?;
    let rows = match git::list_worktree_rows(Path::new(location_path)) {
        Ok(rows) => rows,
        Err(e) => {
            log::warn!("list_worktree_rows {location_path}: {e}");
            return Ok(());
        }
    };
    for (wt_path, head_branch) in rows {
        tx.execute(
            "INSERT OR REPLACE INTO worktrees (path, location_path, head_branch) VALUES (?1, ?2, ?3)",
            params![wt_path.display().to_string(), location_path, head_branch],
        )?;
    }
    Ok(())
}

fn replace_branches(tx: &Transaction<'_>, location_path: &str) -> Result<()> {
    tx.execute(
        "DELETE FROM branches WHERE location_path = ?1",
        params![location_path],
    )?;
    let refs = match git::list_branch_refs(Path::new(location_path)) {
        Ok(refs) => refs,
        Err(e) => {
            log::warn!("list_branch_refs {location_path}: {e}");
            return Ok(());
        }
    };
    for branch in refs {
        tx.execute(
            "INSERT OR IGNORE INTO branches (location_path, name, kind, tip_oid, remote_name)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                location_path,
                branch.name,
                branch.kind.as_str(),
                branch.tip_oid,
                branch.remote_name,
            ],
        )?;
    }
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
            "INSERT INTO locations (path, name, registered_at, source, git_common_dir, excluded)
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
