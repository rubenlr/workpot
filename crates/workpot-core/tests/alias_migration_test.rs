#![allow(clippy::disallowed_methods)]

use rusqlite::Connection;
use workpot_core::infra::migrations;

fn temp_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("workpot.db");
    let mut conn = Connection::open(&db_path).expect("open db");
    migrations::apply_migrations(&mut conn).expect("migrate");
    (dir, conn)
}

#[test]
fn migration_007_adds_alias_column() {
    let (_dir, conn) = temp_db();
    let mut stmt = conn
        .prepare("PRAGMA table_info(repos)")
        .expect("pragma");
    let cols: Vec<(String, String, i64)> = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get(3)?))
        })
        .expect("query")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect");
    let alias = cols
        .iter()
        .find(|(name, _, _)| name == "alias")
        .expect("alias column exists");
    assert_eq!(alias.1, "TEXT");
    assert_eq!(alias.2, 0, "alias must be nullable");
}

#[test]
fn list_repos_returns_alias_from_db() {
    let (_dir, conn) = temp_db();
    let path = "/tmp/alias-list-test";
    conn.execute(
        "INSERT INTO repos (path, name, registered_at, source, git_common_dir, excluded, alias)
         VALUES (?1, 'alias-list-test', 1, 'manual', '.git', 0, 'My Alias')",
        rusqlite::params![path],
    )
    .expect("insert");

    let alias: Option<String> = conn
        .query_row(
            "SELECT alias FROM repos WHERE path = ?1",
            rusqlite::params![path],
            |row| row.get(0),
        )
        .expect("select alias");
    assert_eq!(alias.as_deref(), Some("My Alias"));
}
