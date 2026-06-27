#![allow(clippy::disallowed_methods)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use workpot_core::AppContext;
use workpot_core::domain::config::MigrationConfig;
use workpot_core::services::catalog;
use workpot_core::services::git_state::refresh_and_persist;
use workpot_core::services::launch::launch_repo;
use workpot_core::services::repo_convert::{
    self, ConvertResult, ConvertTarget, PreflightResult, assess_conversion_readiness,
    assess_structural_blockers, catalog_path_swap, persist_structural_preflight_for_repo,
    run_volatile_preflight,
};

fn local_repo_clean_synced(parent: &Path) -> PathBuf {
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

fn dirty_local_repo(parent: &Path) -> PathBuf {
    let path = local_repo_clean_synced(parent);
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

fn unpushed_local_repo(parent: &Path) -> PathBuf {
    let path = local_repo_clean_synced(parent);
    let status = common::git_cmd()
        .args(["commit", "--allow-empty", "-m", "local-only", "-q"])
        .current_dir(&path)
        .status()
        .expect("commit");
    assert!(status.success());
    path
}

fn stash_local_repo(parent: &Path) -> PathBuf {
    let path = local_repo_clean_synced(parent);
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

fn unborn_local_repo(parent: &Path) -> PathBuf {
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

fn no_upstream_local_repo(parent: &Path) -> PathBuf {
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

fn bare_repo_with_two_worktrees(parent: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let (bare_path, wt_main) = bare_repo_with_worktree(parent);
    let status = common::git_cmd()
        .args(["branch", "feature", "main"])
        .current_dir(&bare_path)
        .status()
        .expect("branch feature");
    assert!(status.success());
    let wt_feature = parent.join("wt-feature");
    let status = common::git_cmd()
        .args([
            "worktree",
            "add",
            "-q",
            wt_feature.to_str().expect("utf8"),
            "feature",
        ])
        .current_dir(&bare_path)
        .status()
        .expect("worktree add feature");
    assert!(status.success());
    (bare_path, wt_main, wt_feature)
}

fn test_ctx(parent: &Path) -> AppContext {
    let config_path = parent.join("config.toml");
    let db_path = parent.join("workpot.db");
    fs::write(&config_path, "watch_roots = []\nexcludes = []\n").expect("config");
    AppContext::open_with_paths(config_path, db_path).expect("open ctx")
}

fn git_remote_url(repo: &Path, name: &str) -> String {
    let output = common::git_cmd()
        .args(["remote", "get-url", name])
        .current_dir(repo)
        .output()
        .expect("remote get-url");
    assert!(
        output.status.success(),
        "git remote get-url {name} failed in {}",
        repo.display()
    );
    String::from_utf8(output.stdout)
        .expect("utf8 stdout")
        .trim()
        .to_string()
}

fn git_remote_names(repo: &Path) -> Vec<String> {
    let output = common::git_cmd()
        .args(["remote"])
        .current_dir(repo)
        .output()
        .expect("remote");
    assert!(
        output.status.success(),
        "git remote failed in {}",
        repo.display()
    );
    String::from_utf8(output.stdout)
        .expect("utf8 stdout")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect()
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
    let path = dirty_local_repo(dir.path());
    let result = repo_convert::run_volatile_preflight(&path).expect("preflight");
    assert!(matches!(result, PreflightResult::DirtyWorktree { .. }));
}

#[test]
fn preflight_blocks_detached_head() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = local_repo_clean_synced(dir.path());
    let status = common::git_cmd()
        .args(["checkout", "-q", "--detach", "HEAD"])
        .current_dir(&path)
        .status()
        .expect("detach");
    assert!(status.success());
    let result = repo_convert::run_volatile_preflight(&path).expect("preflight");
    assert_eq!(result, PreflightResult::DetachedHead);
}

#[test]
fn preflight_blocks_unborn() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = unborn_local_repo(dir.path());
    let result = repo_convert::run_volatile_preflight(&path).expect("preflight");
    assert_eq!(result, PreflightResult::UnbornBranch);
}

#[test]
fn preflight_blocks_no_upstream() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = no_upstream_local_repo(dir.path());
    let result = repo_convert::run_volatile_preflight(&path).expect("preflight");
    assert!(matches!(result, PreflightResult::NoUpstream { .. }));
}

#[test]
fn preflight_blocks_unpushed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = unpushed_local_repo(dir.path());
    let result = repo_convert::run_volatile_preflight(&path).expect("preflight");
    assert!(matches!(result, PreflightResult::UnpushedCommits { .. }));
}

#[test]
fn preflight_blocks_stash() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = stash_local_repo(dir.path());
    let result = repo_convert::run_volatile_preflight(&path).expect("preflight");
    assert_eq!(result, PreflightResult::HasStash);
}

#[test]
fn preflight_passes_clean_synced() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = local_repo_clean_synced(dir.path());
    let result = repo_convert::run_volatile_preflight(&path).expect("preflight");
    assert_eq!(result, PreflightResult::Ready);
}

#[test]
fn preflight_blocks_dirty_worktree_in_bare_repo() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = bare_repo_with_dirty_worktree(dir.path());
    let result = repo_convert::run_volatile_preflight(&path).expect("preflight");
    assert!(matches!(result, PreflightResult::DirtyWorktree { .. }));
}

#[test]
fn preflight_bare_passes_clean_synced() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (bare_path, wt_path) = bare_repo_with_worktree(dir.path());
    let _ = wt_path;
    let result = repo_convert::run_volatile_preflight(&bare_path).expect("preflight");
    assert_eq!(result, PreflightResult::Ready);
}

#[test]
fn catalog_path_swap_preserves_tags() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(dir.path());
    let path = local_repo_clean_synced(dir.path());
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
    let path = local_repo_clean_synced(dir.path());
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
fn convert_normal_to_bare_preserves_origin_remote() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(dir.path());
    let path = local_repo_clean_synced(dir.path());
    let expected = git_remote_url(&path, "origin");
    ctx.register_manual(&path).expect("register");

    let result = ctx
        .convert_repo(&path, ConvertTarget::Bare, false)
        .expect("convert");
    let ConvertResult::Converted { to: bare_path, .. } = result else {
        panic!("expected Converted, got {result:?}");
    };

    let actual = git_remote_url(&bare_path, "origin");
    assert_eq!(actual, expected);
    assert!(!actual.contains(".temp"));
}

#[test]
fn convert_bare_to_local_preserves_origin_remote() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(dir.path());
    let (bare_path, _wt) = bare_repo_with_worktree(dir.path());
    let expected = git_remote_url(&bare_path, "origin");
    ctx.register_manual(&bare_path).expect("register");

    let result = ctx
        .convert_repo(&bare_path, ConvertTarget::Local, false)
        .expect("convert");
    let ConvertResult::Converted { to: local_path, .. } = result else {
        panic!("expected Converted, got {result:?}");
    };

    let actual = git_remote_url(&local_path, "origin");
    assert_eq!(actual, expected);
    assert!(!actual.contains(".temp"));
}

#[test]
fn convert_normal_to_bare_preserves_multiple_remotes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(dir.path());
    let path = local_repo_clean_synced(dir.path());
    let upstream_url = dir.path().join("upstream.git");
    fs::create_dir_all(&upstream_url).expect("upstream dir");
    let status = common::git_cmd()
        .args(["init", "--bare", "-q", "-b", "main"])
        .current_dir(&upstream_url)
        .status()
        .expect("upstream init");
    assert!(status.success());
    let status = common::git_cmd()
        .args([
            "remote",
            "add",
            "upstream",
            upstream_url.to_str().expect("utf8"),
        ])
        .current_dir(&path)
        .status()
        .expect("add upstream");
    assert!(status.success());

    let expected_origin = git_remote_url(&path, "origin");
    let expected_upstream = git_remote_url(&path, "upstream");
    ctx.register_manual(&path).expect("register");

    let result = ctx
        .convert_repo(&path, ConvertTarget::Bare, false)
        .expect("convert");
    let ConvertResult::Converted { to: bare_path, .. } = result else {
        panic!("expected Converted, got {result:?}");
    };

    let mut names = git_remote_names(&bare_path);
    names.sort();
    assert_eq!(names, vec!["origin".to_string(), "upstream".to_string()]);
    assert_eq!(git_remote_url(&bare_path, "origin"), expected_origin);
    assert_eq!(git_remote_url(&bare_path, "upstream"), expected_upstream);
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
        .convert_repo(&bare_path, ConvertTarget::Local, false)
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

#[test]
fn indexed_launch_path_prefers_catalog_branch_among_worktrees() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(dir.path());
    let (bare_path, wt_main, wt_feature) = bare_repo_with_two_worktrees(dir.path());
    let bare_key = bare_path
        .canonicalize()
        .expect("canon")
        .display()
        .to_string();
    ctx.register_manual(&bare_path).expect("register");
    let conn = workpot_core::infra::store::open_connection(ctx.database_path()).expect("conn");
    refresh_and_persist(&conn, &bare_path).expect("refresh");
    conn.execute(
        "UPDATE repos SET branch = 'feature' WHERE path = ?1",
        rusqlite::params![bare_key],
    )
    .expect("set branch");

    let resolved = catalog::indexed_launch_path(&conn, &bare_path).expect("resolve");
    assert_eq!(
        resolved,
        wt_feature.canonicalize().expect("canon feature wt")
    );
    assert_ne!(resolved, wt_main.canonicalize().expect("canon main wt"));
}

#[test]
fn launch_bare_catalog_path_updates_last_opened_at() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(
        &config_path,
        r#"watch_roots = []
excludes = []
launch_cmd = "/usr/bin/true {path}"
"#,
    )
    .expect("write config");
    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
    let path = local_repo_clean_synced(dir.path());
    ctx.register_manual(&path).expect("register");

    let result = ctx
        .convert_repo(&path, ConvertTarget::Bare, false)
        .expect("convert");
    let ConvertResult::Converted { to: bare_path, .. } = result else {
        panic!("expected Converted, got {result:?}");
    };
    let bare_key = bare_path.display().to_string();

    launch_repo(&ctx, &bare_key).expect("launch");

    let repos = ctx.list_repos().expect("list");
    let bare = repos
        .iter()
        .find(|r| r.path == bare_path)
        .expect("bare catalog row");
    assert!(
        bare.last_opened_at.is_some(),
        "last_opened_at should update when launching via bare catalog key"
    );
}

fn test_ctx_with_migration(parent: &Path, extra: &str) -> AppContext {
    let config_path = parent.join("config.toml");
    let db_path = parent.join("workpot.db");
    fs::write(
        &config_path,
        format!("watch_roots = []\nexcludes = []\n{extra}"),
    )
    .expect("config");
    AppContext::open_with_paths(config_path, db_path).expect("open ctx")
}

#[test]
fn resolve_project_name_uses_alias_when_configured() {
    use workpot_core::domain::RepoRecord;
    use workpot_core::domain::config::{MigrationConfig, ProjectNameSource};

    let cfg = MigrationConfig {
        project_name_source: ProjectNameSource::Alias,
        ..Default::default()
    };
    let record = RepoRecord {
        path: PathBuf::from("/tmp/repo"),
        name: "folder-name".into(),
        registered_at: 0,
        source: "manual".into(),
        git_common_dir: String::new(),
        branch: None,
        is_dirty: None,
        ahead: None,
        behind: None,
        git_refreshed_at: None,
        git_state_error: None,
        last_opened_at: None,
        pinned: false,
        pin_order: None,
        notes: None,
        tags: vec![],
        alias: Some("display-alias".into()),
        convert_block_reason: None,
    };
    assert_eq!(
        repo_convert::resolve_project_name(&cfg, &record),
        "display-alias"
    );
}

#[test]
fn resolve_project_name_falls_back_to_folder_without_alias() {
    use workpot_core::domain::RepoRecord;
    use workpot_core::domain::config::{MigrationConfig, ProjectNameSource};

    let cfg = MigrationConfig {
        project_name_source: ProjectNameSource::Alias,
        ..Default::default()
    };
    let record = RepoRecord {
        path: PathBuf::from("/tmp/repo"),
        name: "folder-name".into(),
        registered_at: 0,
        source: "manual".into(),
        git_common_dir: String::new(),
        branch: None,
        is_dirty: None,
        ahead: None,
        behind: None,
        git_refreshed_at: None,
        git_state_error: None,
        last_opened_at: None,
        pinned: false,
        pin_order: None,
        notes: None,
        tags: vec![],
        alias: None,
        convert_block_reason: None,
    };
    assert_eq!(
        repo_convert::resolve_project_name(&cfg, &record),
        "folder-name"
    );
}

#[test]
fn convert_rejects_path_traversal_template() {
    use workpot_core::WorkpotError;

    let dir = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx_with_migration(
        dir.path(),
        r#"[migration]
bare_repo_template = "{project}/../../outside/bare.git"
worktree_template = "{project}/wtrees/{worktree}"
"#,
    );
    let path = local_repo_clean_synced(dir.path());
    ctx.register_manual(&path).expect("register");

    let err = ctx
        .convert_repo(&path, ConvertTarget::Bare, true)
        .expect_err("traversal template should fail");
    assert!(
        matches!(err, WorkpotError::ConversionPreflight(ref msg) if msg.contains("escapes parent")),
        "unexpected error: {err:?}"
    );
}

#[test]
fn dry_run_rejects_existing_temp_path() {
    use workpot_core::WorkpotError;

    let dir = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(dir.path());
    let path = local_repo_clean_synced(dir.path());
    ctx.register_manual(&path).expect("register");

    let temp_path = path.with_file_name(format!(
        "{}{}",
        path.file_name().and_then(|n| n.to_str()).expect("name"),
        ".temp"
    ));
    fs::create_dir_all(&temp_path).expect("temp dir");

    let err = ctx
        .convert_repo(&path, ConvertTarget::Bare, true)
        .expect_err("existing temp should fail dry-run");
    assert!(
        matches!(err, WorkpotError::ConversionPreflight(ref msg) if msg.contains("temp path already exists")),
        "unexpected error: {err:?}"
    );
    assert!(path.exists(), "dry-run must not rename source");
}

#[test]
fn convert_blocks_untracked_when_delete_original() {
    use workpot_core::WorkpotError;

    let dir = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx_with_migration(
        dir.path(),
        r#"[migration]
delete_original = true
"#,
    );
    let path = local_repo_clean_synced(dir.path());
    ctx.register_manual(&path).expect("register");
    fs::write(path.join("untracked.txt"), "orphan data\n").expect("untracked");

    let err = ctx
        .convert_repo(&path, ConvertTarget::Bare, false)
        .expect_err("untracked with delete_original should fail");
    assert!(
        matches!(err, WorkpotError::ConversionPreflight(ref msg) if msg.contains("untracked files")),
        "unexpected error: {err:?}"
    );
    assert!(
        path.exists(),
        "conversion must not rename on preflight failure"
    );
}

#[test]
fn convert_rejects_already_bare_layout() {
    use workpot_core::WorkpotError;

    let dir = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx(dir.path());
    let (bare_path, _) = bare_repo_with_worktree(dir.path());
    ctx.register_manual(&bare_path).expect("register");

    let err = ctx
        .convert_repo(&bare_path, ConvertTarget::Bare, false)
        .expect_err("bare→bare should fail");
    assert!(
        matches!(err, WorkpotError::ConversionPreflight(ref msg) if msg.contains("already bare")),
        "unexpected error: {err:?}"
    );
}

#[test]
fn structural_blocks_linked_worktree() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx_with_migration(
        dir.path(),
        "[migration]\nallow_conversion_to_bare_repo = true\n",
    );
    let (_bare_path, wt_path) = bare_repo_with_worktree(dir.path());
    ctx.register_manual(&wt_path).expect("register worktree");

    let config = ctx.config().expect("config");
    ctx.db()
        .with_write(|conn| persist_structural_preflight_for_repo(conn, &config, &wt_path))
        .expect("persist");

    let path_key = wt_path.canonicalize().expect("canon").display().to_string();
    let reason: Option<String> = ctx
        .db()
        .with_read(|conn| {
            conn.query_row(
                "SELECT convert_block_reason FROM repos WHERE path = ?1",
                rusqlite::params![path_key],
                |row| row.get(0),
            )
            .map_err(workpot_core::WorkpotError::from)
        })
        .expect("query");
    assert!(
        reason
            .as_deref()
            .is_some_and(|r| r.contains("git worktree")),
        "expected linked worktree blocker, got {reason:?}"
    );

    ctx.db()
        .with_read(|conn| {
            let result = assess_structural_blockers(conn, &config, &wt_path, ConvertTarget::Bare)?;
            assert!(matches!(result, PreflightResult::Blocked { .. }));
            Ok(())
        })
        .expect("assess");
}

#[test]
fn dirty_repo_persists_null_structural_block_reason() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx_with_migration(
        dir.path(),
        "[migration]\nallow_conversion_to_bare_repo = true\n",
    );
    let path = dirty_local_repo(dir.path());
    ctx.register_manual(&path).expect("register");

    let config = ctx.config().expect("config");
    ctx.db()
        .with_write(|conn| persist_structural_preflight_for_repo(conn, &config, &path))
        .expect("persist");

    let path_key = path.canonicalize().expect("canon").display().to_string();
    let reason: Option<String> = ctx
        .db()
        .with_read(|conn| {
            conn.query_row(
                "SELECT convert_block_reason FROM repos WHERE path = ?1",
                rusqlite::params![path_key],
                |row| row.get(0),
            )
            .map_err(workpot_core::WorkpotError::from)
        })
        .expect("query");
    assert_eq!(reason, None);

    let volatile = run_volatile_preflight(&path).expect("volatile");
    assert!(matches!(volatile, PreflightResult::DirtyWorktree { .. }));
}

#[test]
fn assess_conversion_readiness_rejects_dirty_after_structural_clear() {
    use workpot_core::WorkpotError;

    let dir = tempfile::tempdir().expect("tempdir");
    let ctx = test_ctx_with_migration(
        dir.path(),
        "[migration]\nallow_conversion_to_bare_repo = true\n",
    );
    let path = dirty_local_repo(dir.path());
    ctx.register_manual(&path).expect("register");

    let config = ctx.config().expect("config");
    ctx.db()
        .with_read(|conn| {
            let readiness = assess_conversion_readiness(conn, &config, &path, ConvertTarget::Bare)?;
            assert!(matches!(readiness, PreflightResult::DirtyWorktree { .. }));
            Ok(())
        })
        .expect("assess");

    let err = ctx
        .convert_repo(&path, ConvertTarget::Bare, false)
        .expect_err("dirty repo must fail convert");
    assert!(
        matches!(err, WorkpotError::ConversionPreflight(ref msg) if msg.contains("dirty worktree")),
        "unexpected error: {err:?}"
    );
}

#[test]
fn volatile_preflight_is_independent_of_db_block_reason() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dirty_local_repo(dir.path());
    let result = run_volatile_preflight(&path).expect("volatile");
    assert!(matches!(result, PreflightResult::DirtyWorktree { .. }));
}
