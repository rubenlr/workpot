#![allow(clippy::disallowed_methods)]

mod common;

use common::{
    CloneEnv as Env, clone_env as env, clone_env_with, git_cmd, git_out, last_run_counts,
    listing_for, remote_info, set_policy,
};
use workpot_core::WorkpotError;
use workpot_core::domain::config::{ClonePolicy, GitProtocol};
use workpot_core::services::launch::{ensure_cloned_for_open, launch_repo};
use workpot_core::services::remotes::RemotePatch;

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

#[test]
fn auto_policy_open_path_clones_remote_only_repo() {
    let e = env(&["api"]);
    set_policy(&e, ClonePolicy::Auto, false);
    let planned = e.clone_root.join("api").display().to_string();
    let cloned = ensure_cloned_for_open(&e.ctx, &planned)
        .expect("auto clones on open")
        .expect("path");
    assert!(cloned.join(".git").is_dir());
}

#[test]
fn open_fails_when_the_source_is_no_longer_configured() {
    let e = env(&["api"]);
    let planned = e.clone_root.join("api").display().to_string();
    // Config edited by hand: the index row outlives its source until the next sync.
    e.ctx.config_mut().expect("config").remotes.clear();
    let err = ensure_cloned_for_open(&e.ctx, &planned).expect_err("no source");
    assert!(err.contains("no longer configured"), "{err}");
    assert!(!e.clone_root.join("api").exists());
}

#[test]
fn launch_clones_a_remote_only_repo_then_records_the_open() {
    let e = env(&["api"]);
    set_policy(&e, ClonePolicy::OnOpen, false);
    e.ctx.config_mut().expect("config").launch_cmd = "/usr/bin/true {path}".into();
    let planned = e.clone_root.join("api");

    launch_repo(&e.ctx, &planned.display().to_string()).expect("launch");

    assert!(planned.join(".git").is_dir(), "cloned before launching");
    let repos = e.ctx.list_repos().expect("repos");
    assert_eq!(repos.len(), 1);
    assert!(repos[0].last_opened_at.is_some());
    assert!(e.ctx.list_remote_only().expect("list").is_empty());
}

// ---- clone URL / protocol selection -----------------------------------------------------

/// Index one repo whose ssh and https URLs are two *different* local bare repos, apply the
/// protocol settings, clone it, and return the URL the checkout's `origin` points at.
fn cloned_origin(
    source_protocol: Option<GitProtocol>,
    cli_protocol: Option<GitProtocol>,
) -> (String, String, String) {
    let e = clone_env_with(&["https-side", "ssh-side"], |bares| {
        let mut info = remote_info("acme/platform/api", &bares[0].display().to_string());
        info.https_url = Some(bares[0].display().to_string());
        info.ssh_url = Some(bares[1].display().to_string());
        vec![info]
    });
    *e.api.protocol.lock().expect("protocol") = cli_protocol;
    e.ctx
        .remotes_set(
            "acme",
            RemotePatch {
                protocol: Some(source_protocol),
                ..RemotePatch::default()
            },
        )
        .expect("set protocol");
    let outcome = e.ctx.clone_remote("api").expect("clone");
    let origin = git_out(
        &outcome.local_path,
        &["config", "--get", "remote.origin.url"],
    );
    (
        origin,
        e.bares[0].display().to_string(),
        e.bares[1].display().to_string(),
    )
}

#[test]
fn clone_protocol_precedence_is_source_then_cli_then_https() {
    use GitProtocol::{Https, Ssh};
    // (source override, CLI git_protocol, expect ssh URL?)
    let cases = [
        (None, None, false),
        (None, Some(Ssh), true),
        (None, Some(Https), false),
        (Some(Ssh), Some(Https), true),
        (Some(Https), Some(Ssh), false),
        (Some(Ssh), None, true),
    ];
    for (source, cli, expect_ssh) in cases {
        let (origin, https_url, ssh_url) = cloned_origin(source, cli);
        let expected = if expect_ssh { ssh_url } else { https_url };
        assert_eq!(origin, expected, "source={source:?} cli={cli:?}");
    }
}

#[test]
fn clone_falls_back_to_the_other_protocol_when_one_url_is_missing() {
    // Only an ssh URL, default (https) protocol: fall back to ssh.
    let e = clone_env_with(&["api"], |bares| {
        let mut info = remote_info("acme/platform/api", &bares[0].display().to_string());
        info.https_url = None;
        vec![info]
    });
    let outcome = e.ctx.clone_remote("api").expect("clone via ssh fallback");
    let origin = git_out(
        &outcome.local_path,
        &["config", "--get", "remote.origin.url"],
    );
    assert_eq!(origin, e.bares[0].display().to_string());

    // Only an https URL while ssh is requested: fall back to https.
    let e = clone_env_with(&["api"], |bares| {
        let mut info = remote_info("acme/platform/api", &bares[0].display().to_string());
        info.ssh_url = None;
        vec![info]
    });
    *e.api.protocol.lock().expect("protocol") = Some(GitProtocol::Ssh);
    let outcome = e.ctx.clone_remote("api").expect("clone via https fallback");
    let origin = git_out(
        &outcome.local_path,
        &["config", "--get", "remote.origin.url"],
    );
    assert_eq!(origin, e.bares[0].display().to_string());
}

#[test]
fn clone_without_any_clone_url_fails_and_leaves_nothing_behind() {
    // Indexable through its web URL, but neither ssh nor https is known.
    let e = clone_env_with(&["api"], |_| {
        let mut info = remote_info(
            "acme/platform/api",
            "https://git.example.com/acme/platform/api.git",
        );
        info.ssh_url = None;
        info.https_url = None;
        info.web_url = Some("https://git.example.com/acme/platform/api".into());
        vec![info]
    });
    assert_eq!(e.ctx.list_remote_only().expect("list").len(), 1);
    let err = e.ctx.clone_remote("api").expect_err("no URL");
    assert!(matches!(err, WorkpotError::CloneFailed(_)), "{err:?}");
    assert!(err.to_string().contains("no clone URL"), "{err}");
    assert!(!e.clone_root.join("api").exists());
}

// ---- clone policy during sync -----------------------------------------------------------

fn sync_cloned_names(e: &Env) -> Vec<String> {
    let mut names: Vec<String> = e
        .clone_root
        .read_dir()
        .map(|rd| {
            rd.filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

#[test]
fn manual_and_on_open_policies_never_clone_during_sync() {
    for policy in [ClonePolicy::Manual, ClonePolicy::OnOpen] {
        let e = env(&["api", "web"]);
        set_policy(&e, policy, false);
        let summary = e.ctx.run_sync_full(true).expect("sync");
        assert_eq!(summary.remote.cloned, 0, "{policy:?}: {:?}", summary.remote);
        assert_eq!(summary.remote.errors, 0, "{policy:?}: {:?}", summary.remote);
        assert!(sync_cloned_names(&e).is_empty(), "{policy:?}");
        assert_eq!(e.ctx.list_remote_only().expect("list").len(), 2);
    }
}

#[test]
fn auto_policy_respects_include_archived() {
    let e = clone_env_with(&["api", "web"], |bares| {
        let mut listing = listing_for(&["api", "web"], bares);
        listing[0].archived = true;
        listing
    });
    set_policy(&e, ClonePolicy::Auto, false);
    e.ctx
        .remotes_set(
            "acme",
            RemotePatch {
                include_archived: Some(false),
                ..RemotePatch::default()
            },
        )
        .expect("hide archived");

    let first = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(first.cloned, 1, "{first:?}");
    assert_eq!(sync_cloned_names(&e), vec!["web".to_string()]);

    e.ctx
        .remotes_set(
            "acme",
            RemotePatch {
                include_archived: Some(true),
                ..RemotePatch::default()
            },
        )
        .expect("show archived");
    let second = e.ctx.run_remote_catalog_sync(true).expect("sync again");
    assert_eq!(second.cloned, 1, "{second:?}");
    assert_eq!(
        sync_cloned_names(&e),
        vec!["api".to_string(), "web".to_string()]
    );
}

#[test]
fn auto_policy_skips_an_occupied_target_and_still_clones_into_an_empty_one() {
    let e = env(&["api", "web"]);
    set_policy(&e, ClonePolicy::Auto, false);
    let occupied = e.clone_root.join("web");
    std::fs::create_dir_all(&occupied).expect("occupied");
    std::fs::write(occupied.join("keep.txt"), "mine").expect("file");
    std::fs::create_dir_all(e.clone_root.join("api")).expect("empty target");

    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.cloned, 1, "{summary:?}");
    assert_eq!(summary.errors, 0, "{:?}", summary.source_errors);
    assert!(e.clone_root.join("api/.git").is_dir());
    assert!(occupied.join("keep.txt").is_file());
    assert!(!occupied.join(".git").exists());
}

#[test]
fn auto_policy_reports_a_failing_repo_and_clones_the_rest() {
    let e = env(&["api", "web"]);
    set_policy(&e, ClonePolicy::Auto, false);
    std::fs::remove_dir_all(&e.bares[0]).expect("break api remote");

    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.cloned, 1, "{summary:?}");
    assert_eq!(summary.errors, 1, "{summary:?}");
    let message = &summary.source_errors[0].message;
    assert!(message.starts_with("acme/platform/api:"), "{message}");
    assert!(message.contains("clone failed"), "{message}");
    assert_eq!(sync_cloned_names(&e), vec!["web".to_string()]);
}

#[test]
fn auto_policy_honours_a_user_exclude_glob() {
    let e = env(&["api", "web"]);
    set_policy(&e, ClonePolicy::Auto, false);
    let root = e.dir.path().canonicalize().expect("canon").join("clones");
    e.ctx
        .config_mut()
        .expect("config")
        .excludes
        .push(format!("{}/web/**", root.display()));

    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.cloned, 1, "{summary:?}");
    assert_eq!(sync_cloned_names(&e), vec!["api".to_string()]);
}

#[test]
fn auto_policy_clones_nothing_past_an_invalid_exclude_glob() {
    let e = env(&["api", "web"]);
    set_policy(&e, ClonePolicy::Auto, false);
    e.ctx
        .config_mut()
        .expect("config")
        .excludes
        .push("[".into());

    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.cloned, 0, "{summary:?}");
    assert_eq!(summary.errors, 1, "{summary:?}");
    assert!(
        summary.source_errors[0]
            .message
            .contains("invalid exclude glob"),
        "{:?}",
        summary.source_errors
    );
    assert!(sync_cloned_names(&e).is_empty());
}

// ---- failed registration cleans up ------------------------------------------------------

#[test]
fn failed_registration_removes_the_fresh_checkout_so_a_retry_works() {
    let e = env(&["api", "web"]);
    e.ctx.config_mut().expect("config").limits.max_repos = 1;
    e.ctx.clone_remote("api").expect("first clone fits the cap");

    let err = e.ctx.clone_remote("web").expect_err("catalog is full");
    assert!(
        matches!(err, WorkpotError::LocalCatalogSyncCapExceeded { .. }),
        "{err:?}"
    );
    assert!(
        !e.clone_root.join("web").exists(),
        "unregistered checkout must not be left behind"
    );
    assert_eq!(e.ctx.list_remote_only().expect("list").len(), 1);

    e.ctx.config_mut().expect("config").limits.max_repos = 100;
    let retry = e.ctx.clone_remote("web").expect("retry is not blocked");
    assert!(retry.local_path.join(".git").is_dir());
}

#[test]
fn failed_registration_restores_a_pre_existing_empty_target_dir() {
    let e = env(&["api", "web"]);
    e.ctx.config_mut().expect("config").limits.max_repos = 1;
    e.ctx.clone_remote("api").expect("first clone fits the cap");
    let target = e.clone_root.join("web");
    std::fs::create_dir_all(&target).expect("empty target");

    e.ctx.clone_remote("web").expect_err("catalog is full");

    assert!(target.is_dir(), "the user's empty directory is restored");
    assert_eq!(std::fs::read_dir(&target).expect("read").count(), 0);
}

// ---- reappearing / removed upstream -----------------------------------------------------

#[test]
fn cloned_repo_that_disappears_and_returns_upstream_is_present_again() {
    let e = env(&["api", "web"]);
    let web = e.ctx.clone_remote("web").expect("clone").local_path;
    let both = listing_for(&["api", "web"], &e.bares);

    e.api.set_listing(Ok(vec![both[0].clone()]));
    let gone = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(gone.gone, 1, "{gone:?}");
    assert_eq!(
        e.ctx.removed_upstream_paths().expect("flags"),
        std::collections::HashSet::from([web.display().to_string()])
    );
    let status = &e.ctx.remote_source_statuses().expect("statuses")[0];
    assert_eq!(
        status.gone_count, 1,
        "cloned gone rows are kept, not purged"
    );

    e.api.set_listing(Ok(both));
    let back = e.ctx.run_remote_catalog_sync(true).expect("sync again");
    assert_eq!(back.gone, 0);
    assert!(
        e.ctx.removed_upstream_paths().expect("flags").is_empty(),
        "the removed-upstream flag clears when the repo reappears"
    );
    let status = &e.ctx.remote_source_statuses().expect("statuses")[0];
    assert_eq!((status.repo_count, status.gone_count), (2, 0));
    assert!(web.exists());
}

#[test]
fn auto_clone_counts_are_recorded_on_the_run_row() {
    let e = env(&["api", "web"]);
    set_policy(&e, ClonePolicy::Auto, false);
    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.cloned, 2);
    assert_eq!(last_run_counts(&e.ctx), (2, 0));
}

// ---- orphan catalog row after a failed attach -----------------------------------------------

/// Make the attach step fail after `register_manual` has inserted the `locations` row: the
/// attach creates a project for a fresh clone, and this trigger aborts that insert.
fn break_project_inserts(e: &Env) {
    e.ctx
        .db()
        .with_write(|conn| {
            conn.execute_batch(
                "CREATE TRIGGER induced_attach_failure BEFORE INSERT ON projects
                 BEGIN SELECT RAISE(ABORT, 'induced attach failure'); END;",
            )?;
            Ok(())
        })
        .expect("install trigger");
}

fn fix_project_inserts(e: &Env) {
    e.ctx
        .db()
        .with_write(|conn| {
            conn.execute_batch("DROP TRIGGER induced_attach_failure;")?;
            Ok(())
        })
        .expect("drop trigger");
}

fn location_rows(e: &Env) -> Vec<(String, bool)> {
    e.ctx
        .db()
        .with_read(|conn| {
            let mut stmt = conn.prepare("SELECT path, excluded FROM locations ORDER BY path")?;
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        })
        .expect("locations")
}

#[test]
fn attach_failure_after_the_row_was_inserted_leaves_no_orphan_and_a_retry_works() {
    let e = env(&["api", "web"]);
    break_project_inserts(&e);

    let err = e.ctx.clone_remote("web").expect_err("attach is broken");
    assert!(err.to_string().contains("induced attach failure"), "{err}");
    assert!(
        location_rows(&e).is_empty(),
        "the locations row inserted before the failure must be deleted again: {:?}",
        location_rows(&e)
    );
    assert!(!e.clone_root.join("web").exists(), "fresh checkout removed");
    assert_eq!(e.ctx.list_remote_only().expect("list").len(), 2);

    fix_project_inserts(&e);
    let retry = e
        .ctx
        .clone_remote("web")
        .expect("retry must not hit AlreadyRegistered");
    assert!(retry.local_path.join(".git").is_dir());
    assert_eq!(location_rows(&e).len(), 1);
    assert_eq!(e.ctx.list_remote_only().expect("list").len(), 1);
}

#[test]
fn a_pre_existing_location_row_is_never_deleted_by_the_cleanup() {
    let e = env(&["api", "web"]);
    // A hidden (excluded) row for the planned path: `register_manual` fails with
    // AlreadyRegistered before inserting anything, so the row is not ours to remove.
    let target = e
        .dir
        .path()
        .canonicalize()
        .expect("canon")
        .join("clones/web")
        .display()
        .to_string();
    e.ctx
        .db()
        .with_write(|conn| {
            conn.execute(
                "INSERT INTO locations (path, name, registered_at, source, excluded)
                 VALUES (?1, 'web', 1, 'manual', 1)",
                [&target],
            )?;
            Ok(())
        })
        .expect("seed row");

    let err = e.ctx.clone_remote("web").expect_err("already registered");
    assert!(matches!(err, WorkpotError::AlreadyRegistered(_)), "{err:?}");
    assert_eq!(
        location_rows(&e),
        vec![(target, true)],
        "row survives untouched"
    );
    assert!(!e.clone_root.join("web").exists(), "fresh checkout removed");
}

// ---- clone failures are logged ------------------------------------------------------------

/// Minimal capturing logger: failures that only reach the sync summary as a string must also
/// show up in the log (`warn`). Tests filter by a marker unique to their fixture because the
/// logger is process-global and tests run in parallel.
mod capture {
    use std::sync::{Mutex, Once};

    static LINES: Mutex<Vec<(log::Level, String)>> = Mutex::new(Vec::new());
    static INSTALL: Once = Once::new();
    struct Capture;

    impl log::Log for Capture {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            metadata.level() <= log::Level::Warn
        }
        fn log(&self, record: &log::Record) {
            if self.enabled(record.metadata())
                && let Ok(mut lines) = LINES.lock()
            {
                lines.push((record.level(), record.args().to_string()));
            }
        }
        fn flush(&self) {}
    }

    pub fn install() {
        INSTALL.call_once(|| {
            log::set_logger(&Capture).expect("no other logger in this test binary");
            log::set_max_level(log::LevelFilter::Warn);
        });
    }

    /// Warn-or-worse lines containing every marker.
    pub fn lines_with(markers: &[&str]) -> Vec<String> {
        LINES
            .lock()
            .expect("lines")
            .iter()
            .filter(|(_, msg)| markers.iter().all(|m| msg.contains(m)))
            .map(|(_, msg)| msg.clone())
            .collect()
    }
}

#[test]
fn clone_into_an_occupied_target_is_logged() {
    capture::install();
    let e = env(&["occupied-logged"]);
    let target = e.clone_root.join("occupied-logged");
    std::fs::create_dir_all(&target).expect("target");
    std::fs::write(target.join("keep.txt"), "mine").expect("file");

    let err = e.ctx.clone_remote("occupied-logged").expect_err("refused");
    assert!(err.to_string().contains("not empty"), "{err}");
    let logged = capture::lines_with(&["acme/platform/occupied-logged", "not empty"]);
    assert_eq!(logged.len(), 1, "{logged:?}");
    assert!(logged[0].contains("(source acme) failed"), "{logged:?}");
}

#[test]
fn clone_without_any_clone_url_is_logged() {
    capture::install();
    let e = clone_env_with(&["nourl-logged"], |_| {
        let mut info = remote_info("acme/platform/nourl-logged", "unused");
        info.ssh_url = None;
        info.https_url = None;
        info.web_url = Some("https://git.example.com/acme/platform/nourl-logged".into());
        vec![info]
    });
    e.ctx.clone_remote("nourl-logged").expect_err("no URL");
    let logged = capture::lines_with(&["acme/platform/nourl-logged", "no clone URL"]);
    assert_eq!(logged.len(), 1, "{logged:?}");
}
