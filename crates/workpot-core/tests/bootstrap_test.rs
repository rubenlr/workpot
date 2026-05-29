use rusqlite::Connection;
use std::fs;
use workpot_core::{default_config, AppContext};

#[test]
fn config_creates_defaults() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");

    let _ctx = AppContext::open_with_paths(config_path.clone(), db_path)
        .expect("open should succeed");

    let contents = fs::read_to_string(&config_path).expect("config file exists");
    assert!(contents.contains("watch_roots"));
    assert!(contents.contains("excludes"));
}

#[test]
fn default_config_seeds_only_existing_roots() {
    let home = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(home.path().join("code")).expect("code dir");

    let config = default_config(home.path());
    assert_eq!(config.watch_roots.len(), 1);
    assert!(config.watch_roots[0].ends_with("code"));
}

#[test]
fn open_does_not_overwrite_existing_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let marker = "watch_roots = []\nexcludes = [\"/custom/exclude\"]\n";
    fs::write(&config_path, marker).expect("seed config");

    let _ctx = AppContext::open_with_paths(config_path.clone(), db_path).expect("open");
    let contents = fs::read_to_string(&config_path).expect("read config");
    assert_eq!(contents, marker);
}

#[test]
fn default_config_seeds_code_and_dev_when_present() {
    let home = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(home.path().join("code")).expect("code dir");
    fs::create_dir_all(home.path().join("dev")).expect("dev dir");

    let config = default_config(home.path());
    assert_eq!(config.watch_roots.len(), 2);
    assert!(config.watch_roots.iter().any(|p| p.ends_with("code")));
    assert!(config.watch_roots.iter().any(|p| p.ends_with("dev")));
}

#[test]
fn migrations_apply() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");

    let _ctx = AppContext::open_with_paths(config_path, db_path.clone()).expect("open");

    let conn = Connection::open(&db_path).expect("open db");
    let version: i32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("user_version");
    assert_eq!(version, 3);

    let table_exists: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='repos'",
            [],
            |row| row.get(0),
        )
        .expect("repos table query");
    assert_eq!(table_exists, 1);

    let index_runs_exists: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='index_runs'",
            [],
            |row| row.get(0),
        )
        .expect("index_runs table query");
    assert_eq!(index_runs_exists, 1);
}
