#![allow(clippy::disallowed_methods)]

mod common;

use std::fs;
use std::path::PathBuf;
use workpot_core::domain::GitState;
use workpot_core::services::git_state::GitRefreshResult;
use workpot_core::{AppContext, version};

fn git_worktree(parent: &std::path::Path, name: &str) -> PathBuf {
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
fn version_returns_crate_version() {
    assert!(!version().is_empty());
}

#[test]
fn database_path_and_excludes_list_accessors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(
        &config_path,
        "watch_roots = []\nexcludes = [\"/tmp/workpot-exclude/**\"]\n",
    )
    .expect("write config");

    let ctx = AppContext::open_with_paths(config_path, db_path.clone()).expect("open");
    assert_eq!(ctx.database_path(), db_path.as_path());
    assert!(
        ctx.excludes_list()
            .expect("excludes")
            .iter()
            .any(|e| e.contains("workpot-exclude"))
    );
}

#[test]
fn refresh_git_state_and_persist_via_app_context() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    let repo_path = git_worktree(dir.path(), "refresh-me");
    ctx.register_manual(&repo_path).expect("register");
    let path_key = repo_path
        .canonicalize()
        .expect("canon")
        .display()
        .to_string();

    let state = ctx
        .refresh_git_state(&repo_path)
        .expect("refresh_git_state");
    assert!(state.branch.is_some() || state.error.is_some());

    let persisted = ctx
        .refresh_and_persist_git_state(&repo_path)
        .expect("refresh_and_persist");
    assert!(persisted.error.is_none());

    let repos = ctx.list_repos().expect("list");
    let row = repos
        .iter()
        .find(|r| r.path.display().to_string() == path_key)
        .expect("repo row");
    assert!(row.branch.is_some());
}

#[test]
fn persist_git_refresh_hard_failure_increments_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    let repo_path = git_worktree(dir.path(), "hard-fail");
    ctx.register_manual(&repo_path).expect("register");
    let path_key = repo_path
        .canonicalize()
        .expect("canon")
        .display()
        .to_string();

    let results = vec![GitRefreshResult {
        path: path_key,
        state: GitState {
            branch: None,
            is_dirty: None,
            ahead: None,
            behind: None,
            error: Some("simulated hard failure".to_string()),
        },
    }];

    let summary = ctx.persist_git_refresh_results(results).expect("persist");
    assert_eq!(summary.errors, 1);
    assert_eq!(summary.refreshed, 0);
}

#[test]
fn org_facade_delegates_through_app_context() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");

    let repo_path = git_worktree(dir.path(), "org-facade");
    ctx.register_manual(&repo_path).expect("register");
    let path_key = repo_path
        .canonicalize()
        .expect("canon")
        .display()
        .to_string();

    ctx.set_tags(&path_key, &["backend", "infra"])
        .expect("set_tags");
    ctx.add_tag(&path_key, "cli").expect("add_tag");
    ctx.remove_tag(&path_key, "infra").expect("remove_tag");

    let tags = ctx.list_tags_for_repo(&path_key).expect("list tags");
    assert!(tags.contains(&"backend".to_string()));
    assert!(tags.contains(&"cli".to_string()));
    assert!(!tags.contains(&"infra".to_string()));

    let all_tags = ctx.list_all_tags().expect("list all");
    assert!(all_tags.iter().any(|t| t == "backend"));

    ctx.set_notes(&path_key, Some("notes")).expect("set_notes");
    ctx.set_alias(&path_key, Some("my-alias"))
        .expect("set_alias");
    ctx.set_pin(&path_key, true).expect("set_pin");
    ctx.set_pin_order(&[(&path_key, 0)]).expect("set_pin_order");

    let repos = ctx.list_repos().expect("list");
    let row = repos
        .iter()
        .find(|r| r.path.display().to_string() == path_key)
        .expect("repo row");
    assert_eq!(row.notes.as_deref(), Some("notes"));
    assert_eq!(row.alias.as_deref(), Some("my-alias"));
    assert!(row.pinned);
}

#[test]
fn app_context_open_resolves_default_paths_under_home() {
    let home = tempfile::tempdir().expect("tempdir");
    let prev_home = std::env::var_os("HOME");
    unsafe { std::env::set_var("HOME", home.path()) };

    let run = std::panic::catch_unwind(|| {
        let ctx = AppContext::open().expect("open");
        let config_path = home.path().join(".config/workpot/config.toml");
        let db_path = home
            .path()
            .join("Library/Application Support/workpot/workpot.db");
        assert_eq!(ctx.config_path(), config_path.as_path());
        assert_eq!(ctx.database_path(), db_path.as_path());
        assert!(config_path.is_file(), "open must create default config");
        assert!(db_path.is_file(), "open must create database");
    });

    match prev_home {
        Some(value) => unsafe { std::env::set_var("HOME", value) },
        None => unsafe { std::env::remove_var("HOME") },
    }

    run.expect("app_context_open test panicked");
}
