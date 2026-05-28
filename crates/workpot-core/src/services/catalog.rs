use crate::domain::RepoRecord;
use crate::error::{Result, WorkpotError};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn register_manual(conn: &Connection, path: &Path) -> Result<RepoRecord> {
    if !path.is_dir() {
        return Err(WorkpotError::InvalidPath(format!(
            "not a directory: {}",
            path.display()
        )));
    }

    let git_marker = path.join(".git");
    if !git_marker.is_dir() && !git_marker.is_file() {
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
        "SELECT path, name, registered_at, source FROM repos ORDER BY registered_at",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(RepoRecord {
            path: PathBuf::from(row.get::<_, String>(0)?),
            name: row.get(1)?,
            registered_at: row.get(2)?,
            source: row.get(3)?,
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
