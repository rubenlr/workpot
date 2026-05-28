use std::fs;
use std::path::PathBuf;
use workpot_core::AppContext;

fn git_fixture() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().join("sample-repo");
    fs::create_dir_all(&repo).expect("repo dir");
    fs::create_dir_all(repo.join(".git")).expect(".git dir");
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
    assert!(matches!(err, workpot_core::WorkpotError::NotGitRepo(_)));
}

#[test]
fn register_rejects_duplicate() {
    let (dir, repo_path) = git_fixture();
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    ctx.register_manual(&repo_path).expect("first register");
    let err = ctx.register_manual(&repo_path).unwrap_err();
    assert!(matches!(
        err,
        workpot_core::WorkpotError::AlreadyRegistered(_)
    ));
}
