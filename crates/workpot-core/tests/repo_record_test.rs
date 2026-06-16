use std::path::PathBuf;
use workpot_core::RepoRecord;
use workpot_core::domain::repo::{BRANCH_UNBORN, SOURCE_MANUAL, SOURCE_SCAN};

#[test]
fn repo_record_source_constants_are_stable() {
    assert_eq!(SOURCE_MANUAL, "manual");
    assert_eq!(SOURCE_SCAN, "scan");
    assert_eq!(BRANCH_UNBORN, "unborn");
}

#[test]
fn repo_record_round_trips_field_assignment() {
    let record = RepoRecord {
        path: PathBuf::from("/tmp/workpot-repo"),
        name: "workpot-repo".to_string(),
        registered_at: 100,
        source: SOURCE_MANUAL.to_string(),
        git_common_dir: "/tmp/workpot-repo/.git".to_string(),
        branch: Some("main".to_string()),
        is_dirty: Some(false),
        ahead: Some(0),
        behind: Some(0),
        git_refreshed_at: Some(1_700_000_000),
        git_state_error: None,
        last_opened_at: None,
        pinned: false,
        pin_order: None,
        notes: Some("note".to_string()),
        tags: vec!["cli".to_string()],
        alias: Some("wp".to_string()),
        convert_block_reason: None,
    };

    assert_eq!(record.path, PathBuf::from("/tmp/workpot-repo"));
    assert_eq!(record.source, SOURCE_MANUAL);
    assert_eq!(record.tags, vec!["cli"]);
    assert_eq!(record.alias.as_deref(), Some("wp"));
    assert_eq!(record.notes.as_deref(), Some("note"));
}
