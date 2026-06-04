//! Fuzzy golden-vector tests — every case from `src/lib/fuzzy.test.ts` ported to Rust.
//!
//! These are the automated proof for ROADMAP SC #2 / CLI-03 fuzzy parity (tray TS wiring
//! is out of scope this phase). All test names correspond to the equivalent `it(...)` block
//! in `fuzzy.test.ts`.

#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;
use workpot_core::RepoRecord;
use workpot_core::services::repo_fuzzy::{fuzzy_match, fuzzy_score};

// ---------------------------------------------------------------------------
// Fixture builder — mirrors the `repo(...)` helper in fuzzy.test.ts
// ---------------------------------------------------------------------------

fn repo(
    name: &str,
    path: Option<&str>,
    branch: Option<&str>,
    notes: Option<&str>,
    tags: Vec<&str>,
) -> RepoRecord {
    let default_path = format!("/Users/me/c/{name}");
    RepoRecord {
        path: PathBuf::from(path.unwrap_or(&default_path)),
        name: name.to_string(),
        registered_at: 0,
        source: "manual".to_string(),
        git_common_dir: ".git".to_string(),
        branch: branch.map(|s| s.to_string()),
        is_dirty: None,
        ahead: None,
        behind: None,
        git_refreshed_at: None,
        git_state_error: None,
        last_opened_at: None,
        pinned: false,
        pin_order: None,
        notes: notes.map(|s| s.to_string()),
        tags: tags.into_iter().map(|s| s.to_string()).collect(),
        alias: None,
    }
}

/// Convenience: repo with just a name (branch = "main", others default).
fn named(name: &str) -> RepoRecord {
    repo(name, None, Some("main"), None, vec![])
}

/// Repo with a user alias (06.2-03 dual-field fuzzy).
fn aliased(name: &str, alias: &str) -> RepoRecord {
    let mut r = named(name);
    r.alias = Some(alias.to_string());
    r
}

// ---------------------------------------------------------------------------
// Ported test cases — one-to-one with `fuzzy.test.ts` `it(...)` blocks
// ---------------------------------------------------------------------------

/// fuzzy.test.ts: matches "wp" against workpot name
#[test]
fn matches_wp_against_workpot_name() {
    let r = named("workpot");
    assert!(fuzzy_match("wp", &r));
    assert!(fuzzy_score("wp", &r) > 0);
}

/// fuzzy.test.ts: matches branch "main"
#[test]
fn matches_branch_main() {
    let r = repo("other", None, Some("main"), None, vec![]);
    assert!(fuzzy_match("main", &r));
}

/// fuzzy.test.ts: returns all repos for empty query via score
#[test]
fn empty_query_returns_all_repos() {
    let r = named("x");
    assert!(fuzzy_match("", &r));
    assert_eq!(fuzzy_score("", &r), 1);
}

/// fuzzy.test.ts: rejects query over 256 chars
#[test]
fn rejects_query_over_256_chars() {
    let r = named("workpot");
    let long_query = "x".repeat(257);
    assert!(!fuzzy_match(&long_query, &r));
    assert_eq!(fuzzy_score(&long_query, &r), 0);
}

/// fuzzy.test.ts: matches path segment
#[test]
fn matches_path_segment() {
    let r = repo(
        "other",
        Some("/Users/me/c/workpot"),
        Some("main"),
        None,
        vec![],
    );
    assert!(fuzzy_match("workpot", &r));
}

/// fuzzy.test.ts: trims query whitespace
#[test]
fn trims_query_whitespace() {
    let r = named("workpot");
    assert!(fuzzy_match("  wp  ", &r));
}

/// fuzzy.test.ts: returns false when no field matches
#[test]
fn returns_false_when_no_field_matches() {
    let r = repo("alpha", None, Some("main"), None, vec![]);
    assert!(!fuzzy_match("zzz", &r));
}

/// fuzzy.test.ts: scores name prefix higher than path-only subsequence
#[test]
fn name_prefix_scores_higher_than_path_only_subsequence() {
    let by_name = repo("workpot", Some("/tmp/x"), Some("main"), None, vec![]);
    let by_path = repo("x", Some("/tmp/workpot-extra"), Some("main"), None, vec![]);
    assert!(fuzzy_score("work", &by_name) > fuzzy_score("work", &by_path));
}

/// fuzzy.test.ts: matches notes text
#[test]
fn matches_notes_text() {
    let r = repo("x", None, Some("main"), Some("deployment pipeline"), vec![]);
    assert!(fuzzy_match("pipeline", &r));
}

/// fuzzy.test.ts: matches tag text
#[test]
fn matches_tag_text() {
    let r = repo("x", None, Some("main"), None, vec!["backend"]);
    assert!(fuzzy_match("backend", &r));
}

/// fuzzy.test.ts: does not match unrelated query on note-only repo
#[test]
fn does_not_match_unrelated_query_on_note_only_repo() {
    // branch: null in TS test → None here
    let r = repo("x", None, None, Some("deployment pipeline"), vec![]);
    assert!(!fuzzy_match("zzz", &r));
    assert_eq!(fuzzy_score("zzz", &r), 0);
}

// ---------------------------------------------------------------------------
// fuzzy_golden_vectors — table-driven exhaustive equivalence proof (CLI-03 / SC#2)
//
// Each row: (query, RepoRecord, expected_match: bool)
// Assertion: fuzzy_match(query, &repo) == expected_match
//           AND fuzzy_score > 0 iff expected_match
// ---------------------------------------------------------------------------

#[cfg(test)]
mod fuzzy_golden_vectors {
    use super::*;

    struct GoldenRow {
        query: &'static str,
        repo: RepoRecord,
        expected_match: bool,
    }

    fn gold(query: &'static str, r: RepoRecord, expected_match: bool) -> GoldenRow {
        GoldenRow {
            query,
            repo: r,
            expected_match,
        }
    }

    fn table() -> Vec<GoldenRow> {
        vec![
            // --- name subsequence match ---
            gold("wp", named("workpot"), true),
            gold("wrkpt", named("workpot"), true),
            // --- name: no match ---
            gold("zzz", named("workpot"), false),
            // --- branch match ---
            gold(
                "main",
                repo("other", None, Some("main"), None, vec![]),
                true,
            ),
            gold(
                "feat",
                repo("other", None, Some("feat/login"), None, vec![]),
                true,
            ),
            // --- branch no match ---
            gold(
                "zzz",
                repo("other", None, Some("main"), None, vec![]),
                false,
            ),
            // --- empty query → always match ---
            gold("", named("x"), true),
            gold("", named("workpot"), true),
            // --- whitespace-only query → match all ---
            gold("  ", named("workpot"), true),
            gold("\t", named("x"), true),
            // --- overlong query → no match ---
            gold(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\
                 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\
                 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\
                 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\
                 aaaaaaaaaaaaaaaaaaaaa", // 257 chars
                named("workpot"),
                false,
            ),
            // --- path match ---
            gold(
                "workpot",
                repo(
                    "other",
                    Some("/Users/me/c/workpot"),
                    Some("main"),
                    None,
                    vec![],
                ),
                true,
            ),
            gold(
                "zzz",
                repo(
                    "other",
                    Some("/Users/me/c/workpot"),
                    Some("main"),
                    None,
                    vec![],
                ),
                false,
            ),
            // --- notes match ---
            gold(
                "pipeline",
                repo("x", None, Some("main"), Some("deployment pipeline"), vec![]),
                true,
            ),
            gold(
                "deploy",
                repo("x", None, Some("main"), Some("deployment pipeline"), vec![]),
                true,
            ),
            gold(
                "zzz",
                repo("x", None, Some("main"), Some("deployment pipeline"), vec![]),
                false,
            ),
            // --- tag match ---
            gold(
                "backend",
                repo("x", None, Some("main"), None, vec!["backend"]),
                true,
            ),
            gold(
                "end",
                repo("x", None, Some("main"), None, vec!["backend"]),
                true,
            ),
            gold(
                "zzz",
                repo("x", None, Some("main"), None, vec!["backend"]),
                false,
            ),
            // --- None branch (no panic) ---
            gold("main", repo("x", None, None, None, vec![]), false),
            // --- None notes (no panic) ---
            gold(
                "pipeline",
                repo("x", None, Some("main"), None, vec![]),
                false,
            ),
            // --- case insensitivity ---
            gold("WP", named("workpot"), true),
            gold(
                "MAIN",
                repo("other", None, Some("main"), None, vec![]),
                true,
            ),
            gold(
                "BACKEND",
                repo("x", None, Some("main"), None, vec!["backend"]),
                true,
            ),
            // --- name prefix bonus exists (score check only) ---
            // (match correctness — score comparison tested separately below)
            gold("work", named("workpot"), true),
            gold(
                "work",
                repo("x", Some("/tmp/workpot-extra"), Some("main"), None, vec![]),
                true,
            ),
        ]
    }

    #[test]
    fn fuzzy_golden_all_rows() {
        let rows = table();
        for (i, row) in rows.iter().enumerate() {
            let got_match = fuzzy_match(row.query, &row.repo);
            let got_score = fuzzy_score(row.query, &row.repo);
            assert_eq!(
                got_match, row.expected_match,
                "Row {i}: query={:?} name={:?} expected_match={}; got fuzzy_match={}",
                row.query, row.repo.name, row.expected_match, got_match
            );
            // Score invariant: score > 0 iff match
            if row.expected_match {
                assert!(
                    got_score > 0,
                    "Row {i}: query={:?} name={:?} expected match but score={}",
                    row.query,
                    row.repo.name,
                    got_score
                );
            } else {
                assert_eq!(
                    got_score, 0,
                    "Row {i}: query={:?} name={:?} expected no match but score={}",
                    row.query, row.repo.name, got_score
                );
            }
        }
    }

    /// Verify the name prefix bonus: a repo whose NAME starts with the query
    /// scores higher than a repo where the query only appears in the path.
    #[test]
    fn fuzzy_golden_name_prefix_beats_path_subsequence() {
        let by_name = repo("workpot", Some("/tmp/x"), Some("main"), None, vec![]);
        let by_path = repo("x", Some("/tmp/workpot-extra"), Some("main"), None, vec![]);
        assert!(
            fuzzy_score("work", &by_name) > fuzzy_score("work", &by_path),
            "name prefix score ({}) should exceed path-only score ({})",
            fuzzy_score("work", &by_name),
            fuzzy_score("work", &by_path)
        );
    }
}

// ---------------------------------------------------------------------------
// Alias dual-field fuzzy (06.2-03)
// ---------------------------------------------------------------------------

#[test]
fn alias_query_matches_alias_not_name() {
    let r = aliased("workpot", "wp");
    assert!(fuzzy_score("wp", &r) > 0);
}

#[test]
fn alias_set_name_still_matches() {
    let r = aliased("workpot", "wp");
    assert!(fuzzy_score("workpot", &r) > 0);
}

#[test]
fn alias_and_name_both_match_short_query() {
    let by_alias = aliased("some-long-name", "sln");
    let by_name = named("sln");
    assert!(fuzzy_score("sln", &by_alias) > 0);
    assert!(fuzzy_score("sln", &by_name) > 0);
}

#[test]
fn alias_primary_match_full_alias_string() {
    let r = aliased("alpha", "my-project");
    assert!(fuzzy_score("my-project", &r) > 0);
}

#[test]
fn alias_none_no_regression_vs_unaliased() {
    let baseline = named("workpot");
    let with_none = repo("workpot", None, Some("main"), None, vec![]);
    assert_eq!(
        fuzzy_score("workpot", &baseline),
        fuzzy_score("workpot", &with_none)
    );
    assert!(fuzzy_score("workpot", &with_none) > 0);
}

#[test]
fn alias_neither_name_nor_alias_matches() {
    let r = aliased("alpha", "beta");
    assert_eq!(fuzzy_score("zzz", &r), 0);
    assert!(!fuzzy_match("zzz", &r));
}

#[test]
fn alias_set_name_subsequence_still_scores() {
    let r = aliased("alpha", "beta");
    assert!(fuzzy_score("lph", &r) > 0);
}

#[test]
fn alias_substring_match_scores() {
    let r = aliased("alpha", "beta");
    assert!(fuzzy_score("bet", &r) > 0);
}

#[test]
fn alias_prefix_bonus_beats_path_only() {
    let with_alias = aliased("x", "myproject");
    let path_only = repo("x", Some("/myp/x"), Some("main"), None, vec![]);
    assert!(
        fuzzy_score("myp", &with_alias) > fuzzy_score("myp", &path_only),
        "alias prefix ({}) should beat path-only ({})",
        fuzzy_score("myp", &with_alias),
        fuzzy_score("myp", &path_only)
    );
}

#[test]
fn alias_empty_query_still_matches_all() {
    let r = aliased("anything", "shortcut");
    assert_eq!(fuzzy_score("", &r), 1);
    assert!(fuzzy_match("", &r));
}
