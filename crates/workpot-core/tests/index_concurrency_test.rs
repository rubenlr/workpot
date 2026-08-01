#![allow(clippy::disallowed_methods)]

mod common;

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use workpot_core::AppState;

fn git_worktree(parent: &Path, name: &str) -> std::path::PathBuf {
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
fn list_repos_during_index_stays_fast() {
    unsafe { std::env::set_var("WORKPOT_TEST_INDEX_DELAY_MS", "500") };

    let dir = tempfile::tempdir().expect("tempdir");
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");
    let watch_root = dir.path().join("roots");
    fs::create_dir_all(&watch_root).expect("watch root");
    let _repo = git_worktree(&watch_root, "sample");
    fs::write(
        &config_path,
        format!(
            "watch_roots = [\"{}\"]\nexcludes = []\n",
            watch_root.display()
        ),
    )
    .expect("write config");

    let state = Arc::new(AppState::open_with_paths(config_path, db_path).expect("open"));
    let repo_path = watch_root.join("sample");
    state.register_manual(&repo_path).expect("register");
    let expected_count = state.list_repos().expect("list").len();
    assert_eq!(expected_count, 1);

    let state_for_index = Arc::clone(&state);
    let index_handle = std::thread::spawn(move || {
        state_for_index.run_index_phased().expect("index");
    });

    let mut max_ms = 0u128;
    for _ in 0..100 {
        let start = Instant::now();
        let count = state.list_repos().expect("list during index").len();
        max_ms = max_ms.max(start.elapsed().as_millis());
        assert_eq!(count, expected_count);
        std::thread::sleep(Duration::from_millis(2));
    }

    index_handle.join().expect("index thread");
    unsafe { std::env::remove_var("WORKPOT_TEST_INDEX_DELAY_MS") };

    assert!(
        max_ms < 50,
        "list_repos blocked too long during index: max_ms={max_ms}"
    );
}
