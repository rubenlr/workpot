# Phase 1: Core & persistence - Context

**Gathered:** 2026-05-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Establish `workpot-core`, SQLite schema with migrations, and local config so CLI (and later tray) share one data layer. Phase 1 proves persistence: config read/write, DB creation/migration, and manual registration of at least one git repo path that survives restart. No Tauri tray, no filesystem watch discovery, no git status refresh, no network (DATA-01, DATA-02).

</domain>

<decisions>
## Implementation Decisions

### macOS paths (config vs data)
- **D-01:** Primary config file at `~/.config/workpot/config.toml` (XDG-style; dotfile-friendly, terminal-editable).
- **D-02:** SQLite database at `~/Library/Application Support/workpot/workpot.db` (Apple-native app data location).
- **D-03:** No environment-variable path overrides in Phase 1 — fixed default paths only; defer `WORKPOT_HOME` / `WORKPOT_DATA_DIR` until integration tests or power-user needs justify it.
- **D-04:** First launch creates both artifacts: default `config.toml` (documented empty `watch_roots`, sensible defaults) and empty DB with migrations applied — no explicit `workpot init` gate required.

### Claude's Discretion
- Workspace bootstrap depth (core+CLI only vs empty Tauri stubs), Phase 1 CLI command set, and initial `repos` table column scope — not discussed; follow `.planning/research/ARCHITECTURE.md` Phase 1 slice and ROADMAP success criteria unless planner flags conflict.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope & requirements
- `.planning/ROADMAP.md` — Phase 1 goal, success criteria, DATA-01/DATA-02 mapping
- `.planning/REQUIREMENTS.md` — DATA-01 (local persistence), DATA-02 (no network)

### Stack & architecture
- `.planning/research/STACK.md` — `directories` crate, TOML config, rusqlite bundled, path conventions
- `.planning/research/ARCHITECTURE.md` — `workpot-core` layout, SQLite as materialized view, path-as-id, Phase 1 build order
- `.planning/research/PITFALLS.md` — avoid single JSON config, corruption/migration pitfalls

### Project constraints
- `.planning/PROJECT.md` — local-only, macOS v1, shared core for CLI + tray

</canonical_refs>

<code_context>
## Existing Code Insights

Greenfield repository — no application code yet. Planning artifacts only.

### Reusable Assets
- None yet.

### Established Patterns
- Target layout documented in `.planning/research/ARCHITECTURE.md` (`crates/workpot-core`, `crates/workpot-cli`).

### Integration Points
- Phase 1 CLI is the sole host; Tauri integrates in Phase 4 using the same `AppContext` and paths.

</code_context>

<specifics>
## Specific Ideas

User explicitly chose split-by-role paths: config under `~/.config/workpot`, database under Application Support — aligns with STACK.md recommendation and keeps dotfile workflow separate from bulk app data.

</specifics>

<deferred>
## Deferred Ideas

### Not discussed (planner may decide from research)
- Workspace bootstrap: core+CLI only vs include empty `src-tauri` / `ui` stubs in Phase 1
- Phase 1 CLI verbs beyond what success criteria require
- Initial SQLite schema width (minimal vs forward-compatible nullable columns)
- Env-based path overrides (explicitly deferred past Phase 1 per D-03)

</deferred>

---

*Phase: 1-Core & persistence*
*Context gathered: 2026-05-28*
