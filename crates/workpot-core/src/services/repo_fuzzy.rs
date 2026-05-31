//! Fuzzy filter for repos — direct port of `src/lib/fuzzy.ts`.
//!
//! **D-06:** No third-party fuzzy crates (no nucleo, fuzzy-matcher, etc.).
//! **D-07:** No `#tag` token parsing; tags are scored as plain text fields.
//! **T-06-02-01:** Query longer than 256 chars returns score 0 immediately (DoS guard).

use crate::domain::RepoRecord;

const MAX_QUERY_LEN: usize = 256;

/// Returns `true` if every char of `query` appears in `field` in order (case-sensitive on
/// pre-lowercased inputs). Mirrors `subsequenceMatch` in `fuzzy.ts`.
fn subsequence_match(query: &str, field: &str) -> bool {
    let mut qi = query.chars();
    let mut current = match qi.next() {
        Some(c) => c,
        None => return true, // empty query trivially matches
    };
    for fc in field.chars() {
        if fc == current {
            match qi.next() {
                Some(next) => current = next,
                None => return true, // consumed all query chars
            }
        }
    }
    false
}

/// Score a single field against the query. Mirrors `scoreField` in `fuzzy.ts`.
///
/// Both `query` and `field` must already be lowercased.
fn score_field(query: &str, field: &str, name_bonus: bool) -> i32 {
    // A field matches if it contains query as a substring OR as a subsequence.
    let is_substring = field.contains(query);
    let is_subseq = subsequence_match(query, field);
    if !is_substring && !is_subseq {
        return 0;
    }

    let mut score: i32 = 10;

    if field.starts_with(query) {
        score += 20;
    } else if is_subseq {
        score += 8;
    }

    if name_bonus {
        let mut run: i32 = 0;
        let q_chars: Vec<char> = query.chars().collect();
        let f_chars: Vec<char> = field.chars().collect();
        let limit = q_chars.len().min(f_chars.len());
        for i in 0..limit {
            if f_chars[i] == q_chars[i] {
                run += 1;
            } else {
                break;
            }
        }
        score += run * 2;
    }

    score
}

/// Compute the fuzzy relevance score for `repo` against `query`.
///
/// Mirrors `fuzzyScore` in `fuzzy.ts`:
/// - Trims and lowercases `query`.
/// - Empty/whitespace query → 1 (matches everything).
/// - Query longer than 256 chars → 0 (no match; DoS guard).
/// - Returns the maximum score across name (with name bonus), alias (with name bonus), path,
///   branch, notes, and each tag.
pub fn fuzzy_score(query: &str, repo: &RepoRecord) -> i32 {
    let q = query.trim().to_lowercase();

    if q.is_empty() {
        return 1;
    }
    if q.chars().count() > MAX_QUERY_LEN {
        return 0;
    }

    let name_score = score_field(&q, &repo.name.to_lowercase(), true);
    let alias_score = score_field(
        &q,
        &repo.alias.as_deref().unwrap_or("").to_lowercase(),
        true,
    );
    let path_score = score_field(&q, &repo.path.to_string_lossy().to_lowercase(), false);
    let branch_score = score_field(
        &q,
        &repo.branch.as_deref().unwrap_or("").to_lowercase(),
        false,
    );
    let notes_score = score_field(
        &q,
        &repo.notes.as_deref().unwrap_or("").to_lowercase(),
        false,
    );
    let tag_scores = repo
        .tags
        .iter()
        .map(|t| score_field(&q, &t.to_lowercase(), false));

    let base_max = name_score
        .max(alias_score)
        .max(path_score)
        .max(branch_score)
        .max(notes_score);

    tag_scores.fold(base_max, |acc, s| acc.max(s))
}

/// Returns `true` if `repo` matches `query` (score > 0). Mirrors `fuzzyMatch` in `fuzzy.ts`.
pub fn fuzzy_match(query: &str, repo: &RepoRecord) -> bool {
    fuzzy_score(query, repo) > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_repo(
        name: &str,
        path: &str,
        branch: Option<&str>,
        notes: Option<&str>,
        tags: Vec<&str>,
    ) -> RepoRecord {
        RepoRecord {
            path: PathBuf::from(path),
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

    #[test]
    fn empty_query_matches_all() {
        let r = make_repo("workpot", "/Users/me/c/workpot", Some("main"), None, vec![]);
        assert_eq!(fuzzy_score("", &r), 1);
        assert!(fuzzy_match("", &r));
    }

    #[test]
    fn whitespace_query_matches_all() {
        let r = make_repo("workpot", "/Users/me/c/workpot", Some("main"), None, vec![]);
        assert_eq!(fuzzy_score("   ", &r), 1);
        assert!(fuzzy_match("   ", &r));
    }

    #[test]
    fn overlong_query_no_match() {
        let r = make_repo("workpot", "/Users/me/c/workpot", Some("main"), None, vec![]);
        let long_query = "x".repeat(257);
        assert_eq!(fuzzy_score(&long_query, &r), 0);
        assert!(!fuzzy_match(&long_query, &r));
    }

    #[test]
    fn name_subsequence_match() {
        let r = make_repo("workpot", "/tmp/x", Some("main"), None, vec![]);
        assert!(fuzzy_match("wp", &r));
        assert!(fuzzy_score("wp", &r) > 0);
    }

    #[test]
    fn branch_match() {
        let r = make_repo("other", "/Users/me/c/other", Some("main"), None, vec![]);
        assert!(fuzzy_match("main", &r));
    }

    #[test]
    fn path_match() {
        let r = make_repo("other", "/Users/me/c/workpot", Some("main"), None, vec![]);
        assert!(fuzzy_match("workpot", &r));
    }

    #[test]
    fn notes_match() {
        let r = make_repo(
            "x",
            "/Users/me/c/x",
            Some("main"),
            Some("deployment pipeline"),
            vec![],
        );
        assert!(fuzzy_match("pipeline", &r));
    }

    #[test]
    fn tag_match() {
        let r = make_repo("x", "/Users/me/c/x", Some("main"), None, vec!["backend"]);
        assert!(fuzzy_match("backend", &r));
    }

    #[test]
    fn no_match_unrelated_query() {
        let r = make_repo("alpha", "/Users/me/c/alpha", Some("main"), None, vec![]);
        assert!(!fuzzy_match("zzz", &r));
    }

    #[test]
    fn none_branch_does_not_panic() {
        let r = make_repo("x", "/tmp/x", None, None, vec![]);
        assert_eq!(fuzzy_score("main", &r), 0);
    }

    #[test]
    fn none_notes_does_not_panic() {
        let r = make_repo("x", "/tmp/x", Some("main"), None, vec![]);
        assert_eq!(fuzzy_score("pipeline", &r), 0);
    }
}
