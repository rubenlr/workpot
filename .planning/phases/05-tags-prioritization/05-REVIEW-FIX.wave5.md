---
phase: 05-tags-prioritization
scope: wave-5
fixed_at: 2026-05-31T11:01:00Z
review_path: .planning/phases/05-tags-prioritization/05-REVIEW.wave5-pre-fix.md
iteration: 1
findings_in_scope: 2
fixed: 2
skipped: 0
status: all_fixed
---

# Phase 5: Code Review Fix Report (wave 5)

**Fixed at:** 2026-05-31T11:01:00Z  
**Source review:** wave-5 iteration 1 (pre-fix)  
**Iteration:** 1 of 5 (auto loop stopped — clean on re-review)

**Summary:**
- Findings in scope: 2 (WR-01, WR-02)
- Fixed: 2
- Skipped: 0

**Verification:**
- `npm test` — 105 passed
- `cargo test -p workpot-cli --test cli_smoke` — 22 passed

## Fixed Issues

### WR-01: `appendTagToFilterQuery` substring false positive

**Severity:** WARNING  
**File:** `src/lib/tagFilter.ts`  
**Issue:** `filterQuery.includes("#" + tag)` treated `#foo` as present inside `#foobar`, blocking chip filter for a distinct tag.  
**Fix:** Idempotency via `parseTagFilter(filterQuery).activeTags` (AND-token semantics).

### WR-02: Autocomplete replace regex ignored hyphens

**Severity:** WARNING  
**File:** `src/lib/tagFilter.ts`  
**Issue:** `replaceTrailingTagAutocomplete` used `/#\w*$/` while partial input and `tagAutocompletePrefix` allow `[\w-]*`; completing `my-tag` after `#my-` failed.  
**Fix:** Use `/#([\w-]*)$/` for replacement.

## Tests added

- `tagFilter.test.ts`: substring idempotency case; hyphenated autocomplete completion.

---

_Fixer: gsd-code-fixer (orchestrated)_  
_Iteration: 1_
