---
phase: 05-tags-prioritization
fixed_at: 2026-05-31T10:05:00Z
review_path: .planning/phases/05-tags-prioritization/05-REVIEW.md
iteration: 1
findings_in_scope: 8
fixed: 8
skipped: 0
status: all_fixed
---

# Phase 5: Code Review Fix Report

**Fixed at:** 2026-05-31T10:05:00Z  
**Source review:** `.planning/phases/05-tags-prioritization/05-REVIEW.md`  
**Iteration:** 1

**Summary:**
- Findings in scope: 8 (CR-01 + WR-01 through WR-07)
- Fixed: 8
- Skipped: 0

## Fixed Issues

### CR-01: Recent padding can place never-opened repos in Recent

**Files modified:** `src/lib/sort.ts`, `src/lib/sort.test.ts`  
**Commit:** `3fc0033`  
**Applied fix:** Padding candidates require `last_opened_at != null`; Vitest case for three never-opened repos with `minRecentCount: 3`.

### WR-01: `set_pin` does not enforce `max_pinned` (D-15)

**Files modified:** `crates/workpot-core/src/error.rs`, `crates/workpot-core/src/services/org.rs`, `crates/workpot-core/src/lib.rs`, `crates/workpot-core/tests/org_test.rs`  
**Commit:** `d319eb0`, `509eac8`  
**Applied fix:** `PinCapExceeded` error; count pinned repos before `0 → 1` transition; `AppContext` passes `config.max_pinned`.

### WR-02: `set_notes` has no 500-character limit (D-25)

**Files modified:** `crates/workpot-core/src/error.rs`, `crates/workpot-core/src/services/org.rs`, `crates/workpot-core/tests/org_test.rs`  
**Commit:** `d319eb0`, `509eac8`  
**Applied fix:** Reject notes over 500 chars with `InvalidInput`.

### WR-03: Tag mutations lack input hygiene and consistent NotFound errors

**Files modified:** `crates/workpot-core/src/services/org.rs`, `crates/workpot-core/tests/org_test.rs`  
**Commit:** `d319eb0`, `509eac8`  
**Applied fix:** `ensure_repo_exists` before tag writes; trim/reject empty tags; `NotFound` for missing repo.

### WR-04: Re-pinning an already-pinned repo allocates a new `pin_order`

**Files modified:** `crates/workpot-core/src/services/org.rs`, `crates/workpot-core/tests/org_test.rs`  
**Commit:** `d319eb0`, `509eac8`  
**Applied fix:** Early return when already pinned; `test_pin_repin_is_idempotent`.

### WR-05: Rest section sorts by name, not `registered_at` (spec discretion)

**Files modified:** `.planning/phases/05-tags-prioritization/05-CONTEXT.md`  
**Commit:** `713fee5`  
**Applied fix:** Documented deliberate name sort for Rest in wave 1 (no `registered_at` on tray DTO yet).

### WR-06: Plan 05-02 must-have tests missing (CASCADE, `list_repos` tags)

**Files modified:** `crates/workpot-core/tests/org_test.rs`  
**Commit:** `509eac8`  
**Applied fix:** `test_tags_cascade_on_repo_delete` and `test_list_repos_hydrates_tags`.

### WR-07: `sectionSort` / padding not tested for D-21 never-opened case

**Files modified:** `src/lib/sort.test.ts`  
**Commit:** `3fc0033`  
**Applied fix:** `does not pad recent with never-opened repos (D-21)` test.

## Skipped Issues

None — all in-scope findings were fixed.

---

_Fixed: 2026-05-31T10:05:00Z_  
_Fixer: Claude (gsd-code-fixer)_  
_Iteration: 1_
