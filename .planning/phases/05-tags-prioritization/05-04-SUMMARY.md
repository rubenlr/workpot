---
phase: 05-tags-prioritization
plan: 04
subsystem: api
tags: [tauri, ipc, git2, context-menu, org]

requires:
  - phase: 05-02
    provides: AppContext org methods, RepoRecord Phase 5 fields, Config recency keys
provides:
  - Nine org IPC commands callable from Svelte
  - RepoDto with pinned, pin_order, notes, tags, branches (empty in list_repos)
  - get_tray_config with max_recent_days, min_recent_count, max_pinned
  - repo-context-action event from native context menu
affects: [05-06]

tech-stack:
  added: [git2 direct dep on workpot-tray]
  patterns:
    - "Validate tags/notes at IPC boundary before AppContext lock"
    - "ContextMenuRepo state + app.on_menu_event for popup context"

key-files:
  created: []
  modified:
    - src-tauri/src/commands.rs
    - src-tauri/src/lib.rs
    - src-tauri/Cargo.toml

key-decisions:
  - "branches left empty in list_repos; list_branches IPC loads on detail pane open"
  - "Repo context menu IDs pin/add_tag/remove_tag; tray menu IDs unchanged"

patterns-established:
  - "show_repo_context_menu stores repo_path in ContextMenuRepo before Menu::popup"

requirements-completed: [ORG-01, ORG-02, ORG-03, ORG-04]

duration: 25min
completed: 2026-05-31
---

# Phase 5 Plan 04: Tauri Org IPC Summary

**Tauri IPC layer for tags, notes, pins, branch listing, context menu, and tray config org fields.**

## Accomplishments

- Extended `RepoDto` and `record_to_dto` with pinned, pin_order, notes, tags, and empty `branches`.
- Added `set_tags`, `add_tag`, `remove_tag`, `list_all_tags`, `set_notes`, `set_pin`, `set_pin_order` with input validation.
- Added `list_branches` (git2 local branches via `spawn_blocking`) and `show_repo_context_menu` (Tauri 2 popup).
- Extended `TrayConfigDto` / `get_tray_config` with `max_recent_days`, `min_recent_count`, `max_pinned` from `Config`.
- Registered commands and `on_menu_event` handler emitting `repo-context-action` for Svelte.

## Task Commits

1. **Task 1 + 3: commands.rs org IPC and tray config** — `abfe95c` (feat)
2. **Task 2: lib.rs registration and MenuEvent** — `a7aa876` (feat)

**Note:** Task 3 (`get_tray_config` extension) shipped in the same commit as Task 1 because both modify `commands.rs`.

## Files Created/Modified

- `src-tauri/src/commands.rs` — RepoDto, org commands, validation, context menu, TrayConfigDto
- `src-tauri/src/lib.rs` — invoke_handler registration, ContextMenuRepo, menu event bridge
- `src-tauri/Cargo.toml` — direct `git2` dependency for `list_branches`

## Deviations from Plan

None — plan executed as written. Task 3 combined with Task 1 commit (same file).

## Verification

- `cargo build --package workpot-tray` — exit 0
- `cargo test -p workpot-tray --lib` — 17 passed
- Grep: `commands::set_tags`, `on_menu_event`, `pub fn set_tags`, `list_branches`, `max_recent_days` (≥2) — present

## Self-Check: PASSED

- FOUND: `src-tauri/src/commands.rs`
- FOUND: `src-tauri/src/lib.rs`
- FOUND: commit `abfe95c`
- FOUND: commit `a7aa876`
