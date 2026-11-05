//! Read-side queries for the remote catalog (`remote_repos` / `remote_sync_runs`).

use crate::domain::Config;
use crate::domain::config::{ClonePolicy, RemoteProvider, RemoteSource};
use crate::error::{Result, WorkpotError};
use rusqlite::{Connection, OptionalExtension, Row, params};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// A repo is cloned when a non-excluded location is attached to a project that has the same
/// normalized remote URL. `r` is the `remote_repos` alias.
pub(crate) const CLONED_EXISTS: &str = "EXISTS (
    SELECT 1 FROM locations l
    JOIN project_remotes pr ON pr.project_id = l.project_id
    WHERE l.excluded = 0 AND pr.url_normalized = r.url_normalized
)";

const REMOTE_ROW_SELECT: &str = "SELECT r.source_id, r.full_path, r.name, r.web_url, r.ssh_url,
    r.https_url, r.url_normalized, r.default_branch, r.archived, r.description, r.clone_path,
    r.state, r.first_seen_at, r.last_seen_at";

/// Index of the column a query appends right after [`REMOTE_ROW_SELECT`] (which selects 14).
const EXTRA_COLUMN: usize = 14;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteState {
    Present,
    Gone,
}

impl RemoteState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Gone => "gone",
        }
    }
}

/// One indexed remote repository (cloned or not).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteRepoRecord {
    pub source_id: String,
    pub full_path: String,
    pub name: String,
    pub web_url: Option<String>,
    pub ssh_url: Option<String>,
    pub https_url: Option<String>,
    pub url_normalized: String,
    pub default_branch: Option<String>,
    pub archived: bool,
    pub description: Option<String>,
    /// Planned (or actual) checkout path: `<clone_root>/<path relative to the source>`.
    pub clone_path: PathBuf,
    pub state: RemoteState,
    pub first_seen_at: i64,
    pub last_seen_at: i64,
}

fn remote_record_from_row(row: &Row<'_>) -> rusqlite::Result<RemoteRepoRecord> {
    let state: String = row.get(11)?;
    Ok(RemoteRepoRecord {
        source_id: row.get(0)?,
        full_path: row.get(1)?,
        name: row.get(2)?,
        web_url: row.get(3)?,
        ssh_url: row.get(4)?,
        https_url: row.get(5)?,
        url_normalized: row.get(6)?,
        default_branch: row.get(7)?,
        archived: row.get::<_, i64>(8)? != 0,
        description: row.get(9)?,
        clone_path: PathBuf::from(row.get::<_, String>(10)?),
        state: if state == "gone" {
            RemoteState::Gone
        } else {
            RemoteState::Present
        },
        first_seen_at: row.get(12)?,
        last_seen_at: row.get(13)?,
    })
}

fn hidden_by_archive_filter(config: &Config, record: &RemoteRepoRecord) -> bool {
    record.archived
        && config
            .remote(&record.source_id)
            .is_some_and(|source| !source.include_archived)
}

/// Present remote repos that are not cloned and belong to a configured source.
pub fn list_remote_only(conn: &Connection, config: &Config) -> Result<Vec<RemoteRepoRecord>> {
    let sql = format!(
        "{REMOTE_ROW_SELECT} FROM remote_repos r
         WHERE r.state = 'present' AND NOT {CLONED_EXISTS}
         ORDER BY r.source_id, r.full_path COLLATE NOCASE"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], remote_record_from_row)?;
    let mut out = Vec::new();
    for row in rows {
        let record = row.map_err(WorkpotError::Database)?;
        if config.remote(&record.source_id).is_none() || hidden_by_archive_filter(config, &record) {
            continue;
        }
        out.push(record);
    }
    Ok(out)
}

/// Remote record whose planned clone path is `path`, if any.
pub(crate) fn find_by_clone_path(
    conn: &Connection,
    path: &Path,
) -> Result<Option<RemoteRepoRecord>> {
    let key = path.display().to_string();
    let sql = format!("{REMOTE_ROW_SELECT} FROM remote_repos r WHERE r.clone_path = ?1");
    conn.query_row(&sql, params![key], remote_record_from_row)
        .optional()
        .map_err(WorkpotError::Database)
}

/// Remote record by `(source, full_path)`.
fn find_remote_repo(
    conn: &Connection,
    source_id: &str,
    full_path: &str,
) -> Result<Option<RemoteRepoRecord>> {
    let sql = format!(
        "{REMOTE_ROW_SELECT} FROM remote_repos r WHERE r.source_id = ?1 AND r.full_path = ?2"
    );
    conn.query_row(&sql, params![source_id, full_path], remote_record_from_row)
        .optional()
        .map_err(WorkpotError::Database)
}

/// Resolve a user-supplied identifier (clone path, `source:full_path`, `full_path`, or repo
/// name) to a remote record. Ambiguous names are rejected.
pub(crate) fn resolve_remote_identifier(
    conn: &Connection,
    identifier: &str,
) -> Result<Option<RemoteRepoRecord>> {
    let needle = identifier.trim();
    if needle.is_empty() {
        return Ok(None);
    }
    if let Some(found) = find_by_clone_path(conn, Path::new(needle))? {
        return Ok(Some(found));
    }
    if let Some((source, path)) = needle.split_once(':')
        && let Some(found) = find_remote_repo(conn, source, path)?
    {
        return Ok(Some(found));
    }
    let sql = format!(
        "{REMOTE_ROW_SELECT} FROM remote_repos r
         WHERE r.state = 'present'
           AND (r.full_path = ?1 COLLATE NOCASE OR r.name = ?1 COLLATE NOCASE)"
    );
    let mut stmt = conn.prepare(&sql)?;
    let matches: Vec<RemoteRepoRecord> = stmt
        .query_map(params![needle], remote_record_from_row)?
        .collect::<std::result::Result<_, _>>()?;
    match matches.len() {
        0 => Ok(None),
        1 => Ok(matches.into_iter().next()),
        n => Err(WorkpotError::InvalidInput(format!(
            "{needle:?} matches {n} remote repos; use `source:full_path`"
        ))),
    }
}

/// Whether a remote record currently has a cataloged local clone.
pub(crate) fn is_cloned(conn: &Connection, record: &RemoteRepoRecord) -> Result<bool> {
    let sql = format!(
        "SELECT {CLONED_EXISTS} FROM remote_repos r WHERE r.source_id = ?1 AND r.full_path = ?2"
    );
    conn.query_row(&sql, params![record.source_id, record.full_path], |row| {
        row.get::<_, bool>(0)
    })
    .optional()
    .map(|found| found.unwrap_or(false))
    .map_err(WorkpotError::Database)
}

/// Catalog `locations.path` of the local clone for a remote record, if cloned.
pub(crate) fn local_path_for(
    conn: &Connection,
    record: &RemoteRepoRecord,
) -> Result<Option<String>> {
    conn.query_row(
        "SELECT l.path FROM locations l
         JOIN project_remotes pr ON pr.project_id = l.project_id
         WHERE l.excluded = 0 AND pr.url_normalized = ?1
         ORDER BY (l.path = ?2) DESC, l.registered_at LIMIT 1",
        params![
            record.url_normalized,
            record.clone_path.display().to_string()
        ],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(WorkpotError::Database)
}

/// Cloned remote records whose upstream repo is gone (candidates for prune / UI flag).
pub(crate) fn list_gone_cloned(conn: &Connection) -> Result<Vec<(RemoteRepoRecord, String)>> {
    let sql = format!(
        "{REMOTE_ROW_SELECT}, (
            SELECT l.path FROM locations l
            JOIN project_remotes pr ON pr.project_id = l.project_id
            WHERE l.excluded = 0 AND pr.url_normalized = r.url_normalized
            ORDER BY (l.path = r.clone_path) DESC, l.registered_at LIMIT 1
         ) AS local_path
         FROM remote_repos r
         WHERE r.state = 'gone' AND {CLONED_EXISTS}
         ORDER BY r.source_id, r.full_path"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        Ok((
            remote_record_from_row(row)?,
            row.get::<_, String>(EXTRA_COLUMN)?,
        ))
    })?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(WorkpotError::Database)
}

/// Local clone paths whose upstream repo no longer exists (tray "removed upstream" flag).
pub fn removed_upstream_paths(conn: &Connection) -> Result<HashSet<String>> {
    Ok(list_gone_cloned(conn)?
        .into_iter()
        .map(|(_, path)| path)
        .collect())
}

/// Remote record + cloned flag, for every record of a source (used by auto-clone).
pub(crate) fn list_present_for_source(
    conn: &Connection,
    source_id: &str,
) -> Result<Vec<(RemoteRepoRecord, bool)>> {
    let sql = format!(
        "{REMOTE_ROW_SELECT}, {CLONED_EXISTS} FROM remote_repos r
         WHERE r.source_id = ?1 AND r.state = 'present' ORDER BY r.full_path"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![source_id], |row| {
        Ok((
            remote_record_from_row(row)?,
            row.get::<_, bool>(EXTRA_COLUMN)?,
        ))
    })?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(WorkpotError::Database)
}

/// Status of one configured source for `workpot remotes list` and the tray.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteSourceStatus {
    pub name: String,
    pub provider: RemoteProvider,
    pub host: String,
    pub path: String,
    pub clone_root: PathBuf,
    pub clone_policy: ClonePolicy,
    pub prune_removed: bool,
    pub repo_count: u32,
    pub cloned_count: u32,
    pub gone_count: u32,
    pub last_run_status: Option<String>,
    pub last_run_at: Option<i64>,
    pub last_run_message: Option<String>,
}

pub(crate) fn source_statuses(
    conn: &Connection,
    config: &Config,
) -> Result<Vec<RemoteSourceStatus>> {
    config
        .remotes
        .iter()
        .map(|source| source_status(conn, source))
        .collect()
}

fn source_status(conn: &Connection, source: &RemoteSource) -> Result<RemoteSourceStatus> {
    let counts_sql = format!(
        "SELECT
            COALESCE(SUM(r.state = 'present'), 0),
            COALESCE(SUM(r.state = 'present' AND {CLONED_EXISTS}), 0),
            COALESCE(SUM(r.state = 'gone'), 0)
         FROM remote_repos r WHERE r.source_id = ?1"
    );
    let (repo_count, cloned_count, gone_count): (i64, i64, i64) =
        conn.query_row(&counts_sql, params![source.name], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
    let last: Option<(String, Option<i64>, Option<String>)> = conn
        .query_row(
            "SELECT status, COALESCE(finished_at, started_at), message
             FROM remote_sync_runs WHERE source_id = ?1 ORDER BY id DESC LIMIT 1",
            params![source.name],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let to_u32 = |n: i64| u32::try_from(n).unwrap_or(u32::MAX);
    Ok(RemoteSourceStatus {
        name: source.name.clone(),
        provider: source.provider,
        host: source.effective_host().to_string(),
        path: source.trimmed_path().to_string(),
        clone_root: source.resolved_clone_root(),
        clone_policy: source.clone_policy,
        prune_removed: source.prune_removed,
        repo_count: to_u32(repo_count),
        cloned_count: to_u32(cloned_count),
        gone_count: to_u32(gone_count),
        last_run_status: last.as_ref().map(|l| l.0.clone()),
        last_run_at: last.as_ref().and_then(|l| l.1),
        last_run_message: last.and_then(|l| l.2),
    })
}
