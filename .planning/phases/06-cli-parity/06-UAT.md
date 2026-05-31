---
status: complete
phase: 06-cli-parity
source: 06-01-SUMMARY.md, 06-02-SUMMARY.md, 06-03-SUMMARY.md, 06-04-SUMMARY.md, 06-05-SUMMARY.md
started: 2026-05-31T21:00:00Z
updated: 2026-05-31T17:29:00Z
---

## Current Test

[testing complete]

## Tests

### 1. List indexed repos from terminal
expected: Run `workpot list` on an indexed watch root. Each line shows a priority emoji (📌/🟡/🔥/⬜), shortened parent dir, repo name, branch, and optional tags. Repos appear in Pinned > Dirty > Recent > Rest order with no section headers.
result: pass

### 2. Search filters repos like tray fuzzy filter
expected: `workpot search alpha` prints only repos matching the query, same row format and priority order as list. `workpot search ""` output matches `workpot list` for the same index.
result: pass

### 3. Open repo by name or path
expected: `workpot open alpha` prints `opening: <full canonical path>` and exits 0. Unknown id exits 1 with `repo not found`. Ambiguous name exits 1 with numbered paths.
result: pass

### 4. CLI ordering matches tray algorithm (automated parity)
expected: Rust `repo_priority` tests produce the same flat order as TypeScript `sort.test.ts` golden cases (Pinned > Dirty > Recent > Rest, D-20 dirty beats recent).
result: pass

### 5. CLI fuzzy matches tray algorithm (automated parity)
expected: Rust `fuzzy_match` agrees with `fuzzy.test.ts` golden vectors for the same query + repo fixtures.
result: pass

### 6. Tray vs CLI visual spot-check (manual)
expected: With the same indexed repos, tray default list top-to-bottom matches `workpot list` order; tray filter matches `workpot search` for the same query (no `#tag` syntax).
result: pass

## Summary

total: 6
passed: 6
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

[none yet]
