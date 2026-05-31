use crate::error::Result;
use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

pub fn apply_migrations(conn: &mut Connection) -> Result<()> {
    static MIGRATION_001: &str = include_str!("migrations/001_init.sql");
    static MIGRATION_002: &str = include_str!("migrations/002_discovery.sql");
    static MIGRATION_003: &str = include_str!("migrations/003_git_state.sql");
    static MIGRATION_004: &str = include_str!("migrations/004_repos_source_index.sql");
    static MIGRATION_005: &str = include_str!("migrations/005_tray.sql");
    let steps = [
        M::up(MIGRATION_001),
        M::up(MIGRATION_002),
        M::up(MIGRATION_003),
        M::up(MIGRATION_004),
        M::up(MIGRATION_005),
    ];
    let migrations = Migrations::from_slice(&steps);
    migrations.to_latest(conn)?;
    Ok(())
}
