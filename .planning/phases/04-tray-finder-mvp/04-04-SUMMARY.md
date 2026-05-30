---
phase: 04-tray-finder-mvp
plan: 04
subsystem: ui
tags: [tauri, cursor, launch, tray-menu, shell-words]

requires:
  - phase: 04-tray-finder-mvp
    provides: filter bar, keyboard nav, background git refresh
    plans: [04-01, 04-02, 04-03]
provides:
  - launch_cmd parsing via shell-words and open_in_cursor IPC
  - Enter/Cmd+Enter/dblclick/Cmd+click open flows with panel hide rules
  - Launch error banner (non-modal, dismissible)
  - Tray context menu: Refresh index, Preferences, About, Quit
affects: [phase-5]

tech-stack:
  added: [shell-words]
  patterns: [indexed_launch_path guard before spawn, async run_index from tray menu]

key-files:
  created:
    - src-tauri/src/launch.rs
  modified:
    - crates/workpot-core/src/services/catalog.rs
    - crates/workpot-core/src/lib.rs
    - src-tauri/src/commands.rs
    - src-tauri/src/tray.rs
    - src-tauri/src/lib.rs
    - src-tauri/Cargo.toml
    - src-tauri/capabilities/default.json
    - src-tauri/permissions/tray-commands.toml
    - src/routes/+page.svelte

key-decisions:
  - "shell-words splits launch_cmd; newline in path rejected before substitute"
  - "Tray index refresh emits index-complete; menu handler returns immediately"

patterns-established:
  - "open_in_cursor validates indexed path then spawns and touch_last_opened_at"
  - "launchError separate from list/git error state"

requirements-completed: [UI-04, LAUNCH-01]

duration: 25min
completed: 2026-05-30
---

# Phase 4 Plan 04: Cursor Launch + Tray Menu — Summary

**Enter opens the selected repo in Cursor via configurable `launch_cmd`; failures surface in-panel; tray menu adds index refresh, preferences, about, and quit.**

## Accomplishments

- `build_command` / `launch_repo` with `shell-words` parsing and indexed-path validation
- `open_in_cursor` Tauri command + capability
- Panel: Enter / Cmd+Enter / dblclick / Cmd+click; dismissible error banner; hide on success (non-background)
- Tray menu: async `run_index`, `open` config.toml, About dialog, Quit

## Verification

- `cargo test -p workpot-tray launch` — 4 passed
- `cargo build -p workpot-tray` — ok
- `npm run build` — ok
