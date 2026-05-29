---
phase: 01-core-persistence
fixed_at: 2026-05-30T20:00:00Z
review_path: .planning/phases/01-core-persistence/01-REVIEW.md
iteration: 1
fix_scope: all
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 1: Code Review Fix Report

**Fixed at:** 2026-05-30T20:00:00Z  
**Source review:** `.planning/phases/01-core-persistence/01-REVIEW.md`  
**Iteration:** 1  
**Fix scope:** all (Critical + Warning + Info)

**Summary:**

- Findings in scope: 3
- Fixed: 3
- Skipped: 0

## Fixed Issues

### WR-01: `repo remove` after directory delete fails unless path matches stored canonical key

**Files modified:** `crates/workpot-core/src/services/catalog.rs`, `crates/workpot-core/tests/catalog_test.rs`  
**Applied fix:** `resolve_repo_path_key` queries SQLite by exact key, then by directory basename with single-match guard (ambiguous basenames return `InvalidPath`). `remove_repo_succeeds_when_directory_deleted` now removes via parent-relative path after delete.

### IN-01: Orphan `config.toml.tmp` on crash mid-write

**Files modified:** `crates/workpot-core/src/lib.rs`  
**Applied fix:** `remove_stale_config_temp` runs at `AppContext::open_with_paths` before config load.

### IN-02: First-run `default_config` seeds `~/code` and `~/dev` watch roots

**Files modified:** `crates/workpot-cli/src/main.rs`  
**Applied fix:** `workpot paths` prints watch roots and notes first-run seeding behavior (product behavior unchanged; visibility improved).

---

_Fixer: Claude (gsd-code-fixer)_  
_Scope: all_
