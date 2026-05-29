use crate::domain::RepoRecord;
use crate::error::{Result, WorkpotError};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn register_manual(conn: &Connection, path: &Path) -> Result<RepoRecord> {
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

    let registered_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let rows = conn.execute(
        "INSERT INTO repos (path, name, registered_at, source) VALUES (?1, ?2, ?3, 'manual')",
        params![path_key, name, registered_at],
    );

    match rows {
        Ok(_) => Ok(RepoRecord {
            path: canonical,
            name,
            registered_at,
            source: "manual".to_string(),
            git_common_dir: String::new(),
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
        "SELECT path, name, registered_at, source, git_common_dir FROM repos WHERE excluded = 0 ORDER BY registered_at, path",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(RepoRecord {
            path: PathBuf::from(row.get::<_, String>(0)?),
            name: row.get(1)?,
            registered_at: row.get(2)?,
            source: row.get(3)?,
            git_common_dir: row.get(4)?,
        })
    })?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(WorkpotError::Database)
}

pub fn remove_repo(conn: &Connection, path: &Path) -> Result<()> {
    let canonical = path
        .canonicalize()
        .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", path.display())))?;
    let path_key = canonical.display().to_string();

    let deleted = conn.execute("DELETE FROM repos WHERE path = ?1", params![path_key])?;
    if deleted == 0 {
        return Err(WorkpotError::NotFound(path_key));
    }
    Ok(())
}

pub(crate) fn is_git_worktree(path: &Path) -> bool {
    let marker = path.join(".git");
    if marker.is_dir() {
        return marker.join("HEAD").is_file();
    }
    if marker.is_file() {
        return std::fs::read_to_string(&marker)
            .map(|s| s.starts_with("gitdir:"))
            .unwrap_or(false);
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
         VALUES (?1, ?2, ?3, 'scan', ?4, 0)
         ON CONFLICT(path) DO UPDATE SET
           name = excluded.name,
           git_common_dir = excluded.git_common_dir,
           source = CASE WHEN repos.source = 'manual' THEN 'manual' ELSE 'scan' END",
        params![path_key, name, registered_at, git_common_dir],
    )?;

    Ok(!existed)
}
