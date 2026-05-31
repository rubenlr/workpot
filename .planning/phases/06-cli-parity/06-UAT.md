---
status: complete
phase: 06-cli-parity
source: 06-01-SUMMARY.md, 06-02-SUMMARY.md, 06-03-SUMMARY.md, 06-04-SUMMARY.md, 06-05-SUMMARY.md
started: 2026-05-31T20:45:00Z
updated: 2026-05-31T20:45:00Z
mode: auto
---

## Current Test

[testing complete]

## Tests

### 1. List indexed repos from terminal
expected: Run `workpot list` on an indexed watch root. Each line shows a priority emoji (📌/🟡/🔥/⬜), shortened parent dir, repo name, branch, and optional tags. Repos appear in Pinned > Dirty > Recent > Rest order with no section headers.
result: pass
verified_by: `list_registered_repo_shows_icon_and_name`, `list_empty_index_exits_zero`; live smoke in isolated HOME (2 repos, ⬜ rows, alpha before beta alphabetically in Rest)

### 2. Search filters repos like tray fuzzy filter
expected: `workpot search alpha` prints only repos matching the query, same row format and priority order as list. `workpot search ""` output matches `workpot list` for the same index.
result: pass
verified_by: `search_filters_by_fuzzy_query`, `search_empty_query_equals_list`, `fuzzy_golden_vectors::fuzzy_golden_all_rows` (27 rows); live smoke shows alpha only for `search alpha`

### 3. Open repo by name or path
expected: `workpot open alpha` prints `opening: <full canonical path>` and exits 0. Unknown id exits 1 with `repo not found`. Ambiguous name exits 1 with numbered paths.
result: pass
verified_by: `open_exits_zero_and_prints_opening_prefix`, `open_resolves_by_name_and_prints_full_path`, `open_not_found_exits_one_with_message`, `open_ambiguous_exits_one_with_numbered_paths`; live smoke prints `opening: .../alpha`

### 4. CLI ordering matches tray algorithm (automated parity)
expected: Rust `repo_priority` tests produce the same flat order as TypeScript `sort.test.ts` golden cases (Pinned > Dirty > Recent > Rest, D-20 dirty beats recent).
result: pass
verified_by: `cargo test -p workpot-core --test repo_priority_test` — 11/11 passed

### 5. CLI fuzzy matches tray algorithm (automated parity)
expected: Rust `fuzzy_match` agrees with `fuzzy.test.ts` golden vectors for the same query + repo fixtures.
result: pass
verified_by: `cargo test -p workpot-core --test repo_fuzzy_test` — 13/13 passed including `fuzzy_golden_all_rows`

### 6. Tray vs CLI visual spot-check (manual)
expected: With the same indexed repos, tray default list top-to-bottom matches `workpot list` order; tray filter matches `workpot search` for the same query (no `#tag` syntax).
result: skipped
reason: Phase contract defers tray wiring to a follow-up; CLI-03 ordering/fuzzy parity proven via ported golden-vector tests (06-VERIFICATION.md). Optional manual spot-check not run in --auto.

## Summary

total: 6
passed: 5
issues: 0
pending: 0
skipped: 1
blocked: 0

## Gaps

[none]
