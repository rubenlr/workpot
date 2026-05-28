use crate::error::Result;
use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

pub fn apply_migrations(conn: &mut Connection) -> Result<()> {
    static MIGRATION_UP: &str = include_str!("migrations/001_init.sql");
    let steps = [M::up(MIGRATION_UP)];
    let migrations = Migrations::from_slice(&steps);
    migrations.to_latest(conn)?;
    Ok(())
}
