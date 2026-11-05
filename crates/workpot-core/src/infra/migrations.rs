use crate::error::{Result, WorkpotError};
use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

/// Bootstrap (`001_bootstrap.sql`) + remote catalog (`002_remote_catalog.sql`). Bump when adding a new `M::up`.
pub const LATEST_SCHEMA_VERSION: i32 = 2;

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
    if version >= 1 && !has_locations {
        return Err(WorkpotError::IncompatibleDatabase(format!(
            "database user_version={version} but `locations` is missing; {SCHEMA_RESET_HINT}"
        )));
    }

    Ok(())
}

pub fn apply_migrations(conn: &mut Connection) -> Result<()> {
    reject_incompatible_schema(conn)?;

    static MIGRATION_001: &str = include_str!("migrations/001_bootstrap.sql");
    static MIGRATION_002: &str = include_str!("migrations/002_remote_catalog.sql");
    let steps = [M::up(MIGRATION_001), M::up(MIGRATION_002)];
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

    const MIGRATION_001: &str = include_str!("migrations/001_bootstrap.sql");

    fn count_tables(conn: &Connection, names: &[&str]) -> usize {
        names
            .iter()
            .filter(|n| table_exists(conn, n).expect("check"))
            .count()
    }

    #[test]
    fn latest_schema_version_is_pinned() {
        // Bump deliberately together with a new migration step (and its upgrade test).
        assert_eq!(LATEST_SCHEMA_VERSION, 2);
    }

    #[test]
    fn v1_database_upgrades_to_v2_and_keeps_its_data() {
        let mut conn = Connection::open_in_memory().expect("open");
        Migrations::from_slice(&[M::up(MIGRATION_001)])
            .to_latest(&mut conn)
            .expect("apply v1 only");
        assert_eq!(user_version(&conn).expect("version"), 1);
        assert!(!table_exists(&conn, "remote_repos").expect("check"));
        conn.execute(
            "INSERT INTO locations (path, name, registered_at) VALUES ('/p/a', 'a', 1)",
            [],
        )
        .expect("seed location");

        apply_migrations(&mut conn).expect("upgrade");

        assert_eq!(user_version(&conn).expect("version"), 2);
        assert_eq!(
            count_tables(&conn, &["remote_repos", "remote_sync_runs"]),
            2
        );
        let kept: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM locations WHERE path = '/p/a'",
                [],
                |r| r.get(0),
            )
            .expect("count");
        assert_eq!(kept, 1, "v1 data survives the upgrade");
    }

    #[test]
    fn applying_migrations_twice_is_a_noop() {
        let mut conn = Connection::open_in_memory().expect("open");
        apply_migrations(&mut conn).expect("first");
        conn.execute(
            "INSERT INTO remote_sync_runs (source_id, started_at, status) VALUES ('s', 1, 'ok')",
            [],
        )
        .expect("seed run");
        apply_migrations(&mut conn).expect("second");
        assert_eq!(user_version(&conn).expect("version"), LATEST_SCHEMA_VERSION);
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM remote_sync_runs", [], |r| r.get(0))
            .expect("count");
        assert_eq!(rows, 1);
    }

    #[test]
    fn remote_tables_enforce_their_check_constraints() {
        let mut conn = Connection::open_in_memory().expect("open");
        apply_migrations(&mut conn).expect("migrate");
        let bad_state = conn.execute(
            "INSERT INTO remote_repos (source_id, full_path, name, url_normalized, clone_path,
                                       state, first_seen_at, last_seen_at)
             VALUES ('s', 'a/b', 'b', 'h/a/b', '/c', 'bogus', 1, 1)",
            [],
        );
        assert!(bad_state.is_err(), "state must be present|gone");
        let bad_status = conn.execute(
            "INSERT INTO remote_sync_runs (source_id, started_at, status) VALUES ('s', 1, 'bogus')",
            [],
        );
        assert!(bad_status.is_err(), "status must be ok|error");
        let bad_archived = conn.execute(
            "INSERT INTO remote_repos (source_id, full_path, name, url_normalized, clone_path,
                                       archived, first_seen_at, last_seen_at)
             VALUES ('s', 'a/c', 'c', 'h/a/c', '/c', 2, 1, 1)",
            [],
        );
        assert!(bad_archived.is_err(), "archived must be 0|1");
    }

    #[test]
    fn database_newer_than_this_build_is_rejected() {
        let mut conn = Connection::open_in_memory().expect("open");
        apply_migrations(&mut conn).expect("migrate");
        conn.pragma_update(None, "user_version", LATEST_SCHEMA_VERSION + 1)
            .expect("stamp newer");
        let err = apply_migrations(&mut conn).expect_err("newer must fail");
        match err {
            WorkpotError::IncompatibleDatabase(msg) => assert!(msg.contains("newer"), "{msg}"),
            other => panic!("expected IncompatibleDatabase, got {other:?}"),
        }
    }

    #[test]
    fn any_migrated_version_without_locations_is_rejected() {
        // v1 stamps (pre remote-catalog builds) must be caught too, not only the latest.
        for version in 1..=LATEST_SCHEMA_VERSION {
            let mut conn = Connection::open_in_memory().expect("open");
            conn.pragma_update(None, "user_version", version)
                .expect("stamp");
            let err = apply_migrations(&mut conn).expect_err("corrupt stamp");
            match err {
                WorkpotError::IncompatibleDatabase(msg) => {
                    assert!(msg.contains("`locations` is missing"), "v{version}: {msg}");
                }
                other => panic!("v{version}: expected IncompatibleDatabase, got {other:?}"),
            }
        }
    }

    #[test]
    fn unstamped_empty_database_is_not_mistaken_for_corruption() {
        let mut conn = Connection::open_in_memory().expect("open");
        assert_eq!(user_version(&conn).expect("version"), 0);
        apply_migrations(&mut conn).expect("fresh db migrates");
    }
}
