#![allow(clippy::disallowed_methods)]

mod common;

use git2::{Repository, Signature};
use std::fs;
use std::path::{Path, PathBuf};
use workpot_core::AppContext;

fn init_repo_with_feature_branch(repo_path: &Path) {
    let repo = Repository::init(repo_path).expect("git init");
    let sig = Signature::now("test", "test@example.com").expect("sig");
    let tree_id = repo.index().expect("index").write_tree().expect("tree");
    let tree = repo.find_tree(tree_id).expect("find tree");
    repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
        .expect("commit");
    let tree_id2 = repo.index().expect("index").write_tree().expect("tree");
    let tree2 = repo.find_tree(tree_id2).expect("tree");
    repo.commit(
        Some("refs/heads/feature"),
        &sig,
        &sig,
        "feature",
        &tree2,
        &[],
    )
    .expect("feature commit");
}

fn bare_repo_with_worktree(parent: &Path) -> (PathBuf, PathBuf) {
    let bare_path = parent.join("myproject.git");
    fs::create_dir_all(&bare_path).expect("bare dir");
    let status = common::git_cmd()
        .args(["init", "--bare", "-q", "-b", "main"])
        .current_dir(&bare_path)
        .status()
        .expect("bare init");
    assert!(status.success());
    for (key, val) in [("user.email", "t@example.com"), ("user.name", "Test")] {
        let status = common::git_cmd()
            .args(["config", key, val])
            .current_dir(&bare_path)
            .status()
            .expect("config");
        assert!(status.success());
    }
    common::seed_bare_repo(&bare_path);

    let wt_path = parent.join("wt-main");
    let status = common::git_cmd()
        .args([
            "worktree",
            "add",
            "-q",
            wt_path.to_str().expect("utf8"),
            "main",
        ])
        .current_dir(&bare_path)
        .status()
        .expect("worktree add");
    assert!(status.success());
    let status = common::git_cmd()
        .args(["remote", "add", "origin"])
        .arg(&bare_path)
        .current_dir(&wt_path)
        .status()
        .expect("remote");
    assert!(status.success());
    let status = common::git_cmd()
        .args(["push", "-q", "-u", "origin", "main"])
        .current_dir(&wt_path)
        .status()
        .expect("push upstream");
    assert!(status.success());
    (bare_path, wt_path)
}

#[test]
fn checkout_switches_local_branch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(&config_path, "watch_roots = []\nexcludes = []\n").expect("config");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
    let repo_path = dir.path().join("sample");
    init_repo_with_feature_branch(&repo_path);
    ctx.register_manual(&repo_path).expect("register");

    ctx.checkout_repo_branch(&repo_path, "feature")
        .expect("checkout");

    let opened = Repository::open(&repo_path).expect("open");
    let head = opened.head().expect("head");
    assert_eq!(head.shorthand().ok(), Some("feature"));
}

#[test]
fn checkout_resolves_bare_catalog_key() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(&config_path, "watch_roots = []\nexcludes = []\n").expect("config");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
    let (bare_path, wt_path) = bare_repo_with_worktree(dir.path());

    let bare_repo = Repository::open(&bare_path).expect("open bare");
    let sig = Signature::now("test", "test@example.com").expect("sig");
    let tree_id = bare_repo
        .index()
        .expect("index")
        .write_tree()
        .expect("tree");
    let tree = bare_repo.find_tree(tree_id).expect("tree");
    bare_repo
        .commit(
            Some("refs/heads/feature"),
            &sig,
            &sig,
            "feature",
            &tree,
            &[],
        )
        .expect("feature commit on bare");

    let work_repo = Repository::open(&wt_path).expect("open worktree");
    let mut remote = work_repo.find_remote("origin").expect("origin");
    remote
        .fetch(&["refs/heads/*:refs/remotes/origin/*"], None, None)
        .expect("fetch feature");

    ctx.register_manual(&bare_path).expect("register bare");

    ctx.checkout_repo_branch(&bare_path, "feature")
        .expect("checkout via bare catalog key");

    let opened = Repository::open(&wt_path).expect("reopen worktree");
    let head = opened.head().expect("head");
    assert_eq!(head.shorthand().ok(), Some("feature"));
}
