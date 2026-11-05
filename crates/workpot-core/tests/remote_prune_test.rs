#![allow(clippy::disallowed_methods)]

//! `prune_removed`: safe-only deletion of clones whose upstream repo disappeared.
//!
//! Every fixture lists at least two repos so the sync is not aborted by the empty-listing guard
//! and the prune step really runs; each "keep" test asserts the exact reason.

mod common;

use common::{
    CloneEnv, clone_env, git_cmd, git_in, git_out, gitlab_source, last_run_counts, listing_for,
    make_bare, remote_info, set_policy,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use workpot_core::domain::config::ClonePolicy;
use workpot_core::infra::git;
use workpot_core::services::remote_prune::{self, PruneReport};

/// Clone every repo of `names`, enable prune, then make `gone` disappear upstream (listing and
/// direct lookup). Returns the env and the catalog path of the gone repo's clone.
fn gone_env(names: &[&str], gone: &str, prune: bool) -> (CloneEnv, PathBuf) {
    assert!(
        names.len() >= 2,
        "a single-repo listing aborts before prune"
    );
    let e = clone_env(names);
    let mut gone_path = None;
    for n in names {
        let outcome = e.ctx.clone_remote(n).expect("clone");
        if *n == gone {
            gone_path = Some(outcome.local_path);
        }
    }
    set_policy(&e, ClonePolicy::Manual, prune);
    let remaining: Vec<&str> = names.iter().copied().filter(|n| *n != gone).collect();
    let remaining_bares: Vec<PathBuf> = names
        .iter()
        .zip(&e.bares)
        .filter(|(n, _)| **n != gone)
        .map(|(_, b)| b.clone())
        .collect();
    e.api
        .set_listing(Ok(listing_for(&remaining, &remaining_bares)));
    e.api
        .missing
        .lock()
        .expect("missing")
        .insert(format!("acme/platform/{gone}"));
    (e, gone_path.expect("gone repo cloned"))
}

fn prune_env(names: &[&str], gone: &str) -> (CloneEnv, PathBuf) {
    gone_env(names, gone, true)
}

/// Run `prune_source` directly for source `acme` and return its report.
fn prune_report(e: &CloneEnv) -> PruneReport {
    let source = e
        .ctx
        .config()
        .expect("config")
        .remote("acme")
        .expect("source")
        .clone();
    remote_prune::prune_source(e.ctx.db(), e.api.as_ref(), &source)
}

/// Commit `f.txt` in `repo` and push it, leaving a clean tree that matches origin.
fn commit_and_push_file(repo: &Path, name: &str, content: &str) {
    std::fs::write(repo.join(name), content).expect("write");
    git_in(repo, &["add", name]);
    git_in(repo, &["commit", "-q", "-m", "add file"]);
    git_in(repo, &["push", "-q", "origin", "main"]);
}

/// `mutate` the `web` clone, sync, and assert the clone is kept for exactly `reason`.
fn assert_kept(mutate: impl FnOnce(&Path), reason: &str) {
    let (e, web) = prune_env(&["api", "web"], "web");
    mutate(&web);

    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.errors, 0, "{:?}", summary.source_errors);
    assert_eq!(summary.gone, 1, "{summary:?}");
    assert_eq!(summary.pruned, 0, "must keep ({reason}): {summary:?}");
    assert!(web.exists(), "kept clone must still be on disk ({reason})");
    assert_eq!(
        e.ctx.removed_upstream_paths().expect("flags"),
        HashSet::from([web.display().to_string()]),
        "kept clone is flagged as removed upstream"
    );

    let report = prune_report(&e);
    assert_eq!(report.pruned, 0, "{report:?}");
    assert_eq!(
        report.kept,
        vec![(web.display().to_string(), reason.to_string())]
    );
    assert!(web.exists());
}

/// `mutate` the `web` clone, sync, and assert it is deleted and forgotten.
fn assert_pruned(mutate: impl FnOnce(&Path)) {
    let (e, web) = prune_env(&["api", "web"], "web");
    mutate(&web);

    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.errors, 0, "{:?}", summary.source_errors);
    assert_eq!(summary.pruned, 1, "{summary:?}");
    assert!(!web.exists(), "safe clone must be deleted");
    assert_eq!(e.ctx.list_repos().expect("repos").len(), 1);
    assert!(e.ctx.removed_upstream_paths().expect("flags").is_empty());
}

// ---- happy path / opt-in ------------------------------------------------------------------

#[test]
fn prune_deletes_clean_pushed_clone_removed_upstream() {
    let (e, web) = prune_env(&["api", "web"], "web");
    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.errors, 0, "{:?}", summary.source_errors);
    assert_eq!(summary.gone, 1);
    assert_eq!(summary.pruned, 1);
    assert!(!web.exists());
    assert!(e.clone_root.join("api").exists());
    let repos = e.ctx.list_repos().expect("repos");
    assert_eq!(repos.len(), 1);
    assert!(e.ctx.removed_upstream_paths().expect("flags").is_empty());
    assert_eq!(last_run_counts(&e.ctx), (0, 1), "pruned count is recorded");
}

#[test]
fn prune_is_disabled_by_default() {
    let (e, web) = gone_env(&["api", "web"], "web", false);
    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.errors, 0, "{:?}", summary.source_errors);
    assert_eq!(summary.gone, 1);
    assert_eq!(summary.pruned, 0);
    assert!(web.exists());
    assert_eq!(
        e.ctx.removed_upstream_paths().expect("flags"),
        HashSet::from([web.display().to_string()])
    );
    assert_eq!(prune_report(&e), PruneReport::default());
}

#[test]
fn gone_repo_without_local_clone_is_forgotten() {
    let e = clone_env(&["api", "web"]);
    e.api.set_listing(Ok(listing_for(&["api"], &e.bares[..1])));
    e.ctx.run_remote_catalog_sync(true).expect("sync");
    let statuses = e.ctx.remote_source_statuses().expect("statuses");
    assert_eq!(statuses[0].gone_count, 0, "uncloned gone rows are purged");
    assert_eq!(statuses[0].repo_count, 1);
}

// ---- repo state guards (exact reasons) -------------------------------------------------------

#[test]
fn prune_keeps_clone_with_staged_changes() {
    assert_kept(
        |web| {
            std::fs::write(web.join("f.txt"), "x").expect("write");
            git_in(web, &["add", "f.txt"]);
        },
        "uncommitted changes",
    );
}

#[test]
fn prune_keeps_clone_with_unstaged_edit_of_tracked_file() {
    assert_kept(
        |web| {
            commit_and_push_file(web, "f.txt", "one");
            std::fs::write(web.join("f.txt"), "two").expect("edit");
        },
        "uncommitted changes",
    );
}

#[test]
fn prune_keeps_clone_with_untracked_files() {
    assert_kept(
        |web| std::fs::write(web.join("scratch.txt"), "x").expect("write"),
        "untracked files",
    );
}

#[test]
fn prune_keeps_clone_with_stash_entries() {
    assert_kept(
        |web| {
            commit_and_push_file(web, "f.txt", "one");
            std::fs::write(web.join("f.txt"), "two").expect("edit");
            git_in(web, &["stash", "push", "-q"]);
            // Tree is clean again; only the stash entry is left to lose.
        },
        "stash entries",
    );
}

#[test]
fn prune_keeps_clone_with_unpushed_commits() {
    assert_kept(
        |web| git_in(web, &["commit", "--allow-empty", "-m", "local only", "-q"]),
        "branch main has 1 unpushed commit(s)",
    );
}

#[test]
fn prune_keeps_clone_with_local_branch_without_upstream() {
    assert_kept(
        |web| git_in(web, &["branch", "wip"]),
        "branch wip has no upstream",
    );
}

#[test]
fn prune_keeps_clone_with_detached_head_on_an_unpushed_commit() {
    assert_kept(
        |web| {
            git_in(web, &["checkout", "-q", "--detach"]);
            git_in(
                web,
                &["commit", "--allow-empty", "-m", "detached work", "-q"],
            );
        },
        "detached HEAD at a commit not on any remote branch",
    );
}

#[test]
fn prune_keeps_clone_with_an_unpushed_local_tag() {
    assert_kept(
        |web| {
            // The tagged commit is on no branch, so branch checks alone would not notice it.
            git_in(web, &["checkout", "-q", "--detach"]);
            git_in(web, &["commit", "--allow-empty", "-m", "tagged work", "-q"]);
            git_in(web, &["tag", "keepme"]);
            git_in(web, &["checkout", "-q", "main"]);
        },
        "local tag keepme is not on any remote branch",
    );
}

#[test]
fn prune_keeps_clone_with_a_committed_gitmodules_file() {
    assert_kept(
        |web| {
            commit_and_push_file(
                web,
                ".gitmodules",
                "[submodule \"x\"]\n\tpath = x\n\turl = ./x\n",
            );
        },
        "has submodules",
    );
}

#[test]
fn prune_keeps_clone_with_populated_git_modules_dir() {
    assert_kept(
        |web| std::fs::create_dir_all(web.join(".git/modules/dep")).expect("modules"),
        "has submodules",
    );
}

#[test]
fn prune_keeps_clone_with_linked_worktrees() {
    assert_kept(
        |web| {
            let outside = web
                .parent()
                .and_then(Path::parent)
                .expect("tempdir")
                .join("linked-wt");
            git_in(
                web,
                &[
                    "worktree",
                    "add",
                    "-q",
                    "-b",
                    "feat",
                    &outside.display().to_string(),
                ],
            );
        },
        "has linked worktrees",
    );
}

// ---- things that must NOT block deletion ------------------------------------------------------

#[test]
fn prune_deletes_a_detached_head_that_is_already_on_a_remote_branch() {
    assert_pruned(|web| git_in(web, &["checkout", "-q", "--detach", "origin/main"]));
}

#[test]
fn prune_deletes_a_clone_whose_tags_are_all_on_a_remote_branch() {
    assert_pruned(|web| {
        git_in(web, &["tag", "v1"]);
        git_in(web, &["tag", "-a", "v2", "-m", "annotated"]);
    });
}

// ---- fail-closed arms: a check that cannot run must keep the clone --------------------------

#[test]
fn prune_keeps_clone_when_the_dirty_check_cannot_read_the_index() {
    // A garbage index makes the libgit2 status call fail; "unknown" must mean keep. The dirty,
    // untracked and operation checks share that call, so the first one (dirty) reports it; flipping
    // any of them to fail-open changes the reason or deletes the clone.
    assert_kept(
        |web| std::fs::write(web.join(".git/index"), b"not an index").expect("corrupt index"),
        "uncommitted changes",
    );
}

#[test]
fn prune_keeps_clone_when_a_dangling_origin_head_hides_the_remote_tips() {
    // `origin/HEAD` pointing at a deleted branch (renamed default branch) makes the remote tips
    // unresolvable: neither detached HEAD nor tags can be proven pushed.
    let dangle = |web: &Path| {
        std::fs::write(
            web.join(".git/refs/remotes/origin/HEAD"),
            "ref: refs/remotes/origin/renamed-away\n",
        )
        .expect("dangle origin/HEAD");
    };
    assert_kept(dangle, "could not verify local tags are pushed");
    assert_kept(
        |web| {
            git_in(web, &["checkout", "-q", "--detach", "origin/main"]);
            dangle(web);
        },
        "could not verify detached HEAD is pushed",
    );
}

#[test]
fn prune_keeps_clone_when_the_submodule_check_cannot_read_git_modules() {
    // `.git/modules` that is a regular file: read_dir fails with ENOTDIR, not NotFound.
    assert_kept(
        |web| std::fs::write(web.join(".git/modules"), "not a dir").expect("modules file"),
        "could not verify submodules",
    );
}

// ---- ancestry side of "already on a remote branch" -----------------------------------------

/// Push a second commit so the remote tip is strictly ahead of the first (seed) commit.
fn advance_remote(web: &Path) {
    commit_and_push_file(web, "second.txt", "two");
}

#[test]
fn prune_deletes_a_detached_head_on_an_ancestor_of_a_remote_tip() {
    assert_pruned(|web| {
        advance_remote(web);
        git_in(web, &["checkout", "-q", "--detach", "HEAD~1"]);
    });
}

#[test]
fn prune_deletes_a_clone_whose_tag_is_an_ancestor_of_a_remote_tip() {
    assert_pruned(|web| {
        advance_remote(web);
        git_in(web, &["tag", "v0", "HEAD~1"]);
        git_in(web, &["tag", "-a", "v0-annotated", "-m", "old", "HEAD~1"]);
    });
}

#[test]
fn prune_keeps_a_detached_head_that_diverged_from_an_advanced_remote_tip() {
    // The remote tip is ahead, but the detached commit is a sibling of it, not an ancestor.
    assert_kept(
        |web| {
            advance_remote(web);
            git_in(web, &["checkout", "-q", "--detach", "HEAD~1"]);
            git_in(web, &["commit", "--allow-empty", "-m", "side work", "-q"]);
        },
        "detached HEAD at a commit not on any remote branch",
    );
}

#[test]
fn prune_keeps_a_tag_on_a_commit_that_diverged_from_an_advanced_remote_tip() {
    assert_kept(
        |web| {
            advance_remote(web);
            git_in(web, &["checkout", "-q", "--detach", "HEAD~1"]);
            git_in(web, &["commit", "--allow-empty", "-m", "side work", "-q"]);
            git_in(web, &["tag", "side"]);
            git_in(web, &["checkout", "-q", "main"]);
        },
        "local tag side is not on any remote branch",
    );
}

// ---- tags that do not point at a commit ------------------------------------------------------

#[test]
fn prune_ignores_tags_on_trees_and_blobs() {
    // No history hangs off a tag of a tree/blob, so it must not block deletion.
    assert_pruned(|web| {
        let tree = git_out(web, &["rev-parse", "HEAD^{tree}"]);
        git_in(web, &["tag", "tree-tag", &tree]);
        git_in(
            web,
            &[
                "tag",
                "-a",
                "-m",
                "annotated tree",
                "tree-tag-annotated",
                &tree,
            ],
        );
        std::fs::write(web.join("blob-src.txt"), "blob body").expect("write");
        let blob = git_out(web, &["hash-object", "-w", "blob-src.txt"]);
        std::fs::remove_file(web.join("blob-src.txt")).expect("rm");
        git_in(web, &["tag", "blob-tag", &blob]);
    });
}

#[test]
fn find_unpushed_tag_skips_non_commit_tags_but_fails_on_an_unreadable_object() {
    let (_e, web) = prune_env(&["api", "web"], "web");
    let tree = git_out(&web, &["rev-parse", "HEAD^{tree}"]);
    git_in(&web, &["tag", "tree-tag", &tree]);
    git_in(&web, &["tag", "-a", "-m", "x", "tree-tag-annotated", &tree]);
    assert_eq!(
        git::find_unpushed_tag(&web).expect("tree tags are skipped"),
        None
    );

    // A tag ref whose object is missing from the object database cannot be classified.
    let ghost = "1234567890123456789012345678901234567890";
    std::fs::write(web.join(".git/refs/tags/ghost"), format!("{ghost}\n")).expect("ghost tag");
    assert!(
        git::find_unpushed_tag(&web).is_err(),
        "unknown object must not be skipped"
    );
}

#[test]
fn prune_keeps_clone_with_a_tag_whose_object_is_missing() {
    assert_kept(
        |web| {
            let ghost = "1234567890123456789012345678901234567890";
            std::fs::write(web.join(".git/refs/tags/ghost"), format!("{ghost}\n"))
                .expect("ghost tag");
        },
        "could not verify local tags are pushed",
    );
}

// ---- unreadable linked worktree entries --------------------------------------------------------

#[test]
fn prune_keeps_clone_with_an_unreadable_linked_worktree_entry() {
    let (e, web) = prune_env(&["api", "web"], "web");
    let linked = e.dir.path().join("linked-gone");
    git_in(
        &web,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "feat",
            &linked.display().to_string(),
        ],
    );
    // The branch is not what is under test; keep it from adding its own blocker.
    git_in(&web, &["push", "-q", "-u", "origin", "feat"]);
    // The checkout vanishes (moved/deleted) but git's admin entry stays behind.
    std::fs::remove_dir_all(&linked).expect("remove linked checkout");

    let listing = git::list_worktrees(&web).expect("list");
    assert!(listing.paths.is_empty(), "{listing:?}");
    assert_eq!(listing.unreadable, 1, "{listing:?}");

    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.errors, 0, "{:?}", summary.source_errors);
    assert_eq!(summary.gone, 1, "{summary:?}");
    assert_eq!(summary.pruned, 0, "{summary:?}");
    assert!(web.exists());
    assert_eq!(
        prune_report(&e).kept,
        vec![(
            web.display().to_string(),
            "could not read linked worktree entries".to_string()
        )]
    );
}

// ---- merge / rebase in progress and conflicts ---------------------------------------------------

/// Run `git <args>` in `dir` and assert it fails (a conflicting merge/rebase/stash pop).
fn git_in_expecting_conflict(dir: &Path, args: &[&str]) {
    let status = git_cmd()
        .args(["-c", "user.name=T", "-c", "user.email=t@example.com"])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git");
    assert!(
        !status.status.success(),
        "git {args:?} should have conflicted"
    );
}

/// Leave `web` clean and fully pushed except for a stopped merge whose only trace is the
/// merge state plus conflicted `f.txt`.
fn start_conflicting_merge(web: &Path) {
    commit_and_push_file(web, "f.txt", "ours");
    git_in(web, &["checkout", "-q", "-b", "other", "HEAD~1"]);
    std::fs::write(web.join("f.txt"), "theirs").expect("write");
    git_in(web, &["add", "f.txt"]);
    git_in(web, &["commit", "-q", "-m", "theirs"]);
    git_in(web, &["checkout", "-q", "main"]);
    git_in_expecting_conflict(web, &["merge", "other"]);
    // `other` was only the merge source; with it gone, no branch/stash blocker can interfere.
    git_in(web, &["branch", "-q", "-D", "other"]);
}

#[test]
fn prune_keeps_clone_in_the_middle_of_a_conflicted_merge() {
    assert_kept(
        start_conflicting_merge,
        "repository is in the middle of an operation (merge)",
    );
}

#[test]
fn prune_keeps_clone_with_conflicted_files_even_without_an_operation_in_progress() {
    // A conflicted `stash pop` leaves conflict entries but no merge/rebase state.
    assert_kept(
        |web| {
            commit_and_push_file(web, "f.txt", "base");
            std::fs::write(web.join("f.txt"), "stashed").expect("edit");
            git_in(web, &["stash", "push", "-q"]);
            commit_and_push_file(web, "f.txt", "upstream moved");
            git_in_expecting_conflict(web, &["stash", "pop"]);
            git_in(web, &["stash", "drop", "-q"]);
        },
        "has conflicted files",
    );
}

#[test]
fn prune_keeps_clone_stopped_in_a_conflicted_rebase() {
    assert_kept(
        |web| {
            commit_and_push_file(web, "f.txt", "ours");
            git_in(web, &["checkout", "-q", "-b", "other", "HEAD~1"]);
            std::fs::write(web.join("f.txt"), "theirs").expect("write");
            git_in(web, &["add", "f.txt"]);
            git_in(web, &["commit", "-q", "-m", "theirs"]);
            git_in_expecting_conflict(web, &["rebase", "main"]);
        },
        "repository is in the middle of an operation (rebase)",
    );
}

#[test]
fn prune_deletes_a_clone_whose_merge_was_completed_and_pushed() {
    assert_pruned(|web| {
        commit_and_push_file(web, "f.txt", "ours");
        git_in(web, &["checkout", "-q", "-b", "other", "HEAD~1"]);
        std::fs::write(web.join("g.txt"), "theirs").expect("write");
        git_in(web, &["add", "g.txt"]);
        git_in(web, &["commit", "-q", "-m", "theirs"]);
        git_in(web, &["checkout", "-q", "main"]);
        git_in(web, &["merge", "-q", "--no-edit", "other"]);
        git_in(web, &["branch", "-q", "-d", "other"]);
        git_in(web, &["push", "-q", "origin", "main"]);
    });
}

#[test]
fn prune_deletes_a_clone_whose_conflict_was_resolved_and_pushed() {
    assert_pruned(|web| {
        start_conflicting_merge(web);
        std::fs::write(web.join("f.txt"), "resolved").expect("resolve");
        git_in(web, &["add", "f.txt"]);
        git_in(web, &["commit", "-q", "--no-edit"]);
        git_in(web, &["push", "-q", "origin", "main"]);
    });
}

// ---- placement guards ---------------------------------------------------------------------------

/// Hand-clone `web` to `target`, register it, then make `web` disappear upstream.
fn hand_cloned_gone_env(
    clone_api_first: bool,
    target: impl FnOnce(&CloneEnv) -> PathBuf,
) -> (CloneEnv, PathBuf) {
    let e = clone_env(&["api", "web"]);
    if clone_api_first {
        e.ctx.clone_remote("api").expect("clone api");
    }
    let target = target(&e);
    std::fs::create_dir_all(target.parent().expect("parent")).expect("parent dir");
    let status = git_cmd()
        .args(["clone", "-q"])
        .arg(&e.bares[1])
        .arg(&target)
        .status()
        .expect("hand clone");
    assert!(status.success());
    let registered = e.ctx.register_manual(&target).expect("register").path;
    e.ctx.run_local_catalog_sync().expect("local sync");
    let remote_only: Vec<_> = e
        .ctx
        .list_remote_only()
        .expect("remote only")
        .into_iter()
        .map(|r| r.full_path)
        .collect();
    assert!(
        !remote_only.contains(&"acme/platform/web".to_string()),
        "hand clone is recognised as cloned: {remote_only:?}"
    );

    set_policy(&e, ClonePolicy::Manual, true);
    e.api.set_listing(Ok(listing_for(&["api"], &e.bares[..1])));
    e.api
        .missing
        .lock()
        .expect("missing")
        .insert("acme/platform/web".into());
    (e, registered)
}

fn assert_hand_clone_kept(e: &CloneEnv, path: &Path) {
    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.errors, 0, "{:?}", summary.source_errors);
    assert_eq!(summary.pruned, 0, "{summary:?}");
    assert!(path.join(".git").is_dir(), "hand clone must survive");
    assert_eq!(
        e.ctx.removed_upstream_paths().expect("flags"),
        HashSet::from([path.display().to_string()])
    );
    assert_eq!(
        prune_report(e).kept,
        vec![(
            path.display().to_string(),
            "outside the source clone_root".to_string()
        )]
    );
}

#[test]
fn prune_never_deletes_a_clone_outside_the_source_clone_root() {
    let (e, path) = hand_cloned_gone_env(true, |e| e.dir.path().join("elsewhere/web"));
    assert_hand_clone_kept(&e, &path);
}

#[test]
fn prune_never_deletes_the_clone_root_itself() {
    // The clone root is the checkout: nothing may be deleted, not even a clean pushed one.
    // (`api` stays uncloned so the root dir is free for `git clone`.)
    let (e, path) = hand_cloned_gone_env(false, |e| e.clone_root.clone());
    assert_hand_clone_kept(&e, &path);
}

// ---- host confirmation ----------------------------------------------------------------------------

#[test]
fn prune_keeps_and_unflags_a_repo_the_host_still_lists() {
    // Listing glitch: absent from the listing, but the direct lookup still finds it.
    let (e, web) = gone_env(&["api", "web"], "web", false);
    e.ctx.run_remote_catalog_sync(true).expect("mark gone");
    assert_eq!(e.ctx.removed_upstream_paths().expect("flags").len(), 1);
    e.api.missing.lock().expect("missing").clear();
    set_policy(&e, ClonePolicy::Manual, true);

    let report = prune_report(&e);
    assert_eq!(report.pruned, 0);
    assert_eq!(
        report.kept,
        vec![(
            web.display().to_string(),
            "still exists upstream".to_string()
        )]
    );
    assert!(web.exists());
    assert!(
        e.ctx.removed_upstream_paths().expect("flags").is_empty(),
        "confirmed-present repo is no longer flagged"
    );
    let status = &e.ctx.remote_source_statuses().expect("statuses")[0];
    assert_eq!((status.repo_count, status.gone_count), (2, 0));
}

#[test]
fn prune_never_deletes_when_host_lookup_fails() {
    let (e, web) = prune_env(&["api", "web"], "web");
    // Listing says gone, but the direct lookup errors (not a 404): must keep.
    e.api.missing.lock().expect("missing").clear();
    e.api
        .exists_error
        .lock()
        .expect("exists_error")
        .insert("acme/platform/web".into());

    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.errors, 0, "{:?}", summary.source_errors);
    assert_eq!(summary.gone, 1);
    assert_eq!(summary.pruned, 0);
    assert!(web.exists());
    assert_eq!(e.ctx.removed_upstream_paths().expect("flags").len(), 1);

    let report = prune_report(&e);
    assert_eq!(report.kept.len(), 1, "{report:?}");
    assert_eq!(report.kept[0].0, web.display().to_string());
    assert!(report.kept[0].1.contains("boom"), "{report:?}");
}

#[test]
fn prune_drops_the_catalog_row_when_the_checkout_is_already_gone_from_disk() {
    let (e, web) = prune_env(&["api", "web"], "web");
    std::fs::remove_dir_all(&web).expect("rm checkout");

    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.errors, 0, "{:?}", summary.source_errors);
    assert_eq!(summary.pruned, 1, "{summary:?}");
    assert_eq!(e.ctx.list_repos().expect("repos").len(), 1);
    assert!(e.ctx.removed_upstream_paths().expect("flags").is_empty());
}

#[test]
fn prune_only_touches_clones_of_its_own_source() {
    let e = clone_env(&["api", "web"]);
    let remotes_dir = e.dir.path().join("remotes");
    let tool = make_bare(&remotes_dir, "tool");
    let tool2 = make_bare(&remotes_dir, "tool2");
    let beta_root = e.dir.path().join("clones-beta");
    e.ctx
        .remotes_add(gitlab_source("beta", "beta/tools", &beta_root))
        .expect("add beta");
    let beta_listing = |bares: &[&PathBuf]| {
        bares
            .iter()
            .map(|b| {
                let name = b.file_stem().and_then(|n| n.to_str()).expect("name");
                remote_info(&format!("beta/tools/{name}"), &b.display().to_string())
            })
            .collect::<Vec<_>>()
    };
    e.api
        .set_listing_for("beta", Ok(beta_listing(&[&tool, &tool2])));
    e.ctx.run_remote_catalog_sync(true).expect("index both");

    let web = e.ctx.clone_remote("web").expect("clone web").local_path;
    let beta_tool = e
        .ctx
        .clone_remote("beta:beta/tools/tool")
        .expect("clone tool")
        .local_path;

    // Both clones lose their upstream repo; only `acme` may prune.
    set_policy(&e, ClonePolicy::Manual, true);
    e.api.set_listing(Ok(listing_for(&["api"], &e.bares[..1])));
    e.api.set_listing_for("beta", Ok(beta_listing(&[&tool2])));
    {
        let mut missing = e.api.missing.lock().expect("missing");
        missing.insert("acme/platform/web".into());
        missing.insert("beta/tools/tool".into());
    }

    let summary = e.ctx.run_remote_catalog_sync(true).expect("sync");
    assert_eq!(summary.errors, 0, "{:?}", summary.source_errors);
    assert_eq!(summary.gone, 2, "{summary:?}");
    assert_eq!(summary.pruned, 1, "{summary:?}");
    assert!(!web.exists(), "acme prunes its own clone");
    assert!(beta_tool.exists(), "beta has prune disabled");
    assert_eq!(
        e.ctx.removed_upstream_paths().expect("flags"),
        HashSet::from([beta_tool.display().to_string()])
    );

    // Pruning `acme` again must neither act on nor even report beta's gone clone.
    assert_eq!(prune_report(&e), PruneReport::default());
    assert!(beta_tool.exists());
}
