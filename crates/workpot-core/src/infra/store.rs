use crate::error::Result;
use crate::infra::migrations;
use rusqlite::Connection;
use std::path::Path;

pub fn open_connection(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
    conn.pragma_update(None, "foreign_keys", true)?;
    migrations::apply_migrations(&mut conn)?;
    Ok(conn)
}
