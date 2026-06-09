#![allow(clippy::disallowed_methods)]

mod common;

use globset::GlobSet;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use workpot_core::WorkpotError;
use workpot_core::domain::Config;
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

fn git_init_worktree(parent: &Path, name: &str) -> std::path::PathBuf {
    let repo = parent.join(name);
    fs::create_dir_all(&repo).expect("repo dir");
    let status = common::git_cmd()
        .args(["init", "-q"])
        .current_dir(&repo)
        .status()
        .expect("git init");
    assert!(status.success(), "git init failed for {}", repo.display());
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
    let status = common::git_cmd()
        .args(["init", "--bare", "-q", "-b", "main"])
        .arg(&bare)
        .status()
        .expect("git init --bare");
    assert!(status.success(), "bare init failed");
    common::seed_bare_repo(&bare);

    let clone = root.path().join("clone-tmp");
    let status = common::git_cmd()
        .args(["clone", "-q"])
        .arg(&bare)
        .arg(&clone)
        .status()
        .expect("git clone");
    assert!(status.success(), "clone failed");

    for (key, val) in [("user.email", "t@example.com"), ("user.name", "Test")] {
        let status = common::git_cmd()
            .args(["config", key, val])
            .current_dir(&clone)
            .status()
            .expect("git config");
        assert!(status.success());
    }
    let status = common::git_cmd()
        .args(["commit", "--allow-empty", "-m", "init", "-q"])
        .current_dir(&clone)
        .status()
        .expect("git commit");
    assert!(status.success());
    let status = common::git_cmd()
        .args(["push", "-q", "origin", "HEAD:main"])
        .current_dir(&clone)
        .status()
        .expect("git push");
    assert!(status.success());

    let linked = bare.join("linked");
    let status = common::git_cmd()
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
fn built_in_defaults_include_node_modules() {
    let defaults = discovery::built_in_defaults();
    assert!(
        defaults.iter().any(|g| g.contains("node_modules")),
        "built-in excludes must cover node_modules"
    );
}

#[test]
fn built_in_exclude_blocks_node_modules() {
    let root = tempfile::tempdir().expect("tempdir");
    let modules = root.path().join("node_modules").join("nested");
    fs::create_dir_all(&modules).expect("modules dir");
    let _hidden = git_worktree(&modules, "hidden-repo");
    let _visible = git_worktree(root.path(), "visible-repo");

    let exclude_set = discovery::build_exclude_set(&Config::default()).expect("exclude set");
    let candidates = discovery::scan_root(root.path(), &exclude_set).expect("scan_root");

    let modules_canon = modules.canonicalize().expect("canonicalize modules");
    assert!(
        !candidates.iter().any(|p| p.starts_with(&modules_canon)),
        "built-in node_modules exclude must block discovery"
    );
}

#[test]
fn build_exclude_set_merges_user_glob_with_builtins() {
    let mut config = Config::default();
    config
        .excludes
        .push("**/workpot-custom-exclude-dir/**".to_string());
    let exclude_set = discovery::build_exclude_set(&config).expect("exclude set");

    let custom = PathBuf::from("/tmp/workpot-custom-exclude-dir/nested/repo");
    assert!(
        exclude_set.is_match(&custom),
        "user exclude glob must be honored alongside built-ins"
    );
    assert!(
        discovery::built_in_defaults()
            .iter()
            .any(|g| g.contains("node_modules")),
        "built-in defaults remain available"
    );
}

#[test]
fn build_exclude_set_rejects_invalid_glob() {
    let mut config = Config::default();
    config.excludes.push("[invalid".to_string());
    let err = discovery::build_exclude_set(&config).unwrap_err();
    assert!(matches!(err, WorkpotError::Config(_)));
}

#[cfg(unix)]
#[test]
fn discovery_skips_symlink() {
    let root = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("outside");
    let real_repo = git_worktree(outside.path(), "real");
    let link_parent = root.path().join("links");
    fs::create_dir_all(&link_parent).expect("links dir");
    let link = link_parent.join("to-repo");
    symlink(&real_repo, &link).expect("symlink");

    let candidates = discovery::scan_root(root.path(), &empty_excludes()).expect("scan_root");
    let real_canon = real_repo.canonicalize().expect("canonicalize real");
    assert!(
        !candidates.iter().any(|p| p == &real_canon),
        "symlinked repo must not be discovered when follow_links is false"
    );
}

#[test]
fn scan_root_returns_empty_for_missing_path() {
    let missing = PathBuf::from("/tmp/workpot-discovery-missing-root-nope");
    let candidates = discovery::scan_root(&missing, &empty_excludes()).expect("scan missing root");
    assert!(
        candidates.is_empty(),
        "missing watch root must yield no candidates"
    );
}

#[test]
fn resolve_git_common_dir_errors_on_plain_directory() {
    let root = tempfile::tempdir().expect("tempdir");
    let plain = root.path().join("plain");
    fs::create_dir_all(&plain).expect("plain dir");
    let err = resolve_git_common_dir(&plain).unwrap_err();
    assert!(matches!(err, WorkpotError::GitUnavailable(_)));
}

#[test]
fn list_worktree_paths_empty_without_linked_worktrees() {
    let root = tempfile::tempdir().expect("tempdir");
    let repo = git_init_worktree(root.path(), "normal");
    let canon = repo.canonicalize().expect("canonicalize");
    let linked = workpot_core::infra::git::list_worktree_paths(&canon).expect("list worktrees");
    assert!(
        linked.is_empty(),
        "repo with no extra worktrees should yield an empty linked list (D-04)"
    );
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
