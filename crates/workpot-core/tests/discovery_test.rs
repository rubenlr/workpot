use globset::GlobSet;
use std::fs;
use std::path::Path;
use workpot_core::services::discovery;

fn empty_excludes() -> GlobSet {
    globset::GlobSetBuilder::new()
        .build()
        .expect("empty glob set")
}

fn git_worktree(parent: &Path, name: &str) -> std::path::PathBuf {
    let repo = parent.join(name);
    let git_dir = repo.join(".git");
    fs::create_dir_all(git_dir.join("objects")).expect("objects");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("HEAD");
    repo
}

#[test]
fn discovery_finds_repo_under_root() {
    let root = tempfile::tempdir().expect("tempdir");
    let repo = git_worktree(root.path(), "my-repo");
    let candidates = discovery::scan_root(root.path(), &empty_excludes()).expect("scan_root");
    let canonical = repo.canonicalize().expect("canonicalize");
    assert!(candidates.iter().any(|p| p == &canonical));
}

#[test]
fn discovery_skips_nested_git() {
    let root = tempfile::tempdir().expect("tempdir");
    let parent = git_worktree(root.path(), "parent");
    let nested_git = parent.join("vendor").join("nested");
    fs::create_dir_all(&nested_git).expect("nested dir");
    let nested_dot_git = nested_git.join(".git");
    fs::create_dir_all(nested_dot_git.join("objects")).expect("nested objects");
    fs::write(nested_dot_git.join("HEAD"), "ref: refs/heads/main\n").expect("nested HEAD");

    let candidates = discovery::scan_root(root.path(), &empty_excludes()).expect("scan_root");
    let nested_canonical = nested_git.canonicalize().expect("canonicalize nested");
    assert!(!candidates.iter().any(|p| p == &nested_canonical));
}

#[test]
fn discovery_skips_plain_dir() {
    let root = tempfile::tempdir().expect("tempdir");
    let plain = root.path().join("plain_dir");
    fs::create_dir_all(&plain).expect("plain dir");

    let candidates = discovery::scan_root(root.path(), &empty_excludes()).expect("scan_root");
    let plain_canonical = plain.canonicalize().expect("canonicalize plain");
    assert!(!candidates.iter().any(|p| p == &plain_canonical));
}
