//! Golden-vector tests for `repo_priority` — ported from `src/lib/sort.test.ts`.
//!
//! Each test mirrors a `sectionSort` or `flatSectioned` case from the TypeScript
//! source so that Rust ordering matches the tray default view (CLI-03 parity).
//!
//! Decision notes:
//!   D-20: dirty beats recent — a dirty+recently-opened repo lands in Dirty, not Recent.
//!   D-22: padding floor — Recent is padded to `min_recent_count` from outside-window
//!         repos that have `last_opened_at IS NOT NULL`.

#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;
use workpot_core::domain::{Config, RepoRecord};
use workpot_core::services::repo_priority::{flat_tray_ordered_repos, section_sort};

const NOW: i64 = 1_000_000; // arbitrary fixed "now" in seconds

// ---------------------------------------------------------------------------
// Fixture builder — mirrors `repo()` in sort.test.ts
// ---------------------------------------------------------------------------

fn repo(name: &str) -> RepoRecord {
    RepoRecord {
        path: PathBuf::from(format!("/tmp/{name}")),
        name: name.to_string(),
        registered_at: 0,
        source: "manual".to_string(),
        git_common_dir: ".git".to_string(),
        branch: None,
        is_dirty: None,
        ahead: None,
        behind: None,
        git_refreshed_at: None,
        git_state_error: None,
        last_opened_at: None,
        pinned: false,
        pin_order: None,
        notes: None,
        tags: vec![],
        alias: None,
        convert_block_reason: None,
    }
}

fn pinned(name: &str, pin_order: i64) -> RepoRecord {
    RepoRecord {
        pinned: true,
        pin_order: Some(pin_order),
        ..repo(name)
    }
}

fn dirty(name: &str, last_opened_at: Option<i64>) -> RepoRecord {
    RepoRecord {
        is_dirty: Some(true),
        last_opened_at,
        ..repo(name)
    }
}

fn clean(name: &str, last_opened_at: Option<i64>) -> RepoRecord {
    RepoRecord {
        is_dirty: Some(false),
        last_opened_at,
        ..repo(name)
    }
}

fn config_default() -> Config {
    Config {
        max_recent_days: 14,
        min_recent_count: 3,
        ..Config::default()
    }
}

// ---------------------------------------------------------------------------
// section_sort — ported from sort.test.ts `describe("sectionSort", ...)`
// ---------------------------------------------------------------------------

/// Pinned repos appear only in the `pinned` section; other sections are unaffected.
/// Mirrors: "places pinned repos only in pinned section"
#[test]
fn pinned_repos_land_only_in_pinned_section() {
    let repos = vec![pinned("pin", 0), clean("other", Some(NOW))];
    let sections = section_sort(&repos, &config_default(), NOW);

    assert_eq!(
        sections
            .pinned
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>(),
        ["pin"]
    );
    assert!(sections.dirty.is_empty());
    assert!(sections.recent.iter().any(|r| r.name == "other"));
    assert!(sections.rest.is_empty());
}

/// Dirty repos land in dirty, not in recent — even when recently opened (D-20).
/// Mirrors: "places dirty repos in dirty, not recent"
#[test]
fn dirty_repo_lands_in_dirty_not_recent() {
    let repos = vec![dirty("dirty", Some(NOW - 10))];
    let sections = section_sort(&repos, &config_default(), NOW);

    assert_eq!(
        sections
            .dirty
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>(),
        ["dirty"]
    );
    assert!(
        sections.recent.is_empty(),
        "D-20: dirty must not appear in recent"
    );
}

/// Recent section is padded to `min_recent_count` from outside-window repos (D-22).
/// Mirrors: "pads recent to minRecentCount from outside window"
#[test]
fn recent_padded_to_min_recent_count_from_outside_window_d22() {
    let cfg = config_default(); // min_recent_count = 3, max_recent_days = 14
    let window_secs = 14 * 86_400_i64;
    let repos = vec![
        clean("a", Some(NOW - 100)),             // inside window
        clean("b", Some(NOW - 200)),             // inside window
        clean("c", Some(NOW - window_secs - 1)), // outside window → padding candidate
    ];
    let sections = section_sort(&repos, &cfg, NOW);

    assert_eq!(
        sections.recent.len(),
        3,
        "D-22: recent padded to min_recent_count"
    );
    let mut names: Vec<&str> = sections.recent.iter().map(|r| r.name.as_str()).collect();
    names.sort();
    assert_eq!(names, ["a", "b", "c"]);
    assert!(sections.rest.is_empty());
}

/// Repos with `last_opened_at = None` go to Rest (D-21) — never into Recent.
/// Mirrors: "sends never-opened repos to rest"
#[test]
fn never_opened_repos_land_in_rest_not_recent_d21() {
    let repos = vec![clean("never", None)];
    let cfg = Config {
        min_recent_count: 0,
        ..config_default()
    };
    let sections = section_sort(&repos, &cfg, NOW);

    assert!(sections.recent.is_empty());
    assert_eq!(
        sections
            .rest
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>(),
        ["never"]
    );
}

/// Never-opened repos must not be used as padding candidates (D-21).
/// Mirrors: "does not pad recent with never-opened repos (D-21)"
#[test]
fn padding_never_uses_never_opened_repos_d21() {
    let repos = vec![clean("a", None), clean("b", None), clean("c", None)];
    let sections = section_sort(&repos, &config_default(), NOW);

    assert!(
        sections.recent.is_empty(),
        "D-21: null last_opened_at cannot pad Recent"
    );
    let mut names: Vec<&str> = sections.rest.iter().map(|r| r.name.as_str()).collect();
    names.sort();
    assert_eq!(names, ["a", "b", "c"]);
}

/// Every repo appears exactly once across all four sections.
/// Mirrors: "partitions every repo exactly once"
#[test]
fn every_repo_appears_exactly_once() {
    let repos = vec![
        pinned("p", 0),
        dirty("d", None),
        clean("r", Some(NOW - 1)),
        clean("x", None),
    ];
    let sections = section_sort(&repos, &config_default(), NOW);

    let all: Vec<&RepoRecord> = sections
        .pinned
        .iter()
        .chain(&sections.dirty)
        .chain(&sections.recent)
        .chain(&sections.rest)
        .collect();

    assert_eq!(all.len(), repos.len());
    let paths: std::collections::HashSet<&PathBuf> = all.iter().map(|r| &r.path).collect();
    assert_eq!(paths.len(), repos.len(), "no duplicates across sections");
}

/// Pinned section is sorted by `pin_order` ascending; `None` → 999.
/// Mirrors: "sorts pinned by pin_order"
#[test]
fn pinned_sorted_by_pin_order_ascending() {
    let repos = vec![pinned("a", 2), pinned("b", 0)];
    let sections = section_sort(&repos, &config_default(), NOW);

    assert_eq!(
        sections
            .pinned
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>(),
        ["b", "a"],
        "pin_order 0 before 2"
    );
}

/// A `pin_order = None` is treated as 999 for sort purposes.
#[test]
fn pinned_none_pin_order_treated_as_999() {
    let repos = vec![
        RepoRecord {
            pinned: true,
            pin_order: None,
            ..repo("last")
        },
        pinned("first", 0),
    ];
    let sections = section_sort(&repos, &config_default(), NOW);
    assert_eq!(
        sections
            .pinned
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>(),
        ["first", "last"]
    );
}

/// Rest section is sorted alphabetically by name.
#[test]
fn rest_sorted_alphabetically() {
    let repos = vec![
        clean("zebra", None),
        clean("apple", None),
        clean("mango", None),
    ];
    let cfg = Config {
        min_recent_count: 0,
        ..config_default()
    };
    let sections = section_sort(&repos, &cfg, NOW);

    assert_eq!(
        sections
            .rest
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>(),
        ["apple", "mango", "zebra"]
    );
}

// ---------------------------------------------------------------------------
// flat_tray_ordered_repos — verifies concat order matches flatSectioned(sectionSort(...))
// ---------------------------------------------------------------------------

/// Flat output order: Pinned → Dirty → Recent → Rest.
#[test]
fn flat_output_follows_pinned_dirty_recent_rest_order() {
    let repos = vec![
        clean("rest-z", None),
        dirty("dirty-a", Some(NOW - 5)),
        pinned("pin-b", 1),
        clean("recent-c", Some(NOW - 100)),
        pinned("pin-a", 0),
    ];
    let flat = flat_tray_ordered_repos(&repos, &config_default(), NOW);
    let names: Vec<&str> = flat.iter().map(|r| r.name.as_str()).collect();

    // pinned first (by pin_order)
    assert_eq!(names[0], "pin-a");
    assert_eq!(names[1], "pin-b");
    // dirty next
    assert_eq!(names[2], "dirty-a");
    // recent then rest (exact positions depend on padding, but dirty-a is not in either)
    let dirty_pos = names.iter().position(|n| *n == "dirty-a").unwrap();
    let rest_pos = names.iter().position(|n| *n == "rest-z").unwrap();
    assert!(
        dirty_pos < rest_pos,
        "dirty must precede rest in flat output"
    );
}

/// D-20: a dirty repo with recent last_opened_at appears in the dirty tier, not recent tier.
#[test]
fn dirty_beats_recent_in_flat_output_d20() {
    let repos = vec![
        dirty("dirty-recent", Some(NOW - 10)), // would qualify for Recent if not dirty
        clean("clean-recent", Some(NOW - 50)),
    ];
    let flat = flat_tray_ordered_repos(&repos, &config_default(), NOW);
    let dirty_pos = flat.iter().position(|r| r.name == "dirty-recent").unwrap();
    let clean_pos = flat.iter().position(|r| r.name == "clean-recent").unwrap();
    assert!(
        dirty_pos < clean_pos,
        "D-20: dirty must precede clean-recent"
    );
}
