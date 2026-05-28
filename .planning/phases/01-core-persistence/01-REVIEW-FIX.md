---
phase: 01-core-persistence
fixed_at: 2026-05-29T20:00:00Z
iteration: 1
fix_scope: all
findings_in_scope: 10
fixed: 10
skipped: 0
status: all_fixed
---

# Phase 1: Code Review Fix Report

**Fixed:** 2026-05-29  
**Scope:** all (CR-01, WR-01–WR-05, IN-01–IN-04)  
**Status:** all_fixed

## Fixed

| ID | File | Change |
|----|------|--------|
| CR-01 | `infra/paths.rs` | macOS config under `~/.config/workpot/config.toml`; DB via `data_dir()` |
| WR-01 | `services/catalog.rs` | `is_git_worktree()` requires `.git/HEAD` or gitfile `gitdir:` |
| WR-02 | `infra/store.rs` | `busy_timeout(5s)` on connection open |
| WR-03 | `services/catalog.rs` | Distinct `path does not exist` before `not a directory` |
| WR-04 | `services/catalog.rs` | `is_bare_repo()` via `HEAD` + `objects`; CLI help updated |
| WR-05 | `workpot-core/Cargo.toml` | Removed unused `anyhow` dependency |
| IN-01 | `services/catalog.rs` | `list_repos` filters `WHERE excluded = 0` |
| IN-02 | `domain/config.rs` | Doc comments: fields consumed in Phase 2 |
| IN-03 | `services/catalog.rs` | `ORDER BY registered_at, path` for stable sort |
| IN-04 | `tests/catalog_test.rs` | `remove_repo_deletes_and_not_found` integration test |

## Tests added

- `tests/paths_test.rs` — macOS D-01/D-02 path assertions
- `catalog_test.rs` — empty `.git`, missing path, bare repo, remove/not-found
- Git fixtures create valid `.git/HEAD`

## Verification

```
cargo test -p workpot-core
```

All 10 tests passed (2026-05-29).

---

_Fixer: /gsd-code-review 1 --fix --all_
