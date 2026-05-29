---
status: testing
phase: 02-repo-discovery
source: 02-01-SUMMARY.md, 02-02-SUMMARY.md, 02-03-SUMMARY.md, 02-04-SUMMARY.md, 02-05-SUMMARY.md, ROADMAP.md success criteria, 02-VALIDATION.md
started: 2026-05-29T12:00:00Z
updated: 2026-05-29T12:00:00Z
---

## Current Test

number: 1
name: Cold Start Smoke Test
expected: |
  From a clean shell (or after `cargo build -p workpot-cli`), run `workpot index` against your normal Workpot config/DB.
  Command exits 0, prints a one-line summary like `index: +N -M skipped S` (added/removed/skipped counts), and does not crash on first rescan after build.
awaiting: user response

## Tests

### 1. Cold Start Smoke Test
expected: Fresh build; `workpot index` exits 0 with `index: +… -… skipped …` summary line; no crash on first rescan
result: [passed]

### 2. Watch root discovers nested repos
expected: Add a watch root (config or `workpot roots add`) that contains two or more nested git repos. Run `workpot index`, then `workpot repo list`. All nested repos under that root appear with correct paths.
result: [passed]

### 3. Manual add outside watch roots
expected: `workpot repo add` on a real git repo outside any watch root exits 0. `workpot repo list` shows it. Run `workpot index` again — repo remains listed (manual source preserved on rescan).
result: [passed]

### 4. Excluded path never reappears
expected: `workpot repo remove <path>` on a scan-discovered repo exits 0. Run `workpot index` at least twice. Removed repo does not return in `workpot repo list`.
result: [passed]

### 5. Plain directory not indexed
expected: Under a watch root, a directory without `.git` exists. After `workpot index`, that directory is not listed in `workpot repo list`.
result: [passed]

### 6. Rescan without restart
expected: Without restarting any daemon or re-opening the DB manually, run `workpot index` twice in a row. Both complete exit 0; second run shows sensible/stable counts (no duplicate explosion).
result: [passed]

### 7. Roots add triggers immediate scan
expected: `workpot roots add <path-with-nested-repos>` exits 0. Nested git repos appear in `workpot repo list` without requiring a separate manual step (immediate scan on add).
result: [passed]

### 8. Repo cap blocks over-limit index
expected: Set `limits.max_repos` in config to a low value below your current repo count. `workpot index` exits non-zero (exit 1) with a clear cap error; repo table is not partially corrupted (removed repos from a failed cap run do not appear).
result: [passed]

## Summary

total: 8
passed: 8
issues: 0
pending: 8
skipped: 0
blocked: 0

## Gaps

[none yet]
