# Phase 5: Tags & prioritization - Context

**Gathered:** 2026-05-30
**Status:** Ready for planning

<domain>
## Phase Boundary

Deliver repository organization for 20+ repo collections: tags (store, filter, edit), pins (manual priority with drag reorder), notes (free-text per repo), and a four-tier prioritization model (Pinned / Dirty / Recent / Rest) in the tray. The right-arrow detail pane from this phase is the editing surface for all org metadata. The tray list gets section headers and a tag filter with autocomplete. No CLI tag/note/pin commands in Phase 5 (deferred to Phase 6 CLI parity).

**Requirements:** ORG-01, ORG-02, ORG-03, ORG-04

**Success Criteria (from ROADMAP.md):**
1. User can add tags to a repo and filter by tag in the tray
2. Pinned repos stay above unpinned regardless of other signals
3. Dirty and recently opened repos rank higher than stale clean repos
4. User can save notes on a repo and search matches note text

</domain>

<decisions>
## Implementation Decisions

### Tag storage & schema
- **D-01:** Tags stored in a separate `repo_tags(repo_path TEXT, tag TEXT)` table — normalized, queryable, supports rename/delete. Primary key: `(repo_path, tag)`.
- **D-02:** Notes stored as `notes TEXT NULL` column on the `repos` table (one migration adds it). No separate table.
- **D-03:** Pin state stored as `pinned INTEGER NOT NULL DEFAULT 0` + `pin_order INTEGER NULL` columns on `repos` table. `pin_order` is the drag-sort key within the Pinned section.

### Tag editing UX (tray)
- **D-04:** Tags are editable from the **right-arrow detail pane** (see D-11). No separate tag editing UI.
- **D-05:** **Cmd+Click on a tag chip** (on a repo row or in the detail pane) removes that tag immediately.
- **D-06:** **Right-click context menu** on a repo row includes "Add tag" and "Remove tag" actions. Full right-click menu scope is deferred — this is the Phase 5 minimum.
- **D-07:** Tags are also manageable via CLI (`workpot tag add <repo> <tag>` / `workpot tag remove <repo> <tag>`). CLI commands are Phase 5 scope (tray and CLI both ship).

### Tag filtering in tray
- **D-08:** Tag filter uses **both** mechanisms: type `#tagname` in the filter bar AND click tag chips.
- **D-09:** Tag chips are **hidden until user types `#`** in the filter bar — chips appear as a dropdown/suggestion row. No chips visible at rest state.
- **D-10:** **Dropdown autocomplete** shows all existing tags when user types `#`. Arrow keys or mouse to select.
- **D-11:** Multi-tag filter logic: **AND** — repo must have all active tags to appear.

### Detail pane (right-arrow)
- **D-11:** Right arrow on a repo row opens a **full detail pane** containing:
  - **Branch list** (read-only, current branch highlighted — no checkout, v1 is read-only)
  - **Tags** (editable inline — add/remove)
  - **Notes** (editable textarea — see Notes UX)
  - **Pin toggle** (checked = pinned)
- **D-12:** Left arrow or Esc closes the detail pane and returns to the repo list.

### Pinned section
- **D-13:** Pinned repos appear in a **separate labeled "Pinned" section** at the very top of the tray list, with a visual divider and subtle gray section header.
- **D-14:** Pin/unpin actions: **right-click context menu** (quick) AND **detail pane pin toggle** (D-11).
- **D-15:** Pinned section has a **configurable cap** (`max_pinned` in `config.toml`). Claude's discretion for default value (suggest 5).
- **D-16:** Within the Pinned section, order is **manual drag-to-reorder**. Drag updates `pin_order` column immediately.
- **D-17:** When a tag filter is active, **pins follow the filter** — pinned repos that don't match the active tag(s) are hidden from the Pinned section.
- **D-18:** Pin commands are **tray-only in Phase 5**. CLI pin/unpin deferred to Phase 6.

### Prioritization model (non-pinned repos)
- **D-19:** Non-pinned repos are organized into **three labeled sections** below the Pinned section:
  1. **Dirty** — repos where `is_dirty = true`
  2. **Recent** — repos opened within recency window (not dirty)
  3. **Rest** — everything else (clean, never opened, or outside recency window)
- **D-20:** Section conflict: a repo that is **both dirty and recently opened appears in Dirty** (dirty wins).
- **D-21:** Repos with `last_opened_at IS NULL` go to the **Rest section**.
- **D-22:** **Recency algorithm**: Show repos opened within `max_recent_days`. If the result count is less than `min_recent_count`, pad with the next most-recently-opened repos until `min_recent_count` is reached. The pad repos may exceed `max_recent_days`; the minimum is a floor, not a days constraint. Config keys: `max_recent_days` (default ~14), `min_recent_count` (default ~3).
- **D-23:** **Section headers**: all four sections (Pinned, Dirty, Recent, Rest) show **subtle gray section headers**, same visual pattern.

### Notes UX
- **D-24:** Notes are **edited in the detail pane** via an inline textarea. No separate notes view.
- **D-25:** Textarea displays **3 lines minimum, 5 lines maximum** (CSS `min-rows`/`max-rows` or equivalent). Max **500 characters**.
- **D-26:** Notes **save on blur** (focus loss). No save button. No rollback/undo.
- **D-27:** No markdown support — plain text only.
- **D-28:** Notes are **included in the fuzzy filter** automatically. No special syntax — typing any word also matches note text.

### Claude's Discretion
- Exact `max_pinned` default value in config.toml (suggest 5).
- Exact `max_recent_days` default (suggest 14) and `min_recent_count` default (suggest 3).
- Within-section sort order (suggest `last_opened_at DESC` for Dirty and Recent; `registered_at DESC` for Rest).
- Drag-to-reorder implementation details (CSS draggable, Svelte drag-and-drop library choice).
- Exact Tailwind styling for section headers (font size, color, spacing).
- Tag chip styling in dropdown autocomplete.
- Animation for detail pane slide-in/out on right/left arrow.
- How `pin_order` is re-numbered after drag (sparse gaps ok, or re-sequence — Claude's call).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope & requirements
- `.planning/ROADMAP.md` — Phase 5 goal, success criteria (4 criteria), requirements ORG-01..04
- `.planning/REQUIREMENTS.md` — ORG-01 (tags), ORG-02 (pins), ORG-03 (ranking signals), ORG-04 (notes)

### Prior phase decisions (load-bearing)
- `.planning/phases/01-core-persistence/01-CONTEXT.md` — AppContext, DB paths, migration pattern, config.toml location and keys
- `.planning/phases/02-repo-discovery/02-CONTEXT.md` — repos schema, cap rules
- `.planning/phases/03-git-state/03-CONTEXT.md` — git state columns, rayon batch refresh API
- `.planning/phases/04-tray-finder-mvp/04-CONTEXT.md` — **CRITICAL**: Svelte+Tailwind+TypeScript frontend, D-22 ordering (Phase 5 extends this), D-16 keyboard shortcuts (detail pane uses same nav pattern), D-25 `last_opened_at` column added in `005_tray.sql`, IPC pattern (invoke+event)

### Existing code to extend
- `crates/workpot-core/src/domain/repo.rs` — `RepoRecord` struct; extend with `pinned`, `pin_order`, `notes`, `tags` fields
- `crates/workpot-core/src/lib.rs` — `AppContext` public API; add `set_tags`, `set_notes`, `set_pin`, `list_tags` commands
- `crates/workpot-core/src/infra/migrations.rs` — migration pattern; next migration is `006_org.sql` (adds tags table, pins/notes columns)
- `crates/workpot-core/src/services/catalog.rs` — repo list/query logic; extend for section-tiered ordering

### Project constraints
- `.planning/PROJECT.md` — macOS-only v1, local-only storage, read-only git (no checkout)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `AppContext::list_repos()` — returns `Vec<RepoRecord>`; extend `RepoRecord` to include tags, notes, pinned, pin_order for tray rendering.
- `crates/workpot-core/src/infra/migrations.rs` — `rusqlite_migration` pattern; add `006_org.sql` next (after `005_tray.sql` from Phase 4).
- Phase 4 filter-as-you-type (Svelte reactive `Array.filter()`) — extend to include `notes` field and `#tag` syntax parsing.
- Phase 4 keyboard navigation (arrow keys, Enter, Esc) — extend with right/left arrow for detail pane open/close.

### Established Patterns
- `ALTER TABLE repos ADD COLUMN ...` migrations for nullable fields — notes, pinned, pin_order follow this pattern.
- `WorkpotError` / `anyhow::Context` — continue in new Tauri commands for tag/note/pin mutations.
- Integration tests use `open_with_paths` + `tempfile` — new service tests for tag/pin/note ops follow same pattern.
- Config keys in `config.toml` via `serde + toml` — `max_pinned`, `max_recent_days`, `min_recent_count` follow existing pattern.
- Sequential SQL migration files: `001_init.sql` → `005_tray.sql` (Phase 4) → `006_org.sql` (Phase 5).

### Integration Points
- New `repo_tags` table foreign-keys to `repos.path`. Cascading delete on repo removal.
- `pin_order` and `pinned` columns on `repos` table. Updated via new Tauri IPC commands.
- Tray `list_repos` response extended — include tags array, notes, pinned, pin_order per repo.
- Svelte list component gains section grouping logic (Pinned / Dirty / Recent / Rest tiers).
- Svelte filter gains `#tag` prefix detection and tag autocomplete dropdown.

</code_context>

<specifics>
## Specific Ideas

- Detail pane is the **canonical editing surface** for all org metadata. Right arrow = open, left arrow / Esc = close. Same keyboard UX pattern as a typical file explorer detail panel.
- Right-click context menu minimum for Phase 5: Pin/Unpin + Add tag + Remove tag. Full menu scope is intentionally deferred ("to explore later" per user).
- Cmd+Click on a tag chip is a quick remove — no confirmation needed.
- Tag filter #autocomplete dropdown must appear only when `#` is typed, not on every keystroke.
- Drag-to-reorder in Pinned section: user wants this in Phase 5, even though it adds Svelte drag complexity.
- Notes textarea: plain text, 3-5 lines tall, max 500 chars, save on blur, no undo, no markdown.
- Section headers: all four sections labeled (Pinned / Dirty / Recent / Rest) — even "Rest" gets a label. User confirmed "Yes — subtle section headers."
- Recency min/max algorithm is explicit: "query last repos with max_recent_days, then pad above max days without leaving the list below min_recent_count." The minimum count is a floor, not bound by days.

</specifics>

<deferred>
## Deferred Ideas

- **Full right-click context menu scope** — user said "right click opens a menu that can add|remove tags as well; this right click menu on repo needs to be more explored later, it might have other features to add there." Phase 5 implements minimum (Pin/Unpin, Add/Remove tag). Full menu is a separate discussion.
- **CLI pin/unpin commands** (`workpot pin <repo>` / `workpot unpin <repo>`) — deferred to Phase 6 CLI parity.
- **CLI tag/note commands** — Phase 6 CLI parity.
- **View mode submenu** (carried from Phase 4 deferred) — picker to switch sort modes. Now partially resolved by section model; may not be needed.

</deferred>

---

*Phase: 5-Tags & prioritization*
*Context gathered: 2026-05-30*
