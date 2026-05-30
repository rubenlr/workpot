use rusqlite::Connection;
use std::path::{Path, PathBuf};
use workpot_core::domain::Config;
use workpot_core::error::WorkpotError;
use workpot_core::infra::migrations;
use workpot_core::services::catalog;

fn temp_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("workpot.db");
    let mut conn = Connection::open(&db_path).expect("open db");
    migrations::apply_migrations(&mut conn).expect("migrate");
    (dir, conn)
}

#[test]
fn tray_migration_adds_last_opened_at_column() {
    let (_dir, conn) = temp_db();
    conn.execute(
        "INSERT INTO repos (path, name, registered_at, source, git_common_dir, excluded)
         VALUES ('/tmp/tray-test-repo', 'tray-test-repo', 1, 'manual', '/tmp/tray-test-repo/.git', 0)",
        [],
    )
    .expect("insert repo");

    let last_opened: Option<i64> = conn
        .query_row(
            "SELECT last_opened_at FROM repos WHERE path = '/tmp/tray-test-repo'",
            [],
            |row| row.get(0),
        )
        .expect("select last_opened_at");
    assert_eq!(last_opened, None);

    let repos = catalog::list_repos(&conn).expect("list");
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].last_opened_at, None);
}

#[test]
fn config_defaults_launch_cmd_and_max_visible_rows() {
    let config: Config = toml::from_str("watch_roots = []\nexcludes = []\n").expect("parse");
    assert_eq!(config.launch_cmd, "cursor --new-window {path}");
    assert_eq!(config.max_visible_rows, 15);
    config.validate().expect("valid defaults");
}

#[test]
fn config_rejects_invalid_launch_cmd() {
    let mut config = Config::default();
    config.launch_cmd = "   ".to_string();
    assert_eq!(
        config.validate().unwrap_err(),
        "launch_cmd must not be empty"
    );

    config.launch_cmd = "cursor".to_string();
    assert_eq!(
        config.validate().unwrap_err(),
        "launch_cmd must contain {path} placeholder"
    );
}

#[test]
fn config_rejects_max_visible_rows_out_of_range() {
    let mut config = Config::default();
    config.max_visible_rows = 0;
    assert_eq!(
        config.validate().unwrap_err(),
        "max_visible_rows 0 must be between 1 and 100"
    );

    config.max_visible_rows = 101;
    assert_eq!(
        config.validate().unwrap_err(),
        "max_visible_rows 101 must be between 1 and 100"
    );
}

#[test]
fn touch_last_opened_at_updates_row() {
    let (_dir, conn) = temp_db();
    let path = PathBuf::from("/tmp/tray-touch-repo");
    conn.execute(
        "INSERT INTO repos (path, name, registered_at, source, git_common_dir, excluded)
         VALUES (?1, 'tray-touch-repo', 1, 'manual', '/tmp/.git', 0)",
        rusqlite::params![path.display().to_string()],
    )
    .expect("insert");

    catalog::touch_last_opened_at(&conn, &path).expect("touch");
    let updated: Option<i64> = conn
        .query_row(
            "SELECT last_opened_at FROM repos WHERE path = ?1",
            rusqlite::params![path.display().to_string()],
            |row| row.get(0),
        )
        .expect("select");
    assert!(updated.is_some());
}

#[test]
fn indexed_launch_path_resolves_non_excluded_repo() {
    let (_dir, conn) = temp_db();
    let path_key = "/tmp/tray-indexed-launch-ok";
    conn.execute(
        "INSERT INTO repos (path, name, registered_at, source, git_common_dir, excluded)
         VALUES (?1, 'ok', 1, 'manual', '/tmp/.git', 0)",
        rusqlite::params![path_key],
    )
    .expect("insert");

    let resolved = catalog::indexed_launch_path(&conn, Path::new(path_key)).expect("resolve");
    assert_eq!(resolved.display().to_string(), path_key);
}

#[test]
fn indexed_launch_path_rejects_excluded_repo() {
    let (_dir, conn) = temp_db();
    let path_key = "/tmp/tray-indexed-launch-excluded";
    conn.execute(
        "INSERT INTO repos (path, name, registered_at, source, git_common_dir, excluded)
         VALUES (?1, 'excluded', 1, 'manual', '/tmp/.git', 1)",
        rusqlite::params![path_key],
    )
    .expect("insert");

    let err = catalog::indexed_launch_path(&conn, Path::new(path_key)).expect_err("excluded");
    match &err {
        WorkpotError::NotFound(key) => assert_eq!(key.as_str(), path_key),
        other => panic!("expected NotFound, got: {other:?}"),
    }
}
