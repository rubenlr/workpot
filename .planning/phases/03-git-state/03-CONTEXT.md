# Phase 3: Git state - Context

**Gathered:** 2026-05-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Deliver trustworthy per-repo git state — branch name, dirty flag, and ahead/behind counts — stored in SQLite and refreshed via `workpot index`. Introduces `git2` (libgit2) as the git read layer, replacing all existing subprocess calls. Exposes a `refresh_git_state(path)` core API for Phase 4 tray to call per-repo. No tray UI, no search/filter, no filesystem watcher. Discovery behavior from Phase 2 is unchanged except that `workpot index` now runs git refresh as a second pass after discovery.

</domain>

<decisions>
## Implementation Decisions

### git2 adoption
- **D-01:** Add `git2` crate with `bundled` feature to `workpot-core` — no system libgit2 required, hermetic CI build.
- **D-02:** Migrate ALL existing subprocess calls in `crates/workpot-core/src/infra/git.rs` to git2 equivalents — no `Command::new("git")` remains in the core crate after Phase 3.
- **D-03:** Use `rayon` for parallel git2 repository opens across all indexed repos — required to meet success criterion 4 (<500ms for 50+ repos).
- **D-04:** Repos with no configured upstream → omit ahead/behind from display (store `ahead = NULL`, `behind = NULL`); do not show a "—" placeholder.

### Schema: git state columns on repos table
- **D-05:** Add new columns to `repos` via a migration (e.g. `003_git_state.sql`): `branch TEXT`, `is_dirty INTEGER`, `ahead INTEGER NULL`, `behind INTEGER NULL`, `git_refreshed_at INTEGER NULL`, `git_state_error TEXT NULL`.
- **D-06:** Repos where `git_refreshed_at IS NULL` (never refreshed) → display `?` for branch/dirty/ahead-behind in list output.
- **D-07:** `workpot repo list` shows a staleness age indicator alongside git state (e.g. `branch=main dirty=yes  5m ago`).
- **D-08:** Stale path removal (Phase 2 D-15) unchanged — row is deleted entirely; no tombstone state.
- **D-09:** `git_state_error TEXT NULL` captures the last failure reason when a repo fails to refresh; surfaced in list output for that repo.

### Dirty detection scope
- **D-10:** `is_dirty` = true iff the repo has staged or unstaged changes to tracked files (git2 `INDEX_*` + `WT_MODIFIED` flags on tracked paths). Untracked files are excluded.
- **D-11:** Repos with only untracked files → show as clean (no secondary indicator). One binary `is_dirty` flag only.
- **D-12:** git2 status checks respect `.gitignore` (default git2 behavior — ignored files never contribute to dirty).
- **D-13:** Bare repos → skip dirty check entirely; store `is_dirty = NULL`, display `N/A` in list.
- **D-14:** Each worktree path row gets its own `is_dirty` check independently (aligns with Phase 2 per-path indexing).

### Refresh trigger
- **D-15:** Git state refresh is piggybacked on `workpot index`: discovery walk + DB merge runs first (existing behavior), then git2 parallel refresh of all indexed repos runs as a second pass in the same command.
- **D-16:** Git refresh continues on individual repo failure — store error in `git_state_error`, refresh remaining repos (do not abort the batch).
- **D-17:** `workpot index` output includes git refresh stats: e.g. `42 added, 0 removed / git: 47 refreshed, 2 errors`.
- **D-18:** Phase 3 ships a `refresh_git_state(path: &Path) -> Result<GitState>` function in `workpot-core` for Phase 4 tray to call on a single repo without running a full index.

### Claude's Discretion
- Exact rayon thread pool sizing (default rayon pool is fine for this use case).
- git2 `StatusOptions` bitfield composition (implement using `INCLUDE_UNTRACKED=false`, `RECURSE_UNTRACKED_DIRS=false`, `EXCLUDE_SUBMODULES=true`).
- Exact migration file name and number.
- Output format details for the age indicator (relative time string).
- Whether `list_worktree_paths` uses git2 `worktrees()` API or the parsed porcelain format.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope & requirements
- `.planning/ROADMAP.md` — Phase 3 goal, success criteria (4 criteria), GIT-01..04
- `.planning/REQUIREMENTS.md` — GIT-01..04 definitions (branch, dirty, ahead/behind, background refresh)

### Prior phase decisions (load-bearing)
- `.planning/phases/01-core-persistence/01-CONTEXT.md` — AppContext, DB paths, migration pattern, D-08 (no git2 until Phase 3)
- `.planning/phases/02-repo-discovery/02-CONTEXT.md` — repos schema, git_common_dir grouping (D-05/06), worktree rows (D-03/04), index behavior (D-13..18), cap rules (D-18/D-23)

### Existing code to migrate / extend
- `crates/workpot-core/src/infra/git.rs` — existing subprocess functions; ALL must be replaced with git2
- `crates/workpot-core/src/domain/repo.rs` — `RepoRecord` struct; needs new git state fields
- `crates/workpot-core/src/infra/migrations/002_discovery.sql` — current schema reference
- `crates/workpot-core/src/services/index.rs` — index orchestration to extend with git refresh pass
- `crates/workpot-core/src/services/catalog.rs` — `list_repos` to return git state columns

### Project constraints
- `.planning/PROJECT.md` — local-only, macOS v1, shared core for CLI + tray

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/workpot-core/src/infra/git.rs` — `resolve_git_common_dir`, `list_worktree_paths`; both use subprocess today, both migrate to git2.
- `crates/workpot-core/src/services/index.rs` — existing index orchestration (discovery + DB merge); git refresh adds a second pass after this.
- `crates/workpot-core/src/infra/migrations.rs` — `rusqlite_migration` pattern; add `003_git_state.sql` as next step.
- `crates/workpot-core/src/domain/repo.rs` — `RepoRecord` struct; extend with `branch`, `is_dirty`, `ahead`, `behind`, `git_refreshed_at`, `git_state_error`.
- `crates/workpot-core/src/services/catalog.rs` — `list_repos`; update SELECT to include new columns.

### Established Patterns
- Path canonicalization before all DB keys (D-13 in Phase 1) — unchanged.
- `WorkpotError` in core; `anyhow::Context` at CLI boundary — continue.
- Integration tests use `open_with_paths` + `tempfile` — git2 tests should use `git2::Repository::init` on tempdir.
- Migration files numbered sequentially: `001_init.sql`, `002_discovery.sql` → `003_git_state.sql`.

### Integration Points
- `workpot index` CLI command (in `crates/workpot-cli/src/main.rs`) needs output updated to show git refresh stats (D-17).
- `workpot repo list` output updated to show branch, dirty, ahead/behind, age, and error fields.
- Phase 4 will call `refresh_git_state(path)` from the Tauri backend — API must be in `workpot-core` public surface.

</code_context>

<specifics>
## Specific Ideas

- User wants `workpot index` to be the one command that does both discovery and git state — no separate `workpot git refresh` in Phase 3.
- Age indicator on list: user explicitly wants staleness visible (e.g. `5m ago`) so they know when data was last refreshed.
- Error transparency: user wants per-repo git errors visible in list output, not silently dropped.
- Phase 4 contract: `refresh_git_state(path)` must exist in core by end of Phase 3 so Phase 4 can build on it.

</specifics>

<deferred>
## Deferred Ideas

- Separate `workpot git refresh` / `workpot git sync` standalone command — user chose to piggyback on `workpot index` for now; can be added later if needed.
- Filesystem watcher (`notify`) for automatic git state re-index — Phase 9 / post-v1.
- Ahead/behind with "—" placeholder for no-upstream repos — user chose omit (NULL) over explicit dash; can revisit in Phase 4 UI design.
- Structured git stats in index history table (`index_runs`) — current table tracks discovery only; git refresh counts are CLI output only in Phase 3.

</deferred>

---

*Phase: 3-Git state*
*Context gathered: 2026-05-29*
