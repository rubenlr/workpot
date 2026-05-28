---
status: testing
phase: 01-core-persistence
source: 01-01-SUMMARY.md, 01-02-SUMMARY.md, 01-03-SUMMARY.md, ROADMAP.md success criteria
started: 2026-05-29T00:00:00Z
updated: 2026-05-29T00:00:00Z
---

## Current Test

number: 1
name: Cold Start Smoke Test
expected: |
  Build and install the CLI fresh (`cargo install --path crates/workpot-cli` from repo root).
  Run `workpot paths`. Command exits 0, prints `config:` and `database:` lines pointing at macOS-standard locations, and creates default config plus SQLite DB on first run without errors.
awaiting: user response

## Tests

### 1. Cold Start Smoke Test
expected: Fresh install; `workpot paths` exits 0, prints config and database paths, bootstraps files on first run
result: [pending]

### 2. First-run paths and config
expected: `workpot paths` shows `config:` under Application Support/Preferences area and `database:` under Application Support/workpot; default config.toml exists after first run
result: [pending]

### 3. Register a git repository
expected: `workpot repo add /path/to/real-git-repo` exits 0 and prints `registered:` with the path
result: [pending]

### 4. Repo persists in a new shell
expected: Open a new terminal; `workpot repo list` shows the repo name and path added in test 3
result: [pending]

### 5. Remove a repository
expected: `workpot repo remove /same/path>` exits 0 with `removed:`; `workpot repo list` no longer shows that repo
result: [pending]

### 6. Reject non-git directory
expected: `workpot repo add` on a directory without `.git` fails with a clear error (non-zero exit, human-readable message)
result: [pending]

### 7. Core operations stay offline
expected: `workpot paths`, `repo add`, `repo list`, and `repo remove` complete without network access (no download/update prompts during normal use)
result: [pending]

## Summary

total: 7
passed: 0
issues: 0
pending: 7
skipped: 0
blocked: 0

## Gaps

[none yet]
