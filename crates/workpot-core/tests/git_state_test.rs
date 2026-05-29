use std::path::{Path, PathBuf};
use workpot_core::domain::BRANCH_UNBORN;
use workpot_core::services::git_state::refresh_git_state;

/// Initialize a git repo using git2 (D-02: no Command::new("git") in core tests).
/// Returns (repository, repo_path).
fn init_git_repo(parent: &Path, name: &str) -> (git2::Repository, PathBuf) {
    let repo_path = parent.join(name);
    let repo = git2::Repository::init(&repo_path).expect("git2::Repository::init");
    (repo, repo_path)
}

/// Create a minimal commit in a git2 repo: write file to disk, stage it, commit.
/// The index and working tree are both consistent after this call.
/// Returns the OID of the created commit.
fn make_commit(repo: &git2::Repository, message: &str) -> git2::Oid {
    let workdir = repo.workdir().expect("workdir (not bare)");
    let file_path = workdir.join("file.txt");

    // Write to disk
    std::fs::write(&file_path, b"hello workpot\n").expect("write file.txt");

    // Stage via index
    let mut index = repo.index().expect("index");
    index
        .add_path(std::path::Path::new("file.txt"))
        .expect("index add_path");
    index.write().expect("index write");
    let tree_oid = index.write_tree().expect("write_tree");
    let tree = repo.find_tree(tree_oid).expect("find tree");

    let sig = git2::Signature::now("Test User", "test@example.com").expect("signature");

    // Determine parent commits
    let parent_commit = match repo.head() {
        Ok(head_ref) => {
            let oid = head_ref.target().expect("head target");
            Some(repo.find_commit(oid).expect("find parent commit"))
        }
        Err(_) => None,
    };

    let parents: Vec<&git2::Commit> = parent_commit.iter().collect();

    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
        .expect("commit")
}

/// Stage a file modification and commit.
#[allow(dead_code)]
fn modify_and_commit(repo: &git2::Repository, message: &str) -> git2::Oid {
    let workdir = repo.workdir().expect("workdir (not bare)");
    std::fs::write(workdir.join("file.txt"), b"modified content\n").expect("write");

    let mut index = repo.index().expect("index");
    index
        .add_path(std::path::Path::new("file.txt"))
        .expect("add_path");
    index.write().expect("index write");
    let tree_oid = index.write_tree().expect("write_tree");
    let tree = repo.find_tree(tree_oid).expect("find tree");

    let sig = git2::Signature::now("Test User", "test@example.com").expect("signature");

    let head_oid = repo.head().expect("head").target().expect("target");
    let parent = repo.find_commit(head_oid).expect("parent commit");

    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
        .expect("commit")
}

// ─── GIT-01: branch name tests ────────────────────────────────────────────────

#[test]
fn git_state_branch_normal() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (repo, repo_path) = init_git_repo(dir.path(), "branch-normal");

    make_commit(&repo, "initial commit");

    let state = refresh_git_state(&repo_path).expect("refresh_git_state");

    // git2 default branch is "master" unless config says otherwise;
    // accept either "master" or "main"
    let branch = state.branch.as_deref().expect("branch should be Some");
    assert!(
        branch == "master" || branch == "main",
        "expected 'master' or 'main', got '{branch}'"
    );
    assert!(state.error.is_none(), "error should be None, got {:?}", state.error);
}

#[test]
fn detached_head() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (repo, repo_path) = init_git_repo(dir.path(), "detached-head");

    let oid = make_commit(&repo, "initial commit");

    // Detach HEAD to the commit OID
    repo.set_head_detached(oid).expect("set_head_detached");

    let state = refresh_git_state(&repo_path).expect("refresh_git_state");

    let branch = state.branch.as_deref().expect("branch should be Some for detached HEAD");
    // Must be a 7-char hex short OID — NOT "HEAD" (Pitfall 2)
    assert_ne!(branch, "HEAD", "branch must not be 'HEAD' for detached HEAD");
    assert_eq!(branch.len(), 7, "detached HEAD branch must be 7-char short OID, got '{branch}'");
    // Verify it is a valid hex string
    assert!(
        branch.chars().all(|c| c.is_ascii_hexdigit()),
        "detached HEAD branch must be hex, got '{branch}'"
    );
    assert!(state.error.is_none(), "error should be None, got {:?}", state.error);
}

#[test]
fn unborn_branch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (_repo, repo_path) = init_git_repo(dir.path(), "unborn");
    // No commits created — HEAD points to unborn branch

    let state = refresh_git_state(&repo_path).expect("refresh_git_state must not panic");

    // Must handle gracefully; expect "unborn" sentinel
    let branch = state.branch.as_deref().expect("branch should be Some for unborn");
    assert_eq!(branch, BRANCH_UNBORN, "unborn branch should return '{BRANCH_UNBORN}', got '{branch}'");
    assert!(state.error.is_none(), "error should be None for unborn branch, got {:?}", state.error);
}

// ─── GIT-02: dirty flag tests ─────────────────────────────────────────────────

#[test]
fn dirty_unstaged() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (repo, repo_path) = init_git_repo(dir.path(), "dirty-unstaged");

    make_commit(&repo, "initial commit");
    let _ = repo; // drop borrow

    // Modify the tracked file.txt on disk without staging (file.txt exists on disk from make_commit)
    std::fs::write(repo_path.join("file.txt"), b"modified but not staged\n").expect("write");

    let state = refresh_git_state(&repo_path).expect("refresh_git_state");
    assert_eq!(state.is_dirty, Some(true), "unstaged modification must be dirty");
}

#[test]
fn dirty_staged() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (repo, repo_path) = init_git_repo(dir.path(), "dirty-staged");

    make_commit(&repo, "initial commit");

    // Modify file.txt on disk and stage it (file.txt exists on disk from make_commit)
    std::fs::write(repo_path.join("file.txt"), b"staged change\n").expect("write");
    let mut index = repo.index().expect("index");
    index.add_path(std::path::Path::new("file.txt")).expect("add_path");
    index.write().expect("index write");

    let state = refresh_git_state(&repo_path).expect("refresh_git_state");
    assert_eq!(state.is_dirty, Some(true), "staged modification must be dirty");
}

#[test]
fn untracked_is_clean() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (repo, repo_path) = init_git_repo(dir.path(), "untracked-clean");

    make_commit(&repo, "initial commit");
    let _ = repo; // drop borrow

    // Create an untracked file — do NOT add to index (D-10)
    std::fs::write(repo_path.join("untracked.txt"), b"untracked\n").expect("write");

    let state = refresh_git_state(&repo_path).expect("refresh_git_state");
    assert_eq!(state.is_dirty, Some(false), "untracked files must not count as dirty (D-10)");
}

#[test]
fn bare_no_dirty() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bare_path = dir.path().join("bare-repo");
    git2::Repository::init_bare(&bare_path).expect("init_bare");

    let state = refresh_git_state(&bare_path).expect("refresh_git_state");
    assert_eq!(state.is_dirty, None, "bare repo must return is_dirty=None (D-13)");
}

// ─── GIT-03: ahead/behind tests ───────────────────────────────────────────────

#[test]
fn ahead_behind() {
    let dir = tempfile::tempdir().expect("tempdir");

    // Create a bare "origin" repo
    let origin_path = dir.path().join("origin.git");
    let origin = git2::Repository::init_bare(&origin_path).expect("init_bare origin");

    // Create a local clone-like repo: init + add remote + make initial commit
    // We simulate what a clone does: init local repo, set up remote tracking
    let local_path = dir.path().join("local");
    let local = git2::Repository::init(&local_path).expect("init local");

    // Add initial commit to origin via its object store
    let sig = git2::Signature::now("Test User", "test@example.com").expect("sig");
    let initial_oid = {
        let blob_oid = origin.blob(b"initial\n").expect("blob");
        let tree_oid = {
            let mut tb = origin.treebuilder(None).expect("tb");
            tb.insert("file.txt", blob_oid, 0o100644).expect("insert");
            tb.write().expect("tree")
        };
        let tree = origin.find_tree(tree_oid).expect("find tree");
        origin
            .commit(Some("refs/heads/master"), &sig, &sig, "initial", &tree, &[])
            .expect("origin initial commit")
    };

    // Add the origin remote to local
    local
        .remote("origin", origin_path.to_str().expect("origin path"))
        .expect("add remote");

    // Fetch from origin
    {
        let mut remote = local.find_remote("origin").expect("find remote");
        remote
            .fetch(&["refs/heads/master:refs/remotes/origin/master"], None, None)
            .expect("fetch");
    }

    // Set up local master branch pointing to the fetched commit, with upstream tracking
    let fetch_head_oid = local
        .refname_to_id("refs/remotes/origin/master")
        .expect("refs/remotes/origin/master");
    let fetch_commit = local.find_commit(fetch_head_oid).expect("fetch commit");
    local
        .branch("master", &fetch_commit, false)
        .expect("create local master branch");
    local
        .set_head("refs/heads/master")
        .expect("set HEAD to master");

    // Configure tracking: local master tracks origin/master
    local
        .config()
        .expect("config")
        .set_str("branch.master.remote", "origin")
        .expect("set remote");
    local
        .config()
        .expect("config")
        .set_str("branch.master.merge", "refs/heads/master")
        .expect("set merge");

    // Add one local commit ahead of origin using the index
    let workdir = local.workdir().expect("workdir");
    std::fs::write(workdir.join("local.txt"), b"local change\n").expect("write local.txt");
    let tree2_oid = {
        let mut idx = local.index().expect("local index");
        idx.add_path(std::path::Path::new("local.txt")).expect("add local.txt");
        idx.write().expect("write index");
        idx.write_tree().expect("write_tree")
    };
    let tree2 = local.find_tree(tree2_oid).expect("find tree2");
    let parent_commit = local.find_commit(fetch_head_oid).expect("parent");
    local
        .commit(
            Some("refs/heads/master"),
            &sig,
            &sig,
            "local commit",
            &tree2,
            &[&parent_commit],
        )
        .expect("local commit");

    // suppress "unused" warning — origin was needed for setup
    let _ = (origin, initial_oid);

    let state = refresh_git_state(&local_path).expect("refresh_git_state");
    assert_eq!(state.ahead, Some(1), "should be 1 ahead of origin");
    assert_eq!(state.behind, Some(0), "should be 0 behind origin");
    assert!(state.error.is_none(), "error should be None");
}

#[test]
fn no_upstream() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (repo, repo_path) = init_git_repo(dir.path(), "no-upstream");

    make_commit(&repo, "initial commit");
    // No remote configured

    let state = refresh_git_state(&repo_path).expect("refresh_git_state");
    assert_eq!(state.ahead, None, "no upstream => ahead=None (D-04)");
    assert_eq!(state.behind, None, "no upstream => behind=None (D-04)");
    assert!(state.error.is_none(), "error should be None");
}
