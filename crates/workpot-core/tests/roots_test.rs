#![allow(clippy::disallowed_methods)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use workpot_core::domain::Config;
use workpot_core::{AppContext, WorkpotError};

fn git_worktree(parent: &Path, name: &str) -> std::path::PathBuf {
    let repo = parent.join(name);
    fs::create_dir_all(&repo).expect("repo dir");
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&repo)
        .status()
        .expect("git init");
    assert!(status.success(), "git init failed for {}", repo.display());
    repo
}

fn empty_config_marker() -> &'static str {
    "watch_roots = []\nexcludes = []\n"
}

#[test]
fn default_limits_deserialize() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(&config_path, empty_config_marker()).expect("write config");

    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
    assert_eq!(ctx.config().limits.max_watch_roots, 100);
    assert_eq!(ctx.config().limits.max_repos, 1000);
}

#[test]
fn config_validate_rejects_watch_roots_hard_max() {
    let mut config = Config::default();
    config.limits.max_watch_roots = 5001;
    let err = config.validate().expect_err("validate should fail");
    assert!(err.contains("max_watch_roots"));
    assert!(err.contains("5000"));
}

#[test]
fn limits_reject_over_hard_max() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(
        &config_path,
        "[limits]\nmax_repos = 25000\nwatch_roots = []\nexcludes = []\n",
    )
    .expect("write config");

    assert!(matches!(
        AppContext::open_with_paths(config_path, db_path),
        Err(WorkpotError::Config(_))
    ));
}

#[test]
fn roots_add_persists_watch_root_on_disk() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(&config_path, empty_config_marker()).expect("write config");

    let watch_parent = dir.path().join("roots");
    fs::create_dir_all(&watch_parent).expect("watch parent");
    git_worktree(&watch_parent, "nested-repo");

    let mut ctx = AppContext::open_with_paths(config_path.clone(), db_path).expect("open");
    ctx.roots_add(&watch_parent).expect("roots_add");

    let on_disk = fs::read_to_string(&config_path).expect("read config");
    let watch_canon = watch_parent.canonicalize().expect("canonicalize watch");
    assert!(
        on_disk.contains(watch_canon.to_str().expect("utf-8 path")),
        "watch root must be saved to config.toml before/during scan: {on_disk}"
    );
}

#[test]
fn roots_add_rolls_back_on_index_cap() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(
        &config_path,
        "watch_roots = []\nexcludes = []\n\n[limits]\nmax_repos = 1\nmax_watch_roots = 100\n",
    )
    .expect("write config");

    let watch_parent = dir.path().join("roots");
    fs::create_dir_all(&watch_parent).expect("watch parent");
    git_worktree(&watch_parent, "repo-one");
    git_worktree(&watch_parent, "repo-two");

    let mut ctx = AppContext::open_with_paths(config_path.clone(), db_path).expect("open");
    let err = ctx.roots_add(&watch_parent).expect_err("cap should fail");
    assert!(
        matches!(err, WorkpotError::IndexCapExceeded { .. }),
        "expected IndexCapExceeded, got {err:?}"
    );

    let on_disk = fs::read_to_string(&config_path).expect("read config");
    let watch_canon = watch_parent.canonicalize().expect("canonicalize watch");
    assert!(
        !on_disk.contains(watch_canon.to_str().expect("utf-8 path")),
        "watch root must be rolled back from config on cap failure: {on_disk}"
    );

    let repos = ctx.list_repos().expect("list");
    assert!(
        repos.is_empty(),
        "scan repos from failed roots_add must be pruned, got {:?}",
        repos
    );
}

#[test]
fn roots_add_triggers_scan() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(&config_path, empty_config_marker()).expect("write config");

    let watch_parent = dir.path().join("roots");
    fs::create_dir_all(&watch_parent).expect("watch parent");
    let nested = git_worktree(&watch_parent, "nested-repo");

    let mut ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
    ctx.roots_add(&watch_parent).expect("roots_add");

    let repos = ctx.list_repos().expect("list");
    let nested_canon = nested.canonicalize().expect("canonicalize");
    assert!(
        repos.iter().any(|r| r.path == nested_canon),
        "expected indexed repo under new watch root"
    );
}

#[test]
fn config_validate_rejects_excess_watch_roots() {
    let mut config = Config::default();
    config.limits.max_watch_roots = 2;
    config.watch_roots = vec![
        PathBuf::from("/tmp/workpot-root-a"),
        PathBuf::from("/tmp/workpot-root-b"),
        PathBuf::from("/tmp/workpot-root-c"),
    ];
    let err = config.validate().expect_err("validate should fail");
    assert!(err.contains("watch_roots count"));
}

#[test]
fn roots_remove_skip_prune_keeps_repos() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(&config_path, empty_config_marker()).expect("write config");

    let watch_root = dir.path().join("watch");
    fs::create_dir_all(&watch_root).expect("watch root");
    let under = git_worktree(&watch_root, "repo-under");

    let mut ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
    ctx.roots_add(&watch_root).expect("add watch");
    let under_canon = under.canonicalize().expect("canonicalize under");

    ctx.roots_remove(&watch_root, true)
        .expect("remove watch root without prune");

    let repos = ctx.list_repos().expect("list after skip-prune remove");
    assert!(
        repos.iter().any(|r| r.path == under_canon),
        "skip-prune must keep indexed repos under removed root"
    );
    assert!(
        ctx.roots_list().is_empty(),
        "watch root should be removed from config"
    );
}

#[test]
fn roots_remove_prunes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(&config_path, empty_config_marker()).expect("write config");

    let watch_root = dir.path().join("watch");
    fs::create_dir_all(&watch_root).expect("watch root");
    let under = git_worktree(&watch_root, "repo-under");

    let sibling_root = dir.path().join("sibling");
    fs::create_dir_all(&sibling_root).expect("sibling root");
    let sibling_repo = git_worktree(&sibling_root, "repo-sibling");

    let mut ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
    ctx.roots_add(&watch_root).expect("add watch");
    ctx.register_manual(&sibling_repo).expect("manual sibling");

    let under_canon = under.canonicalize().expect("canonicalize under");
    let sibling_canon = sibling_repo.canonicalize().expect("canonicalize sibling");

    ctx.roots_remove(&watch_root, false)
        .expect("remove watch root");

    let repos = ctx.list_repos().expect("list after prune");
    assert!(
        !repos.iter().any(|r| r.path == under_canon),
        "repo under removed root should be pruned"
    );
    assert!(
        repos.iter().any(|r| r.path == sibling_canon),
        "sibling repo outside removed root prefix must remain"
    );
}

#[test]
fn roots_add_rejects_duplicate_watch_root() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(&config_path, empty_config_marker()).expect("write config");

    let watch_parent = dir.path().join("roots");
    fs::create_dir_all(&watch_parent).expect("watch parent");
    git_worktree(&watch_parent, "nested-repo");

    let mut ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
    ctx.roots_add(&watch_parent).expect("first add");
    assert!(matches!(
        ctx.roots_add(&watch_parent),
        Err(WorkpotError::WatchRootAlreadyExists(_))
    ));
}

#[test]
fn roots_remove_not_found() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(&config_path, empty_config_marker()).expect("write config");

    let missing = dir.path().join("no-such-watch-root");
    fs::create_dir_all(&missing).expect("dir for canonicalize");

    let mut ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
    assert!(matches!(
        ctx.roots_remove(&missing, false),
        Err(WorkpotError::WatchRootNotFound(_))
    ));
}

#[test]
fn reload_config_picks_up_disk_edits() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(&config_path, empty_config_marker()).expect("write config");

    let mut ctx = AppContext::open_with_paths(config_path.clone(), db_path).expect("open");
    assert!(ctx.config().watch_roots.is_empty());

    let new_root = dir.path().join("from-disk");
    fs::create_dir_all(&new_root).expect("new root dir");
    let new_root_str = new_root.to_str().expect("utf8 path");
    fs::write(
        &config_path,
        format!(
            "watch_roots = [\"{new_root_str}\"]\nexcludes = [\"/tmp/workpot-reload-exclude/**\"]\n"
        ),
    )
    .expect("rewrite config");

    ctx.reload_config().expect("reload");
    assert_eq!(ctx.config().watch_roots.len(), 1);
    assert_eq!(
        ctx.config().watch_roots[0]
            .canonicalize()
            .expect("canonicalize"),
        new_root.canonicalize().expect("canonicalize new root")
    );
    assert!(
        ctx.config()
            .excludes
            .iter()
            .any(|e| e.contains("workpot-reload-exclude")),
        "reload must load excludes from disk"
    );
}

#[test]
fn roots_add_rejects_when_at_max_watch_roots() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(
        &config_path,
        "watch_roots = []\nexcludes = []\n\n[limits]\nmax_watch_roots = 1\nmax_repos = 1000\n",
    )
    .expect("write config");

    let root_a = dir.path().join("root-a");
    let root_b = dir.path().join("root-b");
    fs::create_dir_all(&root_a).expect("root a");
    fs::create_dir_all(&root_b).expect("root b");

    let mut ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
    ctx.roots_add(&root_a).expect("first root");
    assert!(matches!(
        ctx.roots_add(&root_b),
        Err(WorkpotError::LimitsExceeded(_))
    ));
}
