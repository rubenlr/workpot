#![allow(clippy::disallowed_methods)]

mod common;

use common::{FakeRemoteApi, git_cmd, gitlab_source, remote_info, seed_bare_repo};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use workpot_core::AppContext;
use workpot_core::domain::config::ClonePolicy;
use workpot_core::services::launch::ensure_cloned_for_open;
use workpot_core::services::remotes::RemotePatch;

struct Env {
    _dir: tempfile::TempDir,
    ctx: AppContext,
    api: Arc<FakeRemoteApi>,
    clone_root: PathBuf,
    bares: Vec<PathBuf>,
}

fn make_bare(dir: &Path, name: &str) -> PathBuf {
    let bare = dir.join(format!("{name}.git"));
    let status = git_cmd()
        .args(["init", "--bare", "-q", "-b", "main"])
        .arg(&bare)
        .status()
        .expect("init bare");
    assert!(status.success());
    seed_bare_repo(&bare);
    bare
}

fn env(names: &[&str]) -> Env {
    let dir = tempfile::tempdir().expect("tempdir");
    let remotes_dir = dir.path().join("remotes");
    std::fs::create_dir_all(&remotes_dir).expect("remotes dir");
    let bares: Vec<PathBuf> = names.iter().map(|n| make_bare(&remotes_dir, n)).collect();
    let listing: Vec<_> = names
        .iter()
        .zip(&bares)
        .map(|(n, b)| remote_info(&format!("acme/platform/{n}"), &b.display().to_string()))
        .collect();
    let api = Arc::new(FakeRemoteApi::new(vec![Ok(listing)]));
    let ctx = AppContext::open_with_paths(
        dir.path().join("config.toml"),
        dir.path().join("workpot.db"),
    )
    .expect("open")
    .with_remote_api(api.clone());
    let clone_root = dir.path().join("clones");
    ctx.remotes_add(gitlab_source("acme", "acme/platform", &clone_root))
        .expect("add");
    ctx.run_remote_catalog_sync(true).expect("index");
    Env {
        _dir: dir,
        ctx,
        api,
        clone_root,
        bares,
    }
}

fn set_policy(e: &Env, policy: ClonePolicy, prune: bool) {
    e.ctx
        .remotes_set(
            "acme",
            RemotePatch {
                clone_policy: Some(policy),
                prune_removed: Some(prune),
                ..RemotePatch::default()
            },
        )
        .expect("set policy");
}

fn git_in(dir: &Path, args: &[&str]) {
    let output = git_cmd()
        .args(["-c", "user.name=T", "-c", "user.email=t@example.com"])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn clone_registers_location_and_flips_cloned_state() {
    let e = env(&["api", "web"]);
    assert_eq!(e.ctx.list_remote_only().expect("list").len(), 2);

    let outcome = e.ctx.clone_remote("acme/platform/api").expect("clone");
    assert!(!outcome.already_cloned);
    assert!(outcome.local_path.join(".git").is_dir());
    assert_eq!(
        outcome.local_path.canonicalize().expect("canon"),
        e.clone_root.join("api").canonicalize().expect("canon")
    );

    let remote_only = e.ctx.list_remote_only().expect("list");
    assert_eq!(remote_only.len(), 1);
    assert_eq!(remote_only[0].full_path, "acme/platform/web");

    let repos = e.ctx.list_repos().expect("repos");
    let cloned = repos
        .iter()
        .find(|r| r.path == outcome.local_path)
        .expect("catalog row");
    assert_eq!(cloned.branch.as_deref(), Some("main"));

    let branches = e
        .ctx
        .db()
        .with_read(|conn| {
            workpot_core::services::catalog::list_location_branches(
                conn,
                &outcome.local_path.display().to_string(),
            )
        })
        .expect("branches");
    assert!(branches.iter().any(|b| b.name == "main"));
}

#[test]
fn clone_twice_is_a_noop() {
    let e = env(&["api"]);
    e.ctx.clone_remote("api").expect("first");
    let again = e.ctx.clone_remote("api").expect("second");
    assert!(again.already_cloned);
}

#[test]
fn failed_clone_cleans_up_and_leaves_repo_remote_only() {
    let e = env(&["api"]);
    std::fs::remove_dir_all(&e.bares[0]).expect("break remote");
    let err = e.ctx.clone_remote("api").expect_err("clone must fail");
    assert!(err.to_string().contains("clone failed"), "{err}");
    assert!(!e.clone_root.join("api").exists());
    assert_eq!(e.ctx.list_remote_only().expect("list").len(), 1);
}

#[test]
fn clone_refuses_non_empty_target_without_touching_it() {
    let e = env(&["api"]);
    let target = e.clone_root.join("api");
    std::fs::create_dir_all(&target).expect("target");
    std::fs::write(target.join("keep.txt"), "mine").expect("file");
    let err = e.ctx.clone_remote("api").expect_err("refuse");
    assert!(err.to_string().contains("not empty"), "{err}");
    assert!(target.join("keep.txt").is_file());
}

#[test]
fn unknown_and_ambiguous_identifiers_are_rejected() {
    let e = env(&["api"]);
    assert!(e.ctx.clone_remote("nope").is_err());
}

#[test]
fn hand_cloned_repo_is_recognised_as_cloned_after_registering() {
    let e = env(&["api", "web"]);
    let manual = e.clone_root.join("api");
    std::fs::create_dir_all(&e.clone_root).expect("root");
    let status = git_cmd()
        .arg("clone")
        .arg("-q")
        .arg(&e.bares[0])
        .arg(&manual)
        .status()
        .expect("clone");
    assert!(status.success());
    e.ctx.register_manual(&manual).expect("register");
    e.ctx.run_local_catalog_sync().expect("local sync");
    let names: Vec<_> = e
        .ctx
        .list_remote_only()
        .expect("list")
        .into_iter()
        .map(|r| r.full_path)
        .collect();
    assert_eq!(names, vec!["acme/platform/web".to_string()]);
}

#[test]
fn auto_policy_clones_everything_on_sync_once() {
    let e = env(&["api", "web"]);
    set_policy(&e, ClonePolicy::Auto, false);
    let first = e.ctx.run_sync_full(true).expect("sync");
    assert_eq!(first.remote.cloned, 2, "{:?}", first.remote);
    assert!(e.clone_root.join("api/.git").is_dir());
    assert!(e.clone_root.join("web/.git").is_dir());
    assert!(e.ctx.list_remote_only().expect("list").is_empty());

    let second = e.ctx.run_sync_full(true).expect("sync again");
    assert_eq!(second.remote.cloned, 0);
    assert_eq!(second.remote.errors, 0);
}

#[test]
fn auto_policy_does_not_reclone_a_repo_the_user_removed() {
    let e = env(&["api", "web"]);
    set_policy(&e, ClonePolicy::Auto, false);
    e.ctx.run_sync_full(true).expect("sync");
    let api_path = e.clone_root.join("api").canonicalize().expect("canon");
    e.ctx.remove_repo(&api_path).expect("remove adds exclude");
    std::fs::remove_dir_all(&api_path).expect("rm dir");

    let summary = e.ctx.run_sync_full(true).expect("sync");
    assert_eq!(summary.remote.cloned, 0, "{:?}", summary.remote);
    assert!(!api_path.exists());
    let names: Vec<_> = e
        .ctx
        .list_remote_only()
        .expect("list")
        .into_iter()
        .map(|r| r.full_path)
        .collect();
    assert_eq!(names, vec!["acme/platform/api".to_string()]);
}

#[test]
fn open_policy_manual_refuses_and_on_open_clones() {
    let e = env(&["api"]);
    let planned = e.clone_root.join("api").display().to_string();

    let err = ensure_cloned_for_open(&e.ctx, &planned).expect_err("manual refuses");
    assert!(err.contains("not cloned"), "{err}");
    assert!(!e.clone_root.join("api").exists());

    set_policy(&e, ClonePolicy::OnOpen, false);
    let cloned = ensure_cloned_for_open(&e.ctx, &planned)
        .expect("on_open clones")
        .expect("path");
    assert!(cloned.join(".git").is_dir());

    // Already cloned: not a remote-only entry any more.
    let local = cloned.display().to_string();
    assert_eq!(ensure_cloned_for_open(&e.ctx, &local).expect("noop"), None);
}

#[test]
fn clone_button_path_works_under_manual_policy() {
    let e = env(&["api"]);
    assert_eq!(
        e.ctx.config().expect("config").remotes[0].clone_policy,
        ClonePolicy::Manual
    );
    e.ctx
        .clone_remote("api")
        .expect("explicit clone under manual");
}

// ---- prune -----------------------------------------------------------------------------

/// Clone `names`, enable prune, then make `gone` disappear upstream.
fn prune_env(names: &[&str], gone: &str) -> Env {
    let e = env(names);
    for n in names {
        e.ctx.clone_remote(n).expect("clone");
    }
    set_policy(&e, ClonePolicy::Manual, true);
    let remaining: Vec<_> = names
        .iter()
        .zip(&e.bares)
        .filter(|(n, _)| *n != &gone)
        .map(|(n, b)| remote_info(&format!("acme/platform/{n}"), &b.display().to_string()))
        .collect();
    e.api.set_listing(Ok(remaining));
    e.api
        .missing
        .lock()
        .expect("missing")
        .insert(format!("acme/platform/{gone}"));
    e
}

#[test]
fn prune_deletes_clean_pushed_clone_removed_upstream() {
    let e = prune_env(&["api", "web"], "web");
    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.gone, 1);
    assert_eq!(summary.pruned, 1);
    assert!(!e.clone_root.join("web").exists());
    assert!(e.clone_root.join("api").exists());
    let repos = e.ctx.list_repos().expect("repos");
    assert_eq!(repos.len(), 1);
    assert!(e.ctx.removed_upstream_paths().expect("flags").is_empty());
}

#[test]
fn prune_keeps_and_flags_dirty_clone() {
    let e = prune_env(&["api", "web"], "web");
    let web = e.clone_root.join("web");
    std::fs::write(web.join("f.txt"), "x").expect("write");
    git_in(&web, &["add", "f.txt"]);
    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.pruned, 0);
    assert!(web.exists());
    let flagged = e.ctx.removed_upstream_paths().expect("flags");
    assert_eq!(flagged.len(), 1);
}

#[test]
fn prune_keeps_clone_with_untracked_files() {
    let e = prune_env(&["web"], "web");
    let web = e.clone_root.join("web");
    std::fs::write(web.join("scratch.txt"), "x").expect("write");
    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.pruned, 0);
    assert!(web.join("scratch.txt").exists());
}

#[test]
fn prune_keeps_clone_with_unpushed_commits() {
    let e = prune_env(&["web"], "web");
    let web = e.clone_root.join("web");
    git_in(&web, &["commit", "--allow-empty", "-m", "local only", "-q"]);
    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.pruned, 0);
    assert!(web.exists());
}

#[test]
fn prune_keeps_clone_with_local_branch_without_upstream() {
    let e = prune_env(&["web"], "web");
    let web = e.clone_root.join("web");
    git_in(&web, &["branch", "wip"]);
    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.pruned, 0);
    assert!(web.exists());
}

#[test]
fn prune_never_deletes_when_host_does_not_confirm_404() {
    let e = prune_env(&["web"], "web");
    // Listing says gone, but the direct lookup errors (not a 404): must keep.
    e.api.missing.lock().expect("missing").clear();
    e.api
        .exists_error
        .lock()
        .expect("exists_error")
        .insert("acme/platform/web".into());
    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    // An empty remaining listing is refused outright; either way nothing is deleted.
    assert_eq!(summary.pruned, 0);
    assert!(e.clone_root.join("web").exists());
}

#[test]
fn prune_is_disabled_by_default() {
    let e = env(&["api", "web"]);
    e.ctx.clone_remote("api").expect("clone");
    e.ctx.clone_remote("web").expect("clone");
    e.api.set_listing(Ok(vec![remote_info(
        "acme/platform/api",
        &e.bares[0].display().to_string(),
    )]));
    e.api
        .missing
        .lock()
        .expect("missing")
        .insert("acme/platform/web".into());
    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.gone, 1);
    assert_eq!(summary.pruned, 0);
    assert!(e.clone_root.join("web").exists());
    assert_eq!(e.ctx.removed_upstream_paths().expect("flags").len(), 1);
}

#[test]
fn gone_repo_without_local_clone_is_forgotten() {
    let e = env(&["api", "web"]);
    e.api.set_listing(Ok(vec![remote_info(
        "acme/platform/api",
        &e.bares[0].display().to_string(),
    )]));
    e.ctx.run_remote_catalog_sync(true).expect("sync");
    let statuses = e.ctx.remote_source_statuses().expect("statuses");
    assert_eq!(statuses[0].gone_count, 0, "uncloned gone rows are purged");
    assert_eq!(statuses[0].repo_count, 1);
}
