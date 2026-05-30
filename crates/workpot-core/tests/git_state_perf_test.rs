use std::path::PathBuf;

#[test]
#[ignore] // opt-in only — creates 50 real repos, slow in normal CI
fn refresh_50_repos() {
    let dir = tempfile::tempdir().expect("tempdir");
    let paths: Vec<PathBuf> = (0..50)
        .map(|i| {
            let p = dir.path().join(format!("repo-{i}"));
            git2::Repository::init(&p).expect("init");
            p
        })
        .collect();
    let start = std::time::Instant::now();
    let results = workpot_core::services::git_state::refresh_all(paths);
    let elapsed = start.elapsed();
    assert_eq!(results.len(), 50);
    assert!(
        elapsed.as_millis() < 500,
        "refresh_all took {}ms, expected <500ms",
        elapsed.as_millis()
    );
}
