# Phase 5: Tags & Prioritization - Pattern Map

**Mapped:** 2026-05-31
**Files analyzed:** 16 new/modified files
**Analogs found:** 15 / 16

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/workpot-core/src/domain/repo.rs` | model | CRUD | itself (extend) | exact |
| `crates/workpot-core/src/domain/config.rs` | model/config | CRUD | itself (extend) | exact |
| `crates/workpot-core/src/infra/migrations.rs` | migration | CRUD | itself (extend) | exact |
| `crates/workpot-core/src/infra/migrations/006_org.sql` | migration | CRUD | `migrations/005_tray.sql` | exact |
| `crates/workpot-core/src/services/org.rs` | service | CRUD | `services/catalog.rs` | exact |
| `crates/workpot-core/src/services/mod.rs` | config | — | itself (extend) | exact |
| `crates/workpot-core/src/lib.rs` | service | CRUD | itself (extend) | exact |
| `crates/workpot-core/tests/org_test.rs` | test | CRUD | `tests/tray_migration_test.rs` + `tests/catalog_test.rs` | exact |
| `src-tauri/src/commands.rs` | controller | request-response | itself (extend) | exact |
| `src-tauri/src/lib.rs` | config | request-response | itself (extend) | exact |
| `src/lib/types.ts` | model | — | itself (extend) | exact |
| `src/lib/sort.ts` | utility | transform | itself (replace) | exact |
| `src/lib/sort.test.ts` | test | transform | itself (extend) | exact |
| `src/lib/trayList.ts` | utility | transform | itself (extend) | exact |
| `src/lib/trayList.test.ts` | test | transform | itself (extend) | exact |
| `src/lib/fuzzy.ts` | utility | transform | itself (extend) | exact |
| `src/lib/fuzzy.test.ts` | test | transform | itself (extend) | exact |
| `src/lib/tagFilter.ts` | utility | transform | `src/lib/fuzzy.ts` | role-match |
| `src/lib/tagFilter.test.ts` | test | transform | `src/lib/fuzzy.test.ts` | exact |
| `src/lib/pinOrder.ts` | utility | transform | `src/lib/sort.ts` | role-match |
| `src/lib/pinOrder.test.ts` | test | transform | `src/lib/sort.test.ts` | exact |
| `src/routes/+page.svelte` | component | request-response | itself (extend) | exact |
| `src/lib/components/DetailPane.svelte` | component | request-response | `src/routes/+page.svelte` | role-match |
| `src/lib/components/TagChip.svelte` | component | event-driven | `src/routes/+page.svelte` (button pattern) | partial |
| `src/lib/components/TagAutocomplete.svelte` | component | event-driven | `src/routes/+page.svelte` (input pattern) | partial |
| `src/lib/components/SectionHeader.svelte` | component | — | no analog | none |

---

## Pattern Assignments

### `crates/workpot-core/src/domain/repo.rs` (model, CRUD)

**Analog:** itself — extend in place

**Current struct** (`repo.rs` lines 8-21):
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRecord {
    pub path: PathBuf,
    pub name: String,
    pub registered_at: i64,
    pub source: String,
    pub git_common_dir: String,
    pub branch: Option<String>,
    pub is_dirty: Option<bool>,
    pub ahead: Option<i64>,
    pub behind: Option<i64>,
    pub git_refreshed_at: Option<i64>,
    pub git_state_error: Option<String>,
    pub last_opened_at: Option<i64>,
}
```

**Add after `last_opened_at`:**
```rust
    // Phase 5 org fields:
    pub pinned: bool,
    pub pin_order: Option<i64>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
```

**Constructor update pattern** — all callers that construct `RepoRecord` must add the new fields. Follow `catalog.rs` line 63-76 (the `register_manual` return) and `catalog.rs` lines 94-108 (the `list_repos` `query_map` closure). The new fields default: `pinned: false`, `pin_order: None`, `notes: None`, `tags: Vec::new()`. The `list_repos` query must be extended with a JOIN or separate tag fetch (see `services/org.rs` pattern below).

---

### `crates/workpot-core/src/domain/config.rs` (model, CRUD)

**Analog:** itself — extend in place

**Serde default function pattern** (`config.rs` lines 7-21):
```rust
fn default_max_watch_roots() -> u32 { 100 }
fn default_max_repos() -> u32 { 1000 }
fn default_launch_cmd() -> String { "cursor --new-window {path}".to_string() }
fn default_max_visible_rows() -> u32 { 15 }
```

**Struct field with default** (`config.rs` lines 52-55):
```rust
#[serde(default = "default_max_visible_rows")]
pub max_visible_rows: u32,
```

**Add three new default functions and fields:**
```rust
fn default_max_pinned() -> u32 { 5 }
fn default_max_recent_days() -> u32 { 14 }
fn default_min_recent_count() -> u32 { 3 }
```

Then add to `Config` struct and `Config::default()`:
```rust
#[serde(default = "default_max_pinned")]
pub max_pinned: u32,
#[serde(default = "default_max_recent_days")]
pub max_recent_days: u32,
#[serde(default = "default_min_recent_count")]
pub min_recent_count: u32,
```

**Validate pattern** (`config.rs` lines 71-106): Add range checks for new fields in the `validate()` method, same pattern as `max_visible_rows`.

---

### `crates/workpot-core/src/infra/migrations.rs` (migration, CRUD)

**Analog:** itself — extend in place

**Full file** (`migrations.rs` lines 1-21):
```rust
use crate::error::Result;
use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

pub fn apply_migrations(conn: &mut Connection) -> Result<()> {
    static MIGRATION_001: &str = include_str!("migrations/001_init.sql");
    static MIGRATION_002: &str = include_str!("migrations/002_discovery.sql");
    static MIGRATION_003: &str = include_str!("migrations/003_git_state.sql");
    static MIGRATION_004: &str = include_str!("migrations/004_repos_source_index.sql");
    static MIGRATION_005: &str = include_str!("migrations/005_tray.sql");
    let steps = [
        M::up(MIGRATION_001),
        M::up(MIGRATION_002),
        M::up(MIGRATION_003),
        M::up(MIGRATION_004),
        M::up(MIGRATION_005),
    ];
    let migrations = Migrations::from_slice(&steps);
    migrations.to_latest(conn)?;
    Ok(())
}
```

**Add after `MIGRATION_005`:**
```rust
    static MIGRATION_006: &str = include_str!("migrations/006_org.sql");
```
And add `M::up(MIGRATION_006)` to the `steps` array.

---

### `crates/workpot-core/src/infra/migrations/006_org.sql` (migration, CRUD)

**Analog:** `migrations/005_tray.sql` (lines 1-1): `ALTER TABLE repos ADD COLUMN last_opened_at INTEGER NULL;`

**Pattern: `ALTER TABLE` for nullable columns, separate `CREATE TABLE` for new tables:**
```sql
ALTER TABLE repos ADD COLUMN notes     TEXT    NULL;
ALTER TABLE repos ADD COLUMN pinned    INTEGER NOT NULL DEFAULT 0;
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

**Critical:** Also update `store.rs::open_connection` to add `PRAGMA foreign_keys = ON` after the WAL pragma (line 12), so CASCADE DELETE on `repo_tags` works when a repo is removed:
```rust
conn.pragma_update(None, "foreign_keys", true)?;
```

---

### `crates/workpot-core/src/services/org.rs` (service, CRUD)

**Analog:** `crates/workpot-core/src/services/catalog.rs`

**Imports pattern** (`catalog.rs` lines 1-8):
```rust
use crate::domain::{Config, RepoRecord, SOURCE_MANUAL, SOURCE_SCAN};
use crate::error::{Result, WorkpotError};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
```

For `org.rs`:
```rust
use crate::error::{Result, WorkpotError};
use rusqlite::{Connection, params};
```

**Transaction pattern for atomic tag replace** (`catalog.rs` lines 178-179, `lib.rs` lines 179-180):
```rust
let tx = self.conn.unchecked_transaction()?;
// ... execute statements ...
tx.commit()?;
```

**`set_tags` atomic replace:**
```rust
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

**UPDATE with not-found check** (`catalog.rs` lines 135-142, `touch_last_opened_at`):
```rust
let updated = conn.execute(
    "UPDATE repos SET last_opened_at = ?1 WHERE path = ?2",
    params![now, path_key],
)?;
if updated == 0 {
    return Err(WorkpotError::NotFound(path_key));
}
Ok(())
```

Apply same pattern for `set_notes`, `set_pin`, `set_pin_order`.

**`query_map` collect pattern** (`catalog.rs` lines 92-111):
```rust
let rows = stmt.query_map([], |row| {
    Ok(SomeRecord { field: row.get(0)? })
})?;
rows.collect::<std::result::Result<Vec<_>, _>>()
    .map_err(WorkpotError::Database)
```

Apply same pattern for `list_tags_for_repo` and `list_all_tags`.

**`resolve_repo_path_key` for input validation** (`catalog.rs` line 164, called at top of every mutation): Call the public version from `catalog` before any org mutation to validate that `repo_path` exists in the DB.

---

### `crates/workpot-core/src/services/mod.rs` (config)

**Analog:** itself — extend in place (line 1-7)

Add `pub mod org;` to the module list.

---

### `crates/workpot-core/src/lib.rs` (service, CRUD)

**Analog:** itself — extend in place

**AppContext method delegation pattern** (`lib.rs` lines 83-100):
```rust
pub fn register_manual(&self, path: &Path) -> Result<RepoRecord> {
    catalog::register_manual(&self.conn, &self.config, path)
}

pub fn list_repos(&self) -> Result<Vec<RepoRecord>> {
    catalog::list_repos(&self.conn)
}

pub fn touch_last_opened_at(&self, path: &Path) -> Result<()> {
    catalog::touch_last_opened_at(&self.conn, path)
}
```

**Add `use crate::services::org;`** to the imports and add delegation methods:
```rust
pub fn set_tags(&self, repo_path: &str, tags: &[&str]) -> Result<()> {
    org::set_tags(&self.conn, repo_path, tags)
}
pub fn add_tag(&self, repo_path: &str, tag: &str) -> Result<()> {
    org::add_tag(&self.conn, repo_path, tag)
}
pub fn remove_tag(&self, repo_path: &str, tag: &str) -> Result<()> {
    org::remove_tag(&self.conn, repo_path, tag)
}
pub fn list_all_tags(&self) -> Result<Vec<String>> {
    org::list_all_tags(&self.conn)
}
pub fn set_notes(&self, repo_path: &str, notes: Option<&str>) -> Result<()> {
    org::set_notes(&self.conn, repo_path, notes)
}
pub fn set_pin(&self, repo_path: &str, pinned: bool) -> Result<()> {
    org::set_pin(&self.conn, repo_path, pinned)
}
pub fn set_pin_order(&self, items: &[(&str, i64)]) -> Result<()> {
    org::set_pin_order(&self.conn, items)
}
```

**`pub(crate) fn connection()`** (`lib.rs` line 119): Already exists for internal use. `org.rs` functions take `&Connection` directly (same as `catalog.rs` functions).

---

### `crates/workpot-core/tests/org_test.rs` (test, CRUD)

**Analog:** `tests/tray_migration_test.rs` (lines 1-16) for DB fixture; `tests/catalog_test.rs` for AppContext-level tests.

**Temp DB fixture pattern** (`tray_migration_test.rs` lines 10-16):
```rust
#![allow(clippy::disallowed_methods)]

fn temp_db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("workpot.db");
    let mut conn = Connection::open(&db_path).expect("open db");
    migrations::apply_migrations(&mut conn).expect("migrate");
    (dir, conn)
}
```

**AppContext fixture pattern** (`catalog_test.rs` lines 21-27):
```rust
fn git_fixture() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().join("sample-repo");
    fs::create_dir_all(&repo).expect("repo dir");
    git_init(&repo);
    (dir, repo)
}
```
And opening:
```rust
let ctx = AppContext::open_with_paths(config_path, db_path).expect("open");
```

**Assert pattern** — use `conn.query_row` to verify DB state after mutation, same as `tray_migration_test.rs` lines 27-40.

**Imports for `org_test.rs`:**
```rust
#![allow(clippy::disallowed_methods)]

use rusqlite::Connection;
use workpot_core::AppContext;
use workpot_core::infra::{migrations, store};
use workpot_core::services::org;
```

---

### `src-tauri/src/commands.rs` (controller, request-response)

**Analog:** itself — extend in place

**Tauri command signature pattern** (`commands.rs` lines 64-79):
```rust
#[tauri::command]
pub fn list_repos(state: State<'_, Arc<Mutex<AppContext>>>) -> Result<Vec<RepoDto>, String> {
    let ctx = state
        .lock()
        .map_err(|_| "AppContext lock poisoned".to_string())?;
    let records = ctx.list_repos().map_err(|e| e.to_string())?;
    Ok(repo_records_to_dtos(records))
}
```

**Async command pattern** (`commands.rs` lines 136-143):
```rust
#[tauri::command]
pub async fn refresh_all_git_state(
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppContext>>>,
) -> Result<(), String> {
    spawn_background_git_refresh(app, state.inner().clone());
    Ok(())
}
```

**`record_to_dto` mapping** (`commands.rs` lines 22-32): All new `RepoRecord` fields (`pinned`, `pin_order`, `notes`, `tags`) must be mapped here. Follow the same pattern — direct field copy for primitives, `.clone()` for `Vec<String>`.

**Input validation pattern:** The org mutation commands (`set_tags`, `set_notes`, `set_pin`, `set_pin_order`) follow the sync command pattern (not async). They lock the mutex, call the `AppContext` method, map errors to `String`. Add input validation before the lock (tag max 64 chars, notes max 500 chars).

**New `RepoDto` struct additions** (`commands.rs` lines 7-16 — the `RepoDto` struct):
```rust
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RepoDto {
    // existing fields...
    // Phase 5 additions:
    pub pinned: bool,
    pub pin_order: Option<i64>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
}
```

**Context menu command pattern** — new `show_repo_context_menu` command uses `AppHandle` + `State`, same signature style as `refresh_all_git_state`. The Tauri 2 menu is built with `tauri::menu::{Menu, MenuItem}` and dispatched with `menu.popup(window)?`. The `MenuEvent` handler must call `app.emit("repo-context-action", payload)` for the Svelte side to receive it (see Shared Patterns below).

---

### `src-tauri/src/lib.rs` (config, request-response)

**Analog:** itself — extend in place

**`invoke_handler` registration pattern** (`lib.rs` lines 37-43):
```rust
.invoke_handler(tauri::generate_handler![
    commands::list_repos,
    commands::get_tray_config,
    commands::refresh_all_git_state,
    commands::open_in_cursor
])
```

Add new commands: `commands::set_tags`, `commands::add_tag`, `commands::remove_tag`, `commands::list_all_tags`, `commands::set_notes`, `commands::set_pin`, `commands::set_pin_order`, `commands::show_repo_context_menu`.

**`on_menu_event` registration** — register the menu event handler inside `.setup()` using `app.on_menu_event(|app, event| { ... })`. The handler emits a typed Tauri event (`app.emit("repo-context-action", payload)`) for the Svelte frontend to listen on.

---

### `src/lib/types.ts` (model)

**Analog:** itself — extend in place

**Current `RepoDto` interface** (`types.ts` lines 11-19):
```typescript
export interface RepoDto {
  path: string;
  name: string;
  branch: string | null;
  is_dirty: boolean | null;
  parent_dir: string;
  last_opened_at: number | null;
  git_state_error: string | null;
}
```

**Add Phase 5 fields** (after `git_state_error`):
```typescript
  pinned: boolean;
  pin_order: number | null;
  notes: string | null;
  tags: string[];
  branches: string[];
```

---

### `src/lib/sort.ts` (utility, transform)

**Analog:** itself — replace content

**Current `traySort` comparator style** (`sort.ts` lines 1-33): Two-tier dirty/recent comparator. Phase 5 replaces the flat comparator with section grouping. The internal within-section sort helpers (`byLastOpenedDesc`) follow the same comparator style.

**Current imports pattern** (`sort.ts` line 1):
```typescript
import type { RepoDto } from "./types";
```

**Section type pattern** (new, based on RESEARCH.md Pattern 4):
```typescript
export type Section = "pinned" | "dirty" | "recent" | "rest";

export interface SectionedRepos {
  pinned: RepoDto[];
  dirty: RepoDto[];
  recent: RepoDto[];
  rest: RepoDto[];
}

export interface SectionConfig {
  maxRecentDays: number;
  minRecentCount: number;
}
```

**Within-section comparator pattern** (same style as existing `traySort`):
```typescript
function byLastOpenedDesc(a: RepoDto, b: RepoDto): number {
  const aTs = a.last_opened_at;
  const bTs = b.last_opened_at;
  if (aTs != null && bTs != null && aTs !== bTs) return bTs - aTs;
  if (aTs != null && bTs == null) return -1;
  if (aTs == null && bTs != null) return 1;
  return a.name.localeCompare(b.name);
}
```

---

### `src/lib/sort.test.ts` (test, transform)

**Analog:** itself — extend in place

**Test helper `repo()` factory pattern** (`sort.test.ts` lines 5-15):
```typescript
function repo(partial: Partial<RepoDto> & Pick<RepoDto, "name">): RepoDto {
  return {
    path: `/tmp/${partial.name}`,
    name: partial.name,
    branch: partial.branch ?? null,
    is_dirty: partial.is_dirty ?? null,
    parent_dir: "",
    last_opened_at: partial.last_opened_at ?? null,
    git_state_error: null,
  };
}
```

**Extend the factory** with Phase 5 fields:
```typescript
    pinned: partial.pinned ?? false,
    pin_order: partial.pin_order ?? null,
    notes: partial.notes ?? null,
    tags: partial.tags ?? [],
    branches: partial.branches ?? [],
```

**Test structure** (`sort.test.ts` lines 17-47): `describe("sectionSort", () => { it("...", () => { ... }); })`. Keep `describe("traySort", ...)` or migrate tests to `sectionSort`.

---

### `src/lib/trayList.ts` (utility, transform)

**Analog:** itself — extend in place

**Current content** (`trayList.ts` lines 1-8):
```typescript
import { fuzzyMatch } from "./fuzzy";
import { traySort } from "./sort";
import type { RepoDto } from "./types";

export function filterAndSortRepos(repos: RepoDto[], query: string): RepoDto[] {
  return repos.filter((r) => fuzzyMatch(query, r)).sort(traySort);
}
```

**Extend imports** to add `sectionSort`, `SectionedRepos`, `SectionConfig`, `matchesTags`, `parseTagFilter`. Update `filterAndSortRepos` to return `SectionedRepos` (or keep returning flat array for backward compat and add a new `filterAndSectionRepos` function that applies tag filter then sections).

---

### `src/lib/trayList.test.ts` (test, transform)

**Analog:** itself — extend in place. Follow `trayList.test.ts` lines 1-37 for structure and the `repo()` factory pattern (same as `sort.test.ts`). Extend factory with Phase 5 fields.

---

### `src/lib/fuzzy.ts` (utility, transform)

**Analog:** itself — extend in place

**`fuzzyScore` scores array** (`fuzzy.ts` lines 50-55):
```typescript
const scores = [
  scoreField(q, repo.name, true),
  scoreField(q, repo.path, false),
  scoreField(q, repo.branch ?? "", false),
];
return Math.max(...scores);
```

**Add after `branch` score:**
```typescript
  scoreField(q, repo.notes ?? "", false),
  ...repo.tags.map((t) => scoreField(q, t, false)),
```

---

### `src/lib/fuzzy.test.ts` (test, transform)

**Analog:** itself — extend in place

**`repo()` factory pattern** (`fuzzy.test.ts` lines 5-15): Extend with Phase 5 fields (same as `sort.test.ts` factory). Add tests for notes and tags matching.

---

### `src/lib/tagFilter.ts` (utility, transform)

**Analog:** `src/lib/fuzzy.ts` for import/export style; RESEARCH.md Pattern 7 for logic.

**Imports pattern** (copy from `fuzzy.ts` line 1):
```typescript
import type { RepoDto } from "./types";
```

**Functions to implement** (from RESEARCH.md Code Examples):
```typescript
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

### `src/lib/tagFilter.test.ts` (test, transform)

**Analog:** `src/lib/fuzzy.test.ts`

**Imports and describe pattern** (`fuzzy.test.ts` lines 1-3):
```typescript
import { describe, expect, it } from "vitest";
import { parseTagFilter, matchesTags } from "./tagFilter";
import type { RepoDto } from "./types";
```

**`repo()` factory** — same pattern as `fuzzy.test.ts` lines 5-15, extended with `tags` field.

---

### `src/lib/pinOrder.ts` (utility, transform)

**Analog:** `src/lib/sort.ts` for style; new logic.

**Imports pattern** (copy from `sort.ts` line 1):
```typescript
import type { RepoDto } from "./types";
```

**Function to implement:**
```typescript
/** Move item at dragSourceIdx to targetIdx, returning a new array with pin_order assigned. */
export function reorderPinned(repos: RepoDto[], from: number, to: number): RepoDto[] {
  if (from === to) return repos;
  const result = [...repos];
  const [item] = result.splice(from, 1);
  result.splice(to, 0, item);
  return result.map((r, i) => ({ ...r, pin_order: i }));
}

/** Build the set_pin_order IPC payload from an ordered list. */
export function toPinOrderPayload(repos: RepoDto[]): Array<{ path: string; order: number }> {
  return repos.map((r, i) => ({ path: r.path, order: i }));
}
```

---

### `src/lib/pinOrder.test.ts` (test, transform)

**Analog:** `src/lib/sort.test.ts`

**Imports and describe pattern** (`sort.test.ts` lines 1-2):
```typescript
import { describe, expect, it } from "vitest";
import { reorderPinned, toPinOrderPayload } from "./pinOrder";
```

**`repo()` factory** — same as `sort.test.ts` lines 5-15, extended with Phase 5 fields.

---

### `src/routes/+page.svelte` (component, request-response)

**Analog:** itself — extend in place

**`invoke` call pattern** (`+page.svelte` lines 150-157):
```typescript
async function loadRepos(clearError = true) {
  try {
    repos = await invoke<RepoDto[]>("list_repos");
    if (clearError) { error = null; }
  } catch (e) {
    error = String(e);
  }
}
```

**`listen` event pattern** (`+page.svelte` lines 181-213):
```typescript
const unlistenPanel = listen("panel-opened", () => { ... });
// cleanup in return:
return () => { void unlistenPanel.then((fn) => fn()); };
```

**Right-arrow key for detail pane:** Add `ArrowRight` handling to `onPanelKeydown` (lines 121-146) and `onFilterKeydown` (lines 90-119), following the same `e.preventDefault()` + state update pattern. Track `detailRepo = $state<RepoDto | null>(null)`.

**Context menu:** Add `oncontextmenu` handler to repo `<button>` (line 275-289), call `invoke("show_repo_context_menu", {...})`. Listen for `"repo-context-action"` event with `listen()` in `onMount`.

**Section rendering:** The flat `{#each displayRepos as repo, i (repo.path)}` (line 273) becomes four section-grouped `{#each}` blocks, each preceded by a `<SectionHeader>` component.

---

### `src/lib/components/DetailPane.svelte` (component, request-response)

**Analog:** `src/routes/+page.svelte` for IPC invoke/listen patterns and Svelte 5 runes style.

**Svelte 5 runes pattern** (`+page.svelte` lines 19-26):
```typescript
let repos = $state<RepoDto[]>([]);
let filterQuery = $state("");
let selectedIndex = $state(0);
let displayRepos = $derived(filterAndSortRepos(repos, filterQuery));
```

**IPC invoke pattern** for notes save-on-blur:
```typescript
async function saveNotes() {
  try {
    await invoke("set_notes", { repoPath: repo.path, notes: notesValue });
  } catch (e) {
    // surface error
  }
}
```

**Props pattern** — use Svelte 5 `let { repo, onClose }: { repo: RepoDto; onClose: () => void } = $props();`.

**Tailwind class pattern** (`+page.svelte` lines 280-288): Use `class="..."` with dark mode variants. Section structure: `flex flex-col gap-2 p-3`.

**Pin toggle:** A `<input type="checkbox">` that calls `invoke("set_pin", { repoPath: repo.path, pinned: !repo.pinned })` on change, then calls `onClose()` (triggers reload in parent).

**Tag chip management:** Inline tag list where Cmd+Click calls `invoke("remove_tag", { repoPath, tag })` and reloads. Add-tag input calls `invoke("add_tag", { repoPath, tag })` on Enter.

---

### `src/lib/components/TagChip.svelte` (component, event-driven)

**Analog:** `src/routes/+page.svelte` `<button>` pattern (lines 275-293).

**Svelte 5 props:** `let { tag, onRemove }: { tag: string; onRemove?: () => void } = $props();`

**Cmd+Click pattern** (from `+page.svelte` line 283 `e.metaKey` check):
```svelte
<button
  type="button"
  onclick={(e) => { if (e.metaKey && onRemove) onRemove(); }}
  class="rounded-full bg-blue-100 px-2 py-0.5 text-xs text-blue-800 dark:bg-blue-900 dark:text-blue-200"
>
  #{tag}
</button>
```

---

### `src/lib/components/TagAutocomplete.svelte` (component, event-driven)

**Analog:** `src/routes/+page.svelte` filter input (lines 244-262) + keyboard nav pattern (lines 90-119).

**Svelte 5 props:** `let { allTags, onSelect }: { allTags: string[]; onSelect: (tag: string) => void } = $props();`

**Reactive filter pattern** (Svelte 5 `$derived`, similar to `+page.svelte` line 30):
```typescript
let inputValue = $state("");
let filtered = $derived(
  allTags.filter((t) => t.toLowerCase().startsWith(inputValue.toLowerCase()))
);
```

**Keyboard nav:** Arrow down/up + Enter, same style as `onFilterKeydown` in `+page.svelte` lines 90-119.

**Visibility trigger:** Show only when `filterQuery.includes("#")` — parent controls via a prop `visible: boolean`.

---

## Shared Patterns

### Tauri command — lock → call → map error
**Source:** `src-tauri/src/commands.rs` lines 64-79
**Apply to:** All new sync Tauri commands (`set_tags`, `set_notes`, `set_pin`, `set_pin_order`, `add_tag`, `remove_tag`, `list_all_tags`)
```rust
#[tauri::command]
pub fn command_name(
    arg: Type,
    state: State<'_, Arc<Mutex<AppContext>>>,
) -> Result<ReturnType, String> {
    let ctx = state
        .lock()
        .map_err(|_| "AppContext lock poisoned".to_string())?;
    ctx.method(arg).map_err(|e| e.to_string())
}
```

### WorkpotError — NotFound guard
**Source:** `crates/workpot-core/src/services/catalog.rs` lines 135-142
**Apply to:** All org mutation functions in `services/org.rs`
```rust
let updated = conn.execute("UPDATE repos SET ... WHERE path = ?1", params![...])?;
if updated == 0 {
    return Err(WorkpotError::NotFound(repo_path.to_string()));
}
Ok(())
```

### SQLite transaction — unchecked_transaction
**Source:** `crates/workpot-core/src/lib.rs` lines 179-180
**Apply to:** `org::set_tags`, `org::set_pin_order` (multi-statement atomic ops)
```rust
let tx = self.conn.unchecked_transaction()?;
// ... statements ...
tx.commit()?;
```

### Svelte IPC invoke + error state
**Source:** `src/routes/+page.svelte` lines 148-157
**Apply to:** All `invoke()` calls in `DetailPane.svelte`, `+page.svelte` context menu handler
```typescript
try {
  await invoke("command_name", { arg: value });
} catch (e) {
  error = String(e);
}
```

### Svelte listen + cleanup
**Source:** `src/routes/+page.svelte` lines 181-213
**Apply to:** `+page.svelte` `"repo-context-action"` listener, any new event listeners in components
```typescript
const unlisten = listen("event-name", (event) => { ... });
return () => { void unlisten.then((fn) => fn()); };
```

### Tauri app event emit (Rust → Svelte)
**Source:** `src-tauri/src/commands.rs` lines 121-129
**Apply to:** `show_repo_context_menu` MenuEvent handler
```rust
let _ = app.emit("repo-context-action", &payload);
```

### Serde default function for Config fields
**Source:** `crates/workpot-core/src/domain/config.rs` lines 7-21
**Apply to:** New `Config` fields `max_pinned`, `max_recent_days`, `min_recent_count`
```rust
fn default_max_pinned() -> u32 { 5 }

#[serde(default = "default_max_pinned")]
pub max_pinned: u32,
```

### Test `temp_db()` fixture
**Source:** `crates/workpot-core/tests/tray_migration_test.rs` lines 10-16
**Apply to:** `tests/org_test.rs` — use same fixture to get a migrated `Connection` for low-level service tests

### Test `AppContext::open_with_paths` fixture
**Source:** `crates/workpot-core/tests/catalog_test.rs` lines 113-117
**Apply to:** `tests/org_test.rs` — use `AppContext::open_with_paths` for higher-level integration tests

### Vitest `repo()` factory
**Source:** `src/lib/sort.test.ts` lines 5-15 and `src/lib/fuzzy.test.ts` lines 5-15
**Apply to:** `tagFilter.test.ts`, `pinOrder.test.ts`, extended `sort.test.ts`, extended `trayList.test.ts`
```typescript
function repo(partial: Partial<RepoDto> & Pick<RepoDto, "name">): RepoDto {
  return {
    path: `/tmp/${partial.name}`,
    name: partial.name,
    branch: partial.branch ?? null,
    is_dirty: partial.is_dirty ?? null,
    parent_dir: "",
    last_opened_at: partial.last_opened_at ?? null,
    git_state_error: null,
    // Phase 5:
    pinned: partial.pinned ?? false,
    pin_order: partial.pin_order ?? null,
    notes: partial.notes ?? null,
    tags: partial.tags ?? [],
    branches: partial.branches ?? [],
  };
}
```

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `src/lib/components/SectionHeader.svelte` | component | — | No existing section header or labeled divider component in codebase. Pure presentational — use Tailwind `text-xs text-neutral-400 font-medium uppercase tracking-wide px-2 py-1` pattern from `+page.svelte` text utility classes. |

---

## Metadata

**Analog search scope:** `crates/workpot-core/src/`, `crates/workpot-cli/src/`, `src/lib/`, `src/routes/`, `src-tauri/src/`
**Files scanned:** 31 source files read
**Pattern extraction date:** 2026-05-31
