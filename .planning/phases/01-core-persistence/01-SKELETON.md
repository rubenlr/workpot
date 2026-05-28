# Walking Skeleton — Workpot

**Phase:** 1
**Generated:** 2026-05-28

## Capability Proven End-To-End

A macOS developer runs `workpot repo add <git-repo-path>` and `workpot repo list` in a fresh shell and sees the same registered repo — config and SQLite created automatically on first use, with no network access.

## Architectural Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Host surface (Phase 1) | CLI only (`workpot-cli`) | Proves shared core before Tauri tray in Phase 4; no Node/toolchain cost now |
| Shared library | `workpot-core` crate | Tauri and CLI will both construct `AppContext` later without duplicating persistence logic |
| Data layer | SQLite via `rusqlite` 0.39 (`bundled`) + `rusqlite_migration` 2.5 | Embedded, WAL-friendly, migration via `user_version`; avoids macOS system SQLite drift |
| Config format | TOML at `~/.config/workpot/config.toml` (D-01) | Human-editable dotfile; separate from bulk app data |
| Database location | `~/Library/Application Support/workpot/workpot.db` (D-02) | Apple-native app data path via `directories::BaseDirs::data_dir()` |
| Path resolution | `BaseDirs` only — never `ProjectDirs` for config | `ProjectDirs::config_dir()` on macOS lands under Application Support, violating D-01 |
| Bootstrap | Lazy `AppContext::open()` on first CLI invocation (D-04) | Creates default config + DB + migrations; no `workpot init` gate |
| Path overrides | Fixed defaults in production (D-03); `open_with_paths` test-only | Hermetic tests without env-based production overrides |
| Repo identity | Canonical path string as PRIMARY KEY | Matches ARCHITECTURE path-as-id; git snapshot columns deferred to Phase 3 migration |
| Git validation (Phase 1) | Filesystem `.git` dir or file check only | No `git2`, no subprocess — satisfies ROADMAP “git repo path” without network or libgit2 |
| Error handling | `thiserror` in core, `anyhow` at CLI edge | Typed library errors; ergonomic binary messages |
| Network / privacy | Zero HTTP client crates in Phase 1 (DATA-02) | Structural enforcement via `cargo tree` ban script + CI `--offline` |
| Directory layout | `crates/workpot-core/{domain,services,infra}` + `crates/workpot-cli` | ARCHITECTURE layering; no `src-tauri/` or `ui/` until Phase 4 |

## Stack Touched in Phase 1

- [x] Project scaffold — Cargo workspace, `rust-toolchain.toml` (1.85+), core + CLI crates
- [x] CLI routing — `workpot paths`, `workpot repo add|list|remove` via clap
- [x] Database — real write (`repo add`) and read (`repo list`) against SQLite
- [x] Config — real read/write of default `config.toml` on first launch
- [x] Local full-stack run — `cargo install --path crates/workpot-cli` then `workpot repo add …` (no deployment; macOS local-only)

## Out of Scope (Deferred to Later Slices)

- Tauri tray, WebView UI, IPC commands (Phase 4)
- `git2`, branch/dirty/ahead-behind columns (Phase 3)
- Watch-root discovery, filesystem watcher, `notify` (Phase 2)
- `tokio`, background refresh queue (Phase 2–3)
- Fuzzy search, Cursor launch, recipes (Phases 4–7)
- Production env overrides (`WORKPOT_HOME`, `WORKPOT_DATA_DIR`)
- Tags, pins, notes, ranking signals (Phase 5)
- Runtime socket auditing for DATA-02 (structural dep ban only in Phase 1)

## Subsequent Slice Plan

Each later phase adds one vertical slice on top of this skeleton without renegotiating paths, `AppContext` entrypoint, or SQLite-as-catalog-store:

- **Phase 2:** Watch-root scan + manual add/exclude merges with catalog (`source = 'scan'`)
- **Phase 3:** Git snapshot refresh into additive migration `002_git_snapshot`
- **Phase 4:** Tauri tray host reusing `AppContext`; filter + open in Cursor
- **Phase 5:** Tags, pins, notes, prioritization signals
- **Phase 6:** CLI parity with tray list/search/open
- **Phase 7:** Recipe runner from config
