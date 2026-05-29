use std::fs;
use std::path::Path;
use std::process::Command;
use workpot_core::domain::Config;
use workpot_core::save_config;
use workpot_core::services::{discovery, excludes};
use workpot_core::AppContext;

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

fn empty_config_marker() -> &'static str {
    "watch_roots = []\nexcludes = []\n"
}

#[test]
fn exclude_blocks_rescan() {
    let root = tempfile::tempdir().expect("tempdir");
    let skip_parent = root.path().join("skip-me");
    fs::create_dir_all(&skip_parent).expect("skip-me dir");
    let _skipped = git_worktree(&skip_parent, "inner");
    let _visible = git_worktree(root.path(), "visible");

    let mut config = Config::default();
    config.excludes.push("**/skip-me/**".to_string());

    let exclude_set = discovery::build_exclude_set(&config).expect("exclude set");
    let candidates = discovery::scan_root(root.path(), &exclude_set).expect("scan_root");

    let skip_canon = skip_parent.canonicalize().expect("canonicalize skip");
    assert!(
        !candidates.iter().any(|p| p.starts_with(&skip_canon)),
        "excluded subtree must not be discovered"
    );
}

#[test]
fn excludes_list_roundtrip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(&config_path, empty_config_marker()).expect("write config");

    let mut ctx = AppContext::open_with_paths(config_path.clone(), db_path).expect("open");
    let glob = "/tmp/workpot-test-exclude/**".to_string();
    ctx.config_mut().excludes.push(glob.clone());
    save_config(&config_path, ctx.config()).expect("save");

    let listed = excludes::list_excludes(ctx.config());
    assert!(listed.iter().any(|g| g == &glob));
}

#[test]
fn manual_add_ignores_exclude_glob() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(
        &config_path,
        "watch_roots = []\nexcludes = [\"**/blocked/**\"]\n",
    )
    .expect("write config");

    let blocked_parent = dir.path().join("blocked");
    fs::create_dir_all(&blocked_parent).expect("blocked parent");
    let repo = git_worktree(&blocked_parent, "repo");

    let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
    let record = ctx.register_manual(&repo).expect("manual add despite exclude");
    assert_eq!(record.path, repo.canonicalize().expect("canonicalize"));
}

#[test]
fn remove_appends_exclude() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(&config_path, empty_config_marker()).expect("write config");

    let repo = git_worktree(dir.path(), "to-remove");
    let repo_canon = repo.canonicalize().expect("canonicalize");

    let mut ctx = AppContext::open_with_paths(config_path.clone(), db_path).expect("open");
    ctx.register_manual(&repo).expect("register");
    ctx.remove_repo(&repo).expect("remove");

    let parent = repo_canon.parent().expect("parent");
    let name = repo_canon.file_name().expect("name").to_string_lossy();
    let base = format!("{}/{}", parent.display(), name);
    let tree = format!("{base}/**");

    let config: Config = toml::from_str(&fs::read_to_string(&config_path).expect("read config"))
        .expect("parse config");
    assert!(
        config.excludes.iter().any(|g| g == &base),
        "expected base exclude {base:?}, got {:?}",
        config.excludes
    );
    assert!(
        config.excludes.iter().any(|g| g == &tree),
        "expected tree exclude {tree:?}, got {:?}",
        config.excludes
    );
}

#[test]
fn remove_then_index_skips() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(&config_path, empty_config_marker()).expect("write config");

    let watch_root = dir.path().join("watch");
    fs::create_dir_all(&watch_root).expect("watch root");
    let repo = git_worktree(&watch_root, "gone");

    let mut ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
    ctx.roots_add(&watch_root).expect("roots add");
    let repo_canon = repo.canonicalize().expect("canonicalize");
    assert!(
        ctx.list_repos()
            .expect("list")
            .iter()
            .any(|r| r.path == repo_canon)
    );

    ctx.remove_repo(&repo).expect("remove with exclude");
    ctx.run_index().expect("rescan");

    assert!(
        !ctx.list_repos()
            .expect("list after rescan")
            .iter()
            .any(|r| r.path == repo_canon),
        "removed repo must not reappear after index"
    );
}

#[test]
fn excludes_remove_persists() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    fs::write(
        &config_path,
        "watch_roots = []\nexcludes = [\"/gone/**\"]\n",
    )
    .expect("write config");

    let mut ctx = AppContext::open_with_paths(config_path.clone(), db_path).expect("open");
    ctx.excludes_remove("/gone/**").expect("remove exclude");

    let config: Config = toml::from_str(&fs::read_to_string(&config_path).expect("read config"))
        .expect("parse config");
    assert!(!config.excludes.iter().any(|g| g == "/gone/**"));
}
