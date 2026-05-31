---
phase: 05-tags-prioritization
plan: 01
subsystem: testing
tags: [vitest, rust, org, tag-filter, pin-order]

requires: []
provides:
  - Rust org integration test stubs (ignored until migration 006 + org service)
  - tagFilter.ts parseTagFilter / matchesTags utilities with Vitest coverage
  - pinOrder.ts reorderPinned / toPinOrderPayload utilities with Vitest coverage
affects: [05-02, 05-03, 05-04]

tech-stack:
  added: []
  patterns:
    - "Wave 0: pure TS utilities ship complete; Rust tests #[ignore] until org module exists"

key-files:
  created:
    - crates/workpot-core/tests/org_test.rs
    - src/lib/tagFilter.ts
    - src/lib/tagFilter.test.ts
    - src/lib/pinOrder.ts
    - src/lib/pinOrder.test.ts
  modified: []

key-decisions:
  - "org_test.rs omits services::org import until plan 02 adds the module; calls documented in comments"
  - "RepoDto Phase 5 fields accessed via intersection types until types.ts extended in plan 03"

patterns-established:
  - "Tag filter: #token parse + AND case-insensitive matchesTags"
  - "Pin reorder: splice + contiguous pin_order 0..N on every drop"

requirements-completed: [ORG-01, ORG-02, ORG-04]

duration: 15min
completed: 2026-05-31
---

# Phase 05 Plan 01: Wave 0 Test Infrastructure

**Wave 0 harness: ignored Rust org stubs compile; tagFilter and pinOrder pure utilities green in Vitest.**

## Performance

- **Duration:** ~15 min
- **Completed:** 2026-05-31
- **Tasks:** 3/3
- **Files modified:** 5 created

## Accomplishments

- `org_test.rs` compiles with six `#[ignore]` stubs referencing future `org::*` APIs
- `parseTagFilter` / `matchesTags` implemented with 9 passing Vitest cases
- `reorderPinned` / `toPinOrderPayload` implemented with 5 passing Vitest cases
- Full `npm test` suite remains green (58 tests)

## Self-Check: PASSED

- `cargo test -p workpot-core org --no-run` — exit 0
- `npm test -- tagFilter` — 9 passed
- `npm test -- pinOrder` — 5 passed
- `npm test` — 58 passed

## Deviations

- **`services::org` import deferred:** Module does not exist until plan 05-02; stubs use commented call sites instead of `use workpot_core::services::org` to satisfy compile-only gate without expanding plan scope.

## Files Created/Modified

- `crates/workpot-core/tests/org_test.rs` — temp_db fixture + ignored org/pin/notes stubs
- `src/lib/tagFilter.ts` — `#tag` parse and AND filter
- `src/lib/tagFilter.test.ts` — Vitest coverage
- `src/lib/pinOrder.ts` — drag reorder + IPC payload helper
- `src/lib/pinOrder.test.ts` — Vitest coverage
