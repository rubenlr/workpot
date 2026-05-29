# Phase 1: Core & persistence - Context

**Gathered:** 2026-05-29 (updated post-implementation)
**Status:** Complete — retroactive decisions locked; UAT is completion gate

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
- **D-04:** First launch creates both artifacts: default `config.toml` and empty DB with migrations applied — no explicit `workpot init` gate; `workpot paths` triggers lazy bootstrap.

### Workspace & CLI (retroactive — locked 2026-05-29)
- **D-05:** Cargo workspace is **core + CLI only** in Phase 1; no Tauri/ui stubs until Phase 4.
- **D-06:** Phase 1 CLI surface is minimal: `workpot paths` and `workpot repo add|list|remove` only.
- **D-07:** `repos` schema locked forward-compatible: `path` PK, `name`, `registered_at`, `source` (`manual`|`scan`), `excluded`; Phase 2 adds `git_common_dir` via migration (see Phase 2 context D-06).
- **D-08:** Git validation is filesystem-only (`.git` dir/file with HEAD, or bare `HEAD`+`objects`); **no git2** until Phase 3.

### Bootstrap & config defaults
- **D-09:** First-run `config.toml` seeds `watch_roots` with **existing** `~/code` and `~/dev` (only directories that exist at bootstrap); `excludes` starts empty.
- **D-10:** Production hosts use **`AppContext::open()`** only (config + DB + migrations); `AppContext::open_with_paths` is for tests — hosts do not open raw `Connection`.
- **D-11:** CLI surfaces human-readable errors via **`anyhow` context** on typed `WorkpotError`; stable exit-code taxonomy deferred.
- **D-12:** DATA-02 enforced by **macOS CI** (`cargo test --offline` + dependency policy in workflow); standalone `scripts/check-no-network-deps.sh` is optional tooling, not a locked contract.

### Catalog semantics
- **D-13:** Repo row identity in Phase 1 is **canonical absolute path string** (always `canonicalize` on register/remove); path remains lookup key until Phase 2 adds `git_common_dir`.
- **D-14:** Registration accepts **normal worktrees and bare repos** (current `catalog.rs` rules).
- **D-15:** Phase 1 `repo remove` is **hard DELETE**; exclude-on-remove is Phase 2 (see `02-CONTEXT.md` D-10).

### UAT & phase completion
- **D-16:** `01-UAT.md` test 2 expects **config under `~/.config/workpot`**, not Application Support (aligns with D-01).
- **D-17:** UAT asserts **seeded `watch_roots`** when `~/code` / `~/dev` exist on the machine under test.
- **D-18:** Path tests use **exact prefixes** printed by `workpot paths` (document expected home-expanded paths).
- **D-19:** Phase 1 is **not complete** until all UAT tests pass (code-complete is insufficient).

### Folded Todos

(none)

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

### Phase 2 handoff (read before discovery work)
- `.planning/phases/02-repo-discovery/02-CONTEXT.md` — discovery, excludes, `git_common_dir`, index merge

### Implementation artifacts (Phase 1 as-built)
- `crates/workpot-core/src/lib.rs` — `AppContext`, `default_config`
- `crates/workpot-core/src/infra/paths.rs` — D-01/D-02 path resolution
- `crates/workpot-core/src/infra/migrations/001_init.sql` — D-07 schema
- `crates/workpot-core/src/services/catalog.rs` — D-13–D-15 catalog rules
- `crates/workpot-cli/src/main.rs` — D-06 CLI surface
- `.planning/phases/01-core-persistence/01-UAT.md` — D-16–D-19 acceptance tests

</canonical_refs>

<code_context>
## Existing Code Insights

Phase 1 shipped; greenfield scaffold is now a two-crate workspace.

### Reusable Assets
- `AppContext` — single open/bootstrap path for CLI and future Tauri host.
- `default_config(home)` — first-run watch root seeding (D-09).
- `catalog::{register_manual, list_repos, remove_repo}` — manual CRUD; Phase 2 extends with scan merge.
- `infra::migrations` — rusqlite_migration v1; `001_init.sql` repos table.

### Established Patterns
- Path canonicalization before all repo DB keys.
- `WorkpotError` in core; `anyhow::Context` at CLI boundary (D-11).
- Integration tests use `open_with_paths` + tempfile (never production paths).

### Integration Points
- Phase 2 consumes `watch_roots`, `excludes`, `source`, `excluded` columns — no Phase 1 schema narrowing.
- Phase 4 Tauri reuses `AppContext::open()` and same paths (D-01, D-02).

</code_context>

<specifics>
## Specific Ideas

- Split-by-role paths unchanged: config dotfile vs Application Support DB (2026-05-28).
- Post-ship update (2026-05-29): lock planner-discretion items as contracts; seed `~/code` + `~/dev` on first run when present; UAT must pass before calling Phase 1 done.

</specifics>

<deferred>
## Deferred Ideas

- Env-based path overrides (`WORKPOT_HOME`, `WORKPOT_DATA_DIR`) — after integration-test need (D-03).
- Structured CLI exit codes / error taxonomy — when automation needs it (D-11).
- Tauri/ui workspace members — Phase 4 (D-05).
- Filesystem watch / `workpot index` — Phase 2.
- git2 status refresh — Phase 3.
- Exclude-on-remove — Phase 2 (`02-CONTEXT.md` D-10); Phase 1 stays hard DELETE (D-15).

</deferred>

---

*Phase: 1-Core & persistence*
*Context gathered: 2026-05-28; updated 2026-05-29*
