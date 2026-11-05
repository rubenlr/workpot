//! `sync-remote-catalog`: index GitHub/GitLab groups into `remote_repos`.
//!
//! Network listing runs with no DB lock held; results are applied in one write transaction.
//! Source failures are soft: they are logged, recorded on `remote_sync_runs`, and the other
//! sources (and the local catalog sync) continue.

use crate::domain::Config;
use crate::domain::config::RemoteSource;
use crate::domain::normalize_remote_url;
use crate::error::{Result, WorkpotError};
use crate::infra::db::DbPool;
use crate::infra::remote_provider::{RemoteApi, RemoteRepoInfo};
use crate::services::git_state::unix_now_secs;
use crate::services::{remote_clone, remote_prune};
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteSourceError {
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemoteSyncSummary {
    /// Configured sources.
    pub sources: u32,
    /// Sources skipped because their last successful listing is within the TTL.
    pub skipped_ttl: u32,
    /// Repos currently indexed as present (across listed sources).
    pub indexed: u32,
    pub added: u32,
    /// Repos newly marked gone (absent from a complete listing).
    pub gone: u32,
    pub cloned: u32,
    pub pruned: u32,
    pub errors: u32,
    pub source_errors: Vec<RemoteSourceError>,
}

/// Checkout path for `full_path`: `<clone_root>/<path relative to the source path>`.
///
/// `None` when a segment is unsafe (`..`, empty, backslash, NUL), so a hostile or odd remote
/// path can never escape `clone_root`.
pub fn clone_path_for(source: &RemoteSource, full_path: &str) -> Option<PathBuf> {
    let base = source.trimmed_path();
    let full = full_path.trim().trim_matches('/');
    let relative = match full.get(..base.len()) {
        Some(head)
            if head.eq_ignore_ascii_case(base)
                && full[base.len()..].starts_with('/')
                && full.len() > base.len() + 1 =>
        {
            &full[base.len() + 1..]
        }
        _ => full,
    };
    let mut path = source.resolved_clone_root();
    for segment in relative.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." || segment.contains(['\\', '\0'])
        {
            return None;
        }
        path.push(segment);
    }
    Some(path)
}

fn url_key(info: &RemoteRepoInfo) -> Option<String> {
    [&info.https_url, &info.ssh_url, &info.web_url]
        .into_iter()
        .flatten()
        .find_map(|url| normalize_remote_url(url))
}

/// Index every configured source. `force` bypasses the TTL.
pub fn run_remote_sync(
    pool: &DbPool,
    config: &Config,
    api: &dyn RemoteApi,
    force: bool,
) -> Result<RemoteSyncSummary> {
    let mut summary = RemoteSyncSummary {
        sources: u32::try_from(config.remotes.len()).unwrap_or(u32::MAX),
        ..RemoteSyncSummary::default()
    };

    pool.with_write(|conn| purge_unconfigured_sources(conn, config))?;

    for source in &config.remotes {
        if !force && within_ttl(pool, config, source)? {
            summary.skipped_ttl += 1;
            continue;
        }
        let started_at = unix_now_secs();
        match sync_source(pool, config, api, source, started_at) {
            Ok(outcome) => {
                summary.indexed += outcome.indexed;
                summary.added += outcome.added;
                summary.gone += outcome.gone;

                let pruned = remote_prune::prune_source(pool, api, source);
                summary.pruned += pruned.pruned;

                let (cloned, clone_errors) =
                    remote_clone::auto_clone_source(pool, config, api, source);
                summary.cloned += cloned;
                for message in clone_errors {
                    summary.errors += 1;
                    summary.source_errors.push(RemoteSourceError {
                        source: source.name.clone(),
                        message,
                    });
                }
                if let Err(e) = pool.with_write(|conn| {
                    record_post_index(conn, &source.name, started_at, cloned, pruned.pruned)
                }) {
                    log::warn!(
                        "remote sync {}: failed to record clone/prune counts: {e}",
                        source.name
                    );
                }
            }
            Err(e) => {
                log::warn!("remote sync {}: {e}", source.name);
                if let Err(audit) = pool.with_write(|conn| {
                    record_error_run(conn, &source.name, started_at, &e.to_string())
                }) {
                    log::warn!(
                        "remote sync {}: failed to record error run: {audit}",
                        source.name
                    );
                }
                summary.errors += 1;
                summary.source_errors.push(RemoteSourceError {
                    source: source.name.clone(),
                    message: e.to_string(),
                });
            }
        }
    }
    if let Err(e) = pool.with_write(purge_gone_uncloned) {
        log::warn!("remote sync: failed to purge stale gone rows: {e}");
    }
    Ok(summary)
}

/// Attach clone/prune counts to the run row written by [`apply_listing`].
fn record_post_index(
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
fn purge_gone_uncloned(conn: &Connection) -> Result<()> {
    conn.execute(
        "DELETE FROM remote_repos AS r WHERE r.state = 'gone' AND NOT EXISTS (
            SELECT 1 FROM locations l
            JOIN project_remotes pr ON pr.project_id = l.project_id
            WHERE l.excluded = 0 AND pr.url_normalized = r.url_normalized
         )",
        [],
    )?;
    Ok(())
}

fn purge_unconfigured_sources(conn: &Connection, config: &Config) -> Result<()> {
    let known: HashSet<&str> = config.remotes.iter().map(|r| r.name.as_str()).collect();
    let mut stmt = conn.prepare("SELECT DISTINCT source_id FROM remote_repos")?;
    let stale: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
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

fn within_ttl(pool: &DbPool, config: &Config, source: &RemoteSource) -> Result<bool> {
    let ttl_secs = i64::from(config.remote_sync_ttl_minutes) * 60;
    if ttl_secs == 0 {
        return Ok(false);
    }
    let last_ok: Option<i64> = pool.with_read(|conn| {
        conn.query_row(
            "SELECT finished_at FROM remote_sync_runs
             WHERE source_id = ?1 AND status = 'ok' ORDER BY id DESC LIMIT 1",
            params![source.name],
            |row| row.get(0),
        )
        .optional()
        .map(Option::flatten)
        .map_err(WorkpotError::Database)
    })?;
    Ok(last_ok.is_some_and(|finished| unix_now_secs() - finished < ttl_secs))
}

#[derive(Debug, Default)]
struct SourceOutcome {
    indexed: u32,
    added: u32,
    gone: u32,
}

struct IndexedRepo {
    info: RemoteRepoInfo,
    clone_path: PathBuf,
    url_normalized: String,
}

fn sync_source(
    pool: &DbPool,
    config: &Config,
    api: &dyn RemoteApi,
    source: &RemoteSource,
    started_at: i64,
) -> Result<SourceOutcome> {
    // Network: no DB lock held.
    let listing = api.list_repos(source)?;

    let mut seen: HashSet<String> = HashSet::new();
    let mut repos: Vec<IndexedRepo> = Vec::new();
    for info in listing {
        if !seen.insert(info.full_path.clone()) {
            continue;
        }
        let Some(clone_path) = clone_path_for(source, &info.full_path) else {
            log::warn!(
                "remote sync {}: skipping {:?} (unsafe path segment)",
                source.name,
                info.full_path
            );
            continue;
        };
        let Some(url_normalized) = url_key(&info) else {
            log::warn!(
                "remote sync {}: skipping {:?} (no clone URL)",
                source.name,
                info.full_path
            );
            continue;
        };
        repos.push(IndexedRepo {
            info,
            clone_path,
            url_normalized,
        });
    }

    let max = config.limits.max_remote_repos;
    if repos.len() > max as usize {
        return Err(WorkpotError::LimitsExceeded(format!(
            "remote source {} lists {} repos (max_remote_repos {max})",
            source.name,
            repos.len()
        )));
    }

    pool.with_write(|conn| apply_listing(conn, source, &repos, started_at))
}

fn apply_listing(
    conn: &Connection,
    source: &RemoteSource,
    repos: &[IndexedRepo],
    started_at: i64,
) -> Result<SourceOutcome> {
    let tx = conn.unchecked_transaction()?;
    let now = unix_now_secs();

    let existing: HashSet<String> = {
        let mut stmt = tx.prepare("SELECT full_path FROM remote_repos WHERE source_id = ?1")?;
        stmt.query_map(params![source.name], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<_, _>>()?
    };
    let present_before: u32 = tx.query_row(
        "SELECT COUNT(*) FROM remote_repos WHERE source_id = ?1 AND state = 'present'",
        params![source.name],
        |row| row.get(0),
    )?;
    if repos.is_empty() && present_before > 0 {
        return Err(WorkpotError::Config(format!(
            "remote source {} returned an empty listing; keeping {present_before} indexed repos",
            source.name
        )));
    }

    let mut outcome = SourceOutcome::default();
    for repo in repos {
        let info = &repo.info;
        if !existing.contains(&info.full_path) {
            outcome.added += 1;
        }
        tx.execute(
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
    }
    outcome.indexed = u32::try_from(repos.len()).unwrap_or(u32::MAX);

    let listed: HashSet<&str> = repos.iter().map(|r| r.info.full_path.as_str()).collect();
    for path in existing.iter().filter(|p| !listed.contains(p.as_str())) {
        let changed = tx.execute(
            "UPDATE remote_repos SET state = 'gone'
             WHERE source_id = ?1 AND full_path = ?2 AND state = 'present'",
            params![source.name, path],
        )?;
        outcome.gone += u32::try_from(changed).unwrap_or(0);
    }

    tx.execute(
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
    tx.commit()?;
    Ok(outcome)
}

fn record_error_run(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::{ClonePolicy, RemoteProvider};

    fn source(provider: RemoteProvider, path: &str) -> RemoteSource {
        RemoteSource {
            name: "acme".into(),
            provider,
            host: None,
            path: path.into(),
            clone_root: "/tmp/acme-root".into(),
            clone_policy: ClonePolicy::Manual,
            prune_removed: false,
            include_archived: true,
            protocol: None,
        }
    }

    #[test]
    fn clone_path_strips_source_prefix() {
        let gl = source(RemoteProvider::Gitlab, "acme/platform");
        assert_eq!(
            clone_path_for(&gl, "acme/platform/sub/api"),
            Some(PathBuf::from("/tmp/acme-root/sub/api"))
        );
        let gh = source(RemoteProvider::Github, "Acme");
        assert_eq!(
            clone_path_for(&gh, "acme/web"),
            Some(PathBuf::from("/tmp/acme-root/web"))
        );
    }

    #[test]
    fn clone_path_keeps_full_path_when_prefix_differs() {
        let gl = source(RemoteProvider::Gitlab, "acme/platform");
        assert_eq!(
            clone_path_for(&gl, "other/thing"),
            Some(PathBuf::from("/tmp/acme-root/other/thing"))
        );
    }

    #[test]
    fn clone_path_rejects_traversal() {
        let gl = source(RemoteProvider::Gitlab, "acme");
        assert_eq!(clone_path_for(&gl, "acme/../etc"), None);
        assert_eq!(clone_path_for(&gl, "acme/a\\b"), None);
        assert_eq!(clone_path_for(&gl, "acme//x"), None);
    }
}
