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

/// Why the on-disk catalog cannot be migrated in place. Such catalogs are backed up and rebuilt.
fn unmigratable_reason(conn: &Connection) -> Result<Option<String>> {
    let version = user_version(conn)?;
    let has_locations = table_exists(conn, "locations")?;

    if table_exists(conn, "repos")? && !has_locations {
        return Ok(Some(format!(
            "pre-`locations` `repos` catalog (user_version={version})"
        )));
    }
    // Stamped as migrated but missing the bootstrap tables (corrupt / partial wipe).
    if version >= 1 && !has_locations {
        return Ok(Some(format!(
            "user_version={version} but `locations` is missing"
        )));
    }
    Ok(None)
}

/// A catalog written by a newer build is never touched: downgrading must not destroy its data.
fn reject_newer_schema(conn: &Connection) -> Result<()> {
    let version = user_version(conn)?;
    if version > LATEST_SCHEMA_VERSION {
        return Err(WorkpotError::IncompatibleDatabase(format!(
            "database user_version {version} is newer than this build ({LATEST_SCHEMA_VERSION}); upgrade Workpot or {SCHEMA_RESET_HINT}"
        )));
    }
    Ok(())
}

/// Copy the database file next to itself before it is wiped. An existing backup is kept (it
/// predates any partial reset). In-memory databases have no file and no backup.
fn backup_database(conn: &Connection, version: i32) -> Result<()> {
    let Some(path) = conn.path().filter(|p| !p.is_empty()) else {
        return Ok(());
    };
    let backup = format!("{path}.pre-migration-v{version}.bak");
    if std::path::Path::new(&backup).exists() {
        log::info!("catalog backup already present at {backup}; keeping it");
        return Ok(());
    }
    conn.execute("VACUUM INTO ?1", [&backup])?;
    log::info!("backed up catalog (user_version={version}) to {backup}");
    Ok(())
}

/// Drop every table, view and trigger and reset `user_version`, in one transaction.
/// Foreign keys are switched off for the wipe so drop order does not matter.
fn wipe_database(conn: &mut Connection) -> Result<()> {
    let fk_was_on: bool = conn.pragma_query_value(None, "foreign_keys", |row| row.get(0))?;
    conn.pragma_update(None, "foreign_keys", false)?;
    let wiped = (|| -> Result<()> {
        let tx = conn.transaction()?;
        let objects = {
            let mut stmt = tx.prepare(
                "SELECT type, name FROM sqlite_master
                 WHERE type IN ('table', 'view', 'trigger') AND name NOT LIKE 'sqlite_%'",
            )?;
            stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?
        };
        for (kind, name) in objects {
            let name = name.replace('"', "\"\"");
            tx.execute_batch(&format!("DROP {kind} IF EXISTS \"{name}\""))?;
        }
        tx.pragma_update(None, "user_version", 0)?;
        tx.commit()?;
        Ok(())
    })();
    conn.pragma_update(None, "foreign_keys", fk_was_on)?;
    wiped
}

pub fn apply_migrations(conn: &mut Connection) -> Result<()> {
    if let Some(reason) = unmigratable_reason(conn)? {
        log::warn!("catalog cannot be migrated ({reason}); backing up and rebuilding it");
        backup_database(conn, user_version(conn)?)?;
        wipe_database(conn)?;
    }
    reject_newer_schema(conn)?;

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

    fn seed_legacy_repos(conn: &Connection, version: i32) {
        conn.execute_batch(&format!(
            "PRAGMA user_version = {version};
             CREATE TABLE repos (path TEXT NOT NULL PRIMARY KEY, name TEXT NOT NULL, registered_at INTEGER NOT NULL);
             CREATE TABLE repo_tags (
               repo_path TEXT NOT NULL, tag TEXT NOT NULL,
               PRIMARY KEY (repo_path, tag),
               FOREIGN KEY (repo_path) REFERENCES repos(path) ON DELETE CASCADE
             );
             CREATE INDEX idx_repo_tags_path ON repo_tags(repo_path);
             INSERT INTO repos VALUES ('/p/a', 'a', 1);
             INSERT INTO repo_tags VALUES ('/p/a', 'work');"
        ))
        .expect("seed legacy");
    }

    fn assert_fresh_latest_schema(conn: &Connection) {
        assert_eq!(user_version(conn).expect("version"), LATEST_SCHEMA_VERSION);
        assert!(!table_exists(conn, "repos").expect("check"));
        assert!(table_exists(conn, "locations").expect("check"));
        assert!(table_exists(conn, "remote_repos").expect("check"));
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM locations", [], |r| r.get(0))
            .expect("count");
        assert_eq!(rows, 0, "rebuilt catalog starts empty");
        let tags: i64 = conn
            .query_row("SELECT COUNT(*) FROM repo_tags", [], |r| r.get(0))
            .expect("count");
        assert_eq!(tags, 0, "no legacy rows survive in re-created tables");
    }

    #[test]
    fn legacy_repos_schema_is_rebuilt_automatically() {
        let mut conn = Connection::open_in_memory().expect("open");
        conn.pragma_update(None, "foreign_keys", true).expect("fk");
        seed_legacy_repos(&conn, 10);

        apply_migrations(&mut conn).expect("legacy catalog is rebuilt, not rejected");
        assert_fresh_latest_schema(&conn);
        let fk: bool = conn
            .pragma_query_value(None, "foreign_keys", |r| r.get(0))
            .expect("fk");
        assert!(fk, "foreign_keys setting is restored after the wipe");

        apply_migrations(&mut conn).expect("second run is a noop");
        assert_fresh_latest_schema(&conn);
    }

    #[test]
    fn rebuild_backs_up_the_database_file_first() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = dir.path().join("workpot.db");
        let mut conn = Connection::open(&db).expect("open");
        seed_legacy_repos(&conn, 10);

        apply_migrations(&mut conn).expect("rebuild");
        assert_fresh_latest_schema(&conn);

        let backup = dir.path().join("workpot.db.pre-migration-v10.bak");
        assert!(backup.exists(), "backup written next to the db");
        let old = Connection::open(&backup).expect("open backup");
        assert!(
            table_exists(&old, "repos").expect("check"),
            "backup keeps old data"
        );
        assert_eq!(user_version(&old).expect("version"), 10);
    }

    #[test]
    fn existing_backup_is_not_overwritten() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = dir.path().join("workpot.db");
        let backup = dir.path().join("workpot.db.pre-migration-v10.bak");
        std::fs::write(&backup, b"earlier backup").expect("seed backup");
        let mut conn = Connection::open(&db).expect("open");
        seed_legacy_repos(&conn, 10);

        apply_migrations(&mut conn).expect("rebuild");
        assert_eq!(std::fs::read(&backup).expect("read"), b"earlier backup");
    }

    #[test]
    fn stamped_without_locations_is_rebuilt() {
        // Covers every stamp, not only the latest (pre remote-catalog builds stamped 1).
        for version in 1..=LATEST_SCHEMA_VERSION {
            let mut conn = Connection::open_in_memory().expect("open");
            conn.pragma_update(None, "user_version", version)
                .expect("stamp");
            apply_migrations(&mut conn).expect("corrupt stamp is rebuilt");
            assert_fresh_latest_schema(&conn);
        }
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
    fn unstamped_empty_database_is_not_mistaken_for_corruption() {
        let mut conn = Connection::open_in_memory().expect("open");
        assert_eq!(user_version(&conn).expect("version"), 0);
        apply_migrations(&mut conn).expect("fresh db migrates");
    }
}
