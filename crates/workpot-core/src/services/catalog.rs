use crate::domain::{Config, RepoRecord, SOURCE_MANUAL, SOURCE_SCAN};
use crate::error::{Result, WorkpotError};
use crate::infra::git::{self, resolve_git_common_dir};
use crate::save_config;
use crate::services::git_state::unix_now_secs;
use rusqlite::{Connection, Row, params};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn repo_record_from_row(row: &Row<'_>) -> rusqlite::Result<(String, RepoRecord)> {
    let path: String = row.get(0)?;
    let pinned: i64 = row.get(12)?;
    Ok((
        path.clone(),
        RepoRecord {
            path: PathBuf::from(path),
            name: row.get(1)?,
            registered_at: row.get(2)?,
            source: row.get(3)?,
            git_common_dir: row.get(4)?,
            branch: row.get(5)?,
            is_dirty: row.get::<_, Option<i64>>(6)?.map(|v| v != 0),
            ahead: row.get(7)?,
            behind: row.get(8)?,
            git_refreshed_at: row.get(9)?,
            git_state_error: row.get(10)?,
            last_opened_at: row.get(11)?,
            pinned: pinned != 0,
            pin_order: row.get(13)?,
            notes: row.get(14)?,
            tags: vec![],
            alias: row.get(15)?,
        },
    ))
}

pub fn register_manual(conn: &Connection, config: &Config, path: &Path) -> Result<RepoRecord> {
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

    if !is_git_worktree(path) && !is_bare_repo(path) {
        return Err(WorkpotError::NotGitRepo(path.to_path_buf()));
    }

    let canonical = path
        .canonicalize()
        .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", path.display())))?;

    let path_key = canonical.display().to_string();
    let name = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let count: i64 =
        conn.query_row("SELECT COUNT(*) FROM repos WHERE excluded = 0", [], |row| {
            row.get(0)
        })?;
    let count = u32::try_from(count).unwrap_or(u32::MAX);
    if count >= config.limits.max_repos {
        return Err(WorkpotError::IndexCapExceeded {
            projected: count.saturating_add(1),
            max: config.limits.max_repos,
        });
    }

    let registered_at = unix_now_secs();

    let git_common_dir = resolve_git_common_dir(&canonical)?.display().to_string();

    let rows = conn.execute(
        "INSERT INTO repos (path, name, registered_at, source, git_common_dir) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![path_key, name, registered_at, SOURCE_MANUAL, git_common_dir],
    );

    match rows {
        Ok(_) => Ok(RepoRecord {
            path: canonical,
            name,
            registered_at,
            source: SOURCE_MANUAL.to_string(),
            git_common_dir,
            branch: None,
            is_dirty: None,
            ahead: None,
            behind: None,
            git_refreshed_at: None,
            git_state_error: None,
            last_opened_at: None,
            pinned: false,
            pin_order: None,
            notes: None,
            tags: vec![],
            alias: None,
        }),
        Err(rusqlite::Error::SqliteFailure(err, _))
            if err.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            Err(WorkpotError::AlreadyRegistered(path_key))
        }
        Err(e) => Err(WorkpotError::Database(e)),
    }
}

pub fn list_repos(conn: &Connection) -> Result<Vec<RepoRecord>> {
    let mut stmt = conn.prepare(
        "SELECT path, name, registered_at, source, git_common_dir,
                branch, is_dirty, ahead, behind, git_refreshed_at, git_state_error, last_opened_at,
                pinned, pin_order, notes, alias
         FROM repos WHERE excluded = 0 ORDER BY registered_at, path",
    )?;

    let mut order: Vec<String> = Vec::new();
    let mut by_path: HashMap<String, RepoRecord> = HashMap::new();

    let rows = stmt.query_map([], repo_record_from_row)?;

    for row in rows {
        let (path, record) = row.map_err(WorkpotError::Database)?;
        order.push(path.clone());
        by_path.insert(path, record);
    }

    let mut tag_stmt = conn.prepare(
        "SELECT repo_tags.repo_path, repo_tags.tag
         FROM repo_tags
         JOIN repos ON repo_tags.repo_path = repos.path
         WHERE repos.excluded = 0
         ORDER BY repo_tags.repo_path, repo_tags.tag COLLATE NOCASE",
    )?;

    let tag_rows = tag_stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for row in tag_rows {
        let (path, tag) = row.map_err(WorkpotError::Database)?;
        if let Some(record) = by_path.get_mut(&path) {
            record.tags.push(tag);
        }
    }

    Ok(order
        .into_iter()
        .filter_map(|path| by_path.remove(&path))
        .collect())
}

/// Look up a registered repo by its SQLite path key.
pub fn get_repo_by_path(conn: &Connection, path_key: &str) -> Result<RepoRecord> {
    conn.query_row(
        "SELECT path, name, registered_at, source, git_common_dir,
                branch, is_dirty, ahead, behind, git_refreshed_at, git_state_error, last_opened_at,
                pinned, pin_order, notes, alias
         FROM repos WHERE path = ?1 AND excluded = 0",
        params![path_key],
        |row| Ok(repo_record_from_row(row)?.1),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => WorkpotError::NotFound(path_key.to_string()),
        e => WorkpotError::Database(e),
    })
}

/// Absolute path for an indexed repo launch, after validating it is in the catalog.
///
/// Bare repos resolve to a linked worktree checkout so IDE launch targets the developer
/// workspace rather than the bare object store. When the catalog row has a `branch`,
/// the worktree checked out on that branch is preferred; otherwise the first listed
/// worktree is used.
pub fn indexed_launch_path(conn: &Connection, path: &Path) -> Result<PathBuf> {
    let (repo_path, path_key) = resolve_repo_location(conn, path)?;
    let record = get_repo_by_path(conn, &path_key)?;
    if is_bare_repo(&repo_path) {
        let worktrees = git::list_worktree_paths(&repo_path)?;
        if worktrees.is_empty() {
            return Err(WorkpotError::GitUnavailable(repo_path));
        }
        let catalog_branch = record.branch;
        if let Some(branch) = catalog_branch {
            for wt in &worktrees {
                if let Ok(state) = git::open_and_query(wt)
                    && state.branch.as_deref() == Some(branch.as_str())
                {
                    return Ok(wt.clone());
                }
            }
        }
        return Ok(worktrees.into_iter().next().expect("non-empty worktrees"));
    }
    Ok(repo_path)
}

/// Record that a repo was opened from the tray (D-25).
pub fn touch_last_opened_at(conn: &Connection, path: &Path) -> Result<()> {
    let path_key = repo_path_key(conn, path)?;
    let now = crate::services::git_state::unix_now_secs();
    let updated = conn.execute(
        "UPDATE repos SET last_opened_at = ?1 WHERE path = ?2",
        params![now, path_key],
    )?;
    if updated == 0 {
        return Err(WorkpotError::NotFound(path_key));
    }
    Ok(())
}

/// Resolve repo location and SQLite path key, falling back when the directory is gone.
fn resolve_repo_location(conn: &Connection, path: &Path) -> Result<(PathBuf, String)> {
    let path_key = repo_path_key(conn, path)?;
    let repo_path = if path.exists() {
        path.canonicalize()
            .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", path.display())))?
    } else {
        PathBuf::from(&path_key)
    };
    Ok((repo_path, path_key))
}

fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// SQLite `repos.path` key for remove/lookup (canonical when the directory exists).
pub(crate) fn repo_path_key(conn: &Connection, path: &Path) -> Result<String> {
    match path.canonicalize() {
        Ok(c) => Ok(c.display().to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let display_key = path.display().to_string();
            if conn
                .query_row(
                    "SELECT 1 FROM repos WHERE path = ?1",
                    params![display_key],
                    |_| Ok(()),
                )
                .is_ok()
            {
                return Ok(display_key);
            }
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // Suffix match only (`%/{basename}`), not substring — avoids `/tmp/foo-extra`
                // matching a lookup for `foo`.
                let suffix_pattern = format!("/{}", escape_like(name));
                let mut stmt = conn.prepare(
                    "SELECT path FROM repos WHERE path = ?1 OR path LIKE '%' || ?2 ESCAPE '\\'",
                )?;
                let candidates: Vec<String> = stmt
                    .query_map(params![name, suffix_pattern], |row| row.get(0))?
                    .collect::<std::result::Result<_, _>>()?;
                let matches: Vec<String> = candidates
                    .into_iter()
                    .filter(|p| {
                        Path::new(p)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(|n| n == name)
                    })
                    .collect();
                match matches.len() {
                    0 => {}
                    1 => return Ok(matches[0].clone()),
                    _ => {
                        return Err(WorkpotError::InvalidPath(format!(
                            "ambiguous repo name '{name}'; use the absolute path from `workpot repo list`"
                        )));
                    }
                }
            }
            Ok(display_key)
        }
        Err(e) => Err(WorkpotError::InvalidPath(format!(
            "{}: {e}",
            path.display()
        ))),
    }
}

/// Non-excluded repo paths whose working tree no longer exists (stale index rows).
pub fn missing_repo_paths(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT path FROM repos WHERE excluded = 0")?;
    let paths: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<std::result::Result<_, _>>()?;

    Ok(paths
        .into_iter()
        .filter(|path_key| !Path::new(path_key).exists())
        .collect())
}

/// Remove repos whose paths are gone. Returns rows deleted.
pub fn prune_missing_repos(conn: &Connection) -> Result<u32> {
    let paths = missing_repo_paths(conn)?;
    let mut pruned = 0u32;
    for path_key in paths {
        let deleted = conn.execute("DELETE FROM repos WHERE path = ?1", params![path_key])?;
        pruned += u32::try_from(deleted).unwrap_or(0);
    }
    if pruned > 0 {
        log::info!("pruned {pruned} missing repo(s) from index");
    }
    Ok(pruned)
}

pub fn remove_repo(conn: &Connection, path: &Path) -> Result<()> {
    let path_key = repo_path_key(conn, path)?;

    let deleted = conn.execute("DELETE FROM repos WHERE path = ?1", params![path_key])?;
    if deleted == 0 {
        return Err(WorkpotError::NotFound(path_key));
    }
    Ok(())
}

/// Delete row and append `{parent}/{name}/**` to config excludes (D-10).
pub fn remove_repo_with_exclude(
    conn: &Connection,
    config_path: &Path,
    config: &mut Config,
    path: &Path,
) -> Result<()> {
    let (repo_path, _path_key) = resolve_repo_location(conn, path)?;

    if repo_path.file_name().is_none() {
        return Err(WorkpotError::InvalidPath(format!(
            "repo path has no directory name: {}",
            repo_path.display()
        )));
    }
    let base = path_to_exclude_glob_prefix(&repo_path);
    let tree = format!("{base}/**");

    let mut config_to_save = config.clone();
    let mut added_globs: Vec<String> = Vec::new();
    for glob in [base, tree] {
        if !config_to_save.excludes.iter().any(|g| g == &glob) {
            config_to_save.excludes.push(glob.clone());
            added_globs.push(glob);
        }
    }

    let persisted_excludes = !added_globs.is_empty();
    if persisted_excludes {
        save_config(config_path, &config_to_save)?;
        *config = config_to_save.clone();
    }

    if let Err(e) = remove_repo(conn, path) {
        if persisted_excludes {
            for glob in &added_globs {
                config_to_save.excludes.retain(|g| g != glob);
            }
            save_config(config_path, &config_to_save)?;
            *config = config_to_save;
        }
        return Err(e);
    }

    Ok(())
}

/// Escape glob metacharacters in one path segment for globset patterns.
fn escape_glob_literal(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for c in segment.chars() {
        match c {
            '\\' | '*' | '?' | '[' | ']' | '{' | '}' => {
                out.push('\\');
                out.push(c);
            }
            other => out.push(other),
        }
    }
    out
}

/// Canonical repo path as a literal glob prefix (per-segment escaping).
fn path_to_exclude_glob_prefix(path: &Path) -> String {
    let mut out = String::new();
    for component in path.components() {
        match component {
            std::path::Component::RootDir => out.push('/'),
            std::path::Component::Normal(name) => {
                if !out.is_empty() && !out.ends_with('/') {
                    out.push('/');
                }
                out.push_str(&escape_glob_literal(&name.to_string_lossy()));
            }
            _ => {}
        }
    }
    out
}

pub(crate) fn is_git_worktree(path: &Path) -> bool {
    let marker = path.join(".git");
    if marker.is_dir() {
        return marker.join("HEAD").is_file();
    }
    if marker.is_file() {
        let content = match std::fs::read_to_string(&marker) {
            Ok(c) => c,
            Err(_) => return false,
        };
        let rest = match content.strip_prefix("gitdir:") {
            Some(r) => r.trim(),
            None => return false,
        };
        let gitdir = PathBuf::from(rest);
        let gitdir = if gitdir.is_absolute() {
            gitdir
        } else {
            path.join(gitdir)
        };
        return gitdir.join("HEAD").is_file();
    }
    false
}

pub(crate) fn is_bare_repo(path: &Path) -> bool {
    path.join("HEAD").is_file() && path.join("objects").is_dir()
}

/// Insert or update a scan-discovered repo. Returns `true` when the path was newly added.
///
/// Does not enforce [`Limits::max_repos`](crate::domain::config::Limits::max_repos); callers must cap
/// before bulk upsert (typically [`crate::services::index::run_full`], which returns
/// [`crate::WorkpotError::IndexCapExceeded`] when the projected count would exceed the limit).
pub fn upsert_scan(conn: &Connection, path: &Path, git_common_dir: &str) -> Result<bool> {
    let canonical = path
        .canonicalize()
        .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", path.display())))?;

    let path_key = canonical.display().to_string();
    let existed = conn
        .query_row(
            "SELECT 1 FROM repos WHERE path = ?1",
            params![path_key],
            |_| Ok(()),
        )
        .is_ok();

    let name = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let registered_at = if existed {
        conn.query_row(
            "SELECT registered_at FROM repos WHERE path = ?1",
            params![path_key],
            |row| row.get(0),
        )
        .unwrap_or(0)
    } else {
        unix_now_secs()
    };

    conn.execute(
        "INSERT INTO repos (path, name, registered_at, source, git_common_dir, excluded)
         VALUES (?1, ?2, ?3, ?4, ?5, 0)
         ON CONFLICT(path) DO UPDATE SET
           name = excluded.name,
           git_common_dir = excluded.git_common_dir,
           source = CASE WHEN repos.source = ?6 THEN ?6 ELSE ?4 END",
        params![
            path_key,
            name,
            registered_at,
            SOURCE_SCAN,
            git_common_dir,
            SOURCE_MANUAL,
        ],
    )?;

    Ok(!existed)
}
