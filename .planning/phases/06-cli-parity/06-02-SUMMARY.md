---
phase: 06-cli-parity
plan: "02"
subsystem: workpot-core/fuzzy
tags: [fuzzy, search, parity, cli, golden-vectors]
dependency_graph:
  requires: []
  provides: [fuzzy_match, fuzzy_score]
  affects: [workpot-cli/search]
tech_stack:
  added: []
  patterns: [direct-port, golden-vectors, subsequence-match]
key_files:
  created:
    - crates/workpot-core/src/services/repo_fuzzy.rs
    - crates/workpot-core/tests/repo_fuzzy_test.rs
  modified:
    - crates/workpot-core/src/services/mod.rs
decisions:
  - "D-06: Direct port of fuzzy.ts algorithm; no nucleo/fuzzy-matcher crates added"
  - "D-07: No #tag token parsing; tags scored as plain text fields"
  - "T-06-02-01: MAX_QUERY_LEN=256 guards applied as score=0 short-circuit"
metrics:
  duration: "~6 minutes"
  completed: "2026-05-31"
  tasks: 2
  files: 3
---

# Phase 06 Plan 02: repo_fuzzy Module Summary

Port `src/lib/fuzzy.ts` fuzzy filter into `workpot-core` as `services/repo_fuzzy.rs`, with golden-vector tests that prove CLI-03 parity (same query + same repo fixture → same match boolean as TS).

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | repo_fuzzy module | 4fa85bf | crates/workpot-core/src/services/repo_fuzzy.rs, services/mod.rs |
| 2 | repo_fuzzy_test.rs + golden vectors | ac3845a | crates/workpot-core/tests/repo_fuzzy_test.rs |

## What Was Built

**Task 1 — `repo_fuzzy.rs`**

Direct port of `src/lib/fuzzy.ts` into Rust:
- `subsequence_match(query, field) -> bool` — same char-by-char walk as TS `subsequenceMatch`
- `score_field(query, field, name_bonus) -> i32` — base 10 + prefix bonus (+20) + subsequence-only bonus (+8) + name run bonus (run×2); both inputs pre-lowercased
- `fuzzy_score(query: &str, repo: &RepoRecord) -> i32` — trims and lowercases query; returns 1 for empty/whitespace; returns 0 for query > 256 chars; returns max score across name (name_bonus=true), path, branch, notes, and each tag
- `fuzzy_match(query, repo) -> bool` — score > 0

None-safe: `repo.branch.as_deref().unwrap_or("")` and same for notes; None fields score 0, never panic.

**Task 2 — `repo_fuzzy_test.rs`**

11 named tests mapping one-to-one to every `it(...)` block in `fuzzy.test.ts`:
- `matches_wp_against_workpot_name`, `matches_branch_main`, `empty_query_returns_all_repos`
- `rejects_query_over_256_chars`, `matches_path_segment`, `trims_query_whitespace`
- `returns_false_when_no_field_matches`, `name_prefix_scores_higher_than_path_only_subsequence`
- `matches_notes_text`, `matches_tag_text`, `does_not_match_unrelated_query_on_note_only_repo`

`fuzzy_golden_vectors` module with table-driven proof (27 rows × `(query, RepoRecord, expected_match)`) covering: name subsequence, branch, path, notes, tag, empty query, whitespace, overlong query (257 chars), no-match, case-insensitive, None fields, and score-invariant (`score > 0 iff match`).

## Verification Results

```
cargo test -p workpot-core --test repo_fuzzy_test  → 13 tests, 0 failed, 0 ignored
cargo test -p workpot-core fuzzy_golden             → 2 tests, 0 failed
cargo test -p workpot-core repo_fuzzy               → 11 unit tests, 0 failed
cargo test -p workpot-core                          → full suite green (no regressions)
```

## Deviations from Plan

None — plan executed exactly as written. No third-party crates added (D-06 respected). No `#tag` token parsing (D-07 respected). DoS guard at MAX_QUERY_LEN=256 implemented (T-06-02-01).

## Known Stubs

None. `fuzzy_score` and `fuzzy_match` are fully implemented and wired to `RepoRecord` fields.

## Threat Flags

T-06-02-01 mitigated: query > 256 chars returns score 0 in `fuzzy_score` before any field scoring.

## Self-Check: PASSED

- [x] `crates/workpot-core/src/services/repo_fuzzy.rs` — exists, 203 lines
- [x] `crates/workpot-core/tests/repo_fuzzy_test.rs` — exists, 292 lines
- [x] `crates/workpot-core/src/services/mod.rs` — `pub mod repo_fuzzy` added
- [x] commit 4fa85bf — `feat(06-02): add repo_fuzzy module`
- [x] commit ac3845a — `test(06-02): add repo_fuzzy_test.rs`
- [x] Full test suite green
