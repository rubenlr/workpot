use crate::error::Result;
use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

pub fn apply_migrations(conn: &mut Connection) -> Result<()> {
    static MIGRATION_001: &str = include_str!("migrations/001_init.sql");
    static MIGRATION_002: &str = include_str!("migrations/002_discovery.sql");
    static MIGRATION_003: &str = include_str!("migrations/003_git_state.sql");
    static MIGRATION_004: &str = include_str!("migrations/004_repos_source_index.sql");
    static MIGRATION_005: &str = include_str!("migrations/005_tray.sql");
    static MIGRATION_006: &str = include_str!("migrations/006_org.sql");
    static MIGRATION_007: &str = include_str!("migrations/007_alias.sql");
    static MIGRATION_008: &str = include_str!("migrations/008_convert_preflight.sql");
    let steps = [
        M::up(MIGRATION_001),
        M::up(MIGRATION_002),
        M::up(MIGRATION_003),
        M::up(MIGRATION_004),
        M::up(MIGRATION_005),
        M::up(MIGRATION_006),
        M::up(MIGRATION_007),
        M::up(MIGRATION_008),
    ];
    let migrations = Migrations::from_slice(&steps);
    migrations.to_latest(conn)?;
    Ok(())
}
