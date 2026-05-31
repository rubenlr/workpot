---
phase: 05-tags-prioritization
scope: wave-1
reviewed: 2026-05-31T14:30:00Z
depth: standard
files_reviewed: 20
files_reviewed_list:
  - crates/workpot-core/src/infra/migrations/006_org.sql
  - crates/workpot-core/src/services/org.rs
  - crates/workpot-core/src/infra/migrations.rs
  - crates/workpot-core/src/infra/store.rs
  - crates/workpot-core/src/domain/repo.rs
  - crates/workpot-core/src/domain/config.rs
  - crates/workpot-core/src/services/catalog.rs
  - crates/workpot-core/src/services/mod.rs
  - crates/workpot-core/src/lib.rs
  - crates/workpot-core/src/error.rs
  - crates/workpot-core/tests/org_test.rs
  - crates/workpot-core/tests/bootstrap_test.rs
  - src/lib/types.ts
  - src/lib/sort.ts
  - src/lib/sort.test.ts
  - src/lib/fuzzy.ts
  - src/lib/fuzzy.test.ts
  - src/lib/trayList.ts
  - src/lib/trayList.test.ts
  - src/lib/openSelection.test.ts
findings:
  critical: 0
  warning: 1
  info: 4
  total: 5
status: issues_found
fix_commits_reviewed:
  - 4aac502
  - 67d0e8c
  - a57effd
  - 164b7bb
---

# Phase 5: Code Review Report (wave 1, final re-review)

**Reviewed:** 2026-05-31T14:30:00Z  
**Depth:** standard  
**Scope:** wave-1 (post iter-2 fixes `4aac502`, `67d0e8c`, `a57effd`, `164b7bb`)  
**Files Reviewed:** 20  
**Status:** issues_found

## Summary

All four iter-2 findings are correctly fixed: `remove_tag` normalizes input, `list_tags_for_repo` returns `NotFound` for missing repos, `set_pin_order` rejects unpinned paths, and `normalize_tag` enforces 64-char / no-`#` limits with matching tests. Prior wave-1 fixes (Recent padding D-21, pin cap, notes limit, tag NotFound/empty validation, idempotent re-pin, CASCADE/`list_repos` tag hydration) remain intact. `cargo test -p workpot-core --test org_test` passes (18 tests).

One warning remains: tag length uses byte count, inconsistent with notes validation and the “64 characters” spec. Four info items are acceptable deferrals for wave 1.

## Warnings

### WR-01: Tag max length uses byte count, not character count

**File:** `crates/workpot-core/src/services/org.rs:25-28`  
**Issue:** `normalize_tag` rejects when `trimmed.len() > 64` (UTF-8 bytes). `set_notes` uses `text.chars().count()` for its 500-char cap. Multi-byte tags (e.g. emoji) can be rejected well below 64 graphemes while still satisfying the product spec; conversely the check is inconsistent across org APIs.

**Fix:**

```rust
if trimmed.chars().count() > 64 {
    return Err(WorkpotError::InvalidInput(
        "tag exceeds 64 characters".into(),
    ));
}
```

Add an `org_test` with a 20-character emoji tag (under 64 graphemes, over 64 bytes if using `.len()`) to lock behavior.

## Info

### IN-01: `filterAndSortRepos` retained alongside `filterAndSectionRepos`

**File:** `src/lib/trayList.ts:8-10`  
**Issue:** Flat sort path remains for tests/legacy; production tray should use `filterAndSectionRepos` only. No runtime bug in wave 1.

**Fix:** Remove when all callers migrate to sectioned API.

### IN-02: `set_notes` allows whitespace-only text

**File:** `crates/workpot-core/src/services/org.rs:104-119`  
**Issue:** `Some("   ")` passes validation and is stored. Detail pane may trim on blur (Plan 05-05); core does not normalize to `NULL`.

**Fix:** Optional: treat trim-empty as `None` before `UPDATE`.

### IN-03: `list_all_tags` exclusion semantics untested

**File:** `crates/workpot-core/src/services/org.rs:91-101`  
**Issue:** Query JOIN-filters `excluded = 0`; no test proves tags on excluded repos are omitted from autocomplete source.

**Fix:** Insert excluded repo + tag; assert tag absent from `list_all_tags`.

### IN-04: `fuzzyScore` does not search `branches[]`

**File:** `src/lib/fuzzy.ts:50-56`  
**Issue:** `RepoDto.branches` is on the type for detail pane but not indexed for search. Acceptable for wave 1 if intentional; `branch` string field is still searched.

**Fix:** Document omission or add `branches` to score fields when branch list ships.

---

_Reviewed: 2026-05-31T14:30:00Z_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_  
_Re-review after: 4aac502, 67d0e8c, a57effd, 164b7bb_
