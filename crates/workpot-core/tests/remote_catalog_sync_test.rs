#![allow(clippy::disallowed_methods)]

mod common;

use common::{FakeRemoteApi, gitlab_source, remote_info};
use std::sync::Arc;
use workpot_core::AppContext;
use workpot_core::domain::config::ClonePolicy;
use workpot_core::infra::remote_provider::RemoteError;
use workpot_core::services::remotes::RemotePatch;

struct Env {
    _dir: tempfile::TempDir,
    ctx: AppContext,
    api: Arc<FakeRemoteApi>,
    clone_root: std::path::PathBuf,
}

fn env(
    listings: Vec<Result<Vec<workpot_core::infra::remote_provider::RemoteRepoInfo>, RemoteError>>,
) -> Env {
    let dir = tempfile::tempdir().expect("tempdir");
    let api = Arc::new(FakeRemoteApi::new(listings));
    let ctx = AppContext::open_with_paths(
        dir.path().join("config.toml"),
        dir.path().join("workpot.db"),
    )
    .expect("open")
    .with_remote_api(api.clone());
    let clone_root = dir.path().join("clones");
    ctx.remotes_add(gitlab_source("acme", "acme/platform", &clone_root))
        .expect("add remote");
    Env {
        _dir: dir,
        ctx,
        api,
        clone_root,
    }
}

fn repos3() -> Vec<workpot_core::infra::remote_provider::RemoteRepoInfo> {
    vec![
        remote_info(
            "acme/platform/api",
            "https://git.example.com/acme/platform/api.git",
        ),
        remote_info(
            "acme/platform/web",
            "https://git.example.com/acme/platform/web.git",
        ),
        remote_info(
            "acme/platform/sub/deep",
            "https://git.example.com/acme/platform/sub/deep.git",
        ),
    ]
}

#[test]
fn index_lists_remote_only_repos_with_planned_clone_paths() {
    let e = env(vec![Ok(repos3())]);
    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.indexed, 3);
    assert_eq!(summary.added, 3);
    assert_eq!(summary.errors, 0);

    let remote_only = e.ctx.list_remote_only().expect("list");
    assert_eq!(remote_only.len(), 3);
    let deep = remote_only
        .iter()
        .find(|r| r.full_path == "acme/platform/sub/deep")
        .expect("deep");
    assert_eq!(deep.clone_path, e.clone_root.join("sub/deep"));
    assert_eq!(deep.source_id, "acme");
}

#[test]
fn second_sync_updates_in_place_and_marks_missing_repos_gone() {
    let e = env(vec![Ok(repos3()), Ok(repos3()[..2].to_vec())]);
    e.ctx.run_remote_catalog_sync(true).expect("first");
    let summary = e.ctx.run_remote_catalog_sync(true).expect("second");
    assert_eq!(summary.added, 0);
    assert_eq!(summary.gone, 1);
    assert_eq!(e.ctx.list_remote_only().expect("list").len(), 2);

    let statuses = e.ctx.remote_source_statuses().expect("statuses");
    assert_eq!(statuses[0].repo_count, 2);
    assert_eq!(statuses[0].gone_count, 0, "uncloned gone rows are purged");
    assert_eq!(statuses[0].last_run_status.as_deref(), Some("ok"));
}

#[test]
fn repo_that_reappears_becomes_present_again() {
    let e = env(vec![Ok(repos3()), Ok(repos3()[..2].to_vec()), Ok(repos3())]);
    for _ in 0..3 {
        e.ctx.run_remote_catalog_sync(true).expect("sync");
    }
    assert_eq!(e.ctx.list_remote_only().expect("list").len(), 3);
}

#[test]
fn provider_error_is_soft_and_recorded_without_touching_index() {
    let e = env(vec![
        Ok(repos3()),
        Err(RemoteError::NotAuthenticated {
            cli: "glab".into(),
            host: "git.example.com".into(),
        }),
    ]);
    e.ctx.run_remote_catalog_sync(true).expect("first");
    let summary = e
        .ctx
        .run_remote_catalog_sync(true)
        .expect("second never errors");
    assert_eq!(summary.errors, 1);
    assert!(summary.source_errors[0].message.contains("glab auth login"));
    assert_eq!(e.ctx.list_remote_only().expect("list").len(), 3);
    let status = &e.ctx.remote_source_statuses().expect("statuses")[0];
    assert_eq!(status.last_run_status.as_deref(), Some("error"));
}

#[test]
fn empty_listing_never_marks_indexed_repos_gone() {
    let e = env(vec![Ok(repos3()), Ok(Vec::new())]);
    e.ctx.run_remote_catalog_sync(true).expect("first");
    let summary = e.ctx.run_remote_catalog_sync(true).expect("second");
    assert_eq!(summary.errors, 1);
    assert_eq!(summary.gone, 0);
    assert_eq!(e.ctx.list_remote_only().expect("list").len(), 3);
}

#[test]
fn ttl_skips_listing_unless_forced() {
    let e = env(vec![Ok(repos3())]);
    let first = e.ctx.run_sync_full(false).expect("first");
    assert_eq!(first.remote.skipped_ttl, 0);
    assert_eq!(e.api.calls(), 1);

    let second = e.ctx.run_sync_full(false).expect("second");
    assert_eq!(second.remote.skipped_ttl, 1);
    assert_eq!(e.api.calls(), 1, "within TTL: no listing");

    e.ctx.run_sync_full(true).expect("forced");
    assert_eq!(e.api.calls(), 2);
}

#[test]
fn cap_exceeded_is_a_soft_source_error() {
    let e = env(vec![Ok(repos3())]);
    e.ctx.config_mut().expect("config").limits.max_remote_repos = 2;
    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.errors, 1);
    assert!(
        summary.source_errors[0]
            .message
            .contains("max_remote_repos")
    );
    assert!(e.ctx.list_remote_only().expect("list").is_empty());
}

#[test]
fn unsafe_remote_paths_are_skipped() {
    let mut listing = repos3();
    listing.push(remote_info(
        "acme/platform/../evil",
        "https://git.example.com/evil.git",
    ));
    let e = env(vec![Ok(listing)]);
    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.indexed, 3);
}

#[test]
fn archived_repos_are_hidden_when_source_excludes_them() {
    let mut listing = repos3();
    listing[0].archived = true;
    let e = env(vec![Ok(listing)]);
    e.ctx
        .remotes_set(
            "acme",
            RemotePatch {
                include_archived: Some(false),
                ..RemotePatch::default()
            },
        )
        .expect("set");
    e.ctx.run_remote_catalog_sync(true).expect("sync");
    let names: Vec<_> = e
        .ctx
        .list_remote_only()
        .expect("list")
        .into_iter()
        .map(|r| r.full_path)
        .collect();
    assert_eq!(names.len(), 2);
    assert!(!names.contains(&"acme/platform/api".to_string()));
}

#[test]
fn removing_a_source_purges_its_index_and_persists_config() {
    let e = env(vec![Ok(repos3())]);
    e.ctx.run_remote_catalog_sync(true).expect("sync");
    e.ctx.remotes_remove("acme").expect("remove");
    assert!(e.ctx.list_remote_only().expect("list").is_empty());
    let raw = std::fs::read_to_string(e.ctx.config_path()).expect("config");
    assert!(
        !raw.lines().any(|l| l.trim() == "[[remotes]]"),
        "config: {raw}"
    );
}

#[test]
fn remotes_config_round_trips_through_config_toml() {
    let e = env(vec![Ok(repos3())]);
    e.ctx
        .remotes_set(
            "acme",
            RemotePatch {
                clone_policy: Some(ClonePolicy::OnOpen),
                prune_removed: Some(true),
                ..RemotePatch::default()
            },
        )
        .expect("set");
    let raw = std::fs::read_to_string(e.ctx.config_path()).expect("config");
    assert!(
        raw.lines().any(|l| l.trim() == "[[remotes]]"),
        "config: {raw}"
    );
    assert!(raw.contains("clone_policy = \"on_open\""), "config: {raw}");
    assert!(
        raw.contains("Workpot configuration"),
        "comments kept: {raw}"
    );

    let reopened = AppContext::open_with_paths(
        e.ctx.config_path().to_path_buf(),
        e.ctx.database_path().to_path_buf(),
    )
    .expect("reopen");
    let config = reopened.config().expect("config");
    assert_eq!(config.remotes.len(), 1);
    assert_eq!(config.remotes[0].clone_policy, ClonePolicy::OnOpen);
    assert!(config.remotes[0].prune_removed);
}

#[test]
fn duplicate_and_overlapping_sources_are_rejected_and_rolled_back() {
    let e = env(vec![Ok(repos3())]);
    let dup = gitlab_source("acme", "other", &e.clone_root.join("x"));
    assert!(e.ctx.remotes_add(dup).is_err());
    let overlap = gitlab_source("second", "other", &e.clone_root.join("nested"));
    assert!(e.ctx.remotes_add(overlap).is_err());
    assert_eq!(e.ctx.config().expect("config").remotes.len(), 1);
}
