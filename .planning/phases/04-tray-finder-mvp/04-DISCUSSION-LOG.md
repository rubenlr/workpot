# Phase 4: Tray finder MVP - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-30
**Phase:** 4-Tray finder MVP
**Areas discussed:** Frontend tech, Repo row layout, IPC / data flow, Tray icon, Panel window chrome, Cursor launch, Git state refresh timing

---

## Frontend tech

| Option | Description | Selected |
|--------|-------------|----------|
| Svelte + SvelteKit + Vite | Official Tauri starter, lightweight, best IPC ergonomics | ✓ |
| Vanilla HTML/CSS/JS | Zero build step, simplest setup | |
| React | Familiar ecosystem, heavier bundle | |
| Leptos WASM (Rust) | Full Rust frontend, steep setup cost | |

**User's choice:** Svelte + SvelteKit + Vite

| Option | Description | Selected |
|--------|-------------|----------|
| src-tauri/ + src/ at repo root | Standard Tauri project layout | ✓ |
| crates/workpot-tray | Cargo workspace member with friction vs Tauri CLI | |
| apps/tray/ | Monorepo-style separate directory | |

**User's choice:** Standard src-tauri/ + src/ layout, added as Cargo workspace member

| Option | Description | Selected |
|--------|-------------|----------|
| Tailwind CSS | Utility-first, tiny purged bundle | ✓ |
| Plain CSS / scoped Svelte styles | Zero dependencies, more verbose | |

**User's choice:** Tailwind CSS

| Option | Description | Selected |
|--------|-------------|----------|
| TypeScript | Type-safe IPC, TS types from @tauri-apps/api | ✓ |
| Plain JavaScript | Fewer config files, no type checking | |

**User's choice:** TypeScript

| Option | Description | Selected |
|--------|-------------|----------|
| System theme (prefers-color-scheme) | Follow macOS system preference | ✓ |
| Dark mode only | Simpler, single theme | |

**User's choice:** System dark/light via CSS prefers-color-scheme

**Notes:** User worked through this area in detail. All choices aligned with recommended options.

---

## Repo row layout

| Option | Description | Selected |
|--------|-------------|----------|
| Name + dirty dot + branch | Clean, scannable | ✓ |
| Name + branch + ahead/behind + dirty | Full git state, denser | |
| Name only | Minimal, git state on hover only | |

**User's choice:** Name + colored dirty dot + branch

| Option | Description | Selected |
|--------|-------------|----------|
| Colored dot | Instant visual scan | ✓ |
| Asterisk (*) suffix | Terminal convention | |

**User's choice:** Colored dot (amber/red = dirty, green = clean)

| Option | Description | Selected |
|--------|-------------|----------|
| No path — name is enough | Clean, minimal | |
| Parent directory (~/c/myrepo) | Disambiguates same-name repos | ✓ |
| Full path always | Most explicit, cluttered | |

**User's choice:** Parent directory shown per row

| Option | Description | Selected |
|--------|-------------|----------|
| Full-width highlight | Standard list selection, clear focus | ✓ |
| Left accent bar | More subtle | |

**User's choice:** Full-width highlight

| Option | Description | Selected |
|--------|-------------|----------|
| Fixed height ~10 rows | Predictable | |
| Dynamic up to 15 rows, configurable | Adapts to list size | ✓ |

**User's choice:** Dynamic height up to 15 rows (configurable via `max_visible_rows` in config.toml)

| Option | Description | Selected |
|--------|-------------|----------|
| 'No repos match' inline | Simple, clear | ✓ |
| 'Try a shorter query' hint | More helpful | |

**User's choice:** Inline "No repos match" message

**Keyboard shortcuts — User's choice:**
- Arrow keys: navigate list
- Tab: cycle rows
- Esc: close panel
- Cmd+R: trigger background git refresh
- Right/Left arrow branch navigation: **deferred** (user raised this; agreed to defer)
- Enter: open in Cursor + close panel
- Cmd+Enter / Cmd+Click: open in Cursor + **keep panel open**

| Option | Description | Selected |
|--------|-------------|----------|
| Dirty first + alphabetical | Simple, predictable | |
| Recently opened first | Recency-based | |
| Alphabetical only | Simple sort | |
| Sectioned with submenu | User's initial vision | |

**User's choice:** Dirty section first, then rest by `last_opened_at` descending. No submenu in Phase 4. User initially described a multi-section view with submenu but agreed to simplify for Phase 4.

| Option | Description | Selected |
|--------|-------------|----------|
| Skip pinned section (Phase 5) | Keeps Phase 4 clean | ✓ |
| Add minimal pin schema | Schema without UI | |
| Pull Phase 5 forward | Expands scope | |

**User's choice:** Skip pinned section in Phase 4

| Option | Description | Selected |
|--------|-------------|----------|
| Add last_opened_at column | Enables recency sorting now | ✓ |
| Defer to Phase 5 | All signals together | |

**User's choice:** Add `last_opened_at INTEGER NULL` to repos via migration 004_tray.sql

| Option | Description | Selected |
|--------|-------------|----------|
| Always auto-focus filter | Launcher UX | ✓ |
| Focus list first | Less common | |

**User's choice:** Always auto-focus filter input on panel open

---

## Git state refresh timing

| Option | Description | Selected |
|--------|-------------|----------|
| Show cached + background refresh | Best perceived performance | ✓ |
| Block on open, then show fresh | Adds open latency | |
| Manual only (Cmd+R) | Fastest open, stalest data | |

**User's choice:** Show cached data immediately, trigger background refresh of all repos

| Option | Description | Selected |
|--------|-------------|----------|
| All indexed repos in background | Complete, uses existing rayon batch | ✓ |
| Only visible repos | Faster, non-visible stay stale | |

**User's choice:** All indexed repos; always show visual spinner indicator during refresh

| Option | Description | Selected |
|--------|-------------|----------|
| Spinner in filter bar area | Unobtrusive, no layout shift | ✓ |
| Tray icon animation | Visible when panel closed | |
| Status text at bottom | Footer bar | |

**User's choice:** Spinner in filter bar area

---

## IPC / data flow

| Option | Description | Selected |
|--------|-------------|----------|
| Tauri commands (invoke) + events | Standard Tauri 2 pattern | ✓ |
| Polling on timer | Simple but wastes IPC | |

**User's choice:** `invoke()` commands for initial data + Tauri events for push updates

| Option | Description | Selected |
|--------|-------------|----------|
| Client-side filtering in Svelte | Sub-millisecond, no IPC per keystroke | ✓ |
| Server-side invoke per keystroke | Rust-side FTS, overkill for Phase 4 | |

**User's choice:** Client-side Array.filter() in Svelte

---

## Tray icon

| Option | Description | Selected |
|--------|-------------|----------|
| Dynamic icon with dirty dot badge | At-a-glance dirty signal | ✓ |
| Static icon always | Simpler | |

**User's choice:** Dynamic icon — dot badge when any indexed repo is dirty

**Right-click context menu — User's choice:** Refresh index, Quit Workpot, About / version, Preferences / config (all 4 selected)

---

## Panel window chrome

| Option | Description | Selected |
|--------|-------------|----------|
| Frameless, rounded corners, backdrop blur | Modern macOS popup feel | ✓ |
| Frameless, flat (no blur) | Solid background, no performance cost | |
| Native window with title bar | Easiest to set up | |

**User's choice:** Frameless, rounded corners, backdrop blur (frosted glass)

| Option | Description | Selected |
|--------|-------------|----------|
| Close on focus loss | Standard macOS tray convention | ✓ |
| Stay open until Esc/tray click | Persistent panel | |

**User's choice:** Close on focus loss

| Option | Description | Selected |
|--------|-------------|----------|
| Toggle: second click closes | Standard macOS tray | ✓ |
| Always open fresh | Non-standard | |

**User's choice:** Toggle (second tray icon click closes panel)

**Additional:** User added Cmd+Enter / Cmd+Click to open without closing (background multi-open).

---

## Cursor launch

**User's choice:** Configurable via `launch_cmd` in config.toml. Default: `cursor --new-window {path}`. User explicitly wanted `--new-window` to always open a fresh window.

| Option | Description | Selected |
|--------|-------------|----------|
| Error banner in panel | Non-modal, panel stays open | ✓ |
| macOS notification | System notification | |

**User's choice:** Error banner inside tray panel

| Option | Description | Selected |
|--------|-------------|----------|
| Close automatically on success | Standard launcher behavior | ✓ |
| Keep open | Multi-repo workflow | |

**User's choice:** Close automatically on normal Enter; stay open on Cmd+Enter

---

## Claude's Discretion

- Icon file format and macOS Template image specification
- Exact Tailwind color tokens for dirty dot
- `AppState` ownership pattern in Tauri backend
- Tauri event names and payload shapes
- `tauri-plugin-shell` vs `std::process::Command` for launch
- Tooltip implementation for full path on hover
- Panel positioning relative to tray icon

## Deferred Ideas

- Branch navigation sublist (right/left arrow drill-down) — user raised, agreed to defer
- View mode submenu (default/dirty/recent/pinned) — user's initial vision, simplified for Phase 4
- Pinned section at top of list — requires Phase 5 pin support (ORG-02)
- Separate `workpot git refresh` CLI command — still deferred from Phase 3
- Filesystem watcher for auto-refresh — Phase 9 / post-v1
