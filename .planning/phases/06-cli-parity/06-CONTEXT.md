# Phase 6: CLI parity - Context

**Gathered:** 2026-05-31
**Status:** Ready for planning

<domain>
## Phase Boundary

Add three top-level CLI commands that mirror the tray's data and ordering: `workpot list` (priority-ordered repo display), `workpot search <query>` (fuzzy filter with same algorithm as tray), and `workpot open <name|path>` (launch Cursor for a matched repo). No pin/unpin CLI. No interactive TUI. No #tag filter syntax in search.

**Requirements:** CLI-01, CLI-02, CLI-03

**Success Criteria (from ROADMAP.md):**
1. `workpot list` shows the same repos and order as the tray default view
2. `workpot search <query>` returns the same results as tray filter
3. `workpot open <name|path>` opens Cursor for the matched repo

</domain>

<decisions>
## Implementation Decisions

### `workpot list` output format
- **D-01:** `workpot list` is a new **top-level command** (not `workpot repo list`). Flat ordered list — no section headers. Priority order: Pinned > Dirty > Recent > Rest.
- **D-02:** Each row starts with an emoji priority icon: 📌 = pinned, 🟡 = dirty, 🔥 = recent, rest = Claude's discretion (suggest ⬜ or a space/dot).
- **D-03:** Row format: `[icon] [parent_dir] [name] [branch] [tags]` — parent directory only (e.g. `~/c`), not full path. Tags shown inline (space-separated if multiple).
- **D-04:** Emoji icons enabled — macOS-only v1, all modern macOS terminals support them.

### `workpot search <query>` behavior
- **D-05:** Print-only (filter-and-exit). Filters repos by query, prints matches in the same priority order and row format as `workpot list`. Composable with pipes.
- **D-06:** Uses the **same fuzzy algorithm as the tray** (nucleo or fuzzy-matcher crate already in workpot-core). Results must match the tray for the same query.
- **D-07:** Text search only — no `#tag` filter syntax in CLI search. Tag-based filtering stays tray-only.

### `workpot open <name|path>` behavior
- **D-08:** Uses `resolve_repo_identifier()` (existing) for name/path/key matching.
- **D-09:** On **ambiguous match** (multiple repos share the same name): error with numbered list of matching paths and instruction to use the full path. Exit 1.
- **D-10:** On **success**: print `opening: /path/to/repo` then exit 0. Uses `launch_cmd` from config (default: `cursor --new-window {path}`).
- **D-11:** On **no match**: error `repo not found: <identifier>`. Exit 1.

### Pin/unpin CLI
- **D-12:** Out of scope for Phase 6. `workpot pin` / `workpot unpin` will not ship. Pin management stays tray-only in v1.

### Claude's Discretion
- Rest-section emoji icon (suggest ⬜ or `·` — subtle, clearly "nothing special").
- Exact column spacing / padding in output rows.
- Whether tags are shown in brackets or plain (e.g. `[backend api]` vs `backend api`).
- Fuzzy-matcher crate selection (nucleo vs fuzzy-matcher) — whichever is already in workpot-core's Cargo.toml.
- Exit code for launch failure in `workpot open` (suggest exit 2 to distinguish from "not found" exit 1).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope & requirements
- `.planning/ROADMAP.md` — Phase 6 goal, success criteria (3 criteria), requirements CLI-01..03
- `.planning/REQUIREMENTS.md` — CLI-01 (list), CLI-02 (search+open), CLI-03 (consistency with tray)

### Existing CLI to extend
- `crates/workpot-cli/src/main.rs` — **CRITICAL**: existing Commands enum, `resolve_repo_identifier()`, `RepoCommands::List` handler — new `list`/`search`/`open` commands add to this file
- `crates/workpot-cli/src/git_display.rs` — `format_git_state()` — may be extended or reused for the new row format

### Core API and ordering logic
- `crates/workpot-core/src/services/catalog.rs` — `list_repos()` return type + ordering; tray-parity ordering (Pinned > Dirty > Recent > Rest) must be implemented here or in CLI layer
- `crates/workpot-core/src/domain/repo.rs` — `RepoRecord` struct with `pinned`, `pin_order`, `is_dirty`, `last_opened_at`, `tags` fields — used for section classification
- `crates/workpot-core/src/lib.rs` — `AppContext` public API (`list_repos`, `launch_cmd`)

### Prior phase decisions (load-bearing)
- `.planning/phases/05-tags-prioritization/05-CONTEXT.md` — **CRITICAL**: D-19..22 (Pinned>Dirty>Recent>Rest ordering rules, recency algorithm with max_recent_days + min_recent_count), D-18 (pin CLI deferred to Phase 6 — confirmed out of scope)
- `.planning/phases/04-tray-finder-mvp/04-CONTEXT.md` — D-33 (`launch_cmd` default `cursor --new-window {path}`), D-30 (client-side filtering — CLI should replicate same filter logic in Rust)
- `.planning/phases/01-core-persistence/01-CONTEXT.md` — config.toml keys, AppContext paths

### Project constraints
- `.planning/PROJECT.md` — macOS-only v1, Cursor-only IDE, local-only storage

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `resolve_repo_identifier(ctx, identifier)` in `main.rs` — resolves name/path/key to canonical path; reuse as-is for `workpot open`
- `AppContext::list_repos()` — returns `Vec<RepoRecord>` with all org fields (pinned, pin_order, is_dirty, last_opened_at, tags); extend ordering to Pinned > Dirty > Recent > Rest for `workpot list`
- `format_git_state()` in `git_display.rs` — branch + dirty + ahead/behind formatting; extend or create new formatter for the emoji row format

### Established Patterns
- `Commands` / `Subcommand` derive pattern in `main.rs` — add `List`, `Search { query: String }`, `Open { repo: String }` variants
- `AppContext::open()` → match arm pattern — new commands follow the same `let ctx = AppContext::open()?` init
- `WorkpotError` / `anyhow::Context` for error propagation — continue in new commands
- `launch_cmd` execution pattern from Phase 4 (`tauri-plugin-shell` or `std::process::Command`) — CLI reuses `std::process::Command`

### Integration Points
- New `List` / `Search` commands read `list_repos()` then sort by priority (Pinned > Dirty > Recent > Rest) — ordering logic should live in `workpot-core` so tray and CLI share it
- `Open` command resolves identifier → launches `launch_cmd` via `std::process::Command`
- Fuzzy match in `Search` — same crate used by tray frontend (check Cargo.toml for nucleo/fuzzy-matcher)

</code_context>

<specifics>
## Specific Ideas

- User wants flat list with leading emoji — not grouped sections. Scan top-to-bottom: all 📌 first, then all 🟡, then all 🔥, then rest. No headers, no separators between groups.
- Row: `[icon] [parent_dir] [name] [branch] [tags]` — parent dir is `~/c` style (home-shortened), not `/Users/rubenlr/c`.
- `workpot search` is purely a filter: takes a query, runs it through the same fuzzy scorer, outputs the subset of repos that match in priority order. Same row format as `workpot list`.
- `workpot open` confirmation message: `opening: /full/path/to/repo` — full path on this one for unambiguous feedback.
- Ambiguous open error format: numbered list like `workpot tag list` style, then `use the full path from 'workpot list'`.

</specifics>

<deferred>
## Deferred Ideas

- **`workpot pin` / `workpot unpin`** — confirmed out of scope for Phase 6. Pin management stays tray-only in v1. Can revisit post-v1.
- **`workpot search` with `#tag` filter syntax** — tray has `#tag` autocomplete; user chose CLI text-only for now. Future phase if needed.
- **Interactive search TUI** (fzf-style) — print-only chosen. Could be a v2 power feature.
- **`workpot list --json` / machine-readable output** — not discussed; could be a quick follow-up for scripting.

</deferred>

---

*Phase: 6-CLI parity*
*Context gathered: 2026-05-31*
