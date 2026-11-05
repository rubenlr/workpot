#![allow(clippy::disallowed_methods)]

mod common;

use common::{FakeRemoteApi, git_cmd, gitlab_source, remote_info};
use std::sync::Arc;
use workpot_core::domain::config::ClonePolicy;
use workpot_core::infra::remote_provider::RemoteError;
use workpot_core::services::remotes::RemotePatch;
use workpot_core::{AppContext, WorkpotError};

struct Env {
    dir: tempfile::TempDir,
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
        dir,
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

fn remote_only_names(e: &Env) -> Vec<String> {
    e.ctx
        .list_remote_only()
        .expect("list")
        .into_iter()
        .map(|r| r.full_path)
        .collect()
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

/// Uncloned gone rows are purged at the end of the run, so a returning repo is re-inserted as
/// new. (The cloned-and-gone revival path lives in `remote_clone_test`.)
#[test]
fn uncloned_repo_that_reappears_is_indexed_again_as_new() {
    let e = env(vec![Ok(repos3()), Ok(repos3()[..2].to_vec()), Ok(repos3())]);
    e.ctx.run_remote_catalog_sync(true).expect("first");
    e.ctx.run_remote_catalog_sync(true).expect("second");
    let third = e.ctx.run_remote_catalog_sync(true).expect("third");
    assert_eq!(third.added, 1, "{third:?}");
    assert_eq!(third.gone, 0);
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
fn empty_listing_for_a_source_with_nothing_indexed_is_a_clean_run() {
    let e = env(vec![Ok(Vec::new())]);
    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.errors, 0, "{:?}", summary.source_errors);
    assert_eq!(summary.indexed, 0);
    let status = &e.ctx.remote_source_statuses().expect("statuses")[0];
    assert_eq!(status.last_run_status.as_deref(), Some("ok"));
    assert_eq!(status.repo_count, 0);
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

fn backdate_runs(e: &Env, source: &str, secs: i64) {
    e.ctx
        .db()
        .with_write(|conn| {
            conn.execute(
                "UPDATE remote_sync_runs SET finished_at = finished_at - ?2 WHERE source_id = ?1",
                rusqlite::params![source, secs],
            )?;
            Ok(())
        })
        .expect("backdate");
}

#[test]
fn ttl_expiry_relists_but_a_recent_run_still_skips() {
    let e = env(vec![Ok(repos3())]);
    e.ctx.run_sync_full(false).expect("first");
    assert_eq!(e.api.calls(), 1);

    // Default TTL is 15 minutes: 10 minutes ago is still fresh, 16 minutes ago is stale.
    backdate_runs(&e, "acme", 10 * 60);
    let fresh = e.ctx.run_sync_full(false).expect("fresh");
    assert_eq!(fresh.remote.skipped_ttl, 1);
    assert_eq!(e.api.calls(), 1);

    backdate_runs(&e, "acme", 6 * 60);
    let stale = e.ctx.run_sync_full(false).expect("stale");
    assert_eq!(stale.remote.skipped_ttl, 0);
    assert_eq!(e.api.calls(), 2, "expired TTL lists again");
}

#[test]
fn zero_ttl_always_lists() {
    let e = env(vec![Ok(repos3())]);
    e.ctx.config_mut().expect("config").remote_sync_ttl_minutes = 0;
    e.ctx.run_sync_full(false).expect("first");
    let second = e.ctx.run_sync_full(false).expect("second");
    assert_eq!(second.remote.skipped_ttl, 0);
    assert_eq!(e.api.calls(), 2);
}

#[test]
fn a_failed_run_does_not_start_the_ttl_window() {
    let e = env(vec![Err(RemoteError::Failed("boom".into())), Ok(repos3())]);
    let first = e.ctx.run_sync_full(false).expect("first");
    assert_eq!(first.remote.errors, 1);

    let second = e.ctx.run_sync_full(false).expect("second");
    assert_eq!(second.remote.skipped_ttl, 0, "an error run must not count");
    assert_eq!(second.remote.indexed, 3);
    assert_eq!(e.api.calls(), 2);
}

#[test]
fn ttl_is_tracked_per_source() {
    let e = env(vec![Ok(repos3())]);
    e.ctx
        .remotes_add(gitlab_source(
            "beta",
            "beta/tools",
            &e.dir.path().join("clones-beta"),
        ))
        .expect("add beta");
    e.ctx.run_sync_full(true).expect("both listed");
    assert_eq!(e.api.calls(), 2);

    backdate_runs(&e, "beta", 2 * 60 * 60);
    let summary = e.ctx.run_sync_full(false).expect("partial");
    assert_eq!(
        summary.remote.skipped_ttl, 1,
        "acme is fresh, beta is stale"
    );
    assert_eq!(e.api.calls(), 3, "only beta is listed again");
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
    let names = remote_only_names(&e);
    assert_eq!(names.len(), 3, "{names:?}");
    assert!(names.iter().all(|n| !n.contains("evil")), "{names:?}");
}

#[test]
fn repos_without_any_clone_url_are_skipped_and_web_url_alone_is_enough() {
    let mut no_url = remote_info("acme/platform/nourl", "unused");
    no_url.ssh_url = None;
    no_url.https_url = None;
    let mut web_only = remote_info("acme/platform/webonly", "unused");
    web_only.ssh_url = None;
    web_only.https_url = None;
    web_only.web_url = Some("https://git.example.com/acme/platform/webonly".into());
    let mut listing = repos3();
    listing.push(no_url);
    listing.push(web_only);
    let e = env(vec![Ok(listing)]);

    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.errors, 0, "{:?}", summary.source_errors);
    let names = remote_only_names(&e);
    assert_eq!(summary.indexed, 4, "{names:?}");
    assert!(!names.contains(&"acme/platform/nourl".to_string()));
    assert!(names.contains(&"acme/platform/webonly".to_string()));
}

#[test]
fn duplicate_listing_entries_are_indexed_once() {
    let mut listing = repos3();
    listing.push(listing[0].clone());
    let e = env(vec![Ok(listing)]);
    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.indexed, 3);
    assert_eq!(summary.added, 3);
    assert_eq!(e.ctx.list_remote_only().expect("list").len(), 3);
}

#[test]
fn listing_exactly_at_the_cap_is_accepted() {
    let e = env(vec![Ok(repos3())]);
    e.ctx.config_mut().expect("config").limits.max_remote_repos = 3;
    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.errors, 0, "{:?}", summary.source_errors);
    assert_eq!(summary.indexed, 3);
}

fn include_archived(e: &Env, include: bool) {
    e.ctx
        .remotes_set(
            "acme",
            RemotePatch {
                include_archived: Some(include),
                ..RemotePatch::default()
            },
        )
        .expect("set");
}

#[test]
fn archived_repos_are_visible_by_default_and_hidden_only_by_the_filter() {
    let mut listing = repos3();
    listing[0].archived = true;
    let e = env(vec![Ok(listing)]);
    e.ctx.run_remote_catalog_sync(true).expect("sync");

    // Default (`include_archived = true`): archived repos are listed.
    assert!(e.ctx.config().expect("config").remotes[0].include_archived);
    assert_eq!(remote_only_names(&e).len(), 3);

    include_archived(&e, false);
    let names = remote_only_names(&e);
    assert_eq!(names.len(), 2);
    assert!(!names.contains(&"acme/platform/api".to_string()));
    // Hiding is a view filter: the archived repo stays indexed as present, never gone.
    let status = &e.ctx.remote_source_statuses().expect("statuses")[0];
    assert_eq!((status.repo_count, status.gone_count), (3, 0));

    include_archived(&e, true);
    assert_eq!(remote_only_names(&e).len(), 3, "flipping back re-shows it");
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

// ---- identifier resolution ------------------------------------------------------------------

fn ambiguous_listing() -> Vec<workpot_core::infra::remote_provider::RemoteRepoInfo> {
    vec![
        remote_info(
            "acme/platform/api",
            "https://git.example.com/acme/platform/api.git",
        ),
        remote_info(
            "acme/platform/sub/api",
            "https://git.example.com/acme/platform/sub/api.git",
        ),
        remote_info(
            "acme/platform/web",
            "https://git.example.com/acme/platform/web.git",
        ),
    ]
}

#[test]
fn unknown_identifier_is_not_found() {
    let e = env(vec![Ok(repos3())]);
    e.ctx.run_remote_catalog_sync(true).expect("sync");
    let err = e.ctx.clone_remote("nope").expect_err("unknown");
    assert!(matches!(err, WorkpotError::NotFound(_)), "{err:?}");
    assert_eq!(e.ctx.resolve_remote_only("nope").expect("resolve"), None);
    assert_eq!(e.ctx.resolve_remote_only("  ").expect("blank"), None);
}

#[test]
fn ambiguous_short_name_is_rejected_but_other_forms_resolve() {
    let e = env(vec![Ok(ambiguous_listing())]);
    e.ctx.run_remote_catalog_sync(true).expect("sync");

    let err = e.ctx.clone_remote("api").expect_err("two repos named api");
    assert!(
        matches!(&err, WorkpotError::InvalidInput(m) if m.contains("matches 2")),
        "{err:?}"
    );
    assert!(e.ctx.resolve_remote_only("api").is_err());

    let path_of = |id: &str| {
        e.ctx
            .resolve_remote_only(id)
            .expect("resolve")
            .map(|r| r.full_path)
    };
    // Unique short name, full path (case-insensitive), `source:full_path`, planned clone path.
    assert_eq!(path_of("web").as_deref(), Some("acme/platform/web"));
    assert_eq!(
        path_of("ACME/platform/sub/api").as_deref(),
        Some("acme/platform/sub/api")
    );
    assert_eq!(
        path_of("acme:acme/platform/sub/api").as_deref(),
        Some("acme/platform/sub/api")
    );
    let planned = e.clone_root.join("sub/api").display().to_string();
    assert_eq!(path_of(&planned).as_deref(), Some("acme/platform/sub/api"));
}

// ---- orchestrator -----------------------------------------------------------------------------

#[test]
fn remote_failure_never_blocks_the_local_pass() {
    let e = env(vec![Err(RemoteError::Failed("boom".into()))]);
    let watch = e.dir.path().join("watch");
    let repo = watch.join("solo");
    std::fs::create_dir_all(&repo).expect("repo dir");
    assert!(
        git_cmd()
            .args(["init", "-q"])
            .current_dir(&repo)
            .status()
            .expect("git init")
            .success()
    );
    e.ctx.config_mut().expect("config").watch_roots.push(watch);

    let full = e
        .ctx
        .run_sync_full(true)
        .expect("local pass still succeeds");
    assert_eq!(full.remote.errors, 1, "{:?}", full.remote);
    assert_eq!(full.local.added, 1, "{:?}", full.local);
    assert_eq!(e.ctx.list_repos().expect("repos").len(), 1);
}

#[test]
fn a_remote_pass_that_cannot_run_is_folded_into_a_wildcard_source_error() {
    let e = env(vec![Ok(repos3())]);
    // Break the index itself (not the provider): the pass fails before any source is listed.
    e.ctx
        .db()
        .with_write(|conn| {
            conn.execute_batch("DROP TABLE remote_repos")?;
            Ok(())
        })
        .expect("drop");

    let full = e
        .ctx
        .run_sync_full(true)
        .expect("sync never errors for remotes");
    assert_eq!(full.remote.sources, 1);
    assert_eq!(full.remote.errors, 1);
    assert_eq!(full.remote.source_errors.len(), 1);
    assert_eq!(full.remote.source_errors[0].source, "*");
    assert_eq!(e.api.calls(), 0, "no source was listed");
}

#[test]
fn sources_removed_from_a_hand_edited_config_are_purged_on_the_next_sync() {
    let e = env(vec![Ok(repos3())]);
    e.ctx
        .remotes_add(gitlab_source(
            "beta",
            "beta/tools",
            &e.dir.path().join("clones-beta"),
        ))
        .expect("add beta");
    e.ctx.run_remote_catalog_sync(true).expect("index both");
    let count = |table: &str, source: &str| -> i64 {
        e.ctx
            .db()
            .with_read(|conn| {
                Ok(conn.query_row(
                    &format!("SELECT COUNT(*) FROM {table} WHERE source_id = ?1"),
                    [source],
                    |row| row.get(0),
                )?)
            })
            .expect("count")
    };
    assert_eq!(count("remote_repos", "beta"), 3);
    assert_eq!(count("remote_sync_runs", "beta"), 1);

    // Config edited by hand while Workpot was not looking.
    e.ctx
        .config_mut()
        .expect("config")
        .remotes
        .retain(|r| r.name != "beta");
    e.ctx.run_remote_catalog_sync(true).expect("sync");

    assert_eq!(count("remote_repos", "beta"), 0);
    assert_eq!(count("remote_sync_runs", "beta"), 0);
    assert_eq!(
        count("remote_repos", "acme"),
        3,
        "configured source untouched"
    );
}
