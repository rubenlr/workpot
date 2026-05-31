---
phase: 05-tags-prioritization
fixed_at: 2026-05-31T16:15:00Z
review_path: .planning/phases/05-tags-prioritization/05-REVIEW.md
iteration: 4
findings_in_scope: 2
fixed: 2
skipped: 0
status: all_fixed
---

# Phase 5: Code Review Fix Report

**Fixed at:** 2026-05-31T16:15:00Z  
**Source review:** `.planning/phases/05-tags-prioritization/05-REVIEW.md` (wave 1, iter-2 + final passes)  
**Iteration:** 4

**Summary:**
- Findings in scope: 2 (WR-01 char count, IN-03 list_all_tags test)
- Fixed: 2
- Skipped: 0

## Fixed Issues

### WR-01: Tag max length uses byte count, not character count

**Files modified:** `crates/workpot-core/src/services/org.rs`, `crates/workpot-core/tests/org_test.rs`  
**Commit:** `9c3e302`  
**Applied fix:** `normalize_tag` now uses `trimmed.chars().count() > 64` (consistent with `set_notes` char counting). Added `test_tags_allow_emoji_under_64_chars` — 20 emoji (80 bytes, 20 graphemes) accepted.

### IN-03: `list_all_tags` exclusion semantics untested

**Files modified:** `crates/workpot-core/tests/org_test.rs`  
**Applied fix:** `test_list_all_tags_omits_excluded_repos` — tag on `excluded = 1` repo absent from autocomplete list.

## Skipped Issues

None (info IN-01, IN-02, IN-04 deferred per wave-1 scope).

---

_Fixed: 2026-05-31T16:15:00Z_  
_Fixer: Claude (gsd-code-fixer)_  
_Iteration: 4_
