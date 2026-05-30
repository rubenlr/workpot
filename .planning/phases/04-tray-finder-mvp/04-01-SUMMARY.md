---
phase: 04-tray-finder-mvp
plan: 01
subsystem: ui
tags: [tauri, svelte, sqlite, tray, ipc]

requires:
  - phase: 03-git-state
    provides: RepoRecord git fields, AppContext, migrations pattern
provides:
  - Tauri 2 workpot-tray binary with menu bar icon and panel window
  - list_repos IPC command over shared SQLite
  - Migration 005 last_opened_at + launch_cmd / max_visible_rows config
  - SvelteKit SPA frontend with repo list UI
affects: [04-tray-finder-mvp, 04-02, 04-03, 04-04]

tech-stack:
  added: [tauri 2, @tauri-apps/api, sveltekit, vite, tailwindcss 4]
  patterns: [Arc<Mutex<AppContext>> in Tauri state, Linux-safe workspace default-members, tray click toggle panel]

key-files:
  created:
    - crates/workpot-core/src/infra/migrations/005_tray.sql
    - crates/workpot-core/tests/tray_migration_test.rs
    - src-tauri/src/commands.rs
    - src-tauri/src/tray.rs
    - src-tauri/tauri.conf.json
    - src/routes/+page.svelte
    - package.json
  modified:
    - Cargo.toml
    - .github/workflows/ci.yml
    - crates/workpot-core/src/domain/config.rs
    - crates/workpot-core/src/services/catalog.rs

key-decisions:
  - "Used migration 005_tray.sql because 004_repos_source_index.sql already occupied migration slot 004"
  - "Ubuntu CI scopes cargo to workpot-core + workpot-cli; macOS builds tray + npm"

patterns-established:
  - "IPC: invoke('list_repos') → RepoDto JSON from cached SQLite (no per-keystroke IPC)"
  - "Tray app: close hides panel; left-click toggles panel visibility"

requirements-completed: [UI-01, UI-02]

duration: 45min
completed: 2026-05-30
---

# Phase 4 Plan 01: Tray Scaffold + List Summary

**Tauri 2 tray app shares workpot-core SQLite via list_repos IPC, with migration 005 last_opened_at and a menu-bar panel listing indexed repos.**

## Performance

- **Duration:** ~45 min
- **Tasks:** 3
- **Files modified:** ~55

## Accomplishments

- Core schema/config: `last_opened_at`, `launch_cmd`, `max_visible_rows`, `touch_last_opened_at`
- `workpot-tray` workspace member with SvelteKit + Tailwind SPA and `list_repos` command
- Linux-safe CI (ubuntu excludes tray; macOS builds tray + `npm ci`)
- Tray icon toggles frameless panel; repo rows show name, branch, dirty dot, parent dir

## Task Commits

1. **Task 1: Core migration, config, RepoRecord** - `de7ef67` (feat)
2. **Task 2: Tauri scaffold + list_repos + CI bundle** - `a14e353` (feat)
3. **Task 3: Tray toggle + Svelte list UI** - `97d680c` (feat)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Migration numbered 005 instead of 004_tray.sql**
- **Found during:** Task 1
- **Issue:** `004_repos_source_index.sql` already registered as migration 004
- **Fix:** Added `005_tray.sql` with same `ALTER TABLE` for `last_opened_at`
- **Files modified:** `crates/workpot-core/src/infra/migrations/005_tray.sql`, `migrations.rs`
- **Committed in:** `de7ef67`

**2. [Rule 1 - Bug] Missing `tauri::Manager` import**
- **Found during:** Task 2 build
- **Issue:** `app.manage()` did not compile
- **Fix:** `use tauri::Manager` in `lib.rs`
- **Committed in:** `a14e353` (amended in same task commit before Task 3)

**Total deviations:** 2 auto-fixed (1 Rule 3, 1 Rule 1)

## Issues Encountered

None beyond deviations above.

## Self-Check

- FOUND: crates/workpot-core/src/infra/migrations/005_tray.sql
- FOUND: src-tauri/src/lib.rs
- FOUND: src-tauri/src/commands.rs
- FOUND: src-tauri/src/tray.rs
- FOUND: src/routes/+page.svelte
- FOUND: de7ef67
- FOUND: a14e353
- FOUND: 97d680c

## Self-Check: PASSED

## Verification

- `cargo test -p workpot-core tray_migration` — pass
- `cargo build -p workpot-tray` — pass
- `npm run build` — pass (Task 2)
- `bash scripts/check-no-network-deps.sh` — pass (includes workpot-tray)
