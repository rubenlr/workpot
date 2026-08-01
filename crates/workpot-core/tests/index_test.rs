#![allow(clippy::disallowed_methods)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use workpot_core::WorkpotError;
use workpot_core::domain::{Config, GitState};
use workpot_core::infra::store;
use workpot_core::services::git_state::GitRefreshResult;
use workpot_core::services::{catalog, index};

fn git_worktree(parent: &Path, name: &str) -> PathBuf {
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
fn index_purges_orphan_scan_repos() {
    let dir = tempfile::tempdir().expect("tempdir");
    let watch_root = dir.path().join("watch");
    fs::create_dir_all(&watch_root).expect("watch root");
    let orphan_parent = dir.path().join("orphan-parent");
    let orphan_repo = git_worktree(&orphan_parent, "solo");
    let orphan_key = orphan_repo
        .canonicalize()
        .expect("canonicalize orphan")
        .display()
        .to_string();

    let db_path = dir.path().join("workpot.db");
    let mut config = Config::default();
    config.watch_roots.push(watch_root);
    let conn = store::open_connection(&db_path).expect("open db");
    conn.execute(
        "INSERT INTO repos (path, name, registered_at, source, git_common_dir, excluded)
         VALUES (?1, 'solo', 0, 'scan', '', 0)",
        rusqlite::params![orphan_key],
    )
    .expect("insert orphan scan row");

    let summary = index::run_full_connection(&conn, &config).expect("run_full");
    assert_eq!(
        summary.removed, 1,
        "scan repo outside configured watch roots must be purged"
    );

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM repos WHERE excluded = 0", [], |row| {
            row.get(0)
        })
        .expect("count");
    assert_eq!(count, 0);
}

#[test]
fn index_full_rescan() {
    let (_dir, conn, config) = open_index_fixture(None);
    let watch_root = config.watch_roots[0].clone();
    git_worktree(&watch_root, "repo-a");
    git_worktree(&watch_root, "repo-b");

    index::run_full_connection(&conn, &config).expect("first run_full");
    let count_after_first: i64 = conn
        .query_row("SELECT COUNT(*) FROM repos WHERE excluded = 0", [], |row| {
            row.get(0)
        })
        .expect("count repos");
    assert_eq!(count_after_first, 2);

    index::run_full_connection(&conn, &config).expect("second run_full");
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

    let summary = index::run_full_connection(&conn, &config).expect("run_full");
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

    index::run_full_connection(&conn, &config).expect("run_full");

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

    index::run_full_connection(&conn, &config).expect("run_full");

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
    index::run_full_connection(&conn, &config).expect("first index");

    fs::remove_dir_all(&repo).expect("remove repo dir");

    let summary = index::run_full_connection(&conn, &config).expect("second index");
    assert_eq!(summary.removed, 1);

    let removed_changes: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM index_changes WHERE action = 'removed'",
            [],
            |row| row.get(0),
        )
        .expect("removed changes");
    assert_eq!(
        removed_changes, 1,
        "stale path must appear in index_changes"
    );

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
    index::run_full_connection(&conn, &config).expect("index keeps valid manual");

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

    let err = index::run_full_connection(&conn, &config).unwrap_err();
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

    let summary = index::run_full_connection(&conn, &config).expect("run_full");
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
fn index_git_summary_accounts_for_all_non_excluded_repos() {
    let (_dir, conn, config) = open_index_fixture(None);
    let watch_root = config.watch_roots[0].clone();
    git_worktree(&watch_root, "repo-a");
    git_worktree(&watch_root, "repo-b");

    let summary = index::run_full_connection(&conn, &config).expect("run_full");

    let non_excluded: i64 = conn
        .query_row("SELECT COUNT(*) FROM repos WHERE excluded = 0", [], |row| {
            row.get(0)
        })
        .expect("count");
    assert_eq!(non_excluded, 2);

    let accounted = summary.git_refreshed + summary.git_errors;
    assert_eq!(
        accounted, non_excluded as u32,
        "git pass must classify every indexed repo (D-16/D-17): {summary:?}"
    );
    assert_eq!(
        summary.git_refreshed, 2,
        "both repos should refresh cleanly"
    );
    assert_eq!(summary.git_errors, 0);
}

#[test]
fn index_second_pass_persists_git_state() {
    let (_dir, conn, config) = open_index_fixture(None);
    let watch_root = config.watch_roots[0].clone();
    let repo_path = git_worktree(&watch_root, "git-state-repo");
    let path_key = repo_path
        .canonicalize()
        .expect("canonical")
        .display()
        .to_string();

    let summary = index::run_full_connection(&conn, &config).expect("run_full");
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

    index::run_full_connection(&conn, &config).expect("first index");

    let plain_key = plain
        .canonicalize()
        .expect("canonical")
        .display()
        .to_string();
    conn.execute(
        "INSERT INTO repos (path, name, registered_at, source, git_common_dir, excluded)
         VALUES (?1, 'not-git', 0, 'manual', '', 0)",
        rusqlite::params![plain_key],
    )
    .expect("seed non-git row for git pass");

    let summary = index::run_full_connection(&conn, &config).expect("second index");
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
fn discover_phase_includes_missing_repo_paths_in_removes() {
    let (_dir, conn, config) = open_index_fixture(None);
    let gone_key = "/tmp/workpot-discover-missing-repo";
    conn.execute(
        "INSERT INTO repos (path, name, registered_at, source, git_common_dir, excluded)
         VALUES (?1, 'gone', 0, 'manual', '', 0)",
        rusqlite::params![gone_key],
    )
    .expect("seed missing row");

    let plan = index::discover_phase(&conn, &config).expect("discover");
    assert!(
        plan.removes.iter().any(|p| p == gone_key),
        "missing path should be scheduled for removal: {:?}",
        plan.removes
    );
}

#[test]
fn merge_catalog_phase_applies_discovery_plan() {
    let (_dir, conn, config) = open_index_fixture(None);
    let watch_root = config.watch_roots[0].clone();
    let repo_path = git_worktree(&watch_root, "phase-merge");
    let path_key = repo_path
        .canonicalize()
        .expect("canonicalize")
        .display()
        .to_string();
    let gcd = workpot_core::infra::git::resolve_git_common_dir(&repo_path)
        .expect("gcd")
        .display()
        .to_string();

    let plan = index::discover_phase(&conn, &config).expect("discover");
    let summary = index::merge_catalog_phase(&conn, &config, plan).expect("merge");
    assert_eq!(summary.added, 1);

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM repos WHERE path = ?1 AND excluded = 0",
            rusqlite::params![path_key],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(count, 1);

    let stored_gcd: String = conn
        .query_row(
            "SELECT git_common_dir FROM repos WHERE path = ?1",
            rusqlite::params![path_key],
            |row| row.get(0),
        )
        .expect("gcd");
    assert_eq!(stored_gcd, gcd);
}

#[test]
fn persist_index_git_phase_updates_repo_git_columns() {
    let (_dir, conn, config) = open_index_fixture(None);
    let watch_root = config.watch_roots[0].clone();
    let repo_path = git_worktree(&watch_root, "persist-git");
    let path_key = repo_path
        .canonicalize()
        .expect("canonicalize")
        .display()
        .to_string();

    index::run_full_connection(&conn, &config).expect("seed catalog row");

    let mut summary = index::IndexSummary::default();
    let git_results = vec![GitRefreshResult {
        path: path_key.clone(),
        state: GitState {
            branch: Some("main".to_string()),
            is_dirty: Some(false),
            ahead: Some(1),
            behind: Some(2),
            error: None,
        },
    }];

    index::persist_index_git_phase(&conn, &config, &mut summary, git_results).expect("persist git");
    assert_eq!(summary.git_refreshed, 1);
    assert_eq!(summary.git_errors, 0);

    let (branch, ahead, behind, git_err): (String, i64, i64, Option<String>) = conn
        .query_row(
            "SELECT branch, ahead, behind, git_state_error FROM repos WHERE path = ?1",
            rusqlite::params![path_key],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("git columns");
    assert_eq!(branch, "main");
    assert_eq!(ahead, 1);
    assert_eq!(behind, 2);
    assert!(git_err.is_none());
}

#[test]
fn index_git_failure_writes_skipped() {
    let (_dir, conn, config) = open_index_fixture(None);
    let watch_root = config.watch_roots[0].clone();
    git_worktree(&watch_root, "good");
    fake_git_dir(&watch_root, "bad");

    index::run_full_connection(&conn, &config).expect("run_full");

    let skipped: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM index_changes WHERE action = 'skipped'",
            [],
            |row| row.get(0),
        )
        .expect("skipped changes");
    assert!(skipped >= 1);
}

fn seed_committed_repo(repo: &Path) {
    for (key, val) in [("user.email", "t@example.com"), ("user.name", "Test")] {
        let status = common::git_cmd()
            .args(["config", key, val])
            .current_dir(repo)
            .status()
            .expect("config");
        assert!(status.success());
    }
    let marker = repo.join("README");
    fs::write(&marker, "tracked\n").expect("write");
    let status = common::git_cmd()
        .args(["add", "README"])
        .current_dir(repo)
        .status()
        .expect("add");
    assert!(status.success());
    let status = common::git_cmd()
        .args(["commit", "-m", "init", "-q"])
        .current_dir(repo)
        .status()
        .expect("commit");
    assert!(status.success());
}

#[test]
fn index_persists_null_structural_block_for_volatile_dirty_repo() {
    let (_dir, conn, mut config) = open_index_fixture(None);
    config.migration.allow_conversion_to_bare_repo = true;
    let watch_root = config.watch_roots[0].clone();
    let repo_path = git_worktree(&watch_root, "dirty-volatile");
    seed_committed_repo(&repo_path);
    fs::write(repo_path.join("README"), "dirty\n").expect("dirty");

    index::run_full_connection(&conn, &config).expect("index");

    let path_key = repo_path
        .canonicalize()
        .expect("canon")
        .display()
        .to_string();
    let (is_dirty, block_reason): (Option<i64>, Option<String>) = conn
        .query_row(
            "SELECT is_dirty, convert_block_reason FROM repos WHERE path = ?1",
            rusqlite::params![path_key],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("row");
    assert_eq!(is_dirty, Some(1));
    assert_eq!(block_reason, None);
}

#[test]
fn index_persists_structural_linked_worktree_block() {
    let (_dir, conn, mut config) = open_index_fixture(None);
    config.migration.allow_conversion_to_bare_repo = true;
    let watch_root = config.watch_roots[0].clone();

    let bare_path = watch_root.join("proj.git");
    fs::create_dir_all(&bare_path).expect("bare dir");
    let status = common::git_cmd()
        .args(["init", "--bare", "-q", "-b", "main"])
        .current_dir(&bare_path)
        .status()
        .expect("bare init");
    assert!(status.success());
    common::seed_bare_repo(&bare_path);

    let wt_path = watch_root.join("proj");
    let status = common::git_cmd()
        .args([
            "worktree",
            "add",
            "-q",
            wt_path.to_str().expect("utf8"),
            "main",
        ])
        .current_dir(&bare_path)
        .status()
        .expect("worktree add");
    assert!(status.success());

    index::run_full_connection(&conn, &config).expect("index");

    let path_key = wt_path.canonicalize().expect("canon").display().to_string();
    let block_reason: Option<String> = conn
        .query_row(
            "SELECT convert_block_reason FROM repos WHERE path = ?1",
            rusqlite::params![path_key],
            |row| row.get(0),
        )
        .expect("row");
    assert!(
        block_reason
            .as_deref()
            .is_some_and(|r| r.contains("git worktree")),
        "expected linked worktree structural block, got {block_reason:?}"
    );
}
