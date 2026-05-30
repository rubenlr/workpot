# Phase 4: Tray finder MVP - Research

**Researched:** 2026-05-30
**Domain:** Tauri 2 system tray, Svelte + Vite frontend, IPC commands/events, macOS panel UX
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Frontend tech**
- D-01: Svelte + SvelteKit + Vite
- D-02: Tailwind CSS
- D-03: TypeScript
- D-04: System dark/light via `prefers-color-scheme` (no in-app toggle)

**Workspace layout**
- D-05: `src-tauri/` + `src/` at repo root
- D-06: `src-tauri` is a Cargo workspace member with direct `workpot-core` path dependency

**Panel window chrome**
- D-07: Frameless, rounded corners, backdrop blur, shadow
- D-08: Close on focus loss
- D-09: Tray icon click toggles panel

**Repo row layout**
- D-10: Name + colored dirty dot + branch + parent directory
- D-11: Full-width selected row highlight
- D-12: Dynamic height up to `max_visible_rows` (default 15, config.toml)
- D-13: Filter input auto-focus on panel open
- D-14: Empty filter → "No repos match" inline message

**Keyboard shortcuts**
- D-16..D-21: Arrows, Tab, Enter, Cmd+Enter, Esc, Cmd+R refresh

**Repo ordering**
- D-22: Dirty repos first, then `last_opened_at` descending
- D-23: No view mode submenu in Phase 4
- D-24: No pinned section in Phase 4

**Schema**
- D-25: Migration `004_tray.sql` adds `last_opened_at INTEGER NULL` to `repos`

**Git refresh**
- D-26: Cached SQLite on open, background refresh all repos
- D-27: Spinner in filter bar during refresh
- D-28: Tauri event → Svelte re-fetch on completion

**IPC**
- D-29: `invoke()` commands + Tauri events for push updates
- D-30: Client-side filter in Svelte (no IPC per keystroke)

**Tray icon**
- D-31: Dynamic icon with dirty badge dot
- D-32: Right-click menu: Refresh index, Quit, About, Preferences

**Cursor launch**
- D-33: `launch_cmd` in config.toml, default `cursor --new-window {path}`
- D-34: Error banner in panel on failure (non-modal)
- D-35: Normal Enter closes panel on success
- D-36: Cmd+Enter / Cmd+Click keeps panel open

### Claude's Discretion
- Icon format (PDF/SVG template image)
- Tailwind color tokens for dirty dot
- AppState ownership (`Arc<Mutex<AppContext>>`)
- Event names and payload shapes
- `tauri-plugin-shell` vs `std::process::Command` for launch
- Tooltip for full path on hover
- Panel positioning relative to tray icon

### Deferred (OUT OF SCOPE)
- Branch navigation sublist, view mode submenu, pinned section, filesystem watcher, tags/notes search fields (Phase 5)
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| UI-01 | Open Workpot from macOS menu bar / tray | Tauri 2 `TrayIconBuilder` + `tray-icon` feature; left-click toggles webview window |
| UI-02 | Prioritized list with git status summary | `AppContext::list_repos()` already returns branch/is_dirty; add tray-side sort (D-22) + row UI (D-10) |
| UI-03 | Filter list instantly while typing | Client-side reactive filter (D-30); auto-focus input (D-13) |
| UI-04 | Select repo and open in Cursor (Enter/click) | Tauri command + shell execute; keyboard handlers (D-16..D-18) |
| SRCH-01 | Fuzzy-search by name, path, branch (metadata) | Client-side scorer on loaded `RepoRecord[]`; tags/notes deferred (no columns yet) |
| SRCH-02 | Results update as user types | Svelte reactive `$:` derived list bound to filter input |
| SRCH-03 | Metadata only, no code search | All data from SQLite via single `list_repos` load — no ripgrep/Spotlight |
| LAUNCH-01 | Open repo in Cursor via CLI | `launch_cmd` template substitution + `std::process::Command` or `tauri-plugin-shell` |
</phase_requirements>

---

## Summary

Phase 4 introduces the first Tauri 2 binary in the workspace. The architecture is a thin Rust shell over existing `workpot-core`: `AppContext` stays the single data authority; Tauri commands serialize `RepoRecord` to JSON for the Svelte frontend; background git refresh reuses `services::git_state::refresh_all` from Phase 3 inside a `tauri::async_runtime::spawn` task so the webview never blocks.

The recommended scaffold is **Tauri 2 + Vite + SvelteKit (SPA mode, `ssr: false`) + Tailwind** at repo root (`src-tauri/`, `src/`). Add `src-tauri` to workspace `members`. Use `TrayIconBuilder::with_id` for the menu bar icon, `on_tray_icon_event` for left-click toggle (D-09), and a separate `Menu` for right-click context items (D-32). The panel window uses `decorations: false`, `transparent: true`, macOS vibrancy (`window_vibrancy` crate or Tauri window effects), and `WindowEvent::Focused(false)` to hide on focus loss (D-08).

IPC pattern: initial load via `invoke('list_repos')`; mutations via `invoke('open_in_cursor', { path, background })`, `invoke('run_index')`, `invoke('refresh_all_git_state')`; push updates via `app.emit('git-refresh-complete', payload)` listened in Svelte with `@tauri-apps/api/event`. Mutex on `AppContext` is required because commands and background tasks share the SQLite connection.

**Primary recommendation:** Four vertical-slice plans — (1) scaffold + list, (2) filter/keyboard/chrome, (3) background refresh + dirty icon, (4) Cursor launch + context menu. Extend core first (`004_tray.sql`, `last_opened_at`, public `refresh_all_git_state`, `launch_cmd` config) in plan 01 alongside minimal tray+list so every subsequent plan adds user-visible capability.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tauri | 2.x | macOS tray + webview + IPC | Project stack (CLAUDE.md); official tray path |
| @tauri-apps/api | 2.x | Svelte `invoke` / `listen` | Matches Tauri 2 IPC surface |
| svelte | 5.x | UI components | CONTEXT D-01 |
| @sveltejs/kit | 2.x | Vite integration (SPA mode) | CONTEXT D-01 |
| vite | 6.x | Frontend bundler | Tauri default |
| tailwindcss | 4.x | Utility CSS | CONTEXT D-02 |
| typescript | 5.x | Typed IPC payloads | CONTEXT D-03 |

### Rust (src-tauri)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tauri | 2.x + `tray-icon` feature | System tray | Required for UI-01 |
| serde + serde_json | 1.x | IPC serialization | All command return types |
| tauri-plugin-shell | 2.x | Optional: spawn `cursor` CLI | If capabilities model preferred over raw `Command` |
| window-vibrancy | 0.5+ | macOS backdrop blur | D-07 frosted glass (macOS-only v1) |

### Frontend (optional)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| @tauri-apps/plugin-shell | 2.x | JS-side shell if needed | Prefer Rust-side launch for LAUNCH-01 |
| fuse.js or fuzzy-matcher | — | SRCH-01 fuzzy rank | Client-side only; keep bundle small |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| SvelteKit SPA | Plain Svelte + Vite | Simpler but violates D-01 |
| `std::process::Command` | tauri-plugin-shell | Shell plugin adds capability config; Command simpler for fixed `launch_cmd` |
| Rust-side filter IPC | Client filter (chosen) | IPC per keystroke violates D-30 and GIT-04 latency goals |
| Electron menubar | Tauri 2 tray | Heavier RAM; rejected in STACK.md |

---

## Architecture Patterns

### System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│ macOS Menu Bar                                               │
│  TrayIcon ──click──► WebviewWindow (frameless panel)       │
│       │                      │                               │
│       └──right-click──► Context Menu                         │
└──────────────────────────────┼───────────────────────────────┘
                               │ invoke / events
┌──────────────────────────────▼───────────────────────────────┐
│ src-tauri (Tauri commands + tray setup)                      │
│  State<Arc<Mutex<AppContext>>>                               │
│  Commands: list_repos, open_in_cursor, refresh_all_git, ...  │
│  spawn: refresh_all → persist → emit('git-refresh-complete') │
└──────────────────────────────┼───────────────────────────────┘
                               │ path dependency
┌──────────────────────────────▼───────────────────────────────┐
│ workpot-core (unchanged authority)                           │
│  AppContext, catalog, git_state::refresh_all, migrations     │
│  SQLite ~/Library/Application Support/workpot/workpot.db     │
└─────────────────────────────────────────────────────────────┘
```

### Pattern 1: Tauri command wrapping AppContext

```rust
#[tauri::command]
fn list_repos(state: State<'_, Arc<Mutex<AppContext>>>) -> Result<Vec<RepoDto>, String> {
    let ctx = state.lock().map_err(|e| e.to_string())?;
    ctx.list_repos().map_err(|e| e.to_string()).map(|rows| rows.into_iter().map(RepoDto::from).collect())
}
```

Register in `tauri::Builder`: `.manage(Arc::new(Mutex::new(AppContext::open()?)))` in setup.

### Pattern 2: Background refresh without blocking UI

```rust
#[tauri::command]
async fn refresh_all_git_state(app: AppHandle, state: State<'_, Arc<Mutex<AppContext>>>) -> Result<(), String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        let summary = {
            let ctx = state.lock().unwrap();
            ctx.refresh_all_git_state() // new AppContext method
        };
        let _ = app.emit("git-refresh-complete", summary);
    });
    Ok(())
}
```

Call on panel `show` from frontend; show spinner until event received (D-26..D-28).

### Pattern 3: Tray toggle panel

Use `TrayIconEvent::Click` with `MouseButton::Left` — if window visible and focused, hide; else show + set_focus + emit focus to filter input via frontend event `panel-opened`.

### Pattern 4: Client-side filter (D-30)

Load full list once on panel open. Svelte:

```typescript
$: filtered = repos
  .filter(r => fuzzyMatch(filterQuery, r.name, r.path, r.branch))
  .sort(traySort); // dirty first, then last_opened_at desc
```

### Pattern 5: launch_cmd substitution (D-33)

```rust
fn build_launch_command(template: &str, path: &Path) -> Result<Command> {
    let cmdline = template.replace("{path}", &path.display().to_string());
    // parse first token as program, rest as args — or use shell_words
}
```

Default: `cursor --new-window {path}`.

---

## Core API Extensions Required

| Extension | Location | Purpose |
|-----------|----------|---------|
| `004_tray.sql` | `migrations/` | `last_opened_at INTEGER NULL` (D-25) |
| `RepoRecord.last_opened_at` | `domain/repo.rs` | IPC payload + sort key |
| `catalog::touch_last_opened_at` | `services/catalog.rs` | Update on successful open |
| `catalog::list_repos` ORDER BY | SQL or sort in tray command | Support D-22 or sort in DTO layer |
| `AppContext::refresh_all_git_state` | `lib.rs` | Wrap `refresh_all` + batch persist (expose internal batch) |
| `Config.launch_cmd`, `Config.max_visible_rows` | `domain/config.rs` | D-33, D-12 |
| `RepoDto` serde struct | `src-tauri` | Stable IPC JSON (path string, git fields, parent_dir computed) |

---

## macOS-Specific Notes

- **Tray icon template image:** Use `Icon::from_path` with PNG; set `icon_as_template(true)` on macOS for menu bar adaptivity (Claude's discretion — document in plan).
- **Panel position:** `TrayIconEvent` click provides `rect`; position window below tray icon using `window.set_position` (logical coords).
- **Focus loss close:** `on_window_event` → `Focused(false)` → `window.hide()` (D-08).
- **Hide on close:** Prevent app exit when window closes — tray app stays in menu bar.

---

## Pitfalls

1. **SQLite connection across threads:** Only one `Connection` — all DB access must hold `Mutex` lock; background refresh must not hold lock during rayon work (collect paths, release lock, run rayon, re-lock for persist) — same pattern as Phase 3 index second pass.

2. **Tauri + workspace:** `src-tauri/Cargo.toml` needs `workpot-core = { path = "../crates/workpot-core" }`; root `Cargo.toml` adds `"src-tauri"` to members; CI must add frontend build steps.

3. **SvelteKit vs simple popup:** Use `adapter-static` + single route `/` to avoid SSR complexity; disable server-side rendering.

4. **Launch command failures:** Capture stderr from `Command`; surface as string in error banner (D-34). Do not swallow `spawn` errors.

5. **SRCH-01 scope creep:** Tags and notes columns do not exist until Phase 5 — Phase 4 fuzzy match covers `name`, `path`, `branch` only; document in plan acceptance criteria.

6. **Git-04 regression:** Background refresh must stay off main thread; never call `refresh_all` synchronously from a command invoked on UI thread without spawn.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` + Tauri integration tests (limited) + manual macOS UAT |
| Config file | none (Rust); `npm test` optional for Svelte unit tests |
| Quick run command | `cargo test --workspace` |
| Full suite command | `cargo test --workspace && cd src-tauri && npm run check` (after scaffold) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| UI-01 | Tray app builds and registers tray icon | build | `cargo build -p workpot-tray` (crate name TBD) | ❌ Wave 0 |
| UI-02 | list_repos returns branch + is_dirty in DTO | unit | `cargo test -p workpot-core list_repos_last_opened` | ❌ Wave 0 |
| D-25 | Migration 004 adds last_opened_at | unit | `cargo test -p workpot-core migration_004` | ❌ Wave 0 |
| D-33 | launch_cmd default in config | unit | `cargo test -p workpot-core config_launch_cmd` | ❌ Wave 0 |
| LAUNCH-01 | open_in_cursor builds correct argv | unit | `cargo test -p workpot-tray launch_cmd_parse` | ❌ Wave 0 |
| SRCH-01 | fuzzy filter ranks name match | unit (TS) | `npm test -- filter.test.ts` | ❌ Wave 0 |
| GIT-04 | refresh command returns immediately | manual | spawn + spinner UAT | manual |
| UI-04/05 | Cursor failure shows banner | manual | UAT with invalid launch_cmd | manual |

### Sampling Rate
- **Per task commit:** `cargo test --workspace`
- **Per plan wave:** `cargo build -p workpot-tray` + `npm run build` in frontend
- **Phase gate:** Manual macOS tray UAT per ROADMAP success criteria

### Wave 0 Gaps
- [ ] `crates/workpot-core/tests/tray_migration_test.rs` — migration 004, last_opened_at round-trip
- [ ] `src-tauri/src/launch.rs` — launch_cmd parser tests
- [ ] `src/lib/fuzzy.test.ts` — SRCH-01 scorer tests

---

## Security Domain

> Local-only tray app; ASVS L1 applies to input validation on paths and shell command execution.

| Pattern | STRIDE | Mitigation |
|---------|--------|------------|
| Path traversal in open_in_cursor | Tampering | Canonicalize repo path before launch; reject paths not in repos table |
| Shell injection via launch_cmd | Tampering | Parse `launch_cmd` with `shell-words` or fixed token split; do not pass through `/bin/sh -c` unless documented |
| Mutex poison | DoS | Map poison error to user-visible string; avoid panic in commands |

---

## Sources

### Primary (HIGH confidence)
- [Tauri 2 System Tray](https://v2.tauri.app/learn/system-tray/) — TrayIconBuilder, events, tray-icon feature
- [Tauri 2 Calling Frontend](https://v2.tauri.app/develop/calling-frontend/) — emit/listen event pattern
- [workpot-core lib.rs](crates/workpot-core/src/lib.rs) — existing AppContext API
- [Phase 4 CONTEXT.md](04-CONTEXT.md) — locked decisions D-01..D-36

### Secondary (MEDIUM confidence)
- [Tauri 2 window customization](https://v2.tauri.app/learn/window-customization/) — frameless, transparency
- WebSearch: Tauri 2 TrayIconBuilder migration from v1 SystemTray

---

## RESEARCH COMPLETE

**Confidence:** HIGH for architecture and IPC; MEDIUM for exact macOS vibrancy API (verify during plan 02 execution).

**Planner flags:**
- No UI-SPEC.md — CONTEXT.md contains 36 locked UI decisions; sufficient for planning
- MVP mode: 4 vertical slice plans recommended
- Schema change is SQLite migration only (not ORM push gate)
