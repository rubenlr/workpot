use std::path::PathBuf;
use workpot_core::AppContext;

/// Initialize a git repo using git2 (same pattern as git_state_test).
fn init_git_repo(parent: &std::path::Path, name: &str) -> (git2::Repository, PathBuf) {
    let repo_path = parent.join(name);
    let repo = git2::Repository::init(&repo_path).expect("git2::Repository::init");
    (repo, repo_path)
}

fn make_commit(repo: &git2::Repository, message: &str) -> git2::Oid {
    let workdir = repo.workdir().expect("workdir");
    let file_path = workdir.join("file.txt");
    std::fs::write(&file_path, b"hello\n").expect("write");

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
fn tray_refresh_all_git_state_refreshes_indexed_repos() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    let (repo_a, path_a) = init_git_repo(dir.path(), "repo-a");
    make_commit(&repo_a, "init a");
    let (repo_b, path_b) = init_git_repo(dir.path(), "repo-b");
    make_commit(&repo_b, "init b");

    ctx.register_manual(&path_a).expect("register a");
    ctx.register_manual(&path_b).expect("register b");

    let summary = ctx.refresh_all_git_state().expect("refresh");
    assert!(summary.refreshed >= 1, "expected refreshed >= 1, got {:?}", summary);
    assert_eq!(summary.errors, 0);

    let repos = ctx.list_repos().expect("list");
    assert_eq!(repos.len(), 2);
    assert!(
        repos.iter().all(|r| r.branch.is_some()),
        "expected branch populated after refresh"
    );
}

#[test]
fn tray_refresh_preserves_git_snapshot_on_hard_failure() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    let (repo, path) = init_git_repo(dir.path(), "repo-preserve");
    make_commit(&repo, "init");
    ctx.register_manual(&path).expect("register");
    let path_canon = path.canonicalize().expect("canonicalize");

    ctx.refresh_all_git_state().expect("initial refresh");
    let branch_before = ctx
        .list_repos()
        .expect("list")
        .into_iter()
        .find(|r| r.path == path_canon)
        .and_then(|r| r.branch)
        .expect("branch after initial refresh");

    std::fs::remove_dir_all(path_canon.join(".git")).expect("remove .git");

    let summary = ctx.refresh_all_git_state().expect("refresh after break");
    assert_eq!(summary.errors, 1);

    let record = ctx
        .list_repos()
        .expect("list")
        .into_iter()
        .find(|r| r.path == path_canon)
        .expect("repo row");
    assert_eq!(record.branch, Some(branch_before));
    assert!(record.git_state_error.is_some());
}
