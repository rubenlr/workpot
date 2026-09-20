use crate::error::{Result, WorkpotError};
use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

/// Single bootstrap step (`001_bootstrap.sql`). Bump when adding a new `M::up`.
pub const LATEST_SCHEMA_VERSION: i32 = 1;

const SCHEMA_RESET_HINT: &str = "quit Workpot, run `workpot db reset`, then `workpot sync` (see docs/sync.md#database-wipe-schema-bootstrap)";

fn table_exists(conn: &Connection, name: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
        [name],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn user_version(conn: &Connection) -> Result<i32> {
    Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

/// Reject pre-bootstrap (`repos`) catalogs and stamped DBs missing `locations`.
fn reject_incompatible_schema(conn: &Connection) -> Result<()> {
    let version = user_version(conn)?;
    let has_repos = table_exists(conn, "repos")?;
    let has_locations = table_exists(conn, "locations")?;

    if has_repos && !has_locations {
        return Err(WorkpotError::IncompatibleDatabase(format!(
            "legacy `repos` catalog (user_version={version}); {SCHEMA_RESET_HINT}"
        )));
    }

    if version > LATEST_SCHEMA_VERSION {
        return Err(WorkpotError::IncompatibleDatabase(format!(
            "database user_version {version} is newer than this build ({LATEST_SCHEMA_VERSION}); upgrade Workpot or {SCHEMA_RESET_HINT}"
        )));
    }

    // Stamped as migrated but missing the bootstrap tables (corrupt / partial wipe).
    if version >= LATEST_SCHEMA_VERSION && !has_locations {
        return Err(WorkpotError::IncompatibleDatabase(format!(
            "database user_version={version} but `locations` is missing; {SCHEMA_RESET_HINT}"
        )));
    }

    Ok(())
}

pub fn apply_migrations(conn: &mut Connection) -> Result<()> {
    reject_incompatible_schema(conn)?;

    static MIGRATION_001: &str = include_str!("migrations/001_bootstrap.sql");
    let steps = [M::up(MIGRATION_001)];
    let migrations = Migrations::from_slice(&steps);
    migrations.to_latest(conn)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn fresh_db_migrates_to_locations() {
        let mut conn = Connection::open_in_memory().expect("open");
        apply_migrations(&mut conn).expect("migrate");
        assert!(table_exists(&conn, "locations").expect("check"));
        assert_eq!(user_version(&conn).expect("version"), LATEST_SCHEMA_VERSION);
    }

    #[test]
    fn legacy_repos_schema_is_rejected() {
        let mut conn = Connection::open_in_memory().expect("open");
        conn.execute_batch(
            "PRAGMA user_version = 7;
             CREATE TABLE repos (
               path TEXT PRIMARY KEY,
               name TEXT NOT NULL,
               registered_at INTEGER NOT NULL
             );",
        )
        .expect("seed legacy");
        let err = apply_migrations(&mut conn).expect_err("legacy must fail");
        match err {
            WorkpotError::IncompatibleDatabase(msg) => {
                assert!(msg.contains("repos"), "{msg}");
                assert!(msg.contains("db reset"), "{msg}");
            }
            other => panic!("expected IncompatibleDatabase, got {other:?}"),
        }
    }

    #[test]
    fn stamped_without_locations_is_rejected() {
        let mut conn = Connection::open_in_memory().expect("open");
        conn.pragma_update(None, "user_version", LATEST_SCHEMA_VERSION)
            .expect("stamp");
        let err = apply_migrations(&mut conn).expect_err("corrupt stamp");
        assert!(
            matches!(err, WorkpotError::IncompatibleDatabase(_)),
            "{err:?}"
        );
    }
}
