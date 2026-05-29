use std::fs;
use std::path::Path;
use workpot_core::domain::Config;
use workpot_core::infra::store;
use workpot_core::services::index;

fn git_worktree(parent: &Path, name: &str) -> std::path::PathBuf {
    let repo = parent.join(name);
    let git_dir = repo.join(".git");
    fs::create_dir_all(git_dir.join("objects")).expect("objects");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("HEAD");
    repo
}

#[test]
fn index_full_rescan_minimal() {
    let dir = tempfile::tempdir().expect("tempdir");
    let watch_root = dir.path().join("watch");
    fs::create_dir_all(&watch_root).expect("watch root");
    git_worktree(&watch_root, "repo-a");
    git_worktree(&watch_root, "repo-b");

    let db_path = dir.path().join("workpot.db");
    let mut config = Config::default();
    config.watch_roots.push(watch_root);

    let conn = store::open_connection(&db_path).expect("open db");

    index::run_full(&conn, &config).expect("first run_full");
    let count_after_first: i64 = conn
        .query_row("SELECT COUNT(*) FROM repos WHERE excluded = 0", [], |row| {
            row.get(0)
        })
        .expect("count repos");
    assert_eq!(count_after_first, 2);

    index::run_full(&conn, &config).expect("second run_full");
    let count_after_second: i64 = conn
        .query_row("SELECT COUNT(*) FROM repos WHERE excluded = 0", [], |row| {
            row.get(0)
        })
        .expect("count repos");
    assert_eq!(count_after_second, 2);
}
