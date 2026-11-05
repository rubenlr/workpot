//! Write-side SQL for the remote catalog (`remote_repos` / `remote_sync_runs`).
//!
//! Split from `remote_catalog_sync` so that module keeps orchestration (listing, TTL, prune,
//! clone) while every statement that mutates the index lives here.

use crate::domain::Config;
use crate::domain::config::RemoteSource;
use crate::error::{Result, WorkpotError};
use crate::infra::remote_provider::RemoteRepoInfo;
use crate::services::git_state::unix_now_secs;
use crate::services::remote_catalog::CLONED_EXISTS;
use rusqlite::{Connection, params};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub(crate) struct SourceOutcome {
    pub(crate) indexed: u32,
    pub(crate) added: u32,
    pub(crate) gone: u32,
}

pub(crate) struct IndexedRepo {
    pub(crate) info: RemoteRepoInfo,
    pub(crate) clone_path: PathBuf,
    pub(crate) url_normalized: String,
}

pub(crate) fn apply_listing(
    conn: &Connection,
    source: &RemoteSource,
    repos: &[IndexedRepo],
    started_at: i64,
) -> Result<SourceOutcome> {
    let tx = conn.unchecked_transaction()?;
    let now = unix_now_secs();

    let existing = indexed_full_paths(&tx, source)?;
    ensure_listing_not_empty(&tx, source, repos)?;

    let mut outcome = SourceOutcome::default();
    for repo in repos {
        if !existing.contains(&repo.info.full_path) {
            outcome.added += 1;
        }
        upsert_remote_repo(&tx, source, repo, now)?;
    }
    outcome.indexed = u32::try_from(repos.len()).unwrap_or(u32::MAX);
    outcome.gone = mark_unlisted_gone(&tx, source, repos, &existing)?;

    record_ok_run(&tx, source, started_at, &outcome)?;
    tx.commit()?;
    Ok(outcome)
}

fn indexed_full_paths(conn: &Connection, source: &RemoteSource) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT full_path FROM remote_repos WHERE source_id = ?1")?;
    let paths = stmt
        .query_map(params![source.name], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<_, _>>()?;
    Ok(paths)
}

/// An empty listing while repos are indexed is treated as a provider glitch, never as "all gone".
fn ensure_listing_not_empty(
    conn: &Connection,
    source: &RemoteSource,
    repos: &[IndexedRepo],
) -> Result<()> {
    if !repos.is_empty() {
        return Ok(());
    }
    let present_before: u32 = conn.query_row(
        "SELECT COUNT(*) FROM remote_repos WHERE source_id = ?1 AND state = 'present'",
        params![source.name],
        |row| row.get(0),
    )?;
    if present_before > 0 {
        return Err(WorkpotError::Config(format!(
            "remote source {} returned an empty listing; keeping {present_before} indexed repos",
            source.name
        )));
    }
    Ok(())
}

fn upsert_remote_repo(
    conn: &Connection,
    source: &RemoteSource,
    repo: &IndexedRepo,
    now: i64,
) -> Result<()> {
    let info = &repo.info;
    conn.execute(
        "INSERT INTO remote_repos (
            source_id, full_path, name, web_url, ssh_url, https_url, url_normalized,
            default_branch, archived, description, clone_path, state,
            first_seen_at, last_seen_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'present', ?12, ?12)
         ON CONFLICT(source_id, full_path) DO UPDATE SET
            name = excluded.name, web_url = excluded.web_url, ssh_url = excluded.ssh_url,
            https_url = excluded.https_url, url_normalized = excluded.url_normalized,
            default_branch = excluded.default_branch, archived = excluded.archived,
            description = excluded.description, clone_path = excluded.clone_path,
            state = 'present', last_seen_at = excluded.last_seen_at",
        params![
            source.name,
            info.full_path,
            info.name,
            info.web_url,
            info.ssh_url,
            info.https_url,
            repo.url_normalized,
            info.default_branch,
            i64::from(info.archived),
            info.description,
            repo.clone_path.display().to_string(),
            now,
        ],
    )?;
    Ok(())
}

/// Mark previously indexed repos that are absent from `repos` as gone; returns how many changed.
fn mark_unlisted_gone(
    conn: &Connection,
    source: &RemoteSource,
    repos: &[IndexedRepo],
    existing: &HashSet<String>,
) -> Result<u32> {
    let listed: HashSet<&str> = repos.iter().map(|r| r.info.full_path.as_str()).collect();
    let mut gone = 0;
    for path in existing.iter().filter(|p| !listed.contains(p.as_str())) {
        let changed = conn.execute(
            "UPDATE remote_repos SET state = 'gone'
             WHERE source_id = ?1 AND full_path = ?2 AND state = 'present'",
            params![source.name, path],
        )?;
        gone += u32::try_from(changed).unwrap_or(0);
    }
    Ok(gone)
}

fn record_ok_run(
    conn: &Connection,
    source: &RemoteSource,
    started_at: i64,
    outcome: &SourceOutcome,
) -> Result<()> {
    conn.execute(
        "INSERT INTO remote_sync_runs (
            source_id, started_at, finished_at, status, repo_count, new_count, gone_count
         ) VALUES (?1, ?2, ?3, 'ok', ?4, ?5, ?6)",
        params![
            source.name,
            started_at,
            unix_now_secs(),
            outcome.indexed,
            outcome.added,
            outcome.gone
        ],
    )?;
    Ok(())
}

pub(crate) fn record_error_run(
    conn: &Connection,
    source_id: &str,
    started_at: i64,
    message: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO remote_sync_runs (source_id, started_at, finished_at, status, message)
         VALUES (?1, ?2, ?3, 'error', ?4)",
        params![source_id, started_at, unix_now_secs(), message],
    )?;
    Ok(())
}

/// Attach clone/prune counts to the run row written by [`apply_listing`].
pub(crate) fn record_post_index(
    conn: &Connection,
    source_id: &str,
    started_at: i64,
    cloned: u32,
    pruned: u32,
) -> Result<()> {
    if cloned == 0 && pruned == 0 {
        return Ok(());
    }
    conn.execute(
        "UPDATE remote_sync_runs SET cloned_count = ?3, pruned_count = ?4
         WHERE id = (SELECT MAX(id) FROM remote_sync_runs WHERE source_id = ?1 AND started_at = ?2)",
        params![source_id, started_at, cloned, pruned],
    )?;
    Ok(())
}

/// Drop `gone` rows that have no local clone left (nothing to flag or prune).
pub(crate) fn purge_gone_uncloned(conn: &Connection) -> Result<()> {
    conn.execute(
        &format!("DELETE FROM remote_repos AS r WHERE r.state = 'gone' AND NOT {CLONED_EXISTS}"),
        [],
    )?;
    Ok(())
}

pub(crate) fn purge_unconfigured_sources(conn: &Connection, config: &Config) -> Result<()> {
    let known: HashSet<&str> = config.remotes.iter().map(|r| r.name.as_str()).collect();
    let mut stmt = conn.prepare("SELECT DISTINCT source_id FROM remote_repos")?;
    let stale: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|row| match row {
            Ok(id) => Some(id),
            Err(e) => {
                log::warn!("remote sync: skipping unreadable source_id row: {e}");
                None
            }
        })
        .filter(|id| !known.contains(id.as_str()))
        .collect();
    for id in stale {
        conn.execute("DELETE FROM remote_repos WHERE source_id = ?1", params![id])?;
        conn.execute(
            "DELETE FROM remote_sync_runs WHERE source_id = ?1",
            params![id],
        )?;
    }
    Ok(())
}
