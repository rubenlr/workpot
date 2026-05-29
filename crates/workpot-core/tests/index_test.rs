use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use workpot_core::domain::Config;
use workpot_core::infra::store;
use workpot_core::services::{catalog, index};
use workpot_core::WorkpotError;

fn git_worktree(parent: &Path, name: &str) -> PathBuf {
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

fn fake_git_dir(parent: &Path, name: &str) -> PathBuf {
    let repo = parent.join(name);
    let git_dir = repo.join(".git");
    fs::create_dir_all(git_dir.join("objects")).expect("objects");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("HEAD");
    repo
}

fn open_index_fixture(max_repos: Option<u32>) -> (tempfile::TempDir, rusqlite::Connection, Config) {
    let dir = tempfile::tempdir().expect("tempdir");
    let watch_root = dir.path().join("watch");
    fs::create_dir_all(&watch_root).expect("watch root");
    let db_path = dir.path().join("workpot.db");
    let mut config = Config::default();
    config.watch_roots.push(watch_root.clone());
    if let Some(cap) = max_repos {
        config.limits.max_repos = cap;
    }
    let conn = store::open_connection(&db_path).expect("open db");
    (dir, conn, config)
}

#[test]
fn index_full_rescan() {
    let (_dir, conn, config) = open_index_fixture(None);
    let watch_root = config.watch_roots[0].clone();
    git_worktree(&watch_root, "repo-a");
    git_worktree(&watch_root, "repo-b");

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
    let (_dir, conn, config) = open_index_fixture(None);
    let watch_root = config.watch_roots[0].clone();
    git_worktree(&watch_root, "good-repo");
    fake_git_dir(&watch_root, "fake-repo");

    let summary = index::run_full(&conn, &config).expect("run_full");
    assert_eq!(summary.skipped, 1, "fake repo should be skipped");

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM repos WHERE excluded = 0", [], |row| {
            row.get(0)
        })
        .expect("count repos");
    assert_eq!(count, 1);
}

#[test]
fn index_backfills_git_common_dir() {
    let (_dir, conn, config) = open_index_fixture(None);
    let watch_root = config.watch_roots[0].clone();
    let repo = git_worktree(&watch_root, "backfill-me");
    let path_key = repo.canonicalize().expect("canon").display().to_string();

    conn.execute(
        "INSERT INTO repos (path, name, registered_at, source, git_common_dir, excluded)
         VALUES (?1, 'backfill-me', 0, 'scan', '', 0)",
        rusqlite::params![path_key],
    )
    .expect("seed row");

    index::run_full(&conn, &config).expect("run_full");

    let gcd: String = conn
        .query_row(
            "SELECT git_common_dir FROM repos WHERE path = ?1",
            rusqlite::params![path_key],
            |row| row.get(0),
        )
        .expect("gcd");
    assert!(!gcd.is_empty(), "git_common_dir should be backfilled");
}

#[test]
fn index_preserves_manual_source() {
    let (_dir, conn, config) = open_index_fixture(None);
    let watch_root = config.watch_roots[0].clone();
    let repo = git_worktree(&watch_root, "manual-repo");
    catalog::register_manual(&conn, &config, &repo).expect("manual register");

    index::run_full(&conn, &config).expect("run_full");

    let source: String = conn
        .query_row(
            "SELECT source FROM repos WHERE path = ?1",
            rusqlite::params![repo.canonicalize().expect("canon").display().to_string()],
            |row| row.get(0),
        )
        .expect("source");
    assert_eq!(source, "manual");
}

#[test]
fn index_removes_stale_path() {
    let (dir, conn, config) = open_index_fixture(None);
    let watch_root = config.watch_roots[0].clone();
    let repo = git_worktree(&watch_root, "gone-repo");
    index::run_full(&conn, &config).expect("first index");

    fs::remove_dir_all(&repo).expect("remove repo dir");

    let summary = index::run_full(&conn, &config).expect("second index");
    assert_eq!(summary.removed, 1);

    let removed_changes: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM index_changes WHERE action = 'removed'",
            [],
            |row| row.get(0),
        )
        .expect("removed changes");
    assert_eq!(removed_changes, 1, "stale path must appear in index_changes");

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM repos WHERE excluded = 0", [], |row| {
            row.get(0)
        })
        .expect("count");
    assert_eq!(count, 0);
    let _ = dir;
}

#[test]
fn index_validates_manual_outside_roots() {
    let dir = tempfile::tempdir().expect("tempdir");
    let watch_root = dir.path().join("watch");
    fs::create_dir_all(&watch_root).expect("watch");
    let outside = dir.path().join("outside");
    let repo = git_worktree(&outside, "solo");

    let db_path = dir.path().join("workpot.db");
    let mut config = Config::default();
    config.watch_roots.push(watch_root);
    let conn = store::open_connection(&db_path).expect("open");

    catalog::register_manual(&conn, &config, &repo).expect("manual outside roots");
    index::run_full(&conn, &config).expect("index keeps valid manual");

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM repos WHERE excluded = 0", [], |row| {
            row.get(0)
        })
        .expect("count");
    assert_eq!(count, 1);
}

#[test]
fn index_cap_abort() {
    let (_dir, conn, config) = open_index_fixture(Some(1));
    let watch_root = config.watch_roots[0].clone();
    git_worktree(&watch_root, "one");
    git_worktree(&watch_root, "two");

    let err = index::run_full(&conn, &config).unwrap_err();
    assert!(matches!(err, WorkpotError::IndexCapExceeded { .. }));

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM repos WHERE excluded = 0", [], |row| {
            row.get(0)
        })
        .expect("count");
    assert_eq!(count, 0, "cap abort must not partially merge repos");

    let cap_runs: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM index_runs WHERE status = 'cap_exceeded'",
            [],
            |row| row.get(0),
        )
        .expect("cap runs");
    assert_eq!(cap_runs, 1);
}

#[test]
fn index_writes_history() {
    let (_dir, conn, config) = open_index_fixture(None);
    let watch_root = config.watch_roots[0].clone();
    git_worktree(&watch_root, "hist-repo");

    let summary = index::run_full(&conn, &config).expect("run_full");
    assert_eq!(summary.added, 1);

    let runs: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM index_runs WHERE status = 'ok'",
            [],
            |row| row.get(0),
        )
        .expect("runs");
    assert_eq!(runs, 1);

    let changes: i64 = conn
        .query_row("SELECT COUNT(*) FROM index_changes", [], |row| row.get(0))
        .expect("changes");
    assert!(changes >= 1);
}

#[test]
fn index_second_pass_persists_git_state() {
    let (_dir, conn, config) = open_index_fixture(None);
    let watch_root = config.watch_roots[0].clone();
    let repo_path = git_worktree(&watch_root, "git-state-repo");
    let path_key = repo_path.canonicalize().expect("canonical").display().to_string();

    let summary = index::run_full(&conn, &config).expect("run_full");
    assert!(
        summary.git_refreshed >= 1,
        "expected at least one successful git refresh, got {:?}",
        summary
    );

    let (branch, refreshed_at, git_err): (Option<String>, Option<i64>, Option<String>) = conn
        .query_row(
            "SELECT branch, git_refreshed_at, git_state_error FROM repos WHERE path = ?1",
            rusqlite::params![path_key],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("repo git columns");
    assert!(branch.is_some(), "branch set after index git pass");
    assert!(refreshed_at.is_some(), "git_refreshed_at set after index");
    assert!(git_err.is_none(), "healthy repo has no git_state_error");
}

#[test]
fn index_git_pass_counts_refresh_errors() {
    let (_dir, conn, config) = open_index_fixture(None);
    let watch_root = config.watch_roots[0].clone();
    git_worktree(&watch_root, "good");

    let plain = watch_root.join("not-git");
    fs::create_dir_all(&plain).expect("plain dir");

    index::run_full(&conn, &config).expect("first index");

    let plain_key = plain.canonicalize().expect("canonical").display().to_string();
    conn.execute(
        "INSERT INTO repos (path, name, registered_at, source, git_common_dir, excluded)
         VALUES (?1, 'not-git', 0, 'manual', '', 0)",
        rusqlite::params![plain_key],
    )
    .expect("seed non-git row for git pass");

    let summary = index::run_full(&conn, &config).expect("second index");
    assert!(
        summary.git_refreshed >= 1,
        "healthy repo should refresh: {summary:?}"
    );
    assert!(
        summary.git_errors >= 1,
        "non-git row should count as git error (D-16/D-17): {summary:?}"
    );

    let git_err: Option<String> = conn
        .query_row(
            "SELECT git_state_error FROM repos WHERE path = ?1",
            rusqlite::params![plain_key],
            |row| row.get(0),
        )
        .expect("git_state_error column");
    assert!(
        git_err.is_some(),
        "failed refresh must persist git_state_error (D-09)"
    );
}

#[test]
fn index_git_failure_writes_skipped() {
    let (_dir, conn, config) = open_index_fixture(None);
    let watch_root = config.watch_roots[0].clone();
    git_worktree(&watch_root, "good");
    fake_git_dir(&watch_root, "bad");

    index::run_full(&conn, &config).expect("run_full");

    let skipped: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM index_changes WHERE action = 'skipped'",
            [],
            |row| row.get(0),
        )
        .expect("skipped changes");
    assert!(skipped >= 1);
}
