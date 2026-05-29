use std::fs;
use std::path::Path;
use std::process::Command;
use workpot_core::domain::Config;
use workpot_core::infra::store;
use workpot_core::services::index;

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

/// Minimal `.git` layout that passes discovery but is not a real repository.
fn fake_git_dir(parent: &Path, name: &str) -> std::path::PathBuf {
    let repo = parent.join(name);
    let git_dir = repo.join(".git");
    fs::create_dir_all(git_dir.join("objects")).expect("objects");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("HEAD");
    repo
}

#[test]
fn index_full_rescan_minimal() {
    let dir = tempfile::tempdir().expect("tempdir");
    let watch_root = dir.path().join("watch");
    fs::create_dir_all(&watch_root).expect("watch root");
    git_worktree(&watch_root, "repo-a");
    git_worktree(&watch_root, "repo-b");

    let db_path = dir.path().join("workpot.db");
    let mut config = Config::default();
    config.watch_roots.push(watch_root);

    let conn = store::open_connection(&db_path).expect("open db");

    index::run_full(&conn, &config).expect("first run_full");
    let count_after_first: i64 = conn
        .query_row("SELECT COUNT(*) FROM repos WHERE excluded = 0", [], |row| {
            row.get(0)
        })
        .expect("count repos");
    assert_eq!(count_after_first, 2);

    index::run_full(&conn, &config).expect("second run_full");
    let count_after_second: i64 = conn
        .query_row("SELECT COUNT(*) FROM repos WHERE excluded = 0", [], |row| {
            row.get(0)
        })
        .expect("count repos");
    assert_eq!(count_after_second, 2);
}

#[test]
fn index_skips_on_git_failure() {
    let dir = tempfile::tempdir().expect("tempdir");
    let watch_root = dir.path().join("watch");
    fs::create_dir_all(&watch_root).expect("watch root");
    git_worktree(&watch_root, "good-repo");
    fake_git_dir(&watch_root, "fake-repo");

    let db_path = dir.path().join("workpot.db");
    let mut config = Config::default();
    config.watch_roots.push(watch_root);

    let conn = store::open_connection(&db_path).expect("open db");

    let summary = index::run_full(&conn, &config).expect("run_full");
    assert_eq!(summary.skipped, 1, "fake repo should be skipped");

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM repos WHERE excluded = 0", [], |row| {
            row.get(0)
        })
        .expect("count repos");
    assert_eq!(count, 1);
}
