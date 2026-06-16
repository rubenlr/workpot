#![allow(clippy::disallowed_methods)]

use rusqlite::Connection;
use workpot_core::infra::store;

#[test]
fn open_pool_enables_wal_and_foreign_keys() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("workpot.db");
    let pool = store::open_pool(&db_path).expect("open pool");

    pool.with_read(|conn| {
        let mode: String = conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .expect("journal_mode");
        assert_eq!(mode.to_lowercase(), "wal");

        let fk: i32 = conn
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .expect("foreign_keys");
        assert_eq!(fk, 1);
        Ok(())
    })
    .expect("with_read");
}

#[test]
fn open_pool_read_write_connections_share_schema() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("workpot.db");
    let pool = store::open_pool(&db_path).expect("open pool");

    pool.with_write(|conn| {
        conn.execute(
            "INSERT INTO repos (path, name, registered_at, source, git_common_dir, excluded)
             VALUES ('/tmp/pool-test', 'pool-test', 1, 'manual', '', 0)",
            [],
        )?;
        Ok(())
    })
    .expect("with_write");

    pool.with_read(|conn| {
        let name: String = conn.query_row(
            "SELECT name FROM repos WHERE path = '/tmp/pool-test'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(name, "pool-test");
        Ok(())
    })
    .expect("with_read");
}

#[test]
fn open_connection_matches_pool_write_semantics() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("workpot.db");
    let conn = store::open_connection(&db_path).expect("open connection");
    assert_schema_ready(&conn);
}

fn assert_schema_ready(conn: &Connection) {
    let version: i32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("user_version");
    assert_eq!(version, 7);

    let repos_exists: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='repos'",
            [],
            |row| row.get(0),
        )
        .expect("repos table");
    assert_eq!(repos_exists, 1);
}
