use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;

fn git_fixture(parent: &std::path::Path) -> PathBuf {
    let repo = parent.join("sample-repo");
    fs::create_dir_all(&repo).expect("repo dir");
    let git_dir = repo.join(".git");
    fs::create_dir_all(git_dir.join("objects")).expect("objects");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("HEAD");
    repo
}

fn workpot_cmd(home: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("workpot").expect("workpot binary");
    cmd.env("HOME", home);
    cmd
}

#[test]
fn paths_prints_config_and_database() {
    let home = tempfile::tempdir().expect("tempdir");

    workpot_cmd(home.path())
        .arg("paths")
        .assert()
        .success()
        .stdout(predicate::str::contains("config:"))
        .stdout(predicate::str::contains("database:"));
}

#[test]
fn repo_add_list_remove_roundtrip() {
    let home = tempfile::tempdir().expect("tempdir");
    let repo_path = git_fixture(home.path());
    let canonical = repo_path.canonicalize().expect("canonicalize");

    workpot_cmd(home.path())
        .args(["repo", "add", repo_path.to_str().expect("utf8 path")])
        .assert()
        .success()
        .stdout(predicate::str::contains("registered:"));

    workpot_cmd(home.path())
        .args(["repo", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(canonical.to_str().expect("utf8 path")));

    workpot_cmd(home.path())
        .args(["repo", "remove", repo_path.to_str().expect("utf8 path")])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed:"));

    workpot_cmd(home.path())
        .args(["repo", "list"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn repo_add_rejects_non_git() {
    let home = tempfile::tempdir().expect("tempdir");
    let plain = home.path().join("plain-dir");
    fs::create_dir_all(&plain).expect("dir");

    workpot_cmd(home.path())
        .args(["repo", "add", plain.to_str().expect("utf8 path")])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not a git repository").or(
            predicate::str::contains("NotGitRepo"),
        ));
}
