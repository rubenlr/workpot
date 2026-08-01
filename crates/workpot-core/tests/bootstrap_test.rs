#![allow(clippy::disallowed_methods)]

use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use workpot_core::domain::Config;
use workpot_core::{AppContext, WorkpotError, default_config};

#[test]
fn config_creates_defaults() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");

    let _ctx =
        AppContext::open_with_paths(config_path.clone(), db_path).expect("open should succeed");

    let contents = fs::read_to_string(&config_path).expect("config file exists");
    assert!(contents.contains("watch_roots"));
    assert!(contents.contains("excludes"));
    assert!(
        contents.contains('#'),
        "default config should include documentation comments"
    );
    assert!(
        contents.contains("{path}"),
        "launch_cmd comment should document {{path}} placeholder"
    );
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
fn open_removes_stale_config_tmp() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let tmp_path = dir.path().join("config.tmp");

    fs::write(&config_path, "watch_roots = []\nexcludes = []\n").expect("seed config");
    fs::write(&tmp_path, "partial write").expect("seed stale tmp");

    let _ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
    assert!(
        !tmp_path.exists(),
        "stale config.tmp should be removed on open"
    );
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
    assert_eq!(version, 10);

    let table_exists: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='repos'",
            [],
            |row| row.get(0),
        )
        .expect("repos table query");
    assert_eq!(table_exists, 1);

    let local_catalog_sync_runs_exists: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='local_catalog_sync_runs'",
            [],
            |row| row.get(0),
        )
        .expect("local_catalog_sync_runs table query");
    assert_eq!(local_catalog_sync_runs_exists, 1);

    let hidden_branches_exists: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='repo_hidden_branches'",
            [],
            |row| row.get(0),
        )
        .expect("repo_hidden_branches table query");
    assert_eq!(hidden_branches_exists, 1);
}

#[test]
fn open_enables_wal_journal_mode() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");

    let _ctx = AppContext::open_with_paths(config_path, db_path.clone()).expect("open");

    let conn = Connection::open(&db_path).expect("open db");
    let mode: String = conn
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .expect("journal_mode");
    assert_eq!(mode.to_lowercase(), "wal");
}

#[test]
fn config_validate_rejects_hard_max_repos() {
    let mut config = Config::default();
    config.limits.max_repos = 25_000;
    assert!(config.validate().is_err());
}

#[test]
fn config_validate_rejects_too_many_watch_roots() {
    let mut config = Config::default();
    config.limits.max_watch_roots = 100;
    config.watch_roots = (0..101)
        .map(|i| PathBuf::from(format!("/tmp/workpot-root-{i}")))
        .collect();
    assert!(config.validate().is_err());
}

#[test]
fn config_fetch_defaults_and_empty_ok() {
    let config = Config::default();
    assert_eq!(config.fetch, "git -C {path} fetch --prune --no-tags");
    assert!(config.validate().is_ok());

    let empty = Config {
        fetch: String::new(),
        ..Config::default()
    };
    assert!(empty.validate().is_ok());

    let whitespace = Config {
        fetch: "   ".to_string(),
        ..Config::default()
    };
    assert!(whitespace.validate().is_ok());
}

#[test]
fn config_fetch_rejects_missing_path_placeholder() {
    let config = Config {
        fetch: "git fetch".to_string(),
        ..Config::default()
    };
    let err = config.validate().expect_err("missing {path}");
    assert!(err.contains("{path}"), "got {err}");
}

#[test]
fn load_config_rejects_malformed_toml() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(&config_path, "watch_roots = [[[\n").expect("bad toml");

    assert!(matches!(
        AppContext::open_with_paths(config_path, db_path),
        Err(WorkpotError::Config(_))
    ));
}
