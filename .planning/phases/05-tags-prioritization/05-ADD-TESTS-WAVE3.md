---
phase: 05-tags-prioritization
wave: 3
plans: [06, 07]
created: 2026-05-31
mode: auto
max_iterations: 5
iterations_used: 1
---

# Phase 5 — Add Tests (Wave 3)

**Scope:** Plans 05-06 (tray integration) + 05-07 (CLI `workpot tag`)

## Classification (approved — auto)

### TDD (unit)

| File | Rationale |
|------|-----------|
| `src/lib/detailRepoSync.ts` | CR-02: resync detail pane after `loadRepos` |
| `src/lib/trayList.ts` | `flatSectioned` order for `flatVisible` keyboard index |
| `src/lib/openSelection.ts` | Background-open selection restore (plan 06) |
| `crates/workpot-cli/tests/cli_smoke.rs` | `tag add/list/remove`, validation, repo name resolution, Unicode grapheme cap |

### E2E (browser)

| File | Rationale |
|------|-----------|
| Tray detail pane, sections, drag reorder, context menu | Manual per VALIDATION.md — Tauri webview |

### Skip

| File | Rationale |
|------|-----------|
| `src/routes/+page.svelte` | Thin wiring; resync + list logic in `detailRepoSync` / `trayList` / `openSelection` |
| `src/lib/types.ts` | Types only |
| `crates/workpot-cli/src/main.rs` | Private fns; covered by cli_smoke integration |

## Tests added this run

| File | New cases |
|------|-----------|
| `detailRepoSync.test.ts` | Stale tag refresh, removed repo, null path |
| `trayList.test.ts` | `flatSectioned` pinned→dirty→recent→rest order |
| `cli_smoke.rs` | Tag roundtrip, name resolution, `#` reject, 64/65 grapheme é |

## Results

| Category | Generated | Passing | Failing | Blocked |
|----------|-----------|---------|---------|---------|
| Unit (TS) | 4 cases / 2 files | 94/94 vitest | 0 | 0 |
| Unit (CLI integration) | 5 | 21/21 cli_smoke | 0 | 0 |
| E2E | 0 | — | — | 4 manual (tray UI) |

## Commands

- TS: `npm test`
- CLI: `cargo test -p workpot-cli --test cli_smoke`
- Wave 3 combined: `npm test && cargo test -p workpot-cli --test cli_smoke`

## Coverage gaps

- Keyboard suppression while detail open (WR-05; behavior in `+page.svelte`, manual UAT)
- Pin drag-drop and list-row tag remove error paths (manual)
- Ambiguous repo name CLI error (integration-only; no duplicate-name fixture yet)

## Bugs discovered

None.
