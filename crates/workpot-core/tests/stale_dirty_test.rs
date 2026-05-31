//! Stale-dirty detection policy — TDD plan 06.2-02 (16 behavior cases).

#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;
use workpot_core::RepoRecord;
use workpot_core::domain::Config;
use workpot_core::services::stale_dirty::has_stale_dirty;

const SECS_PER_DAY: i64 = 86_400;

fn make_repo(is_dirty: Option<bool>, last_opened_at: Option<i64>) -> RepoRecord {
    RepoRecord {
        path: PathBuf::from("/tmp/stale-dirty-test"),
        name: "stale-dirty-test".to_string(),
        registered_at: 0,
        source: "manual".to_string(),
        git_common_dir: ".git".to_string(),
        branch: Some("main".to_string()),
        is_dirty,
        ahead: None,
        behind: None,
        git_refreshed_at: None,
        git_state_error: None,
        last_opened_at,
        pinned: false,
        pin_order: None,
        notes: None,
        tags: vec![],
        alias: None,
    }
}

fn make_clean_repo(last_opened_at: Option<i64>) -> RepoRecord {
    make_repo(Some(false), last_opened_at)
}

fn make_dirty_repo(last_opened_at: Option<i64>) -> RepoRecord {
    make_repo(Some(true), last_opened_at)
}

// --- has_stale_dirty (cases 1–11) ---

#[test]
fn empty_repo_list_is_not_stale_dirty() {
    assert!(!has_stale_dirty(&[], 7, 1_000_000));
}

#[test]
fn clean_repo_recently_opened_is_not_stale_dirty() {
    let now = 1_000_000_i64;
    let repos = vec![make_clean_repo(Some(now))];
    assert!(!has_stale_dirty(&repos, 7, now));
}

#[test]
fn dirty_repo_opened_now_is_not_stale_dirty() {
    let now = 1_000_000_i64;
    let repos = vec![make_dirty_repo(Some(now))];
    assert!(!has_stale_dirty(&repos, 7, now));
}

#[test]
fn dirty_repo_at_exact_threshold_is_stale_dirty() {
    let stale_dirty_days = 7_u32;
    let threshold = stale_dirty_days as i64 * SECS_PER_DAY;
    let now = 2_000_000_i64;
    let repos = vec![make_dirty_repo(Some(now - threshold))];
    assert!(has_stale_dirty(&repos, stale_dirty_days, now));
}

#[test]
fn dirty_repo_one_second_under_threshold_is_not_stale_dirty() {
    let stale_dirty_days = 7_u32;
    let threshold = stale_dirty_days as i64 * SECS_PER_DAY;
    let now = 2_000_000_i64;
    let repos = vec![make_dirty_repo(Some(now - threshold + 1))];
    assert!(!has_stale_dirty(&repos, stale_dirty_days, now));
}

#[test]
fn dirty_never_opened_is_stale_dirty_immediately() {
    let repos = vec![make_dirty_repo(None)];
    assert!(has_stale_dirty(&repos, 7, 1_000_000));
}

#[test]
fn bare_repo_never_opened_is_not_stale_dirty() {
    let repos = vec![make_repo(None, None)];
    assert!(!has_stale_dirty(&repos, 7, 1_000_000));
}

#[test]
fn multiple_repos_one_stale_dirty_returns_true() {
    let stale_dirty_days = 7_u32;
    let threshold = stale_dirty_days as i64 * SECS_PER_DAY;
    let now = 3_000_000_i64;
    let repos = vec![
        make_clean_repo(Some(now)),
        make_dirty_repo(Some(now - threshold)),
    ];
    assert!(has_stale_dirty(&repos, stale_dirty_days, now));
}

#[test]
fn multiple_repos_all_clean_returns_false() {
    let now = 3_000_000_i64;
    let repos = vec![
        make_clean_repo(Some(now)),
        make_clean_repo(Some(now - SECS_PER_DAY * 30)),
    ];
    assert!(!has_stale_dirty(&repos, 7, now));
}

#[test]
fn one_day_threshold_one_second_short_is_not_stale_dirty() {
    let now = 5_000_000_i64;
    let repos = vec![make_dirty_repo(Some(now - SECS_PER_DAY + 1))];
    assert!(!has_stale_dirty(&repos, 1, now));
}

#[test]
fn one_day_threshold_exactly_one_day_is_stale_dirty() {
    let now = 5_000_000_i64;
    let repos = vec![make_dirty_repo(Some(now - SECS_PER_DAY))];
    assert!(has_stale_dirty(&repos, 1, now));
}

// --- Config.stale_dirty_days (cases 12–16) ---

#[test]
fn config_validate_rejects_stale_dirty_days_zero() {
    let config = Config {
        stale_dirty_days: 0,
        ..Default::default()
    };
    assert_eq!(
        config.validate().unwrap_err(),
        "stale_dirty_days 0 must be between 1 and 365"
    );
}

#[test]
fn config_validate_rejects_stale_dirty_days_over_365() {
    let config = Config {
        stale_dirty_days: 366,
        ..Default::default()
    };
    assert_eq!(
        config.validate().unwrap_err(),
        "stale_dirty_days 366 must be between 1 and 365"
    );
}

#[test]
fn config_validate_accepts_stale_dirty_days_seven() {
    let config = Config {
        stale_dirty_days: 7,
        ..Default::default()
    };
    config.validate().expect("7 is valid");
}

#[test]
fn config_default_stale_dirty_days_is_seven() {
    assert_eq!(Config::default().stale_dirty_days, 7);
}

#[test]
fn config_deserializes_missing_stale_dirty_days_to_seven() {
    let config: Config = toml::from_str("watch_roots = []\nexcludes = []\n").expect("parse");
    assert_eq!(config.stale_dirty_days, 7);
}

// --- has_stale_dirty_dto bridge (plan 06.2-09) ---

/// Replicates `has_stale_dirty_dto` in `src-tauri/src/commands.rs` for parity testing.
fn dto_equivalent(
    is_dirty: Option<bool>,
    last_opened_at: Option<i64>,
    stale_dirty_days: u32,
    now_secs: i64,
) -> bool {
    let threshold_secs = stale_dirty_days as i64 * SECS_PER_DAY;
    is_dirty == Some(true)
        && {
            let age = match last_opened_at {
                Some(t) => now_secs - t,
                None => i64::MAX,
            };
            age >= threshold_secs
        }
}

/// Validates that `has_stale_dirty_dto` in `src-tauri/src/commands.rs` follows the same
/// policy as `has_stale_dirty`. If the two implementations diverge, update both and keep
/// this test passing.
#[test]
fn has_stale_dirty_dto_matches_has_stale_dirty() {
    let stale_dirty_days = 7_u32;
    let threshold = stale_dirty_days as i64 * SECS_PER_DAY;
    let now = 2_000_000_i64;

    let cases: Vec<(Option<bool>, Option<i64>)> = vec![
        (Some(true), None),
        (Some(true), Some(now - threshold)),
        (Some(true), Some(now - threshold + 1)),
        (Some(false), Some(now)),
        (None, None),
    ];

    for (is_dirty, last_opened_at) in cases {
        let repo = make_repo(is_dirty, last_opened_at);
        let core = has_stale_dirty(&[repo], stale_dirty_days, now);
        let dto = dto_equivalent(is_dirty, last_opened_at, stale_dirty_days, now);
        assert_eq!(
            core, dto,
            "policy mismatch for is_dirty={is_dirty:?} last_opened_at={last_opened_at:?}"
        );
    }
}
