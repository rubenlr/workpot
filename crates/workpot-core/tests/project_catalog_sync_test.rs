#![allow(clippy::disallowed_methods)]

mod common;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use workpot_core::domain::{Config, normalize_remote_url, project_id_for_root};
use workpot_core::infra::store;
use workpot_core::services::local_catalog_sync;

fn git_worktree(parent: &Path, name: &str) -> PathBuf {
    let repo = parent.join(name);
    fs::create_dir_all(&repo).expect("repo dir");
    let status = common::git_cmd()
        .args(["init", "-q", "-b", "main"])
        .current_dir(&repo)
        .status()
        .expect("git init");
    assert!(status.success(), "git init failed for {}", repo.display());
    for (key, val) in [("user.email", "t@example.com"), ("user.name", "Test")] {
        let status = common::git_cmd()
            .args(["config", "--local", key, val])
            .current_dir(&repo)
            .status()
            .expect("config");
        assert!(status.success());
    }
    let status = common::git_cmd()
        .args(["commit", "--allow-empty", "-m", "init", "-q"])
        .current_dir(&repo)
        .status()
        .expect("commit");
    assert!(status.success());
    repo
}

fn add_remote(repo: &Path, name: &str, url: &str) {
    let status = common::git_cmd()
        .args(["remote", "add", name, url])
        .current_dir(repo)
        .status()
        .expect("remote add");
    assert!(status.success(), "remote add {name} failed");
}

fn open_fixture() -> (tempfile::TempDir, rusqlite::Connection, Config) {
    let dir = tempfile::tempdir().expect("tempdir");
    let watch_root = dir.path().join("watch");
    fs::create_dir_all(&watch_root).expect("watch root");
    let db_path = dir.path().join("workpot.db");
    let mut config = Config::default();
    config.watch_roots.push(watch_root);
    // Avoid network fetch noise against fake remotes.
    config.fetch = String::new();
    let conn = store::open_connection(&db_path).expect("open db");
    (dir, conn, config)
}

fn path_key(repo: &Path) -> String {
    repo.canonicalize()
        .expect("canonicalize")
        .display()
        .to_string()
}

fn project_id_for_location(conn: &rusqlite::Connection, location: &str) -> String {
    conn.query_row(
        "SELECT project_id FROM locations WHERE path = ?1",
        rusqlite::params![location],
        |row| row.get::<_, Option<String>>(0),
    )
    .expect("project_id")
    .expect("project_id must be set after sync")
}

#[test]
fn two_clones_same_origin_share_one_project() {
    let (dir, conn, config) = open_fixture();
    let watch = &config.watch_roots[0];

    // Keep bare origin outside the watch root so it is not cataloged as a third location.
    let bare = dir.path().join("origin.git");
    fs::create_dir_all(&bare).expect("bare dir");
    let status = common::git_cmd()
        .args(["init", "--bare", "-q", "-b", "main"])
        .arg(&bare)
        .status()
        .expect("bare init");
    assert!(status.success());
    common::seed_bare_repo(&bare);

    let clone_a = watch.join("clone-a");
    let clone_b = watch.join("clone-b");
    for dest in [&clone_a, &clone_b] {
        let status = common::git_cmd()
            .args([
                "clone",
                "-q",
                bare.to_str().expect("utf8"),
                dest.to_str().expect("utf8"),
            ])
            .status()
            .expect("clone");
        assert!(status.success());
    }

    local_catalog_sync::run_full_connection(&conn, &config).expect("sync");

    let key_a = path_key(&clone_a);
    let key_b = path_key(&clone_b);
    let id_a = project_id_for_location(&conn, &key_a);
    let id_b = project_id_for_location(&conn, &key_b);
    assert_eq!(id_a, id_b, "same origin must attach to one project");

    let project_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
        .expect("count projects");
    assert_eq!(project_count, 1);

    let location_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM locations WHERE project_id = ?1 AND excluded = 0",
            rusqlite::params![id_a],
            |row| row.get(0),
        )
        .expect("count locations");
    assert_eq!(location_count, 2);
}

#[test]
fn upstream_and_origin_elects_upstream_root() {
    let (_dir, conn, config) = open_fixture();
    let watch = &config.watch_roots[0];
    let repo = git_worktree(watch, "with-upstream");
    add_remote(&repo, "origin", "git@github.com:fork/app.git");
    add_remote(&repo, "upstream", "git@github.com:org/app.git");

    local_catalog_sync::run_full_connection(&conn, &config).expect("sync");

    let key = path_key(&repo);
    let project_id = project_id_for_location(&conn, &key);
    let expected_root = normalize_remote_url("git@github.com:org/app.git").expect("norm");
    assert_eq!(project_id, project_id_for_root(&expected_root));

    let (root_norm, root_name): (String, Option<String>) = conn
        .query_row(
            "SELECT p.root_remote_normalized, pr.remote_name
             FROM projects p
             JOIN project_remotes pr ON pr.project_id = p.id AND pr.role = 'root'
             WHERE p.id = ?1",
            rusqlite::params![project_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("root remote");
    assert_eq!(root_norm, expected_root);
    assert_eq!(root_name.as_deref(), Some("upstream"));

    let roles: HashSet<String> = conn
        .prepare("SELECT role FROM project_remotes WHERE project_id = ?1")
        .expect("prep")
        .query_map(rusqlite::params![project_id], |row| row.get(0))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("roles");
    assert!(roles.contains("root"));
    assert!(roles.contains("fork"));
}

#[test]
fn fork_only_clone_joins_after_fork_alias_known() {
    let (_dir, conn, config) = open_fixture();
    let watch = &config.watch_roots[0];

    let primary = git_worktree(watch, "primary");
    add_remote(&primary, "origin", "git@github.com:me/app.git");
    add_remote(&primary, "upstream", "git@github.com:org/app.git");

    local_catalog_sync::run_full_connection(&conn, &config).expect("first sync");
    let primary_id = project_id_for_location(&conn, &path_key(&primary));
    let expected =
        project_id_for_root(&normalize_remote_url("git@github.com:org/app.git").expect("norm"));
    assert_eq!(primary_id, expected);

    let fork_only = git_worktree(watch, "fork-only");
    add_remote(&fork_only, "origin", "git@github.com:me/app.git");

    local_catalog_sync::run_full_connection(&conn, &config).expect("second sync");
    let fork_id = project_id_for_location(&conn, &path_key(&fork_only));
    assert_eq!(
        fork_id, primary_id,
        "fork-only clone must join via known fork alias"
    );

    let project_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
        .expect("count");
    assert_eq!(project_count, 1);
}

#[test]
fn overlapping_projects_merge_on_shared_url() {
    let (_dir, conn, config) = open_fixture();
    let watch = &config.watch_roots[0];

    let fork_repo = git_worktree(watch, "fork-first");
    add_remote(&fork_repo, "origin", "git@github.com:me/app.git");
    local_catalog_sync::run_full_connection(&conn, &config).expect("fork sync");
    let fork_project = project_id_for_location(&conn, &path_key(&fork_repo));
    assert_eq!(
        fork_project,
        project_id_for_root(&normalize_remote_url("git@github.com:me/app.git").expect("norm"))
    );

    let org_repo = git_worktree(watch, "org-only");
    add_remote(&org_repo, "origin", "git@github.com:org/app.git");
    local_catalog_sync::run_full_connection(&conn, &config).expect("org sync");
    let org_project = project_id_for_location(&conn, &path_key(&org_repo));
    assert_ne!(fork_project, org_project);

    // Introduce shared URL on the org checkout → attach conflict + merge pass.
    add_remote(&org_repo, "mine", "git@github.com:me/app.git");
    local_catalog_sync::run_full_connection(&conn, &config).expect("overlap sync");

    let id_fork = project_id_for_location(&conn, &path_key(&fork_repo));
    let id_org = project_id_for_location(&conn, &path_key(&org_repo));
    assert_eq!(id_fork, id_org, "overlap merge must unify projects");

    let project_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
        .expect("count");
    assert_eq!(project_count, 1);

    // Neither side is upstream-rooted; sticky survivor root is whichever 4A picks
    // (location_count / created_at / id). Only require it is one of the two remotes.
    let root: String = conn
        .query_row(
            "SELECT root_remote_normalized FROM projects WHERE id = ?1",
            rusqlite::params![id_fork],
            |row| row.get(0),
        )
        .expect("root");
    let me = normalize_remote_url("git@github.com:me/app.git").expect("norm");
    let org = normalize_remote_url("git@github.com:org/app.git").expect("norm");
    assert!(
        root == me || root == org,
        "merged root must be one of the overlapping remotes, got {root}"
    );
}

#[test]
fn sync_persists_branches_and_worktrees_for_normal_repo() {
    let (_dir, conn, config) = open_fixture();
    let watch = &config.watch_roots[0];
    let repo = git_worktree(watch, "graph-repo");

    local_catalog_sync::run_full_connection(&conn, &config).expect("sync");
    let key = path_key(&repo);

    let branch_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM branches WHERE location_path = ?1",
            rusqlite::params![key],
            |row| row.get(0),
        )
        .expect("branches");
    assert!(
        branch_count >= 1,
        "expected at least one local branch row, got {branch_count}"
    );

    let wt_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM worktrees WHERE location_path = ?1",
            rusqlite::params![key],
            |row| row.get(0),
        )
        .expect("worktrees");
    assert_eq!(wt_count, 1);

    let wt_path: String = conn
        .query_row(
            "SELECT path FROM worktrees WHERE location_path = ?1",
            rusqlite::params![key],
            |row| row.get(0),
        )
        .expect("wt path");
    assert_eq!(wt_path, key);
}

#[test]
fn sticky_root_fork_location_does_not_flip_upstream_project() {
    let (_dir, conn, config) = open_fixture();
    let watch = &config.watch_roots[0];

    let upstream_repo = git_worktree(watch, "upstream-rooted");
    add_remote(&upstream_repo, "origin", "git@github.com:me/app.git");
    add_remote(&upstream_repo, "upstream", "git@github.com:org/app.git");
    local_catalog_sync::run_full_connection(&conn, &config).expect("upstream sync");

    let project_id = project_id_for_location(&conn, &path_key(&upstream_repo));
    let root_before: String = conn
        .query_row(
            "SELECT root_remote_normalized FROM projects WHERE id = ?1",
            rusqlite::params![project_id],
            |row| row.get(0),
        )
        .expect("root before");
    assert_eq!(
        root_before,
        normalize_remote_url("git@github.com:org/app.git").expect("norm")
    );

    let fork_repo = git_worktree(watch, "fork-join");
    add_remote(&fork_repo, "origin", "git@github.com:me/app.git");
    local_catalog_sync::run_full_connection(&conn, &config).expect("fork sync");

    let fork_id = project_id_for_location(&conn, &path_key(&fork_repo));
    assert_eq!(fork_id, project_id);

    let root_after: String = conn
        .query_row(
            "SELECT root_remote_normalized FROM projects WHERE id = ?1",
            rusqlite::params![project_id],
            |row| row.get(0),
        )
        .expect("root after");
    assert_eq!(
        root_after, root_before,
        "fork location must not flip upstream-rooted project root"
    );
}
