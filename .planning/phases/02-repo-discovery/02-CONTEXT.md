# Phase 2: Repo discovery - Context

**Gathered:** 2026-05-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Automatically find git repositories under configurable watch roots, with manual add and exclude control (INDEX-01..INDEX-05). Deliver filesystem discovery, config-driven watch roots, exclude globs, `workpot index` rescan, and persistence merge rules. No per-repo git branch/dirty/ahead-behind (Phase 3), no tray UI (Phase 4), no filesystem watcher (on-demand scan only). No repo+branch launch or clone flows (later phases).

</domain>

<decisions>
## Implementation Decisions

### Discovery traversal
- **D-01:** Skip nested `.git` inside an already-indexed repository tree (reduces submodule noise under a parent checkout).
- **D-02:** Index both bare repositories and normal worktrees; symlinked directories are not followed during walks.
- **D-03:** Sibling worktrees outside the parent tree are discovered as separate paths; all worktree paths are indexed (one logical repo, multiple rows/paths).
- **D-04:** Bare repos: persist a row for the bare path **and** separate rows for each linked worktree path.

### Repo identity & grouping
- **D-05:** Stable grouping key is canonical absolute `git_common_dir` (not user-facing path alone) — paths may move/rename; directory paths are not the long-term identity.
- **D-06:** Phase 2 may keep `path` as row lookup/unique key per checkout; link rows sharing the same `git_common_dir`. Planner to define schema migration from Phase 1 path-only model.
- **D-07:** If a repo directory moves: treat as missing on rescan (auto-remove stale path) and new discovery creates a new row — no automatic path re-key in Phase 2.

### Exclude semantics
- **D-08:** Config `excludes` entries are **glob patterns** matched during discovery walks.
- **D-09:** Ship built-in default exclude globs (e.g. `node_modules`, `.git` internals, `.Trash`, common build dirs) in addition to user config.
- **D-10:** `workpot repo remove <path>` deletes the DB row **and** appends a parent+name exclude glob to config (e.g. `{parent}/foo/**`) so rescan will not re-add; user edits config or uses excludes CLI to undo.
- **D-11:** Manual `workpot repo add` is allowed even when path matches an exclude glob (manual overrides scan-only excludes).
- **D-12:** `workpot excludes list|remove` manages exclude globs in Phase 2 (not config-only).

### Rescan & `workpot index`
- **D-13:** Top-level command `workpot index` performs full watch-root discovery (INDEX-05).
- **D-14:** On rescan, rows with `source=manual` stay manual when also under a watch root.
- **D-15:** Paths that no longer exist on disk are auto-removed during index (manual and scan).
- **D-16:** Manual-only repos outside watch roots are not discovered anew but are **validated** on index (still exists, still git).
- **D-17:** Default output: one summary line (counts added/removed/skipped); persist **index run history** (one row per scan) and a **per-scan change log** (added/removed/skipped paths with full directory). Git “update” deltas deferred to Phase 3.
- **D-18:** When repo cap is reached during scan: **stop with error**, exit code 1 (no partial index).

### Watch roots (INDEX-01)
- **D-19:** `workpot roots add|list|remove` plus hand-editing `config.toml` both supported; reload on next open/index.
- **D-20:** `workpot roots add` triggers an immediate scan of that root.
- **D-21:** `workpot roots remove` prunes all indexed repos under that root by default; `--skip-prune` keeps existing rows.
- **D-22:** No practical cap on watch-root count for normal use; enforce **max watch roots: 100 default**, configurable up to **5000 hard max** (security / abuse guard).
- **D-23:** **Max indexed repos: 1000 default**, configurable up to **20000 hard max**; same rationale.

### Limits & security
- **D-24:** Hard caps are not expected in normal use; they exist to prevent pathological scans/memory use.

### Claude's Discretion
- Exact built-in default glob list; walk implementation (`walkdir` vs alternatives); schema details for `git_common_dir`, scan history tables, and how `repo_git_id` is exposed if distinct from `git_common_dir`; validation rules for config limit fields.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope & requirements
- `.planning/ROADMAP.md` — Phase 2 goal, success criteria, INDEX-01..05
- `.planning/REQUIREMENTS.md` — INDEX-01..05 definitions

### Stack & architecture
- `.planning/research/ARCHITECTURE.md` — Discovery service, walkdir + `.git` detect, catalog merge, anti-patterns (no `$HOME` watch)
- `.planning/research/PITFALLS.md` — Nested git/worktree pitfalls, exclude defaults, scan bounds
- `.planning/research/STACK.md` — notify/walkdir noted for later; Phase 2 is scan-on-demand

### Prior phase decisions
- `.planning/phases/01-core-persistence/01-CONTEXT.md` — macOS config/data paths, lazy bootstrap, `repos` table with `source` and `excluded`

### Project constraints
- `.planning/PROJECT.md` — watch roots + manual add/exclude, local-only, macOS v1

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/workpot-core/src/domain/config.rs` — `watch_roots`, `excludes` fields ready for Phase 2.
- `crates/workpot-core/src/services/catalog.rs` — `register_manual`, `list_repos`, `remove_repo`; git worktree/bare detection (`is_git_worktree`, `is_bare_repo`).
- `crates/workpot-core/src/infra/migrations/001_init.sql` — `repos` with `source` (`manual`|`scan`), `excluded` flag.
- `crates/workpot-cli/src/main.rs` — `workpot paths`, `workpot repo add|list|remove` wiring via `AppContext`.

### Established Patterns
- Path canonicalization before DB keys; human errors via `WorkpotError`.
- Phase 1 path-as-PRIMARY-KEY — Phase 2 introduces `git_common_dir` grouping (migration required).

### Integration Points
- New services: discovery walk, index orchestration, roots/excludes CLI subcommands.
- Config load/save path unchanged (`~/.config/workpot/config.toml`).

</code_context>

<specifics>
## Specific Ideas

- Multiple watch roots for organizational separation (GitHub, GitLab, employer, OSS, personal) — symlinks skipped; literal roots only.
- Future product modes: **bare** vs **normal** (affects clone target layout, worktree defaults, launch); Phase 2 indexes both; launch is repo + branch with all worktrees available when present.
- Future bare worktree layout: `{bare}.git/wt/{name}`; for normal repos Workpot does not create worktrees in v1 — index/remove only.
- First-run suggestion: scan `~` non-hidden directories depth ≤2 and let user pick roots — **deferred past Phase 2** (ship roots CLI + manual config first).
- User wants stable git identity beyond paths because directories are renamed/deleted; `git_common_dir` chosen as grouping key.

</specifics>

<deferred>
## Deferred Ideas

### Out of phase scope (capture for roadmap)
- Interactive first-run watch-root wizard (`~` depth-2 scan + multi-select) — after roots CLI lands.
- Filesystem watcher (`notify`) for automatic re-index — architecture Phase 9 / post-v1 tray freshness.
- Bare vs normal **mode** switching, clone-as-bare/normal, repo+branch **launch** UX — Phases 3–4+.
- Worktree **creation** by Workpot (`barerepo.git/wt/{name}` convention) — not Phase 2.
- Index change log “update” rows for git ahead/behind — Phase 3 git snapshot.
- Automatic path re-key on directory move (same `git_common_dir`) — user chose stale-remove + rediscover for Phase 2.

</deferred>

---

*Phase: 2-Repo discovery*
*Context gathered: 2026-05-29*
