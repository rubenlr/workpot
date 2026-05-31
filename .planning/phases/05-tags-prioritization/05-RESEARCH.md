# Phase 5: Tags & Prioritization - Research

**Researched:** 2026-05-31
**Domain:** Rust/SQLite data layer extension + Svelte 5 tray UI extension (tags, pins, notes, section ordering, detail pane)
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Tag storage & schema**
- D-01: Tags stored in a separate `repo_tags(repo_path TEXT, tag TEXT)` table — normalized, queryable, supports rename/delete. Primary key: `(repo_path, tag)`.
- D-02: Notes stored as `notes TEXT NULL` column on the `repos` table (one migration adds it). No separate table.
- D-03: Pin state stored as `pinned INTEGER NOT NULL DEFAULT 0` + `pin_order INTEGER NULL` columns on `repos` table. `pin_order` is the drag-sort key within the Pinned section.

**Tag editing UX (tray)**
- D-04: Tags are editable from the right-arrow detail pane. No separate tag editing UI.
- D-05: Cmd+Click on a tag chip (on a repo row or in the detail pane) removes that tag immediately.
- D-06: Right-click context menu on a repo row includes "Add tag" and "Remove tag" actions. Full right-click menu scope is deferred.
- D-07: Tags are also manageable via CLI (`workpot tag add <repo> <tag>` / `workpot tag remove <repo> <tag>`). CLI commands are Phase 5 scope.

**Tag filtering in tray**
- D-08: Tag filter uses both mechanisms: type `#tagname` in the filter bar AND click tag chips.
- D-09: Tag chips are hidden until user types `#` in the filter bar.
- D-10: Dropdown autocomplete shows all existing tags when user types `#`.
- D-11: Multi-tag filter logic: AND — repo must have all active tags to appear.

**Detail pane (right-arrow)**
- D-11: Right arrow on a repo row opens a full detail pane containing: branch list (read-only), tags (editable inline), notes (editable textarea), pin toggle.
- D-12: Left arrow or Esc closes the detail pane and returns to the repo list.

**Pinned section**
- D-13: Pinned repos appear in a separate labeled "Pinned" section at the very top.
- D-14: Pin/unpin: right-click context menu AND detail pane pin toggle.
- D-15: Pinned section has a configurable cap (`max_pinned` in `config.toml`).
- D-16: Within the Pinned section, order is manual drag-to-reorder. Drag updates `pin_order` column immediately.
- D-17: When a tag filter is active, pins follow the filter.
- D-18: Pin commands are tray-only in Phase 5. CLI pin/unpin deferred to Phase 6.

**Prioritization model (non-pinned repos)**
- D-19: Non-pinned repos organized into three labeled sections: Dirty / Recent / Rest.
- D-20: Dirty and recently opened → Dirty wins.
- D-21: Repos with `last_opened_at IS NULL` go to Rest section.
- D-22: Recency algorithm: repos opened within `max_recent_days`; pad up to `min_recent_count` if result is less. Config keys: `max_recent_days`, `min_recent_count`.
- D-23: All four sections (Pinned, Dirty, Recent, Rest) show subtle gray section headers.

**Notes UX**
- D-24: Notes edited in the detail pane via inline textarea.
- D-25: Textarea 3 lines minimum, 5 lines maximum. Max 500 characters.
- D-26: Notes save on blur. No save button. No rollback/undo.
- D-27: No markdown — plain text only.
- D-28: Notes are included in the fuzzy filter automatically (no special syntax).

### Claude's Discretion
- Exact `max_pinned` default value in config.toml (suggest 5).
- Exact `max_recent_days` default (suggest 14) and `min_recent_count` default (suggest 3).
- Within-section sort order (suggest `last_opened_at DESC` for Dirty and Recent; `registered_at DESC` for Rest).
- Drag-to-reorder implementation details (CSS draggable, library choice).
- Exact Tailwind styling for section headers (font size, color, spacing).
- Tag chip styling in dropdown autocomplete.
- Animation for detail pane slide-in/out on right/left arrow.
- How `pin_order` is re-numbered after drag (sparse gaps ok, or re-sequence).

### Deferred Ideas (OUT OF SCOPE)
- Full right-click context menu scope (Phase 5 implements minimum: Pin/Unpin, Add/Remove tag).
- CLI pin/unpin commands — deferred to Phase 6.
- CLI tag/note commands — Phase 6 CLI parity.
- View mode submenu (carried from Phase 4).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ORG-01 | User can assign one or more tags to a repository | Migration 006 `repo_tags` table; `set_tags`/`list_tags` API; Svelte chip editing in detail pane; `#tag` filter bar with autocomplete |
| ORG-02 | User can pin repositories to keep them at the top of the list | Migration 006 `pinned`/`pin_order` columns; `set_pin` API; Tauri 2 popup menu; Svelte drag-to-reorder in Pinned section |
| ORG-03 | System ranks repositories using signals (dirty, recently opened, pinned) with manual overrides winning | Four-tier section sort in `catalog.rs` and Svelte `sort.ts`; config keys `max_recent_days`/`min_recent_count`/`max_pinned` |
| ORG-04 | User can add free-text notes to a repository | Migration 006 `notes TEXT NULL` on repos; `set_notes` API; Svelte detail pane textarea; fuzzy filter extended to include notes field |
</phase_requirements>

---

## Summary

Phase 5 extends the existing Workpot stack in three coordinated layers: (1) a SQLite migration `006_org.sql` that adds `repo_tags`, `pinned`/`pin_order`, and `notes` to the schema; (2) new Rust service functions and Tauri IPC commands in `workpot-core` and `workpot-tray`; and (3) Svelte 5 frontend changes that replace the flat sorted list with a four-section grouped list (Pinned / Dirty / Recent / Rest) and add a detail pane, tag filter with autocomplete, and drag-to-reorder in the Pinned section.

The Rust side is well-understood: the migration pattern (`rusqlite_migration` + numbered SQL files) is established, `RepoRecord` needs four new fields (`pinned`, `pin_order`, `notes`, `tags: Vec<String>`), and `AppContext` needs `set_tags`, `add_tag`, `remove_tag`, `list_tags`, `set_notes`, `set_pin`, `get_pin_order`, `set_pin_order`. The four-tier ordering replaces `traySort` in `sort.ts` and the existing flat `list_repos` query becomes a section-aware grouped structure returned to the frontend.

The Svelte frontend complexity is the highest-risk area. The current `+page.svelte` is a single 325-line file with all logic inline. Phase 5 adds a detail pane (right arrow opens, left/Esc closes), drag-to-reorder in the Pinned section, a `#tag` autocomplete dropdown, and section headers. These are best implemented as extracted Svelte components. The recommended drag-to-reorder approach is HTML5 `draggable` attribute with native drag events — no new npm package needed for the small Pinned section (capped at `max_pinned`, default 5).

**Primary recommendation:** Implement migration 006, extend `RepoRecord` and `AppContext` first (Wave 1), then new IPC commands and Tauri popup menu (Wave 2), then Svelte section grouping and detail pane (Wave 3), then drag-to-reorder and `#tag` autocomplete (Wave 4), then CLI `tag` subcommand (Wave 5).

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Tag storage & retrieval | Database (SQLite) | — | Normalized `repo_tags` table; queried and joined in Rust before IPC |
| Pin/notes storage | Database (SQLite) | — | Columns on `repos` table; same access path as existing fields |
| Four-tier section ordering | Frontend (Svelte) | — | Client-side grouping on pre-loaded `RepoDto[]`; no IPC per sort |
| Tag autocomplete dropdown | Frontend (Svelte) | API (list_tags IPC) | All tags loaded once; dropdown filters client-side |
| Detail pane branch list | Frontend (Svelte) | — | Uses `branches` field already present in `RepoDto` via git2 refresh |
| Right-click context menu | API/Backend (Tauri Rust) | — | Tauri 2 `Menu::popup` on right-click from frontend; handler invokes IPC |
| Drag-to-reorder persistence | API/Backend (Tauri IPC) | Database | Frontend emits new order; `set_pin_order` persists to SQLite |
| Fuzzy search (notes + tags) | Frontend (Svelte) | — | `fuzzyMatch` in `fuzzy.ts` extended with notes and tags fields |
| CLI tag commands | CLI (workpot-cli) | Core (workpot-core) | `workpot tag add/remove/list` via `AppContext` tag API |
| Config keys (max_pinned etc.) | Core (workpot-core Config) | — | Added to `config.rs` `Config` struct with serde defaults |

---

## Standard Stack

### Core (no new dependencies needed)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| rusqlite | 0.40 [VERIFIED: Cargo.lock] | SQLite ops for `repo_tags`, pins, notes | Already in use; bundled feature active |
| rusqlite_migration | 2.6 [VERIFIED: Cargo.lock] | Migration 006_org.sql delivery | Established migration pattern in codebase |
| serde + toml | workspace versions [VERIFIED: codebase] | New config keys in Config struct | Existing TOML config pattern |
| tauri 2 | 2.x [VERIFIED: Cargo.toml] | Popup menu (context menu) for right-click | Tauri 2 Menu::popup is native |
| Svelte 5 | 5.x [VERIFIED: package.json] | Detail pane, section headers, autocomplete | Already in use |
| Tailwind CSS 4 | 4.x [VERIFIED: package.json] | Section header styling, chip styling | Already in use |
| TypeScript | ~5.6.2 [VERIFIED: package.json] | Updated RepoDto, section types | Already in use |
| Vitest | 3.x [VERIFIED: package.json] | Unit tests for sort, filter, section logic | Already in use |
| clap 4 | 4.x [VERIFIED: Cargo.toml workspace] | CLI `workpot tag` subcommand | Already in use for all CLI |

### Supporting (no new dependencies for Phase 5)

HTML5 `draggable` attribute with native drag events is sufficient for the Pinned section drag-to-reorder (max 5 items). No external drag library needed. This avoids adding an npm dependency and removes Svelte 5 compatibility risk.

### Alternatives Considered for Drag-to-Reorder

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| HTML5 native draggable | `sortablejs` 1.15.7 [VERIFIED: npm] | SortableJS adds smoother animations and touch support. But the Pinned section is desktop-only (macOS tray) and capped at 5 items — native drag is simpler and has zero dependency footprint. |
| HTML5 native draggable | `svelte-dnd-action` 0.9.69 [VERIFIED: npm] | Explicitly supports Svelte 3/4 only (npm description confirms). Not compatible with Svelte 5 runes. |
| Tauri 2 `Menu::popup` | `tauri-plugin-context-menu` crate | Plugin adds a JS-side API surface. Tauri 2 native `Menu::popup_menu` (via `ContextMenu` trait on `Menu`) accomplishes the same thing from Rust without an extra dependency. |

**Installation (no new packages):**
```bash
# No new npm or cargo packages required for Phase 5.
# All needed libraries are already in the workspace.
```

---

## Package Legitimacy Audit

> No new packages are introduced in Phase 5. All dependencies are already installed and verified in the workspace.

| Package | Registry | Age | Disposition |
|---------|----------|-----|-------------|
| (none new) | — | — | — |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

*slopcheck was not available at research time, but no new packages are being introduced — this section is N/A.*

---

## Architecture Patterns

### System Architecture Diagram

```
User input (tray panel)
  │
  ├─ type "#" in filter bar ──→ TagAutocomplete dropdown (Svelte) ──→ filterQuery state update
  │
  ├─ arrow keys / Enter ──────→ keyboard nav (existing filterNavigation.ts)
  │
  ├─ → arrow on repo row ─────→ DetailPane.svelte (slide in)
  │     ├─ branch list (read-only from RepoDto.branches)
  │     ├─ tag chips (inline add/remove via invoke set_tags)
  │     ├─ notes textarea (blur → invoke set_notes)
  │     └─ pin toggle (invoke set_pin → reload)
  │
  ├─ right-click on repo row ─→ JS contextmenu event → invoke show_repo_context_menu
  │                              └─ Tauri Rust: Menu::popup → handler dispatch
  │                                   ├─ Pin/Unpin → invoke set_pin
  │                                   ├─ Add Tag → invoke add_tag
  │                                   └─ Remove Tag → invoke remove_tag
  │
  └─ drag within Pinned ──────→ HTML5 drag events → new order array
                                → invoke set_pin_order(path, new_order)
                                → SQLite update pin_order column
                                → reload repos

IPC Layer (invoke)
  list_repos ──────────────────→ catalog::list_repos_with_org (extended SQL JOIN)
  set_tags ────────────────────→ catalog::set_tags (DELETE+INSERT in tx)
  add_tag / remove_tag ────────→ catalog::add_tag / catalog::remove_tag
  list_all_tags ───────────────→ catalog::list_all_tags (SELECT DISTINCT from repo_tags)
  set_notes ───────────────────→ UPDATE repos SET notes = ?
  set_pin ─────────────────────→ UPDATE repos SET pinned = ?, pin_order = ?
  set_pin_order ───────────────→ UPDATE repos SET pin_order = ? per path
  show_repo_context_menu ──────→ Tauri Menu::popup (Rust side)

Storage
  repos table: + pinned, pin_order, notes columns (migration 006)
  repo_tags table: repo_path FK, tag (migration 006)
```

### Recommended Project Structure

New files for Phase 5:

```
crates/workpot-core/src/
├─ infra/migrations/
│   └─ 006_org.sql                    # new: repo_tags table + pins/notes columns
├─ services/
│   └─ org.rs                         # new: tag, pin, notes CRUD (or extend catalog.rs)
crates/workpot-cli/src/
│   main.rs                           # extend: add Tag subcommand
src/
├─ lib/
│   ├─ types.ts                       # extend: RepoDto + tags/notes/pinned/pin_order
│   ├─ sort.ts                        # replace: traySort → sectionSort (four tiers)
│   ├─ sort.test.ts                   # extend: new section tests
│   ├─ fuzzy.ts                       # extend: match notes + tags fields
│   ├─ fuzzy.test.ts                  # extend: notes/tags match tests
│   ├─ trayList.ts                    # extend: section grouping
│   ├─ trayList.test.ts               # extend: section grouping tests
│   ├─ tagFilter.ts                   # new: #tag parsing + AND filter logic
│   ├─ tagFilter.test.ts              # new
│   ├─ pinOrder.ts                    # new: drag reorder array helpers
│   └─ pinOrder.test.ts               # new
├─ lib/components/
│   ├─ DetailPane.svelte               # new: branch list, tags, notes, pin toggle
│   ├─ TagChip.svelte                  # new: chip with Cmd+Click remove
│   ├─ TagAutocomplete.svelte          # new: dropdown for #tag autocomplete
│   └─ SectionHeader.svelte            # new: subtle gray header
src-tauri/src/
│   commands.rs                       # extend: new IPC commands
│   lib.rs                            # extend: register new commands + invoke_handler
```

**Decision on `org.rs` vs extending `catalog.rs`:** Create a new `services/org.rs` for tag/pin/notes CRUD. `catalog.rs` is already 394 lines focused on registration and discovery. `org.rs` isolates the organization concern cleanly. Both call into the same `Connection`.

### Pattern 1: Migration 006_org.sql

```sql
-- Source: established rusqlite_migration pattern in migrations.rs
-- Enable foreign key enforcement for this migration
PRAGMA foreign_keys = ON;

ALTER TABLE repos ADD COLUMN notes TEXT NULL;
ALTER TABLE repos ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
ALTER TABLE repos ADD COLUMN pin_order INTEGER NULL;

CREATE TABLE repo_tags (
    repo_path TEXT NOT NULL,
    tag       TEXT NOT NULL,
    PRIMARY KEY (repo_path, tag),
    FOREIGN KEY (repo_path) REFERENCES repos(path) ON DELETE CASCADE
);

CREATE INDEX idx_repo_tags_path ON repo_tags(repo_path);
CREATE INDEX idx_repo_tags_tag  ON repo_tags(tag);
```

**Important:** `PRAGMA foreign_keys = ON` must be set per-connection for cascade delete to work. The current `store.rs::open_connection` does not enable it. Either add it to `store.rs` (applies globally — safest) or handle cascade manually in `remove_repo`. **Recommended:** add `conn.pragma_update(None, "foreign_keys", true)?;` to `store.rs::open_connection` right after the WAL pragma. This is safe because the existing schema has no FK constraints to inadvertently break. [VERIFIED: SQLite foreign_keys PRAGMA docs]

### Pattern 2: `set_tags` atomic replace in Rust

```rust
// Source: established rusqlite tx pattern in catalog.rs
pub fn set_tags(conn: &Connection, repo_path: &str, tags: &[&str]) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM repo_tags WHERE repo_path = ?1", params![repo_path])?;
    for tag in tags {
        tx.execute(
            "INSERT OR IGNORE INTO repo_tags (repo_path, tag) VALUES (?1, ?2)",
            params![repo_path, tag],
        )?;
    }
    tx.commit()?;
    Ok(())
}
```

### Pattern 3: RepoDto extension (TypeScript)

```typescript
// Source: src/lib/types.ts — extend existing RepoDto
export interface RepoDto {
  path: string;
  name: string;
  branch: string | null;
  is_dirty: boolean | null;
  parent_dir: string;
  last_opened_at: number | null;
  git_state_error: string | null;
  // Phase 5 additions:
  pinned: boolean;
  pin_order: number | null;
  notes: string | null;
  tags: string[];
  branches: string[];   // branch list for detail pane (read-only)
}
```

### Pattern 4: Section grouping in TypeScript

```typescript
// Source: informed by existing traySort pattern in src/lib/sort.ts
// Replaces flat traySort with section-aware grouping
export type Section = "pinned" | "dirty" | "recent" | "rest";

export interface SectionedRepos {
  pinned: RepoDto[];
  dirty: RepoDto[];
  recent: RepoDto[];
  rest: RepoDto[];
}

export function sectionSort(
  repos: RepoDto[],
  config: { maxRecentDays: number; minRecentCount: number },
  nowSeconds: number
): SectionedRepos {
  const pinned = repos
    .filter((r) => r.pinned)
    .sort((a, b) => (a.pin_order ?? 999) - (b.pin_order ?? 999));

  const nonPinned = repos.filter((r) => !r.pinned);
  const dirty = nonPinned
    .filter((r) => r.is_dirty === true)
    .sort(byLastOpenedDesc);

  const nonDirty = nonPinned.filter((r) => r.is_dirty !== true);
  const windowSecs = config.maxRecentDays * 86400;
  const recentByTime = nonDirty.filter(
    (r) => r.last_opened_at != null && nowSeconds - r.last_opened_at < windowSecs
  );
  // pad to min_recent_count
  // ...
  const rest = /* remaining */ [];
  return { pinned, dirty, recent: ..., rest };
}
```

### Pattern 5: HTML5 drag-to-reorder for Pinned section

```typescript
// Native HTML5 draggable — no library needed
// dragStart captures index; dragOver prevents default; drop swaps positions and invokes set_pin_order
function handleDragStart(e: DragEvent, idx: number) {
  dragSourceIdx = idx;
  e.dataTransfer!.effectAllowed = "move";
}
function handleDrop(e: DragEvent, targetIdx: number) {
  e.preventDefault();
  if (dragSourceIdx === targetIdx) return;
  const newOrder = reorder(pinnedRepos, dragSourceIdx, targetIdx);
  // invoke("set_pin_order", { items: newOrder.map((r, i) => ({ path: r.path, order: i })) })
}
```

### Pattern 6: Tauri 2 popup context menu (Rust)

```rust
// Source: Tauri 2 Menu API — menu::Menu::popup
// In commands.rs:
#[tauri::command]
pub async fn show_repo_context_menu(
    window: tauri::Window,
    path: String,
    is_pinned: bool,
    tags: Vec<String>,
    state: State<'_, Arc<Mutex<AppContext>>>,
) -> Result<(), String> {
    use tauri::menu::{Menu, MenuItem};
    let pin_label = if is_pinned { "Unpin" } else { "Pin" };
    let menu = Menu::with_items(&window.app_handle(), &[
        &MenuItem::with_id(&window.app_handle(), "pin", pin_label, true, None::<&str>)?,
        // ... add/remove tag items
    ])?;
    menu.popup(window)?;
    Ok(())
}
// Menu item selections arrive as MenuEvent on the window — need listen("tauri://menu-event", ...)
// in Svelte to handle the result. Alternative: use channel/event pattern with app.emit().
```

**Note:** The popup menu pattern in Tauri 2 uses `window.popup_menu()` which dispatches `MenuEvent`. The Svelte side must listen for these events to execute the mutation. An alternative is to return the selection over a Tauri channel (oneshot). The simplest approach: emit a Tauri app event from the MenuEvent handler, listen in Svelte. [CITED: https://docs.rs/tauri/latest/tauri/menu/struct.Menu.html]

### Anti-Patterns to Avoid

- **Svelte 4 component libraries in Svelte 5:** `svelte-dnd-action` explicitly does not support Svelte 5 runes. Do not install it. HTML5 native drag is the correct choice.
- **Filtering in SQL (server-side) instead of client-side:** The existing Phase 4 pattern is client-side fuzzy filter on pre-loaded `RepoDto[]`. Phase 5 extends this. Do not add SQL LIKE for tag filter — it breaks the "filter IPC per keystroke" anti-pattern.
- **Extending `catalog.rs` for org ops:** `catalog.rs` is already 394 lines focused on registration/discovery. Add a new `services/org.rs` module to keep separation of concerns.
- **Forgetting `PRAGMA foreign_keys = ON`:** SQLite cascade delete does NOT work without this pragma set per-connection. The current `open_connection` in `store.rs` does not enable it. Add it before relying on cascade behavior for `repo_tags` when a repo is removed.
- **Storing tags on `RepoRecord` as `Option<Vec<String>>`:** Tags will never be "unknown" (unlike git state) — always return `Vec<String>` (empty if no tags). This simplifies TypeScript types.
- **Arbitrary `pin_order` re-sequencing:** Sparse gaps in `pin_order` are fine. Re-sequencing after every drag adds complexity for no benefit. Assign `pin_order` as the new index position (0, 1, 2...) only on drag — do not attempt to keep values contiguous when no drag has occurred.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Tag autocomplete UI | Custom input+list from scratch | `<datalist>` element or Svelte component with `$derived` filtered list | Standard HTML datalist or a tiny `<ul>` dropdown is simpler and more accessible |
| Atomic tag replace | Try-catch partial updates | Single `unchecked_transaction()` + DELETE+INSERT | rusqlite tx pattern already established in codebase |
| Section header component | Per-section inline div | Extracted `SectionHeader.svelte` with slot | Eliminates 4x template duplication |
| Context menu event dispatch | Custom event bus | Tauri `app.emit()` → Svelte `listen()` | Established pattern from Phase 4 git-refresh-complete events |
| `#tag` parsing | Regex from scratch | Simple `filterQuery.startsWith("#")` + `split(" ")` | The decision is explicit: `#tagname` tokens only |

**Key insight:** The organization layer is almost entirely CRUD on simple fields. Avoid over-engineering. The sections and filter live in client-side TypeScript functions that are easily unit-tested with Vitest.

---

## Common Pitfalls

### Pitfall 1: SQLite cascade delete silently doing nothing

**What goes wrong:** `DELETE FROM repos WHERE path = ?` does not cascade to `repo_tags`, leaving orphaned tag rows.
**Why it happens:** SQLite foreign key enforcement is disabled by default and requires `PRAGMA foreign_keys = ON` per-connection.
**How to avoid:** Add `conn.pragma_update(None, "foreign_keys", true)?;` to `store.rs::open_connection` (right after the WAL pragma). Alternatively, add explicit `DELETE FROM repo_tags WHERE repo_path = ?` in `remove_repo` — but the pragma approach is cleaner and handles future FK constraints.
**Warning signs:** `repo_tags` rows accumulate after repo removal; `list_all_tags` returns tags for repos not in the `repos` table.

### Pitfall 2: `unchecked_transaction()` panics when called inside an active transaction

**What goes wrong:** `set_tags` or `set_pin_order` called while a batch git refresh tx is open.
**Why it happens:** `unchecked_transaction()` is not re-entrant in rusqlite.
**How to avoid:** All org mutations come from synchronous Tauri command handlers that hold the `Arc<Mutex<AppContext>>` lock — they cannot run concurrently with the async git refresh. The existing pattern (lock → mutate → unlock) prevents overlap. Verify the mutex lock is held for the full org mutation.
**Warning signs:** Runtime panic with "cannot start a transaction within a transaction".

### Pitfall 3: Tauri popup menu — MenuEvent not received by Svelte

**What goes wrong:** Right-click shows the menu, user selects an item, nothing happens.
**Why it happens:** `MenuEvent` is dispatched to the Tauri runtime, not automatically forwarded to the webview. The Rust `MenuEvent` handler must explicitly emit a Tauri app event for Svelte to hear.
**How to avoid:** In the `MenuEvent` handler (registered via `app.on_menu_event`), emit a typed event (e.g. `"repo-context-action"` with `{ path, action }` payload) that Svelte `listen()`s to.
**Warning signs:** Console silence after menu click; no Tauri event received in Svelte.

### Pitfall 4: `RepoDto` deserialization mismatch after adding fields

**What goes wrong:** Tray shows blank tags/notes/pins after Phase 5 deploy.
**Why it happens:** `list_repos` Tauri command returns `RepoDto` that must include all new fields. If the Rust-side `record_to_dto` in `commands.rs` does not map the new `RepoRecord` fields, they will be `null`/absent in TypeScript.
**How to avoid:** Update both `RepoRecord` (Rust domain), `record_to_dto` in `commands.rs`, and `RepoDto` in TypeScript `types.ts` in the same commit. Add a test asserting all new fields serialize correctly.
**Warning signs:** TypeScript type errors at `repo.tags` (undefined); all repos show no tags even after adding them.

### Pitfall 5: Section filter not respecting active tag filter for Pinned section

**What goes wrong:** Pinned repos appear in the Pinned section even when they don't match the active `#tag` filter (violates D-17).
**Why it happens:** The section grouping function does not apply the tag filter before splitting into sections.
**How to avoid:** Apply the full filter (fuzzy + tag) to `repos` first, then run section grouping on the filtered set. The existing `filterAndSortRepos` → split into sections flow naturally prevents this if implemented correctly.
**Warning signs:** UAT scenario: user has `#backend` filter active; a pinned repo without that tag still appears in Pinned section.

### Pitfall 6: `pin_order` conflicts when two sessions race

**What goes wrong:** Two tray windows (unlikely but possible) both drag-reorder and overwrite each other's `pin_order`.
**Why it happens:** There is no optimistic concurrency on `pin_order` updates.
**How to avoid:** This is an edge case not worth solving in Phase 5 (single-user macOS app). `reload_repos` after every `set_pin_order` IPC call ensures the UI reflects the DB truth.
**Warning signs:** Not applicable at v1 scale — acceptable known limitation.

---

## Code Examples

### 006_org.sql migration

```sql
-- Source: codebase pattern from 005_tray.sql + SQLite FK docs
PRAGMA foreign_keys = ON;

ALTER TABLE repos ADD COLUMN notes    TEXT    NULL;
ALTER TABLE repos ADD COLUMN pinned   INTEGER NOT NULL DEFAULT 0;
ALTER TABLE repos ADD COLUMN pin_order INTEGER NULL;

CREATE TABLE repo_tags (
    repo_path TEXT NOT NULL,
    tag       TEXT NOT NULL COLLATE NOCASE,
    PRIMARY KEY (repo_path, tag),
    FOREIGN KEY (repo_path) REFERENCES repos(path) ON DELETE CASCADE
);

CREATE INDEX idx_repo_tags_path ON repo_tags(repo_path);
CREATE INDEX idx_repo_tags_tag  ON repo_tags(tag);
```

Note: `COLLATE NOCASE` on `tag` makes tag lookup case-insensitive at the DB level, which simplifies deduplication.

### Rust org service skeleton

```rust
// Source: catalog.rs patterns
// crates/workpot-core/src/services/org.rs

pub fn list_tags_for_repo(conn: &Connection, repo_path: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT tag FROM repo_tags WHERE repo_path = ?1 ORDER BY tag"
    )?;
    let tags = stmt.query_map(params![repo_path], |row| row.get(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(tags)
}

pub fn list_all_tags(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT tag FROM repo_tags \
         JOIN repos ON repo_tags.repo_path = repos.path \
         WHERE repos.excluded = 0 ORDER BY tag"
    )?;
    let tags = stmt.query_map([], |row| row.get(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(tags)
}

pub fn set_notes(conn: &Connection, repo_path: &str, notes: Option<&str>) -> Result<()> {
    let updated = conn.execute(
        "UPDATE repos SET notes = ?1 WHERE path = ?2",
        params![notes, repo_path],
    )?;
    if updated == 0 {
        return Err(WorkpotError::NotFound(repo_path.to_string()));
    }
    Ok(())
}

pub fn set_pin(conn: &Connection, repo_path: &str, pinned: bool) -> Result<()> {
    // When pinning, set pin_order to MAX(pin_order)+1 so new pins go to bottom of Pinned section.
    // When unpinning, clear pin_order.
    let next_order: i64 = if pinned {
        conn.query_row(
            "SELECT COALESCE(MAX(pin_order), -1) + 1 FROM repos WHERE pinned = 1",
            [],
            |row| row.get(0),
        )?
    } else {
        -1 // ignored
    };

    conn.execute(
        "UPDATE repos SET pinned = ?1, pin_order = ?2 WHERE path = ?3",
        params![pinned as i64, if pinned { Some(next_order) } else { None::<i64> }, repo_path],
    )?;
    Ok(())
}
```

### Fuzzy filter extension (TypeScript)

```typescript
// Source: src/lib/fuzzy.ts — extend scoreField calls
export function fuzzyScore(query: string, repo: RepoDto): number {
  // ...existing...
  const scores = [
    scoreField(q, repo.name, true),
    scoreField(q, repo.path, false),
    scoreField(q, repo.branch ?? "", false),
    // Phase 5: add notes and tags
    scoreField(q, repo.notes ?? "", false),
    ...repo.tags.map((t) => scoreField(q, t, false)),
  ];
  return Math.max(...scores);
}
```

### Tag filter parsing (TypeScript)

```typescript
// Source: D-08/D-09/D-11 decisions
export function parseTagFilter(query: string): { baseQuery: string; activeTags: string[] } {
  const tokens = query.trim().split(/\s+/);
  const activeTags = tokens
    .filter((t) => t.startsWith("#") && t.length > 1)
    .map((t) => t.slice(1).toLowerCase());
  const baseQuery = tokens.filter((t) => !t.startsWith("#")).join(" ");
  return { baseQuery, activeTags };
}

export function matchesTags(repo: RepoDto, activeTags: string[]): boolean {
  if (activeTags.length === 0) return true;
  const repoTags = repo.tags.map((t) => t.toLowerCase());
  return activeTags.every((at) => repoTags.includes(at));
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Flat sorted list (Phase 4 `traySort`) | Four-section grouped list (Phase 5) | Phase 5 | `sort.ts`/`trayList.ts` must be refactored, not just extended |
| `filterAndSortRepos` single pass | Filter first, then section-group | Phase 5 | Test coverage must cover the combined pipeline |
| No tags/pins/notes in `RepoRecord` | Extended `RepoRecord` + new IPC commands | Phase 5 | `commands.rs::record_to_dto` must be updated in the same commit as the struct |

**Deprecated/outdated:**
- `traySort` in `sort.ts`: The two-tier sort (dirty vs rest) is replaced by a four-tier section model. The old sort can still be used within sections (e.g., sort within Dirty by `last_opened_at DESC`) but the flat sort is superseded.
- `filterAndSortRepos` in `trayList.ts`: Must be updated to return `SectionedRepos` instead of a flat array, or a separate `sectionRepos` function added and `filterAndSortRepos` deprecated.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Tauri 2 `Menu::popup` is accessible via `menu::Menu` + `ContextMenu` trait without additional feature flags | Standard Stack / Patterns | Planner would need to add a Tauri feature flag or use `tauri-plugin-context-menu` crate instead |
| A2 | `COLLATE NOCASE` on `repo_tags.tag` is sufficient for case-insensitive tag matching | Code Examples / 006_org.sql | Tags could still show duplicates if client normalizes to lowercase separately |
| A3 | `list_repos` extended to JOIN `repo_tags` returns `tags` as aggregated JSON or multiple rows; the preferred approach is a separate `list_tags_for_repo` call from the DTO mapper | Architecture | Could be unified into one SQL query with GROUP_CONCAT for efficiency |
| A4 | The detail pane branch list uses branches already in `RepoRecord` via git2 refresh (no new IPC call) | Architecture | If `RepoRecord` does not include branch list (only current branch), a new `list_branches` IPC command is needed |

**Note on A4:** `RepoRecord` currently has `branch: Option<String>` (current branch only, not a list). The detail pane D-11 shows a "branch list (read-only, current branch highlighted)". This requires either: (a) adding a `list_branches(path)` IPC call invoked when detail pane opens, or (b) pre-loading branch lists for all repos in `list_repos`. Option (a) is correct — loading branch lists for all repos upfront adds latency and size. The planner should add a `list_branches` Tauri command that uses `git2::Repository::branches` and returns `Vec<String>`. This is a confirmed gap not covered by existing `RepoRecord`.

---

## Open Questions

1. **`list_repos` SQL join for tags — aggregate or separate call?**
   - What we know: `RepoRecord` must include `tags: Vec<String>`. SQL options: `GROUP_CONCAT` in the main query, or call `list_tags_for_repo` N times in Rust after loading repos.
   - What's unclear: Performance at 100+ repos. `GROUP_CONCAT` in SQLite via Rust requires parsing the result string. The N+1 approach is cleaner but is 100+ extra queries.
   - Recommendation: Use a single SQL query with `LEFT JOIN repo_tags` and collect tags per path in Rust using a `HashMap<String, Vec<String>>` accumulation pattern. This is standard SQLite + Rust idiom and avoids N+1.

2. **Notes textarea `max-rows` in Tailwind 4 / Svelte**
   - What we know: D-25 specifies 3 min rows, 5 max rows.
   - What's unclear: Tailwind 4 does not have `rows` utilities natively. CSS `field-sizing: content` is the modern approach but browser support may be limited in the WebView.
   - Recommendation: Use `rows="3"` HTML attribute on `<textarea>` for minimum, and CSS `max-height` computed from line-height × 5 for maximum. This is reliable in WebKit (macOS WebView).

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | Core Cargo build | ✓ | 1.96+ (inferred from workspace) | — |
| Node.js + npm | Frontend build | ✓ | ≥20 (package.json engines) | — |
| macOS (Tauri tray) | Popup menu, panel | ✓ | macOS 25 (Darwin 25.5.0) | — |
| Tauri 2 CLI | `npm run tauri build` | ✓ | @tauri-apps/cli ^2 in package.json | — |
| SQLite (bundled) | rusqlite + bundled feature | ✓ | bundled in rusqlite 0.40 | — |

No missing dependencies. All required tools confirmed present.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Vitest 3.x (frontend) + cargo test / cargo-nextest (Rust) |
| Config file | `vite.config.ts` (test section), `Cargo.toml` workspaces |
| Quick run command | `npm test` (Vitest) / `cargo test -p workpot-core` (Rust) |
| Full suite command | `npm test && cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ORG-01 | Tag CRUD (add/remove/list) | unit (Rust) | `cargo test -p workpot-core org` | ❌ Wave 0: `tests/org_test.rs` |
| ORG-01 | `#tag` filter parses and ANDs correctly | unit (TS) | `npm test -- tagFilter` | ❌ Wave 0: `src/lib/tagFilter.test.ts` |
| ORG-01 | Fuzzy filter matches tag field | unit (TS) | `npm test -- fuzzy` | ✅ extend `fuzzy.test.ts` |
| ORG-02 | `set_pin` / `set_pin_order` mutations | unit (Rust) | `cargo test -p workpot-core org` | ❌ Wave 0: `tests/org_test.rs` |
| ORG-02 | Pinned section appears first in section grouping | unit (TS) | `npm test -- trayList` | ✅ extend `trayList.test.ts` |
| ORG-02 | `pinOrder` reorder array helper | unit (TS) | `npm test -- pinOrder` | ❌ Wave 0: `src/lib/pinOrder.test.ts` |
| ORG-03 | Section grouping: Pinned > Dirty > Recent > Rest | unit (TS) | `npm test -- sort` | ✅ extend `sort.test.ts` |
| ORG-03 | Recency algorithm: min_recent_count padding | unit (TS) | `npm test -- sort` | ✅ extend `sort.test.ts` |
| ORG-04 | `set_notes` persists and fuzzy-matches | unit (Rust) | `cargo test -p workpot-core org` | ❌ Wave 0: `tests/org_test.rs` |
| ORG-04 | Notes field included in fuzzy filter | unit (TS) | `npm test -- fuzzy` | ✅ extend `fuzzy.test.ts` |

### Sampling Rate
- **Per task commit:** `npm test && cargo test -p workpot-core`
- **Per wave merge:** `npm test && cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/workpot-core/tests/org_test.rs` — covers ORG-01 tag CRUD, ORG-02 pin mutations, ORG-04 notes CRUD
- [ ] `src/lib/tagFilter.ts` + `src/lib/tagFilter.test.ts` — covers `#tag` parsing and AND filter logic
- [ ] `src/lib/pinOrder.ts` + `src/lib/pinOrder.test.ts` — covers drag-reorder array helpers
- [ ] `src/lib/components/` directory — for DetailPane, TagChip, TagAutocomplete, SectionHeader Svelte components

---

## Security Domain

> `security_enforcement: true` in config.json (ASVS level 1).

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | Local-only app, no auth |
| V3 Session Management | No | No sessions |
| V4 Access Control | No | Single-user local app |
| V5 Input Validation | Yes | Tag length limits, notes max 500 chars (D-25), Tauri command input validation |
| V6 Cryptography | No | No encryption needed for local metadata |

### Known Threat Patterns for This Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Oversized tag/note input causing DB bloat | Tampering | Validate in Tauri command: tag max 64 chars, notes max 500 chars; reject with error |
| Tag containing path separators or special chars | Tampering | Normalize: strip `/`, `\`, null bytes from tags before insert. Tags are plain labels — alphanumeric + `-_` is sufficient |
| `repo_path` in IPC commands not validated against DB | Spoofing | Use `catalog::resolve_repo_path_key` pattern (already established) before any org mutation |
| XSS via tag/notes rendered in webview | Tampering | Svelte auto-escapes `{value}` interpolation. Do not use `@html`. Notes are plain text (D-27) |

**Input validation rules (Tauri command layer):**
- Tag: max 64 chars, trim whitespace, reject empty string after trim, reject tags containing `#` (reserved for filter syntax)
- Notes: max 500 chars (D-25), trim trailing whitespace before persist
- `repo_path`: must resolve to an existing non-excluded entry in `repos` table before mutation

---

## Project Constraints (from CLAUDE.md)

| Directive | Impact on Phase 5 |
|-----------|------------------|
| macOS only for v1 | Popup menu uses Tauri 2 macOS-native Menu API; no cross-platform abstraction needed |
| Tauri 2.x + Rust shared core | New org service goes in `workpot-core`; CLI and tray both consume it |
| SQLite local-only | Tags, notes, pins stored in existing `workpot.db` via migration 006 |
| Cursor launch required | Unchanged — Phase 5 does not touch launch path |
| No cloud sync / Firebase | Tags/notes are local-only — no sync API |
| Privacy: local-only index | Notes and tags never leave disk |
| Read-only git (no checkout) | Branch list in detail pane is display-only — no `git checkout` command |

---

## Sources

### Primary (HIGH confidence)
- Codebase inspection: `crates/workpot-core/src/` — `RepoRecord`, `catalog.rs`, `migrations.rs`, `store.rs`, `lib.rs` (established patterns for migration, IPC, error handling)
- Codebase inspection: `src/lib/` — `types.ts`, `sort.ts`, `fuzzy.ts`, `trayList.ts`, `+page.svelte` (established Svelte 5 runes patterns)
- `package.json` — Svelte 5, Tailwind 4, Vitest 3, TypeScript 5.6 (all verified in repo)
- `src-tauri/Cargo.toml` — Tauri 2, workpot-core path dep (verified in repo)
- `05-CONTEXT.md` — locked design decisions D-01 through D-28

### Secondary (MEDIUM confidence)
- [Tauri 2 Menu API (docs.rs)](https://docs.rs/tauri/latest/tauri/menu/struct.Menu.html) — confirms `Menu::popup` / `ContextMenu` trait exists in Tauri 2 [CITED]
- WebSearch — confirmed `svelte-dnd-action` is Svelte 3/4 only (npm description verified); confirmed HTML5 native drag as viable zero-dep alternative [VERIFIED: npm view]
- WebSearch — confirmed `sortablejs` 1.15.7, created 2014, legitimate package [VERIFIED: npm]

### Tertiary (LOW confidence)
- A4 assumption about `list_branches` — based on code inspection of `RepoRecord` having only `branch: Option<String>`. Needs planner to confirm approach (separate IPC call vs pre-load). [ASSUMED]

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — all packages already in workspace, no new deps required
- Architecture: HIGH — established rusqlite + Svelte 5 patterns, confirmed by code inspection
- Pitfalls: HIGH — derived from actual code reading (store.rs FK pragma gap, existing tx pattern)
- Drag-to-reorder: MEDIUM — HTML5 native drag recommended based on zero-dep tradeoff; functional but lacks animations

**Research date:** 2026-05-31
**Valid until:** 2026-07-01 (stable tech stack; Svelte 5 runes API is stable)
