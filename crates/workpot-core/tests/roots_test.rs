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
