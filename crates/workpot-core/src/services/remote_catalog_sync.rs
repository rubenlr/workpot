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
use crate::services::remote_catalog_store::{
    IndexedRepo, SourceOutcome, apply_listing, purge_gone_uncloned, purge_unconfigured_sources,
    record_error_run, record_post_index,
};
use crate::services::{remote_clone, remote_prune};
use rusqlite::{OptionalExtension, params};
use std::collections::HashSet;
use std::path::PathBuf;

const SECS_PER_MINUTE: i64 = 60;

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
fn clone_path_for(source: &RemoteSource, full_path: &str) -> Option<PathBuf> {
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
pub(crate) fn run_remote_sync(
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
        sync_one_source(pool, config, api, source, &mut summary);
    }
    if let Err(e) = pool.with_write(purge_gone_uncloned) {
        log::warn!("remote sync: failed to purge stale gone rows: {e}");
    }
    Ok(summary)
}

fn push_source_error(summary: &mut RemoteSyncSummary, source: &RemoteSource, message: String) {
    summary.errors += 1;
    summary.source_errors.push(RemoteSourceError {
        source: source.name.clone(),
        message,
    });
}

/// Index one source, then prune and auto-clone per its policy. Failures are folded into `summary`.
fn sync_one_source(
    pool: &DbPool,
    config: &Config,
    api: &dyn RemoteApi,
    source: &RemoteSource,
    summary: &mut RemoteSyncSummary,
) {
    let started_at = unix_now_secs();
    match sync_source(pool, config, api, source, started_at) {
        Ok(outcome) => {
            summary.indexed += outcome.indexed;
            summary.added += outcome.added;
            summary.gone += outcome.gone;
            prune_and_clone_source(pool, config, api, source, started_at, summary);
        }
        Err(e) => {
            log::warn!("remote sync {}: {e}", source.name);
            let message = e.to_string();
            if let Err(audit) =
                pool.with_write(|conn| record_error_run(conn, &source.name, started_at, &message))
            {
                log::warn!(
                    "remote sync {}: failed to record error run: {audit}",
                    source.name
                );
            }
            push_source_error(summary, source, message);
        }
    }
}

/// Post-index steps for a successfully listed source: prune removed upstreams, auto-clone, and
/// record both counts on the run row.
fn prune_and_clone_source(
    pool: &DbPool,
    config: &Config,
    api: &dyn RemoteApi,
    source: &RemoteSource,
    started_at: i64,
    summary: &mut RemoteSyncSummary,
) {
    let pruned = remote_prune::prune_source(pool, api, source);
    summary.pruned += pruned.pruned;

    let (cloned, clone_errors) = remote_clone::auto_clone_source(pool, config, api, source);
    summary.cloned += cloned;
    for message in clone_errors {
        push_source_error(summary, source, message);
    }
    if let Err(e) = pool
        .with_write(|conn| record_post_index(conn, &source.name, started_at, cloned, pruned.pruned))
    {
        log::warn!(
            "remote sync {}: failed to record clone/prune counts: {e}",
            source.name
        );
    }
}

fn within_ttl(pool: &DbPool, config: &Config, source: &RemoteSource) -> Result<bool> {
    let ttl_secs = i64::from(config.remote_sync_ttl_minutes) * SECS_PER_MINUTE;
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

/// Listing entries that are safe to index: de-duplicated, with a contained clone path and a
/// usable clone URL. Anything else is skipped with a warning.
fn indexable_repos(source: &RemoteSource, listing: Vec<RemoteRepoInfo>) -> Vec<IndexedRepo> {
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
    repos
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
    let repos = indexable_repos(source, listing);

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
