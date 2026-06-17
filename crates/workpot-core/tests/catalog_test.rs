#![allow(clippy::disallowed_methods)]

mod common;

use std::fs;
use std::path::PathBuf;
use workpot_core::AppContext;
use workpot_core::WorkpotError;
use workpot_core::domain::{SOURCE_MANUAL, SOURCE_SCAN};
use workpot_core::infra::{git, store};
use workpot_core::services::catalog;

fn git_init(repo: &std::path::Path) {
    let status = common::git_cmd()
        .args(["init", "-q"])
        .current_dir(repo)
        .status()
        .expect("git init");
    assert!(status.success(), "git init failed for {}", repo.display());
}

fn git_fixture() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().join("sample-repo");
    fs::create_dir_all(&repo).expect("repo dir");
    git_init(&repo);
    (dir, repo)
}

fn bare_git_fixture() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().join("bare-repo");
    fs::create_dir_all(&repo).expect("bare dir");
    let status = common::git_cmd()
        .args(["init", "-q", "--bare"])
        .current_dir(&repo)
        .status()
        .expect("git init --bare");
    assert!(status.success(), "git init --bare failed");
    (dir, repo)
}

fn relative_gitdir_fixture() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let main = dir.path().join("main");
    fs::create_dir_all(&main).expect("main dir");
    git_init(&main);
    let linked = dir.path().join("linked");
    fs::create_dir_all(&linked).expect("linked dir");
    fs::write(linked.join(".git"), "gitdir: ../main/.git\n").expect(".git file");
    (dir, linked)
}

fn gitdir_file_worktree_fixture() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let main = dir.path().join("main");
    fs::create_dir_all(&main).expect("main dir");
    git_init(&main);
    for (key, val) in [("user.email", "t@example.com"), ("user.name", "Test")] {
        let status = common::git_cmd()
            .args(["config", key, val])
            .current_dir(&main)
            .status()
            .expect("git config");
        assert!(status.success(), "git config {key} failed");
    }
    let status = common::git_cmd()
        .args(["commit", "--allow-empty", "-m", "init", "-q"])
        .current_dir(&main)
        .status()
        .expect("git commit");
    assert!(status.success(), "git commit failed");
    let linked = dir.path().join("linked-worktree");
    let status = common::git_cmd()
        .args([
            "worktree",
            "add",
            "-q",
            linked.to_str().expect("utf-8 path"),
        ])
        .current_dir(&main)
        .status()
        .expect("git worktree add");
    assert!(status.success(), "git worktree add failed");
    (dir, linked)
}

#[test]
fn list_repos_returns_git_state_after_index() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(&config_path, "watch_roots = []\nexcludes = []\n").expect("write config");

    let watch = dir.path().join("watch");
    fs::create_dir_all(&watch).expect("watch");
    let repo_path = watch.join("indexed");
    fs::create_dir_all(&repo_path).expect("repo dir");
    git_init(&repo_path);

    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
    ctx.roots_add(&watch).expect("roots_add");
    ctx.run_index().expect("index");

    let repos = ctx.list_repos().expect("list");
    let canon = repo_path.canonicalize().expect("canonicalize");
    let record = repos
        .iter()
        .find(|r| r.path == canon)
        .unwrap_or_else(|| panic!("expected repo at {}", canon.display()));
    assert!(
        record.git_refreshed_at.is_some(),
        "list_repos must surface git_refreshed_at after index git pass"
    );
    assert!(
        record.branch.is_some(),
        "list_repos must surface branch after refresh"
    );
    assert_eq!(
        record.is_dirty,
        Some(false),
        "fresh init repo should be clean"
    );
    assert!(record.git_state_error.is_none());
}

#[test]
fn register_manual_sets_source_manual() {
    let (dir, repo_path) = git_fixture();
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    ctx.register_manual(&repo_path).expect("register");
    let repos = ctx.list_repos().expect("list");
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].source, "manual");
}

#[test]
fn register_manual_leaves_git_columns_null() {
    let (dir, repo_path) = git_fixture();
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    ctx.register_manual(&repo_path).expect("register");
    let record = &ctx.list_repos().expect("list")[0];
    assert!(
        record.git_refreshed_at.is_none(),
        "manual register must not set git_refreshed_at (D-06)"
    );
    assert!(record.branch.is_none(), "branch unset until git refresh");
    assert!(record.is_dirty.is_none());
    assert!(record.ahead.is_none() && record.behind.is_none());
    assert!(record.git_state_error.is_none());
}

#[test]
fn register_accepts_relative_gitdir_file() {
    let (dir, repo_path) = relative_gitdir_fixture();
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    let record = ctx
        .register_manual(&repo_path)
        .expect("register relative gitdir worktree");
    assert_eq!(record.path, repo_path.canonicalize().expect("canonicalize"));
}

#[test]
fn repo_persists_across_reopen() {
    let (dir, repo_path) = git_fixture();
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");

    {
        let ctx =
            AppContext::open_with_paths(config_path.clone(), db_path.clone()).expect("first open");
        let record = ctx.register_manual(&repo_path).expect("register");
        let canonical = repo_path.canonicalize().expect("canonicalize");
        assert_eq!(record.path, canonical);
    }

    {
        let ctx = AppContext::open_with_paths(config_path, db_path).expect("second open");
        let repos = ctx.list_repos().expect("list");
        assert_eq!(repos.len(), 1);
        assert_eq!(
            repos[0].path,
            repo_path.canonicalize().expect("canonicalize")
        );
    }
}

#[test]
fn register_rejects_non_git() {
    let dir = tempfile::tempdir().expect("tempdir");
    let not_git = dir.path().join("plain-dir");
    fs::create_dir_all(&not_git).expect("dir");

    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    let err = ctx.register_manual(&not_git).unwrap_err();
    assert!(matches!(err, WorkpotError::NotGitRepo(_)));
}

#[test]
fn register_rejects_empty_git_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fake_repo = dir.path().join("fake-repo");
    fs::create_dir_all(&fake_repo).expect("repo dir");
    fs::create_dir_all(fake_repo.join(".git")).expect("empty .git");

    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    let err = ctx.register_manual(&fake_repo).unwrap_err();
    assert!(matches!(err, WorkpotError::NotGitRepo(_)));
}

#[test]
fn register_rejects_file_not_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = dir.path().join("not-a-dir.txt");
    fs::write(&file_path, "plain file").expect("file");

    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    let err = ctx.register_manual(&file_path).unwrap_err();
    assert!(matches!(
        err,
        WorkpotError::InvalidPath(ref msg) if msg.contains("not a directory")
    ));
}

#[test]
fn register_rejects_missing_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("does-not-exist");

    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    let err = ctx.register_manual(&missing).unwrap_err();
    assert!(matches!(err, WorkpotError::InvalidPath(msg) if msg.contains("path does not exist")));
}

#[test]
fn register_accepts_bare_repo() {
    let (dir, repo_path) = bare_git_fixture();
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    let record = ctx.register_manual(&repo_path).expect("register bare");
    assert_eq!(record.path, repo_path.canonicalize().expect("canonicalize"));
}

#[test]
fn register_rejects_duplicate() {
    let (dir, repo_path) = git_fixture();
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    ctx.register_manual(&repo_path).expect("first register");
    let err = ctx.register_manual(&repo_path).unwrap_err();
    assert!(matches!(err, WorkpotError::AlreadyRegistered(_)));
}

#[test]
fn register_accepts_gitdir_file_worktree() {
    let (dir, repo_path) = gitdir_file_worktree_fixture();
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    let record = ctx
        .register_manual(&repo_path)
        .expect("register gitdir worktree");
    assert_eq!(record.path, repo_path.canonicalize().expect("canonicalize"));
}

#[test]
fn register_rejects_invalid_gitdir_target() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().join("bad-worktree");
    fs::create_dir_all(&repo).expect("worktree dir");
    fs::write(repo.join(".git"), "gitdir: /nonexistent/nowhere\n").expect(".git file");

    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    let err = ctx.register_manual(&repo).unwrap_err();
    assert!(matches!(err, WorkpotError::NotGitRepo(_)));
}

#[test]
fn list_repos_skips_excluded_rows() {
    let (dir, repo_path) = git_fixture();
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");

    {
        let ctx = AppContext::open_with_paths(config_path.clone(), db_path.clone()).expect("open");
        ctx.register_manual(&repo_path).expect("register");
    }

    {
        let conn = rusqlite::Connection::open(&db_path).expect("open db");
        let path_key = repo_path
            .canonicalize()
            .expect("canonicalize")
            .display()
            .to_string();
        conn.execute("UPDATE repos SET excluded = 1 WHERE path = ?1", [&path_key])
            .expect("mark excluded");
    }

    let ctx = AppContext::open_with_paths(config_path, db_path).expect("reopen");
    assert!(ctx.list_repos().expect("list").is_empty());
}

#[test]
fn remove_repo_succeeds_when_directory_deleted() {
    let (dir, repo_path) = git_fixture();
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let repo_name = repo_path
        .file_name()
        .expect("repo dir name")
        .to_string_lossy()
        .into_owned();
    let relative_remove = dir.path().join(&repo_name);

    {
        let ctx = AppContext::open_with_paths(config_path.clone(), db_path.clone()).expect("open");
        ctx.register_manual(&repo_path).expect("register");
        fs::remove_dir_all(&repo_path).expect("delete repo dir");
        ctx.remove_repo(&relative_remove)
            .expect("remove after delete via basename lookup");
        assert!(ctx.list_repos().expect("list").is_empty());
    }
}

#[test]
fn remove_repo_with_exclude_persists_excludes_before_row_removed() {
    let (dir, repo_path) = git_fixture();
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path.clone(), db_path.clone()).expect("open");

    ctx.register_manual(&repo_path).expect("register");
    ctx.remove_repo(&repo_path).expect("remove");

    let config_text = fs::read_to_string(&config_path).expect("read config");
    assert!(
        config_text.contains("/**"),
        "exclude tree glob must be on disk when row is removed: {config_text}"
    );

    let conn = rusqlite::Connection::open(&db_path).expect("open db");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM repos", [], |row| row.get(0))
        .expect("count");
    assert_eq!(
        count, 0,
        "repo row should be removed after excludes persisted"
    );
}

#[test]
fn remove_repo_with_exclude_does_not_persist_excludes_when_remove_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let first = dir.path().join("first").join("foo");
    let second = dir.path().join("second").join("foo");
    fs::create_dir_all(&first).expect("first foo dir");
    fs::create_dir_all(&second).expect("second foo dir");
    git_init(&first);
    git_init(&second);

    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path.clone(), db_path).expect("open");

    ctx.register_manual(&first).expect("register first foo");
    ctx.register_manual(&second).expect("register second foo");

    let err = ctx
        .remove_repo(PathBuf::from("foo").as_path())
        .expect_err("ambiguous basename remove should fail");
    assert!(matches!(err, WorkpotError::InvalidPath(_)));
    assert!(
        ctx.config().expect("config").excludes.is_empty(),
        "exclude globs must not be saved when repo row deletion fails"
    );
    assert_eq!(ctx.list_repos().expect("list").len(), 2);
}

#[test]
fn remove_repo_with_exclude_escapes_glob_metacharacters_in_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().join("star*repo");
    fs::create_dir_all(&repo).expect("repo dir");
    git_init(&repo);

    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path.clone(), db_path).expect("open");

    ctx.register_manual(&repo).expect("register");
    ctx.remove_repo(&repo).expect("remove");

    let config = std::fs::read_to_string(&config_path).expect("read config");
    let canon = repo.canonicalize().expect("canonicalize");
    let escaped_name = "star\\*repo";
    assert!(
        config.contains(&format!(
            "{}/{}",
            canon.parent().expect("parent").display(),
            escaped_name
        )),
        "expected escaped glob segment in config: {config}"
    );
}

#[test]
fn remove_repo_by_basename_does_not_match_similar_directory_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let parent = dir.path().join("collision-parent");
    fs::create_dir_all(&parent).expect("parent");
    let foo = parent.join("foo");
    let foo_extra = parent.join("foo-extra");
    fs::create_dir_all(&foo).expect("foo dir");
    fs::create_dir_all(&foo_extra).expect("foo-extra dir");
    git_init(&foo);
    git_init(&foo_extra);

    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    ctx.register_manual(&foo).expect("register foo");
    ctx.register_manual(&foo_extra).expect("register foo-extra");

    fs::remove_dir_all(&foo).expect("delete foo dir");
    ctx.remove_repo(&parent.join("foo"))
        .expect("remove foo by basename only");

    let remaining = ctx.list_repos().expect("list");
    assert_eq!(remaining.len(), 1);
    assert_eq!(
        remaining[0].path,
        foo_extra.canonicalize().expect("canonicalize")
    );
}

#[test]
fn remove_repo_by_basename_with_like_metacharacters_in_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().join("foo%bar");
    fs::create_dir_all(&repo).expect("repo dir");
    git_init(&repo);

    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let repo_name = repo
        .file_name()
        .expect("repo dir name")
        .to_string_lossy()
        .into_owned();
    let relative_remove = dir.path().join(&repo_name);

    {
        let ctx = AppContext::open_with_paths(config_path.clone(), db_path.clone()).expect("open");
        ctx.register_manual(&repo).expect("register");
        fs::remove_dir_all(&repo).expect("delete repo dir");
        ctx.remove_repo(&relative_remove)
            .expect("remove after delete via basename with % in name");
        assert!(ctx.list_repos().expect("list").is_empty());
    }

    let conn = rusqlite::Connection::open(&db_path).expect("open db");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM repos", [], |row| row.get(0))
        .expect("count repos");
    assert_eq!(
        count, 0,
        "row for foo%bar must be removed, not a LIKE false match"
    );
}

#[test]
fn upsert_scan_returns_true_on_insert_false_on_update() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().join("scan-repo");
    fs::create_dir_all(&repo).expect("repo dir");
    git_init(&repo);
    let conn = store::open_connection(&dir.path().join("workpot.db")).expect("open db");
    let canon = repo.canonicalize().expect("canonicalize");
    let gcd = git::resolve_git_common_dir(&canon)
        .expect("gcd")
        .display()
        .to_string();

    assert!(
        catalog::upsert_scan(&conn, &canon, &gcd).expect("insert"),
        "first upsert must report newly added"
    );
    assert!(
        !catalog::upsert_scan(&conn, &canon, &gcd).expect("update"),
        "second upsert must report existing path"
    );
}

#[test]
fn upsert_scan_preserves_manual_source_on_conflict() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().join("manual-repo");
    fs::create_dir_all(&repo).expect("repo dir");
    git_init(&repo);
    let conn = store::open_connection(&dir.path().join("workpot.db")).expect("open db");
    let canon = repo.canonicalize().expect("canonicalize");
    let path_key = canon.display().to_string();
    let gcd = git::resolve_git_common_dir(&canon)
        .expect("gcd")
        .display()
        .to_string();

    conn.execute(
        "INSERT INTO repos (path, name, registered_at, source, git_common_dir, excluded)
         VALUES (?1, 'manual-repo', 42, ?2, 'old-gcd', 0)",
        rusqlite::params![path_key, SOURCE_MANUAL],
    )
    .expect("seed manual row");

    catalog::upsert_scan(&conn, &canon, &gcd).expect("upsert over manual");

    let source: String = conn
        .query_row(
            "SELECT source FROM repos WHERE path = ?1",
            rusqlite::params![path_key],
            |row| row.get(0),
        )
        .expect("source");
    assert_eq!(source, SOURCE_MANUAL);
}

#[test]
fn upsert_scan_updates_git_common_dir_for_scan_rows() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().join("scan-repo");
    fs::create_dir_all(&repo).expect("repo dir");
    git_init(&repo);
    let conn = store::open_connection(&dir.path().join("workpot.db")).expect("open db");
    let canon = repo.canonicalize().expect("canonicalize");
    let path_key = canon.display().to_string();
    let gcd = git::resolve_git_common_dir(&canon)
        .expect("gcd")
        .display()
        .to_string();

    conn.execute(
        "INSERT INTO repos (path, name, registered_at, source, git_common_dir, excluded)
         VALUES (?1, 'scan-repo', 0, ?2, 'stale-gcd', 0)",
        rusqlite::params![path_key, SOURCE_SCAN],
    )
    .expect("seed scan row");

    catalog::upsert_scan(&conn, &canon, &gcd).expect("upsert");

    let stored: String = conn
        .query_row(
            "SELECT git_common_dir FROM repos WHERE path = ?1",
            rusqlite::params![path_key],
            |row| row.get(0),
        )
        .expect("gcd");
    assert_eq!(stored, gcd);
}

#[test]
fn get_repo_by_path_returns_record_and_not_found() {
    let (dir, repo_path) = git_fixture();
    let conn = store::open_connection(&dir.path().join("workpot.db")).expect("open db");
    let canon = repo_path.canonicalize().expect("canonicalize");
    let path_key = canon.display().to_string();
    let gcd = git::resolve_git_common_dir(&canon)
        .expect("gcd")
        .display()
        .to_string();

    conn.execute(
        "INSERT INTO repos (path, name, registered_at, source, git_common_dir, excluded)
         VALUES (?1, 'sample-repo', 42, ?2, ?3, 0)",
        rusqlite::params![path_key, SOURCE_MANUAL, gcd],
    )
    .expect("seed row");

    let record = catalog::get_repo_by_path(&conn, &path_key).expect("lookup");
    assert_eq!(record.path, canon);
    assert_eq!(record.name, "sample-repo");
    assert_eq!(record.registered_at, 42);
    assert_eq!(record.source, SOURCE_MANUAL);
    assert_eq!(record.git_common_dir, gcd);

    let err = catalog::get_repo_by_path(&conn, "/no/such/repo").expect_err("missing path");
    assert!(matches!(err, WorkpotError::NotFound(_)));
}

#[test]
fn missing_repo_paths_lists_only_gone_paths() {
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = store::open_connection(&dir.path().join("workpot.db")).expect("open db");
    let (_fixture_dir, repo_path) = git_fixture();
    let present_key = repo_path
        .canonicalize()
        .expect("canonicalize")
        .display()
        .to_string();
    let gone_key = "/tmp/workpot-missing-repo-path";

    conn.execute(
        "INSERT INTO repos (path, name, registered_at, source, git_common_dir, excluded)
         VALUES (?1, 'present', 0, 'manual', '', 0)",
        rusqlite::params![present_key],
    )
    .expect("present row");
    conn.execute(
        "INSERT INTO repos (path, name, registered_at, source, git_common_dir, excluded)
         VALUES (?1, 'gone', 0, 'manual', '', 0)",
        rusqlite::params![gone_key],
    )
    .expect("gone row");

    let missing = catalog::missing_repo_paths(&conn).expect("missing paths");
    assert_eq!(missing, vec![gone_key.to_string()]);
}

#[test]
fn remove_repo_deletes_and_not_found() {
    let (dir, repo_path) = git_fixture();
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    ctx.register_manual(&repo_path).expect("register");
    ctx.remove_repo(&repo_path).expect("remove");
    assert!(ctx.list_repos().expect("list").is_empty());

    let err = ctx.remove_repo(&repo_path).unwrap_err();
    assert!(matches!(err, WorkpotError::NotFound(_)));
}
