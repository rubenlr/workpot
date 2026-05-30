---
phase: 04-tray-finder-mvp
plan: 03
subsystem: ui
tags: [tauri, git, rayon, tray, events, spinner]

requires:
  - phase: 04-tray-finder-mvp
    provides: filter bar, list_repos, panel-opened event
    plans: [04-01, 04-02]
provides:
  - AppContext::refresh_all_git_state with GitRefreshSummary
  - refresh_all_git_state Tauri command (async spawn)
  - git-refresh-complete event + tray dirty icon swap
  - Panel refresh spinner and Cmd+R manual refresh
affects: [04-04]

tech-stack:
  added: [tauri image-png]
  patterns: [rayon batch outside mutex, single tx persist, panel-open triggers refresh]

key-files:
  created:
    - crates/workpot-core/tests/tray_refresh_test.rs
    - src-tauri/icons/tray-default.png
    - src-tauri/icons/tray-dirty.png
  modified:
    - crates/workpot-core/src/lib.rs
    - crates/workpot-core/src/services/git_state.rs
    - src-tauri/src/commands.rs
    - src-tauri/src/tray.rs
    - src/routes/+page.svelte
    - src/lib/types.ts

key-decisions:
  - "Panel open loads cached list immediately; Rust spawns refresh without blocking tray thread"
  - "Tray icons embedded via include_bytes for dev/prod parity"

patterns-established:
  - "spawn_background_git_refresh shared by panel show and IPC command"
  - "Frontend sets refreshing on panel-opened; clears on git-refresh-complete"

requirements-completed: [UI-02, GIT-04]

duration: 25min
completed: 2026-05-30
---

# Phase 4 Plan 03: Background Git Refresh — Summary

**Tray panel shows cached repos instantly, refreshes git state in the background, and surfaces progress via spinner plus optional dirty tray icon.**

## Performance

- **Duration:** ~25 min
- **Completed:** 2026-05-30
- **Tasks:** 3
- **Files modified:** 11

## Accomplishments

- `refresh_all_git_state` batches rayon refresh and persists in one SQLite transaction
- Non-blocking Tauri command emits `git-refresh-complete` with `GitRefreshSummary`
- Svelte spinner + Cmd+R without touching 04-02 filter/keyboard logic

## Task Commits

1. **Task 1: AppContext::refresh_all_git_state** - `ebf555e` (feat)
2. **Task 2: Tauri async refresh command** - `ba94c0f` (feat)
3. **Task 3: Spinner, Cmd+R, tray icons** - `3ef6888` (feat)

## Self-Check: PASSED

- `cargo test -p workpot-core tray_refresh` — passed
- `cargo build -p workpot-tray` — passed
- `npm run build` — passed
- `npm test` — passed

## Deviations

- `tray-dirty.png` is a copy of default when PIL unavailable; icon swap logic is wired; visual dot can be improved later.

## Next

Plan 04-04: Cursor launch, error banner, tray context menu (SDK wave 4).
