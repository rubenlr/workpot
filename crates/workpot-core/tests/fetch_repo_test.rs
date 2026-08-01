use std::path::PathBuf;
use workpot_core::services::fetch_repo::fetch_repo;

#[test]
fn empty_fetch_cmd_is_noop() {
    let path = PathBuf::from("/tmp/some-repo");
    assert!(fetch_repo(&path, "").is_ok());
    assert!(fetch_repo(&path, "   ").is_ok());
}

#[test]
fn invalid_template_errs_without_running() {
    let path = PathBuf::from("/tmp/some-repo");
    let err = fetch_repo(&path, "git fetch").expect_err("missing {path}");
    assert!(
        err.contains("{path}"),
        "expected placeholder error, got {err}"
    );
}
