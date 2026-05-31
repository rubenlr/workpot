---
phase: 05-tags-prioritization
scope: wave-1
reviewed: 2026-05-31T12:00:00Z
depth: standard
files_reviewed: 19
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
  - src/lib/repoRow.test.ts
findings:
  critical: 1
  warning: 7
  info: 3
  total: 11
status: issues_found
---

# Phase 5: Code Review Report (wave 1)

**Reviewed:** 2026-05-31T12:00:00Z  
**Depth:** standard  
**Scope:** wave-1 (plans 05-02 org layer, 05-03 TypeScript tray data)  
**Files Reviewed:** 19  
**Status:** issues_found

## Summary

Wave 1 delivers migration `006_org.sql`, `org.rs` CRUD, `catalog::list_repos` tag hydration via a second query (not N+1), config keys for section limits, and TypeScript `sectionSort` / `fuzzyScore` / `filterAndSectionRepos`. SQL tag/pin paths are parameterized and FK + CASCADE are correctly declared.

The highest-risk defect is in **recent-section padding**: never-opened repos (`last_opened_at IS NULL`) can be promoted into **Recent**, which contradicts Phase 5 context **D-21** and the “most-recently-opened” wording in **D-22**. Rust org APIs also omit **D-15** pin cap and **D-25** notes length enforcement (likely deferred to tray/IPC, but core is the authority today). Plan 05-02 must-haves are not fully covered by tests (CASCADE, `list_repos` tags).

## Critical Issues

### CR-01: Recent padding can place never-opened repos in Recent

**File:** `src/lib/sort.ts:76-85`  
**Issue:** When `recentByTime.length < minRecentCount`, padding pulls from `nonDirty` candidates without requiring `last_opened_at != null`. Repos with `last_opened_at IS NULL` are included in `candidates` and can fill the Recent section (e.g. three never-opened repos with `minRecentCount: 3` all land in Recent). Context **D-21** assigns never-opened repos to **Rest**; **D-22** says pad with the “next most-recently-opened” repos (implies at least one open timestamp).

**Fix:**

```typescript
const candidates = nonDirty
  .filter((r) => !inRecent.has(r) && r.last_opened_at != null)
  .sort(byLastOpenedDesc);
```

Add a Vitest case: `minRecentCount: 3`, three repos with `last_opened_at: null` → all in `rest`, `recent` empty.

## Warnings

### WR-01: `set_pin` does not enforce `max_pinned` (D-15)

**File:** `crates/workpot-core/src/services/org.rs:70-89`  
**Issue:** Config defines and validates `max_pinned`, but pinning always succeeds and appends a new `pin_order`. Tray can show more than the configured cap until something enforces it.

**Fix:** Before `UPDATE` on pin=true, `SELECT COUNT(*) FROM repos WHERE pinned = 1 AND excluded = 0`; if `count >= config.max_pinned` (pass `Config` or limit into `set_pin`), return a dedicated error (e.g. `WorkpotError::PinCapExceeded`). Re-pinning an already-pinned repo should not consume a slot.

### WR-02: `set_notes` has no 500-character limit (D-25)

**File:** `crates/workpot-core/src/services/org.rs:59-67`  
**Issue:** Notes accept arbitrary-length strings at the persistence layer. Detail pane spec caps at 500 characters; without core validation, CLI/IPC callers can persist oversized notes.

**Fix:** Reject `notes` where `notes.map(|s| s.chars().count()).unwrap_or(0) > 500` with `WorkpotError::InvalidInput` (or config-driven limit).

### WR-03: Tag mutations lack input hygiene and consistent NotFound errors

**File:** `crates/workpot-core/src/services/org.rs:4-34`  
**Issue:** `set_tags` / `add_tag` on a missing `repo_path` run `DELETE` (0 rows) then `INSERT`, which fails with FK/SQLite error instead of `WorkpotError::NotFound` like `set_notes` / `set_pin`. Empty or whitespace-only tags are not rejected and can be stored.

**Fix:** Check repo existence (or `UPDATE` row count) before mutating tags; trim/reject empty tags; map missing repo to `NotFound`.

### WR-04: Re-pinning an already-pinned repo allocates a new `pin_order`

**File:** `crates/workpot-core/src/services/org.rs:70-77`  
**Issue:** `set_pin(..., true)` always runs `MAX(pin_order)+1`, so idempotent “pin” calls reorder pinned repos unexpectedly.

**Fix:** If `pinned = 1` already, return `Ok(())` without changing `pin_order`, or only assign order when transitioning `0 → 1`.

### WR-05: Rest section sorts by name, not `registered_at` (spec discretion)

**File:** `src/lib/sort.ts:89-91`  
**Issue:** Context discretion suggests `registered_at DESC` for Rest; implementation uses `name.localeCompare`. Stale repos surface in alphabetical order, not registration recency.

**Fix:** If product wants registration recency, add `registered_at` to `RepoDto` and sort Rest by it; otherwise document the deliberate name sort in CONTEXT.

### WR-06: Plan 05-02 must-have tests missing (CASCADE, `list_repos` tags)

**File:** `crates/workpot-core/tests/org_test.rs`  
**Issue:** Plan must-haves require CASCADE on repo delete and `list_repos` tags via JOIN. `org_test.rs` covers direct `org::*` only; no `DELETE FROM repos` + tag cleanup test, no `catalog::list_repos` asserting `tags` populated.

**Fix:** Add `test_tags_cascade_on_repo_delete` and `test_list_repos_hydrates_tags` (two repos, tags on one, assert `list_repos()[i].tags`).

### WR-07: `sectionSort` / padding not tested for D-21 never-opened case

**File:** `src/lib/sort.test.ts:79-106`  
**Issue:** Padding test only uses repos with non-null `last_opened_at`. The D-21 vs D-22 conflict in CR-01 would not be caught by CI.

**Fix:** Extend tests per CR-01 fix.

## Info

### IN-01: `filterAndSortRepos` retained alongside `filterAndSectionRepos`

**File:** `src/lib/trayList.ts:8-10`  
**Issue:** Plan 05-03 describes replacing flat sort with sectioned API; both exports remain. Low risk if tray migrates in later wave; duplicate sort paths if both stay long-term.

**Fix:** Mark `filterAndSortRepos` deprecated or remove once `+page.svelte` uses sections only.

### IN-02: `fuzzyScore` does not search `branches[]` or `parent_dir`

**File:** `src/lib/fuzzy.ts:50-56`  
**Issue:** `RepoDto.branches` is on the type for detail pane but not indexed for search. Acceptable for wave 1 if intentional; branch string field is still searched.

**Fix:** Document omission or add `branches` to score fields when branch list ships.

### IN-03: `org_test` does not cover `list_all_tags` exclusion semantics

**File:** `crates/workpot-core/src/services/org.rs:46-56`  
**Issue:** `list_all_tags` JOIN-filters `excluded = 0`; no test proves tags on excluded repos are omitted from autocomplete source.

**Fix:** Insert excluded repo + tag; assert tag absent from `list_all_tags`.

---

_Reviewed: 2026-05-31T12:00:00Z_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
