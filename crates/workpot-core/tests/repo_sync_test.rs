#![allow(clippy::disallowed_methods)]

use std::fs;
use std::path::PathBuf;
use workpot_core::AppContext;
use workpot_core::services::repo_sync::{SyncDirection, run_repo_sync};
use workpot_core::services::sync_cmd::build_sync_command;

fn init_git_repo(parent: &std::path::Path, name: &str) -> (git2::Repository, PathBuf) {
    let repo_path = parent.join(name);
    let repo = git2::Repository::init(&repo_path).expect("git2::Repository::init");
    (repo, repo_path)
}

fn make_commit(repo: &git2::Repository, message: &str) -> git2::Oid {
    let workdir = repo.workdir().expect("workdir");
    let file_path = workdir.join("file.txt");
    fs::write(&file_path, b"hello\n").expect("write");
    let mut index = repo.index().expect("index");
    index
        .add_path(std::path::Path::new("file.txt"))
        .expect("add_path");
    index.write().expect("index write");
    let tree_oid = index.write_tree().expect("write_tree");
    let tree = repo.find_tree(tree_oid).expect("find tree");
    let sig = git2::Signature::now("Test", "t@example.com").expect("sig");
    let parent_commit = match repo.head() {
        Ok(head_ref) => {
            let oid = head_ref.target().expect("target");
            Some(repo.find_commit(oid).expect("parent"))
        }
        Err(_) => None,
    };
    let parents: Vec<&git2::Commit> = parent_commit.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
        .expect("commit")
}

#[test]
fn build_sync_command_substitutes_path_and_branch() {
    let (program, args) = build_sync_command(
        "git -C {path} pull origin {branch}",
        std::path::Path::new("/tmp/repo"),
        "develop",
    )
    .expect("parse");
    assert_eq!(program, "git");
    assert!(args.contains(&"develop".to_string()));
}

#[test]
fn run_repo_sync_rejects_unindexed_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
    let err = run_repo_sync(&ctx, "/tmp/not-indexed", "main", SyncDirection::Push)
        .expect_err("not indexed");
    assert!(
        err.to_lowercase().contains("not found"),
        "expected not found, got: {err}"
    );
}

#[test]
fn run_repo_sync_rejects_empty_branch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
    let (repo, path) = init_git_repo(dir.path(), "sync-empty-branch");
    make_commit(&repo, "init");
    ctx.register_manual(&path).expect("register");
    let path_str = path.display().to_string();
    let err = run_repo_sync(&ctx, &path_str, "", SyncDirection::Pull).expect_err("empty branch");
    assert!(err.contains("branch"));
}

#[test]
fn run_repo_sync_success_refreshes_state() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(
        &config_path,
        r#"
watch_roots = []
excludes = []
push_cmd = "/usr/bin/true {path} {branch}"
pull_cmd = "/usr/bin/true {path} {branch}"
"#,
    )
    .expect("write config");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
    let (repo, path) = init_git_repo(dir.path(), "sync-ok");
    make_commit(&repo, "init");
    ctx.register_manual(&path).expect("register");
    let path_str = path.canonicalize().expect("canon").display().to_string();
    run_repo_sync(&ctx, &path_str, "main", SyncDirection::Pull).expect("sync");
    let repos = ctx.list_repos().expect("list");
    assert_eq!(repos.len(), 1);
    assert!(repos[0].branch.is_some(), "branch should be refreshed");
    assert!(repos[0].git_refreshed_at.is_some());
}
