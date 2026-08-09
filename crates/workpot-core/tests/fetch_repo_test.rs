#![allow(clippy::disallowed_methods)]

mod common;

use std::path::PathBuf;
use tempfile::tempdir;
use workpot_core::infra::git::{ensure_repo_fetch_refspecs, standard_fetch_refspec};
use workpot_core::services::fetch_repo::fetch_repo;

#[test]
fn empty_fetch_cmd_is_noop() {
    let path = PathBuf::from("/tmp/some-repo");
    assert!(fetch_repo(&path, "").is_ok());
    assert!(fetch_repo(&path, "   ").is_ok());
}

#[test]
fn invalid_template_errs_without_running() {
    let path = PathBuf::from("/tmp/some-repo");
    let err = fetch_repo(&path, "git fetch").expect_err("missing {path}");
    assert!(
        err.contains("{path}"),
        "expected placeholder error, got {err}"
    );
}

fn bare_with_url_only_origin(bare: &std::path::Path) {
    let status = common::git_cmd()
        .args(["init", "--bare", "-q", "-b", "main"])
        .arg(bare)
        .status()
        .expect("init bare");
    assert!(status.success());
    let repo = git2::Repository::open(bare).expect("open");
    let mut config = repo.config().expect("config");
    config
        .set_str("remote.origin.url", "https://example.com/foo.git")
        .expect("set url");
}

fn strip_fetch_refspec(repo_path: &std::path::Path, remote: &str) {
    let repo = git2::Repository::open(repo_path).expect("open");
    let mut config = repo.config().expect("config");
    let key = format!("remote.{remote}.fetch");
    while let Ok(()) = config.remove_multivar(&key, ".*") {}
}

fn fetch_refspec_present(repo_path: &std::path::Path, remote: &str) -> bool {
    common::git_cmd()
        .args(["config", "--get-all", &format!("remote.{remote}.fetch")])
        .current_dir(repo_path)
        .status()
        .expect("get fetch")
        .success()
}

#[test]
fn fetch_repo_skips_refspec_repair_when_fetch_cmd_empty() {
    let dir = tempdir().expect("tempdir");
    let bare = dir.path().join("bare.git");
    bare_with_url_only_origin(&bare);
    assert!(
        !fetch_refspec_present(&bare, "origin"),
        "fixture must start with no fetch refspec"
    );

    assert!(fetch_repo(&bare, "").is_ok());
    assert!(fetch_repo(&bare, "   ").is_ok());
    assert!(
        !fetch_refspec_present(&bare, "origin"),
        "empty fetch_cmd must skip refspec repair"
    );
}

#[test]
fn ensure_repo_fetch_refspecs_repairs_bare_without_fetch() {
    let dir = tempdir().expect("tempdir");
    let bare = dir.path().join("bare.git");
    bare_with_url_only_origin(&bare);

    assert!(
        !fetch_refspec_present(&bare, "origin"),
        "fixture must start with no fetch refspec"
    );

    let repaired = ensure_repo_fetch_refspecs(&bare).expect("ensure");
    assert_eq!(repaired, 1);

    let output = common::git_cmd()
        .args(["config", "--get-all", "remote.origin.fetch"])
        .current_dir(&bare)
        .output()
        .expect("get fetch after");
    assert!(output.status.success());
    let specs = String::from_utf8(output.stdout).expect("utf8");
    assert!(
        specs.contains(&standard_fetch_refspec("origin")),
        "got {specs:?}"
    );

    let again = ensure_repo_fetch_refspecs(&bare).expect("idempotent");
    assert_eq!(again, 0);
}

#[test]
fn ensure_repo_fetch_refspecs_repairs_multiple_remotes() {
    let dir = tempdir().expect("tempdir");
    let bare = dir.path().join("bare.git");
    bare_with_url_only_origin(&bare);
    {
        let repo = git2::Repository::open(&bare).expect("open");
        let mut config = repo.config().expect("config");
        config
            .set_str("remote.upstream.url", "https://example.com/upstream.git")
            .expect("set upstream url");
    }
    strip_fetch_refspec(&bare, "origin");
    strip_fetch_refspec(&bare, "upstream");

    let repaired = ensure_repo_fetch_refspecs(&bare).expect("ensure");
    assert_eq!(repaired, 2);

    for name in ["origin", "upstream"] {
        let output = common::git_cmd()
            .args(["config", "--get-all", &format!("remote.{name}.fetch")])
            .current_dir(&bare)
            .output()
            .expect("get fetch");
        assert!(output.status.success(), "missing fetch for {name}");
        let specs = String::from_utf8(output.stdout).expect("utf8");
        assert!(
            specs.contains(&standard_fetch_refspec(name)),
            "expected multi-branch for {name}, got {specs:?}"
        );
    }
}

#[test]
fn ensure_repo_fetch_refspecs_idempotent_when_already_correct() {
    let dir = tempdir().expect("tempdir");
    let origin = dir.path().join("origin.git");
    let work = dir.path().join("work");
    assert!(
        common::git_cmd()
            .args(["init", "--bare", "-q", "-b", "main"])
            .arg(&origin)
            .status()
            .expect("init origin")
            .success()
    );
    common::seed_bare_repo(&origin);
    assert!(
        common::git_cmd()
            .args([
                "clone",
                "-q",
                origin.to_str().expect("utf8"),
                work.to_str().expect("utf8"),
            ])
            .status()
            .expect("clone")
            .success()
    );

    let before = common::git_cmd()
        .args(["config", "--get-all", "remote.origin.fetch"])
        .current_dir(&work)
        .output()
        .expect("get fetch before");
    assert!(before.status.success());
    let before_specs = String::from_utf8(before.stdout).expect("utf8");

    let repaired = ensure_repo_fetch_refspecs(&work).expect("ensure");
    assert_eq!(repaired, 0);

    let after = common::git_cmd()
        .args(["config", "--get-all", "remote.origin.fetch"])
        .current_dir(&work)
        .output()
        .expect("get fetch after");
    assert!(after.status.success());
    let after_specs = String::from_utf8(after.stdout).expect("utf8");
    assert_eq!(after_specs, before_specs);
}

#[test]
fn fetch_repo_repairs_refspec_before_fetch_command() {
    let dir = tempdir().expect("tempdir");
    let origin = dir.path().join("origin.git");
    let bare = dir.path().join("bare.git");
    let work = dir.path().join("work");

    assert!(
        common::git_cmd()
            .args(["init", "--bare", "-q", "-b", "main"])
            .arg(&origin)
            .status()
            .expect("init origin")
            .success()
    );
    common::seed_bare_repo(&origin);

    assert!(
        common::git_cmd()
            .args([
                "clone",
                "-q",
                origin.to_str().expect("utf8"),
                work.to_str().expect("utf8"),
            ])
            .status()
            .expect("clone work")
            .success()
    );

    assert!(
        common::git_cmd()
            .args(["checkout", "-qb", "feature"])
            .current_dir(&work)
            .status()
            .expect("branch")
            .success()
    );
    std::fs::write(work.join("FEATURE"), "x").expect("feature file");
    assert!(
        common::git_cmd()
            .args(["add", "FEATURE"])
            .current_dir(&work)
            .status()
            .expect("add")
            .success()
    );
    assert!(
        common::git_cmd()
            .args([
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "commit",
                "-qm",
                "feat",
            ])
            .current_dir(&work)
            .status()
            .expect("commit")
            .success()
    );
    assert!(
        common::git_cmd()
            .args(["push", "-u", "origin", "feature"])
            .current_dir(&work)
            .status()
            .expect("push feature")
            .success()
    );

    assert!(
        common::git_cmd()
            .args([
                "clone",
                "--bare",
                "-q",
                "--single-branch",
                "--branch",
                "main",
                origin.to_str().expect("utf8"),
                bare.to_str().expect("utf8"),
            ])
            .status()
            .expect("clone bare")
            .success()
    );
    // Strip fetch refspec to match convert leftover
    {
        let repo = git2::Repository::open(&bare).expect("open bare");
        let mut config = repo.config().expect("config");
        let key = "remote.origin.fetch";
        while let Ok(()) = config.remove_multivar(key, ".*") {}
    }

    fetch_repo(&bare, "git -C {path} fetch --prune --no-tags").expect("fetch");

    let output = common::git_cmd()
        .args(["branch", "-r"])
        .current_dir(&bare)
        .output()
        .expect("branch -r");
    assert!(output.status.success());
    let remotes = String::from_utf8(output.stdout).expect("utf8");
    assert!(
        remotes.contains("origin/feature"),
        "expected origin/feature after repair+fetch, got {remotes:?}"
    );
    assert!(
        remotes.contains("origin/main"),
        "expected origin/main after repair+fetch, got {remotes:?}"
    );
}
