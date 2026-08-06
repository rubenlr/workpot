use crate::error::Result;
use crate::infra::db::DbPool;
use rusqlite::Connection;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// Open a single read-write connection (unit tests in local_catalog_sync/catalog).
pub fn open_connection(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
    conn.pragma_update(None, "foreign_keys", true)?;
    crate::infra::migrations::apply_migrations(&mut conn)?;
    Ok(conn)
}

/// Open read + write WAL connections for production [`crate::AppState`].
pub fn open_pool(path: &Path) -> Result<DbPool> {
    DbPool::open(path)
}

/// Result of deleting the SQLite main file and WAL/SHM sidecars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseReset {
    /// Resolved main database path (`workpot.db`).
    pub database: PathBuf,
    /// Paths successfully unlinked (subset of main + `-wal` + `-shm`).
    pub deleted: Vec<PathBuf>,
}

fn with_os_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Paths SQLite may create beside `db_path` when journal_mode=WAL.
pub fn database_file_candidates(db_path: &Path) -> [PathBuf; 3] {
    [
        db_path.to_path_buf(),
        with_os_suffix(db_path, "-wal"),
        with_os_suffix(db_path, "-shm"),
    ]
}

/// Delete `db_path` and SQLite WAL/SHM sidecars if present.
///
/// Idempotent: missing files are skipped. Unlink failures (e.g. locked by a
/// running tray process) are returned as [`crate::WorkpotError::Io`].
pub fn reset_database(db_path: &Path) -> Result<DatabaseReset> {
    let mut deleted = Vec::new();
    for path in database_file_candidates(db_path) {
        match std::fs::remove_file(&path) {
            Ok(()) => {
                log::info!("deleted database file {}", path.display());
                deleted.push(path);
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {
                log::info!("database file already absent {}", path.display());
            }
            Err(e) => {
                log::error!("failed to delete database file {}: {e}", path.display());
                return Err(e.into());
            }
        }
    }
    Ok(DatabaseReset {
        database: db_path.to_path_buf(),
        deleted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn reset_when_missing_succeeds() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = dir.path().join("workpot.db");
        let report = reset_database(&db).expect("reset");
        assert_eq!(report.database, db);
        assert!(report.deleted.is_empty());
        assert!(!db.exists());
    }

    #[test]
    fn reset_deletes_db_wal_and_shm() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = dir.path().join("workpot.db");
        let wal = with_os_suffix(&db, "-wal");
        let shm = with_os_suffix(&db, "-shm");
        fs::write(&db, b"db").expect("db");
        fs::write(&wal, b"wal").expect("wal");
        fs::write(&shm, b"shm").expect("shm");

        let report = reset_database(&db).expect("reset");
        assert_eq!(report.deleted, vec![db.clone(), wal.clone(), shm.clone()]);
        assert!(!db.exists());
        assert!(!wal.exists());
        assert!(!shm.exists());
    }

    #[test]
    fn reopen_after_reset_applies_migrations() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = dir.path().join("workpot.db");
        {
            let conn = open_connection(&db).expect("open");
            drop(conn);
        }
        assert!(db.exists());
        reset_database(&db).expect("reset");
        assert!(!db.exists());
        let conn = open_connection(&db).expect("reopen");
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='locations'",
                [],
                |row| row.get(0),
            )
            .expect("query");
        assert_eq!(count, 1);
    }
}
