use std::fs;
use std::path::PathBuf;
use workpot_core::AppContext;
use workpot_core::WorkpotError;

fn git_fixture() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().join("sample-repo");
    fs::create_dir_all(&repo).expect("repo dir");
    let git_dir = repo.join(".git");
    fs::create_dir_all(git_dir.join("objects")).expect("objects");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("HEAD");
    (dir, repo)
}

fn bare_git_fixture() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().join("bare-repo");
    fs::create_dir_all(repo.join("objects")).expect("objects");
    fs::write(repo.join("HEAD"), "ref: refs/heads/main\n").expect("HEAD");
    (dir, repo)
}

fn gitdir_file_worktree_fixture() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let git_dir = dir.path().join("actual.git");
    fs::create_dir_all(git_dir.join("objects")).expect("objects");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("HEAD");

    let repo = dir.path().join("linked-worktree");
    fs::create_dir_all(&repo).expect("worktree dir");
    fs::write(
        repo.join(".git"),
        format!("gitdir: {}\n", git_dir.display()),
    )
    .expect(".git file");
    (dir, repo)
}

#[test]
fn repo_persists_across_reopen() {
    let (dir, repo_path) = git_fixture();
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");

    {
        let ctx = AppContext::open_with_paths(config_path.clone(), db_path.clone())
            .expect("first open");
        let record = ctx.register_manual(&repo_path).expect("register");
        let canonical = repo_path.canonicalize().expect("canonicalize");
        assert_eq!(record.path, canonical);
    }

    {
        let ctx = AppContext::open_with_paths(config_path, db_path).expect("second open");
        let repos = ctx.list_repos().expect("list");
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].path, repo_path.canonicalize().expect("canonicalize"));
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
    assert_eq!(
        record.path,
        repo_path.canonicalize().expect("canonicalize")
    );
}

#[test]
fn register_rejects_file_not_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = dir.path().join("not-a-dir");
    fs::write(&file_path, "x").expect("file");

    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    let err = ctx.register_manual(&file_path).unwrap_err();
    assert!(matches!(
        err,
        WorkpotError::InvalidPath(msg) if msg.contains("not a directory")
    ));
}

#[test]
fn list_repos_skips_excluded_rows() {
    let (dir, repo_path) = git_fixture();
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");

    {
        let ctx = AppContext::open_with_paths(config_path.clone(), db_path.clone())
            .expect("open");
        ctx.register_manual(&repo_path).expect("register");
    }

    {
        let conn = rusqlite::Connection::open(&db_path).expect("open db");
        let path_key = repo_path.canonicalize().expect("canonicalize").display().to_string();
        conn.execute("UPDATE repos SET excluded = 1 WHERE path = ?1", [&path_key])
            .expect("mark excluded");
    }

    let ctx = AppContext::open_with_paths(config_path, db_path).expect("reopen");
    assert!(ctx.list_repos().expect("list").is_empty());
}

#[test]
fn remove_repo_deletes_and_not_found() {
    let (dir, repo_path) = git_fixture();
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let mut ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    ctx.register_manual(&repo_path).expect("register");
    ctx.remove_repo(&repo_path).expect("remove");
    assert!(ctx.list_repos().expect("list").is_empty());

    let err = ctx.remove_repo(&repo_path).unwrap_err();
    assert!(matches!(err, WorkpotError::NotFound(_)));
}
