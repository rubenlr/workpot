use crate::domain::{Config, RepoRecord, SOURCE_MANUAL, SOURCE_SCAN};
use crate::error::{Result, WorkpotError};
use crate::infra::git::resolve_git_common_dir;
use crate::save_config;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM repos WHERE excluded = 0",
        [],
        |row| row.get(0),
    )?;
    let count = u32::try_from(count).unwrap_or(u32::MAX);
    if count >= config.limits.max_repos {
        return Err(WorkpotError::IndexCapExceeded {
            projected: count.saturating_add(1),
            max: config.limits.max_repos,
        });
    }

    let registered_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

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
                branch, is_dirty, ahead, behind, git_refreshed_at, git_state_error
         FROM repos WHERE excluded = 0 ORDER BY registered_at, path",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(RepoRecord {
            path: PathBuf::from(row.get::<_, String>(0)?),
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
        })
    })?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(WorkpotError::Database)
}

/// Resolve repo location and SQLite path key, falling back when the directory is gone.
fn resolve_repo_location(conn: &Connection, path: &Path) -> Result<(PathBuf, String)> {
    let path_key = resolve_repo_path_key(conn, path)?;
    let repo_path = if path.exists() {
        path.canonicalize()
            .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", path.display())))?
    } else {
        PathBuf::from(&path_key)
    };
    Ok((repo_path, path_key))
}

/// SQLite `repos.path` key for remove/lookup (canonical when the directory exists).
fn resolve_repo_path_key(conn: &Connection, path: &Path) -> Result<String> {
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
                let suffix_pattern = format!("%/{name}");
                let mut stmt = conn.prepare(
                    "SELECT path FROM repos WHERE path = ?1 OR path LIKE ?2",
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
        Err(e) => Err(WorkpotError::InvalidPath(format!("{}: {e}", path.display()))),
    }
}

pub fn remove_repo(conn: &Connection, path: &Path) -> Result<()> {
    let path_key = resolve_repo_path_key(conn, path)?;

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

    let parent = repo_path.parent().ok_or_else(|| {
        WorkpotError::InvalidPath(format!("repo path has no parent: {}", repo_path.display()))
    })?;
    let name = repo_path.file_name().ok_or_else(|| {
        WorkpotError::InvalidPath(format!(
            "repo path has no directory name: {}",
            repo_path.display()
        ))
    })?;
    let base = format!("{}/{}", parent.display(), name.to_string_lossy());
    let tree = format!("{base}/**");

    let mut config_to_save = config.clone();
    let mut changed = false;
    for glob in [base, tree] {
        if !config_to_save.excludes.iter().any(|g| g == &glob) {
            config_to_save.excludes.push(glob);
            changed = true;
        }
    }
    if changed {
        save_config(config_path, &config_to_save)?;
        *config = config_to_save;
    }

    remove_repo(conn, path)
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
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
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
