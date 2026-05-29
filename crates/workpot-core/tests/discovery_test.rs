use globset::GlobSet;
use std::fs;
use std::path::Path;
use std::process::Command;
use workpot_core::infra::git::resolve_git_common_dir;
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
fn discovery_includes_bare_and_worktree() {
    let root = tempfile::tempdir().expect("tempdir");
    let bare = root.path().join("bare.git");
    let status = Command::new("git")
        .args(["init", "--bare", "-q"])
        .arg(&bare)
        .status()
        .expect("git init --bare");
    assert!(status.success(), "bare init failed");

    let clone = root.path().join("clone-tmp");
    let status = Command::new("git")
        .args(["clone", "-q"])
        .arg(&bare)
        .arg(&clone)
        .status()
        .expect("git clone");
    assert!(status.success(), "clone failed");

    for (key, val) in [("user.email", "t@example.com"), ("user.name", "Test")] {
        let status = Command::new("git")
            .args(["config", key, val])
            .current_dir(&clone)
            .status()
            .expect("git config");
        assert!(status.success());
    }
    let status = Command::new("git")
        .args(["commit", "--allow-empty", "-m", "init", "-q"])
        .current_dir(&clone)
        .status()
        .expect("git commit");
    assert!(status.success());
    let status = Command::new("git")
        .args(["push", "-q", "origin", "HEAD:main"])
        .current_dir(&clone)
        .status()
        .expect("git push");
    assert!(status.success());

    let linked = bare.join("linked");
    let status = Command::new("git")
        .args(["-C"])
        .arg(&bare)
        .args(["worktree", "add", "-q"])
        .arg(&linked)
        .arg("main")
        .status()
        .expect("worktree add");
    assert!(status.success());

    let candidates = discovery::scan_root(root.path(), &empty_excludes()).expect("scan_root");
    let bare_canon = bare.canonicalize().expect("bare canon");
    let linked_canon = linked.canonicalize().expect("linked canon");
    assert!(
        candidates.iter().any(|p| p == &bare_canon),
        "bare path missing from candidates"
    );
    assert!(
        candidates.iter().any(|p| p == &linked_canon),
        "linked worktree missing from candidates"
    );
    let bare_gcd = resolve_git_common_dir(&bare_canon).expect("bare gcd");
    let linked_gcd = resolve_git_common_dir(&linked_canon).expect("linked gcd");
    assert_eq!(bare_gcd, linked_gcd);
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
