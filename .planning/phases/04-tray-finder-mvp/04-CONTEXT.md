# Phase 4: Tray finder MVP - Context

**Gathered:** 2026-05-30
**Status:** Ready for planning

<domain>
## Phase Boundary

Deliver the daily driver loop: a macOS Tauri tray app (new `src-tauri/` + `src/` structure at repo root) that shows a prioritized repo list with git state, lets the user filter as they type, and opens the selected repo in Cursor. This is the first Tauri crate in the workspace. Core API (`list_repos`, `refresh_and_persist_git_state`, rayon batch refresh) already exists from Phases 1–3. No tags/pins (Phase 5), no full CLI parity (Phase 6), no recipes (Phase 7).

**Requirements:** UI-01, UI-02, UI-03, UI-04, SRCH-01, SRCH-02, SRCH-03, LAUNCH-01

**Success criteria (from ROADMAP.md):**
1. Tray icon visible in macOS menu bar; opens finder panel
2. Prioritized list with branch and dirty indicator per repo
3. Typing filters list in real time
4. Enter opens repo in Cursor
5. Failed Cursor launch shows clear error (not silent no-op)

</domain>

<decisions>
## Implementation Decisions

### Frontend tech
- **D-01:** Frontend framework: **Svelte + SvelteKit + Vite**. Standard Tauri 2 community pattern; smallest bundle, best IPC ergonomics with `@tauri-apps/api`.
- **D-02:** CSS: **Tailwind CSS**. Utility-first, tiny purged bundle, fast to prototype for a focused popup.
- **D-03:** Language: **TypeScript**. Type-safe IPC payloads. Tauri's `@tauri-apps/api` ships TS types.
- **D-04:** Theme: **System dark/light via CSS `prefers-color-scheme`**. No in-app toggle. Follows macOS system preference.

### Workspace layout
- **D-05:** Tauri app lives at repo root: `src-tauri/` (Rust backend) + `src/` (Svelte frontend). Standard Tauri project layout.
- **D-06:** `src-tauri` is a **Cargo workspace member** — added to `[workspace] members` in root `Cargo.toml`. Shared `Cargo.lock`, unified `cargo build`, direct `workpot-core` dependency.

### Panel window chrome
- **D-07:** Panel is **frameless** with rounded corners, backdrop blur (frosted glass), and shadow. Matches macOS control center popup aesthetic.
- **D-08:** Panel **closes on focus loss** (click outside). Standard macOS tray popup behavior.
- **D-09:** Tray icon click **toggles** the panel (second click closes). Standard macOS tray convention.

### Repo row layout
- **D-10:** Per-row columns: **repo name** (primary) + **colored dot** (dirty indicator: amber/red = dirty, green = clean) + **branch name** + **parent directory** (e.g. `~/c/myrepo`).
- **D-11:** Selected row: **full-width highlight** with accent background. Standard list selection.
- **D-12:** Panel height: **dynamic**, grows up to `max_visible_rows` (default 15), scrollable above that. `max_visible_rows` is configurable in `config.toml`.
- **D-13:** Filter input **auto-focuses** when panel opens. User starts typing immediately without clicking.
- **D-14:** Empty filter state: inline **"No repos match"** message in list area.
- **D-15:** No path shown beyond parent directory. Full path on hover (tooltip) is Claude's discretion.

### Keyboard shortcuts (in panel)
- **D-16:** Arrow keys (Up/Down) — navigate list selection.
- **D-17:** Tab — cycle rows (alternative to arrows).
- **D-18:** Enter — open selected repo in Cursor; panel closes.
- **D-19:** Cmd+Enter (or Cmd+Click) — open in Cursor **without closing panel** (background open for multi-open workflows).
- **D-20:** Esc — close panel.
- **D-21:** Cmd+R — trigger background git state refresh for all repos (spinner shows in filter bar area).

### Repo ordering (Phase 4)
- **D-22:** Default ordering: **dirty repos section first**, then remaining repos by `last_opened_at` descending (most recently opened first).
- **D-23:** No view mode submenu in Phase 4. One ordering only. Submenu deferred to Phase 5+.
- **D-24:** No pinned section in Phase 4. Pins are Phase 5 (ORG-02).

### Schema addition
- **D-25:** Add `last_opened_at INTEGER NULL` column to `repos` table via new migration (e.g. `004_tray.sql`). Updated every time a repo is opened from the tray.

### Git state refresh timing
- **D-26:** On panel open: **show cached SQLite data immediately** (no blocking), then **trigger background refresh of ALL indexed repos** using existing rayon batch from Phase 3.
- **D-27:** Refresh progress indicator: **spinner inside the filter bar area**. Visible while background refresh is running.
- **D-28:** UI updates when background refresh emits completion event (Tauri event → Svelte re-fetches from Rust).

### IPC pattern
- **D-29:** Svelte uses **`invoke()` commands** to fetch initial data and trigger actions. Rust emits **Tauri events** to push updates (e.g. `git-refresh-complete`). Standard Tauri 2 command + event pattern.
- **D-30:** Filtering is **client-side in Svelte** — all repos loaded once into reactive state, filter is `Array.filter()` on the pre-loaded list. No IPC per keystroke.

### Tray icon
- **D-31:** **Dynamic icon**: shows a colored dot badge when one or more indexed repos are dirty. Updated after each background git refresh.
- **D-32:** Right-click context menu items: **Refresh index** (full index run), **Quit Workpot**, **About / version**, **Preferences / config** (opens `config.toml` in default editor).

### Cursor launch
- **D-33:** Launch command is **configurable in `config.toml`** as `launch_cmd`. Default: `cursor --new-window {path}` where `{path}` is replaced with the absolute repo path at runtime.
- **D-34:** On launch failure: **error banner** inside the tray panel (non-modal). Panel stays open. User can fix and retry.
- **D-35:** On successful launch (normal Enter): panel **closes automatically**.
- **D-36:** On Cmd+Enter / Cmd+Click: panel **stays open** (background open — no auto-close).

### Claude's Discretion
- Icon file format and macOS Template image specification (PDF or SVG @ 1x/2x).
- Exact Tailwind color tokens for dirty dot (amber-500/red-500 vs. custom).
- `AppState` ownership in Tauri backend (Arc<Mutex<AppContext>> pattern).
- Exact Tauri event names and payload shapes.
- `tauri-plugin-shell` vs. `std::process::Command` for launch command execution.
- Tooltip implementation for full path on row hover.
- Panel positioning relative to tray icon (below icon, anchored to menu bar).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope & requirements
- `.planning/ROADMAP.md` — Phase 4 goal, success criteria (5 criteria), requirements UI-01..04, SRCH-01..03, LAUNCH-01
- `.planning/REQUIREMENTS.md` — UI-01..04 (tray, list, filter, Cursor open), SRCH-01..03 (fuzzy search, filter-as-you-type, metadata only), LAUNCH-01 (Cursor via CLI)

### Prior phase decisions (load-bearing)
- `.planning/phases/01-core-persistence/01-CONTEXT.md` — AppContext, DB paths, migration pattern, config.toml location
- `.planning/phases/02-repo-discovery/02-CONTEXT.md` — repos schema, cap rules, index behavior
- `.planning/phases/03-git-state/03-CONTEXT.md` — git state columns, D-18 (`refresh_git_state` API), rayon batch refresh, `refresh_and_persist_git_state`

### Existing code to extend
- `crates/workpot-core/src/lib.rs` — `AppContext` public API: `list_repos`, `refresh_git_state`, `refresh_and_persist_git_state`, `run_index`
- `crates/workpot-core/src/services/git_state.rs` — `refresh_all` rayon batch (reuse for background refresh)
- `crates/workpot-core/src/infra/migrations.rs` — migration pattern; add `004_tray.sql` for `last_opened_at`
- `crates/workpot-core/src/domain/repo.rs` — `RepoRecord` struct; extend with `last_opened_at`

### Project constraints
- `.planning/PROJECT.md` — macOS-only v1, Cursor-only IDE, local-only, shared Rust core for CLI + tray

### External reference
- Tauri 2 system tray docs: https://v2.tauri.app/learn/system-tray/ (referenced in CLAUDE.md)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `AppContext::list_repos()` — returns `Vec<RepoRecord>` with branch, is_dirty, ahead, behind, git_refreshed_at; ready to serialize as IPC payload.
- `AppContext::refresh_and_persist_git_state(path)` — single-repo refresh + DB write; exposed for per-repo tray calls.
- `crates/workpot-core/src/services/git_state.rs` `refresh_all` (internal) — rayon parallel batch refresh; surface via a new `AppContext::refresh_all_git_state()` command for the tray's background refresh.
- `AppContext::run_index()` — full index (discovery + git refresh); used for "Refresh index" context menu item.
- `crates/workpot-core/src/infra/migrations.rs` — `rusqlite_migration` pattern; add `004_tray.sql` next.

### Established Patterns
- Path canonicalization before all DB keys — unchanged.
- `WorkpotError` / `anyhow::Context` error boundary — continue in Tauri commands.
- `write_atomic` for config writes — reuse when persisting launch_cmd config changes.
- Integration tests use `open_with_paths` + `tempfile` — Tauri backend tests should do the same.
- Migration files numbered sequentially: `001_init.sql`, `002_discovery.sql`, `003_git_state.sql` → `004_tray.sql`.

### Integration Points
- New `src-tauri/` crate depends on `workpot-core` as a Cargo workspace path dependency.
- Tauri backend wraps `AppContext` in `Arc<Mutex<>>`, exposed via Tauri `State<>`.
- `src/` Svelte app communicates via `@tauri-apps/api` `invoke()` + `listen()`.
- `workpot-cli` remains unchanged; CLI and tray share the same `workpot-core` DB and config.

</code_context>

<specifics>
## Specific Ideas

- User wants the panel to feel like Raycast/Spotlight: type immediately, arrow to navigate, Enter to open. No mouse required.
- "At a glance" means dirty dot + branch visible without scrolling or hovering. Repo name is the primary element.
- `launch_cmd` default is `cursor --new-window {path}` — user explicitly wants `--new-window` so it always opens a fresh Cursor window.
- Cmd+Enter / Cmd+Click = "open in background and keep looking" — useful when opening multiple repos in a single tray session.
- Background refresh always runs on panel open, always shows spinner. No "only refresh if stale X minutes" logic in Phase 4.
- `max_visible_rows` in config.toml — user wants this to be user-configurable (default 15).

</specifics>

<deferred>
## Deferred Ideas

- **Branch navigation sublist** — Right arrow on a repo row opens a branch list; Left arrow goes back. New capability beyond Phase 4 scope. Candidate for Phase 5 or standalone phase.
- **View mode submenu** — Picker in tray panel to switch between default/dirty/recent/pinned sort modes. Phase 5 when priority signals are complete (ORG-01..04).
- **Pinned section in tray list** — "3 last pinned" section at top of list. Requires Phase 5 pin support (ORG-02).
- **Separate `workpot git refresh` CLI command** — Standalone refresh without full index. Deferred from Phase 3; still deferred.
- **Filesystem watcher** — Auto-refresh on git changes without user trigger. Phase 9 / post-v1.

</deferred>

---

*Phase: 4-Tray finder MVP*
*Context gathered: 2026-05-30*
