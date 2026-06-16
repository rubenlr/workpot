#![allow(clippy::disallowed_methods)]

use std::fs;
use workpot_core::{WorkpotError, annotate_config_comments, default_config, init_config_file};

#[test]
fn init_config_file_writes_documented_defaults() {
    let home = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(home.path().join("code")).expect("code dir");
    let config_path = home.path().join("config.toml");

    init_config_file(&config_path, home.path(), false).expect("init config");
    let contents = fs::read_to_string(&config_path).expect("read config");
    assert!(
        contents.contains('#'),
        "init config should include comments"
    );
    assert!(
        contents.contains("code"),
        "should seed existing ~/code root"
    );
}

#[test]
fn init_config_file_rejects_existing_without_force() {
    let home = tempfile::tempdir().expect("tempdir");
    let config_path = home.path().join("config.toml");
    fs::write(&config_path, "watch_roots = []\nexcludes = []\n").expect("seed");

    let err = init_config_file(&config_path, home.path(), false).expect_err("should reject");
    assert!(matches!(err, WorkpotError::Config(_)));
    assert!(
        err.to_string().contains("already exists"),
        "unexpected message: {err}"
    );
}

#[test]
fn init_config_file_force_overwrites_existing() {
    let home = tempfile::tempdir().expect("tempdir");
    let config_path = home.path().join("config.toml");
    fs::write(&config_path, "watch_roots = []\nexcludes = [\"/stale\"]\n").expect("seed");

    init_config_file(&config_path, home.path(), true).expect("force init");
    let contents = fs::read_to_string(&config_path).expect("read config");
    assert!(
        !contents.contains("/stale"),
        "force init should replace contents"
    );
    assert!(contents.contains("launch_cmd"), "rendered defaults present");
}

#[test]
fn annotate_config_comments_backfills_bare_toml() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let config = default_config(dir.path());
    fs::write(
        &config_path,
        toml::to_string_pretty(&config).expect("serialize"),
    )
    .expect("write bare config");

    let added = annotate_config_comments(&config_path).expect("annotate");
    assert!(added > 0, "bare config should receive comments");

    let contents = fs::read_to_string(&config_path).expect("read annotated");
    assert!(
        contents.contains('#'),
        "annotated file should contain comments"
    );
}
