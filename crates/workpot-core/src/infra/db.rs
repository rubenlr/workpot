use crate::error::{Result, WorkpotError};
use crate::infra::migrations;
use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

fn lock_poison<T>(_: std::sync::PoisonError<T>) -> WorkpotError {
    WorkpotError::Config("database connection lock poisoned".to_string())
}

/// Read + write SQLite connections to the same database file (WAL allows concurrent readers).
pub struct DbPool {
    read: Mutex<Connection>,
    write: Mutex<Connection>,
}

impl DbPool {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut write = Connection::open(path)?;
        configure_connection(&mut write)?;
        migrations::apply_migrations(&mut write)?;

        let mut read = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        configure_connection(&mut read)?;

        Ok(Self {
            read: Mutex::new(read),
            write: Mutex::new(write),
        })
    }

    pub fn with_read<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        let guard = self.read.lock().map_err(lock_poison)?;
        f(&guard)
    }

    pub fn with_write<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        let guard = self.write.lock().map_err(lock_poison)?;
        f(&guard)
    }
}

fn configure_connection(conn: &mut Connection) -> Result<()> {
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
    conn.pragma_update(None, "foreign_keys", true)?;
    Ok(())
}
