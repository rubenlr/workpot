---
phase: 05-tags-prioritization
wave: 1
plans: [02, 03]
created: 2026-05-31
mode: auto
---

# Phase 5 — Add Tests (Wave 1)

**Scope:** Plans 05-02 (Rust org layer) + 05-03 (TypeScript tray data)

## Classification (approved — auto)

### TDD (unit)

| File | Rationale |
|------|-----------|
| `crates/workpot-core/src/services/org.rs` | Tag/pin/notes CRUD, caps, validation |
| `crates/workpot-core/src/services/catalog.rs` | `list_repos` tag hydration |
| `src/lib/sort.ts` | Four-tier `sectionSort`, recency padding |
| `src/lib/fuzzy.ts` | Notes/tags fuzzy scoring |
| `src/lib/trayList.ts` | `filterAndSectionRepos` pipeline |
| `src/lib/tagFilter.ts` | `#tag` parse + AND filter (wave 0) |

### E2E (browser)

| File | Rationale |
|------|-----------|
| Tray Svelte UI | Deferred to wave 2–3 (IPC + components) per CONTEXT manual-only table |

### Skip

| File | Rationale |
|------|-----------|
| `006_org.sql`, `migrations.rs`, `store.rs` | Covered by migration/bootstrap + org integration tests |
| `types.ts`, `config.rs`, `repo.rs` | Type/schema only |
| `lib.rs` delegation | Thin wrappers; org.rs + AppContext bootstrap tested |

## Tests added this run

| File | New cases |
|------|-----------|
| `org_test.rs` | `set_tags` replace atomicity, `notes` clear, migration `repo_tags` table |
| `fuzzy.test.ts` | Unrelated query on note-only repo |
| `trayList.test.ts` | Multi-tag AND via `filterAndSectionRepos` |

## Results

| Category | Generated | Passing | Failing | Blocked |
|----------|-----------|---------|---------|---------|
| Unit (Rust) | 3 | 23/23 org_test | 0 | 0 |
| Unit (TS) | 2 | 75/75 vitest | 0 | 0 |
| E2E | 0 | — | — | 4 manual (tray UI, wave 2+) |

## Commands

- Rust: `cargo test -p workpot-core --test org_test`
- TS: `npm test`
- Wave 1 combined: `npm test && cargo test -p workpot-core --test org_test`

## Coverage gaps

- AppContext delegation smoke (low value; org.rs fully covered)
- E2E tray: detail pane, drag reorder, context menu (wave 2–3)
- `set_tags` concurrent transaction stress (manual/perf; atomicity asserted via replace test)

## Bugs discovered

None.
