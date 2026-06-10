#![allow(clippy::disallowed_methods)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use workpot_core::AppContext;
use workpot_core::domain::config::MigrationConfig;
use workpot_core::services::repo_convert::{
    self, ConvertResult, ConvertTarget, PreflightResult, catalog_path_swap,
};

fn normal_repo_clean_synced(parent: &Path) -> PathBuf {
    let bare_path = parent.join("remote.git");
    fs::create_dir_all(&bare_path).expect("bare dir");
    let status = common::git_cmd()
        .args(["init", "--bare", "-q", "-b", "main"])
        .current_dir(&bare_path)
        .status()
        .expect("bare init");
    assert!(status.success());
    common::seed_bare_repo(&bare_path);

    let clone_path = parent.join("repo");
    let status = common::git_cmd()
        .args([
            "clone",
            "-q",
            bare_path.to_str().expect("utf8"),
            clone_path.to_str().expect("utf8"),
        ])
        .status()
        .expect("clone");
    assert!(status.success());

    for (key, val) in [("user.email", "t@example.com"), ("user.name", "Test")] {
        let status = common::git_cmd()
            .args(["config", key, val])
            .current_dir(&clone_path)
            .status()
            .expect("config");
        assert!(status.success(), "git config {key}");
    }
    clone_path
}

fn dirty_normal_repo(parent: &Path) -> PathBuf {
    let path = normal_repo_clean_synced(parent);
    let marker = path.join("README");
    fs::write(&marker, "tracked\n").expect("write");
    let status = common::git_cmd()
        .args(["add", "README"])
        .current_dir(&path)
        .status()
        .expect("add");
    assert!(status.success());
    let status = common::git_cmd()
        .args(["commit", "-m", "add readme", "-q"])
        .current_dir(&path)
        .status()
        .expect("commit");
    assert!(status.success());
    fs::write(&marker, "dirty\n").expect("dirty");
    path
}

fn unpushed_normal_repo(parent: &Path) -> PathBuf {
    let path = normal_repo_clean_synced(parent);
    let status = common::git_cmd()
        .args(["commit", "--allow-empty", "-m", "local-only", "-q"])
        .current_dir(&path)
        .status()
        .expect("commit");
    assert!(status.success());
    path
}

fn stash_normal_repo(parent: &Path) -> PathBuf {
    let path = normal_repo_clean_synced(parent);
    let marker = path.join("README");
    fs::write(&marker, "tracked\n").expect("write");
    let status = common::git_cmd()
        .args(["add", "README"])
        .current_dir(&path)
        .status()
        .expect("add");
    assert!(status.success());
    let status = common::git_cmd()
        .args(["commit", "-m", "add readme", "-q"])
        .current_dir(&path)
        .status()
        .expect("commit");
    assert!(status.success());
    let status = common::git_cmd()
        .args(["push", "-q"])
        .current_dir(&path)
        .status()
        .expect("push");
    assert!(status.success());
    fs::write(&marker, "wip\n").expect("wip");
    let status = common::git_cmd()
        .args(["stash", "-q"])
        .current_dir(&path)
        .status()
        .expect("stash");
    assert!(status.success());
    path
}

fn unborn_normal_repo(parent: &Path) -> PathBuf {
    let path = parent.join("unborn");
    fs::create_dir_all(&path).expect("mkdir");
    let status = common::git_cmd()
        .args(["init", "-q", "-b", "main"])
        .arg(&path)
        .status()
        .expect("init");
    assert!(status.success());
    for (key, val) in [("user.email", "t@example.com"), ("user.name", "Test")] {
        let status = common::git_cmd()
            .args(["config", key, val])
            .current_dir(&path)
            .status()
            .expect("config");
        assert!(status.success());
    }
    path
}

fn no_upstream_normal_repo(parent: &Path) -> PathBuf {
    let path = parent.join("no-upstream");
    fs::create_dir_all(&path).expect("mkdir");
    let status = common::git_cmd()
        .args(["init", "-q", "-b", "main"])
        .arg(&path)
        .status()
        .expect("init");
    assert!(status.success());
    for (key, val) in [("user.email", "t@example.com"), ("user.name", "Test")] {
        let status = common::git_cmd()
            .args(["config", key, val])
            .current_dir(&path)
            .status()
            .expect("config");
        assert!(status.success());
    }
    let status = common::git_cmd()
        .args(["commit", "--allow-empty", "-m", "init", "-q"])
        .current_dir(&path)
        .status()
        .expect("commit");
    assert!(status.success());
    path
}

fn bare_repo_with_worktree(parent: &Path) -> (PathBuf, PathBuf) {
    let bare_path = parent.join("myproject.git");
    fs::create_dir_all(&bare_path).expect("bare dir");
    let status = common::git_cmd()
        .args(["init", "--bare", "-q", "-b", "main"])
        .current_dir(&bare_path)
        .status()
        .expect("bare init");
    assert!(status.success());
    for (key, val) in [("user.email", "t@example.com"), ("user.name", "Test")] {
        let status = common::git_cmd()
            .args(["config", key, val])
            .current_dir(&bare_path)
            .status()
            .expect("config");
        assert!(status.success());
    }
    common::seed_bare_repo(&bare_path);

    let wt_path = parent.join("wt-main");
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
    let status = common::git_cmd()
        .args(["remote", "add", "origin"])
        .arg(&bare_path)
        .current_dir(&wt_path)
        .status()
        .expect("remote");
    assert!(status.success());
    let status = common::git_cmd()
        .args(["push", "-q", "-u", "origin", "main"])
        .current_dir(&wt_path)
        .status()
        .expect("push upstream");
    assert!(status.success());
    (bare_path, wt_path)
}

fn bare_repo_with_dirty_worktree(parent: &Path) -> PathBuf {
    let (bare_path, wt_path) = bare_repo_with_worktree(parent);
    let marker = wt_path.join("README");
    fs::write(&marker, "tracked\n").expect("write");
    let status = common::git_cmd()
        .args(["add", "README"])
        .current_dir(&wt_path)
        .status()
        .expect("add");
    assert!(status.success());
    let status = common::git_cmd()
        .args(["commit", "-m", "add readme", "-q"])
        .current_dir(&wt_path)
        .status()
        .expect("commit");
    assert!(status.success());
    fs::write(&marker, "dirty\n").expect("dirty");
    bare_path
}

fn test_ctx(parent: &Path) -> AppContext {
    let config_path = parent.join("config.toml");
    let db_path = parent.join("workpot.db");
    fs::write(&config_path, "watch_roots = []\nexcludes = []\n").expect("config");
    AppContext::open_with_paths(config_path, db_path).expect("open ctx")
}

#[test]
fn sanitize_worktree_replaces_slash_with_dot() {
    assert_eq!(
        repo_convert::sanitize_worktree("feature/my-branch"),
        "feature.my-branch"
    );
}

#[test]
fn sanitize_worktree_no_slash_unchanged() {
    assert_eq!(repo_convert::sanitize_worktree("main"), "main");
}

#[test]
fn unique_worktree_no_collision() {
    assert_eq!(
        repo_convert::unique_worktree_name("feature/x", &[]),
        "feature.x"
    );
}

#[test]
fn unique_worktree_collision_appends_hash() {
    let result = repo_convert::unique_worktree_name("feature/x", &["feature.x".to_string()]);
    assert!(result.starts_with("feature.x."));
    assert!(result.len() > "feature.x".len());
}

#[test]
fn resolve_template_substitutes_project() {
    assert_eq!(
        repo_convert::resolve_template("{project}/bare.git", "myproject", ""),
        "myproject/bare.git"
    );
}

#[test]
fn resolve_template_substitutes_both() {
    assert_eq!(
        repo_convert::resolve_template("{project}/wtrees/{worktree}", "myproject", "feature.login"),
        "myproject/wtrees/feature.login"
    );
}

#[test]
fn migration_config_defaults_match_spec() {
    let cfg = MigrationConfig::default();
    assert_eq!(cfg.temp_suffix, ".temp");
    assert!(!cfg.delete_original);
    assert_eq!(cfg.bare_repo_template, "{project}/bare.git");
    assert_eq!(cfg.worktree_template, "{project}/wtrees/{worktree}");
}

#[test]
fn migration_config_serde_round_trip() {
    let cfg = MigrationConfig::default();
    let toml = toml::to_string(&cfg).expect("serialize");
    let back: MigrationConfig = toml::from_str(&toml).expect("deserialize");
    assert_eq!(cfg, back);
}

#[test]
fn preflight_blocks_dirty() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dirty_normal_repo(dir.path());
    let result = repo_convert::run_preflight(&path).expect("preflight");
    assert!(matches!(result, PreflightResult::DirtyWorktree { .. }));
}

#[test]
fn preflight_blocks_unborn() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = unborn_normal_repo(dir.path());
    let result = repo_convert::run_preflight(&path).expect("preflight");
    assert_eq!(result, PreflightResult::UnbornBranch);
}

#[test]
fn preflight_blocks_no_upstream() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = no_upstream_normal_repo(dir.path());
    let result = repo_convert::run_preflight(&path).expect("preflight");
    assert!(matches!(result, PreflightResult::NoUpstream { .. }));
}

#[test]
fn preflight_blocks_unpushed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = unpushed_normal_repo(dir.path());
    let result = repo_convert::run_preflight(&path).expect("preflight");
    assert!(matches!(result, PreflightResult::UnpushedCommits { .. }));
}

#[test]
fn preflight_blocks_stash() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = stash_normal_repo(dir.path());
    let result = repo_convert::run_preflight(&path).expect("preflight");
    assert_eq!(result, PreflightResult::HasStash);
}

#[test]
fn preflight_passes_clean_synced() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = normal_repo_clean_synced(dir.path());
    let result = repo_convert::run_preflight(&path).expect("preflight");
    assert_eq!(result, PreflightResult::Ready);
}

#[test]
fn preflight_blocks_dirty_worktree_in_bare_repo() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = bare_repo_with_dirty_worktree(dir.path());
    let result = repo_convert::run_preflight(&path).expect("preflight");
    assert!(matches!(result, PreflightResult::DirtyWorktree { .. }));
}

#[test]
fn preflight_bare_passes_clean_synced() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (bare_path, wt_path) = bare_repo_with_worktree(dir.path());
    let _ = wt_path;
    let result = repo_convert::run_preflight(&bare_path).expect("preflight");
    assert_eq!(result, PreflightResult::Ready);
}

#[test]
fn catalog_path_swap_preserves_tags() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(dir.path());
    let path = normal_repo_clean_synced(dir.path());
    ctx.register_manual(&path).expect("register");
    let old_key = path.canonicalize().expect("canon").display().to_string();
    ctx.set_tags(&old_key, &["keep-me"]).expect("set tags");

    let new_path = dir.path().join("new-location");
    fs::create_dir_all(&new_path).expect("mkdir");
    let new_key = new_path
        .canonicalize()
        .expect("canon")
        .display()
        .to_string();

    let conn = workpot_core::infra::store::open_connection(ctx.database_path()).expect("conn");
    catalog_path_swap(&conn, &old_key, &new_key, "new-location", "/tmp/fake.git").expect("swap");

    let repos = ctx.list_repos().expect("list");
    assert!(
        repos
            .iter()
            .any(|r| r.path.display().to_string() == new_key)
    );
    assert!(
        !repos
            .iter()
            .any(|r| r.path.display().to_string() == old_key)
    );
    let tags = ctx.list_tags_for_repo(&new_key).expect("tags");
    assert_eq!(tags, vec!["keep-me".to_string()]);
}

#[test]
fn convert_normal_to_bare() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(dir.path());
    let path = normal_repo_clean_synced(dir.path());
    let old_key = path.canonicalize().expect("canon").display().to_string();
    ctx.register_manual(&path).expect("register");
    ctx.set_tags(&old_key, &["migrated"]).expect("tag");

    let result = ctx
        .convert_repo(&path, ConvertTarget::Bare, false)
        .expect("convert");
    let ConvertResult::Converted { from, to } = result else {
        panic!("expected Converted, got {result:?}");
    };
    assert_eq!(from, path.canonicalize().expect("canon"));
    assert!(
        to.join("HEAD").is_file(),
        "bare repo should exist at {to:?}"
    );
    let project = path.file_name().and_then(|n| n.to_str()).expect("name");
    let worktree_path = dir.path().join(project).join("wtrees").join("main");
    assert!(
        worktree_path.join(".git").exists(),
        "expected worktree at {}",
        worktree_path.display()
    );

    let repos = ctx.list_repos().expect("list");
    assert!(
        !repos
            .iter()
            .any(|r| r.path.display().to_string() == old_key)
    );
    assert!(repos.iter().any(|r| r.path == to));
    let new_key = to.display().to_string();
    let tags = ctx.list_tags_for_repo(&new_key).expect("tags");
    assert_eq!(tags, vec!["migrated".to_string()]);
}

#[test]
fn convert_bare_to_normal() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(dir.path());
    let (bare_path, _wt) = bare_repo_with_worktree(dir.path());
    let _old_key = bare_path
        .canonicalize()
        .expect("canon")
        .display()
        .to_string();
    ctx.register_manual(&bare_path).expect("register");

    let result = ctx
        .convert_repo(&bare_path, ConvertTarget::Normal, false)
        .expect("convert");
    let ConvertResult::Converted { from, to } = result else {
        panic!("expected Converted");
    };
    assert_eq!(from, bare_path.canonicalize().expect("canon"));
    assert!(to.join(".git").exists(), "normal checkout at {to:?}");

    let repos = ctx.list_repos().expect("list");
    assert!(repos.iter().any(|r| r.path == to));
    assert!(to.join(".git").exists());
}
