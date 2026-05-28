# Architecture Research

**Domain:** macOS local-first multi-repo git workspace launcher (tray + CLI)
**Researched:** 2026-05-28
**Confidence:** HIGH (stack patterns from official Tauri/Rust docs); MEDIUM (product-specific boundaries — greenfield, no prior Workpot code)

## Standard Architecture

### System Overview

Workpot is a **local control plane** over many git repos: discover → snapshot git state → persist metadata → rank/search → act (open Cursor, run recipes). There is no server, no sync, and no code-indexing in v1. The filesystem path is canonical identity; SQLite is a **materialized view** of git + user metadata, not a second source of truth.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Presentation (2 entrypoints)                     │
├──────────────────────────────┬──────────────────────────────────────────┤
│  Tauri Shell                 │  CLI (clap)                               │
│  • TrayIcon + popup window   │  • workpot search | open | index | recipe │
│  • Filter-as-you-type UI     │  • JSON/human output for scripts          │
│  • IPC commands → core       │  • Calls same AppContext as tray          │
└──────────────┬───────────────┴──────────────────┬───────────────────────┘
               │                                   │
┌──────────────▼───────────────────────────────────▼───────────────────────┐
│                    Application services (workpot-core)                    │
│  ┌─────────────┐ ┌──────────────┐ ┌────────────┐ ┌──────────────────┐  │
│  │ Catalog     │ │ Refresh      │ │ Search &   │ │ Recipe runner    │  │
│  │ (CRUD repos │ │ (discover +  │ │ Rank       │ │ (steps: shell,   │  │
│  │  tags,pins) │ │  git snapshot)│ │ (fuzzy +   │ │  cursor, chain)  │  │
│  └──────┬──────┘ └──────┬───────┘ │  signals)  └────────┬─────────┘  │
│         │               │         └──────────┬──────────┘            │
│         │               │                    │                         │
│  ┌──────▼───────────────▼────────────────────▼─────────────────────┐  │
│  │ Launcher (Cursor) — spawn `cursor <path>` / open -a Cursor       │  │
│  └──────────────────────────────────────────────────────────────────┘  │
├──────────────────────────────────────────────────────────────────────────┤
│                         Domain (pure types + rules)                       │
│  RepoRecord(path id) · GitSnapshot · Tag · Pin · Recipe · WatchPolicy   │
├──────────────────────────────────────────────────────────────────────────┤
│                         Infrastructure adapters                           │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌───────────────┐  │
│  │ SQLite store │ │ Git client   │ │ FS discovery │ │ FS watcher    │  │
│  │ rusqlite WAL │ │ git2-rs      │ │ walkdir +    │ │ notify +      │  │
│  │ migrations   │ │ status/branch│ │ .git detect  │ │ debouncer     │  │
│  └──────────────┘ └──────────────┘ └──────────────┘ └───────────────┘  │
└──────────────────────────────────────────────────────────────────────────┘
         ▲                              ▲
         │                              │
    ~/Library/Application Support/   Watch roots on disk
    workpot/workpot.db               (git repos = source of truth)
```

Tauri’s process model fits this shape: **one Rust core process** owns tray, global state, DB, and background refresh; the WebView is a thin UI that calls commands and listens for events ([Tauri process model](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/concept/process-model.md), [project structure](https://v2.tauri.app/start/project-structure/)).

### Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|----------------|------------------------|
| **workpot-core** | Domain types, business rules, orchestration; no Tauri/UI deps | Rust library crate; `AppContext` holding config + store + services |
| **Catalog service** | Register/exclude repos, tags, pins, notes; enforce path-as-id | CRUD on `repos` table; merge manual overrides with scan results |
| **Discovery** | Find git repos under watch roots; respect exclude globs | `walkdir` + stop at nested `.git`; optional depth limits |
| **Git snapshot** | Per-repo: branch, clean/dirty, ahead/behind, last activity signal | `git2` `statuses()`, `head()`, `graph_ahead_behind()` ([git2 status](https://docs.rs/git2/latest/git2/struct.Repository.html)) |
| **Refresh pipeline** | Full or incremental re-index; coalesce concurrent refreshes | Tokio task + job queue; single writer to SQLite |
| **Store** | Persist catalog + snapshots + user metadata | `rusqlite` + WAL + `rusqlite_migration` ([rusqlite WAL](https://context7.com/rusqlite/rusqlite/llms.txt)) |
| **Search & rank** | Metadata filter + fuzzy match + score (dirty, recent, pinned) | In-memory fuse/nucleo over query result set, or SQLite FTS for names only in v1 |
| **Launcher** | Open repo in Cursor | `std::process::Command` → `cursor <path>` ([Cursor CLI](https://cursor.com/docs/cli/using)); fallback `open -a Cursor` |
| **Recipe runner** | Execute declarative steps (shell, open IDE, sequences) | Load YAML/TOML from config dir; subprocess per step with cwd = repo path |
| **FS watcher** | Debounced invalidation of affected repos | `notify-debouncer-mini` on watch roots ([notify](https://github.com/notify-rs/notify)) |
| **workpot-cli** | Power-user surface; scriptable output | `clap` binary depending on `workpot-core` only |
| **Tauri shell** | Tray, popup window, IPC glue, macOS lifecycle | `src-tauri` as workspace member; `lib.rs` = setup; commands delegate to core |
| **Tray UI** | Prioritized list, filter input, keyboard nav | Minimal Vite frontend; no business logic in JS |

## Recommended Project Structure

```
workpot/
├── Cargo.toml                    # [workspace] members below
├── crates/
│   ├── workpot-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # AppContext, public API
│   │       ├── domain/           # RepoRecord, GitSnapshot, Recipe, ...
│   │       ├── services/         # catalog, refresh, search, recipes, launcher
│   │       └── infra/            # sqlite, git2, discover, watcher traits + impls
│   └── workpot-cli/
│       ├── Cargo.toml
│       └── src/main.rs           # clap → AppContext
├── src-tauri/
│   ├── Cargo.toml                # depends on workpot-core
│   ├── tauri.conf.json
│   ├── capabilities/
│   └── src/
│       ├── main.rs               # thin: workpot_app_lib::run()
│       └── lib.rs                # tray setup, command registration, macOS policy
├── ui/                           # tray popup only (small surface)
│   ├── src/
│   └── package.json
└── recipes/                      # optional: example recipe schemas (not runtime data)
```

### Structure Rationale

- **Cargo workspace with `workpot-core` as the product:** Tauri is an optional shell over the library, not the center of the repo ([Tauri project structure](https://v2.tauri.app/start/project-structure/), [workspace + Tauri pattern](https://github.com/tauri-apps/tauri/issues/4232#issuecomment)). Enables fast unit tests without launching WebView.
- **`domain/` vs `services/` vs `infra/`:** Matches proven local-git catalog tools (e.g. repoindex’s service/domain/infra split). Keeps git/SQLite swappable and testable with fakes.
- **`src-tauri` stays thin:** Tray events, window show/hide, and `invoke` handlers only; no indexing logic in `lib.rs` beyond wiring.
- **`ui/` minimal:** Tray popup is a list + input; ranking and git logic stay in Rust for CLI parity and consistency.

## Architectural Patterns

### Pattern 1: Shared core, multiple hosts (CLI + Tauri)

**What:** One `workpot-core` library; CLI and Tauri are thin hosts that construct `AppContext` and call the same services.

**When to use:** Always — PROJECT.md requires CLI and tray with identical behavior.

**Trade-offs:** Slightly more crate boilerplate upfront; pays off immediately for testing and feature parity.

**Example:**
```rust
// crates/workpot-core/src/lib.rs
pub struct AppContext {
    config: Config,
    store: SqliteStore,
    refresh: RefreshService,
    search: SearchService,
    launcher: CursorLauncher,
    recipes: RecipeRunner,
}

impl AppContext {
    pub fn open_repo(&self, path: &Path) -> Result<()> {
        self.launcher.open_folder(path)
    }
}
```

### Pattern 2: SQLite as materialized view over git

**What:** Git repos on disk are source of truth; DB rows are snapshots at `indexed_at`. User fields (tags, pins, notes) merge on top. Full rebuild = `DELETE` + rescan or versioned refresh, not bidirectional sync.

**When to use:** All indexing and git-state display.

**Trade-offs:** Stale data between refreshes (mitigate with watcher + debounce). Avoids inventing a parallel repo model.

**Example:**
```sql
-- repos.path is PRIMARY KEY (identity)
CREATE TABLE repos (
  path TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  branch TEXT,
  is_dirty INTEGER,
  ahead INTEGER,
  behind INTEGER,
  last_seen_at INTEGER,
  pinned INTEGER DEFAULT 0
);
CREATE TABLE repo_tags (path TEXT, tag TEXT, PRIMARY KEY (path, tag));
```

### Pattern 3: Single-writer refresh queue

**What:** All mutations to SQLite go through one async worker (mpsc channel). UI/CLI enqueue `RefreshJob::{Full, Paths(Vec)}`; worker serializes git I/O and DB writes.

**When to use:** From first store implementation — prevents WAL lock contention and duplicate git scans.

**Trade-offs:** UI must tolerate eventual consistency (show `indexing…` or last-known snapshot). Correct for tray glance UX.

### Pattern 4: Path-triggered incremental refresh

**What:** Watch roots with debounced FS events; map changed paths → repo root (walk up to `.git`) → enqueue single-repo refresh. Periodic full scan as safety net (e.g. daily or on `workpot index --full`).

**When to use:** After full refresh works; not day one unless timeboxed.

**Trade-offs:** `notify` can miss events on huge trees; watch roots should be intentional (`~/dev`), not `$HOME` ([notify limits](https://docs.rs/notify/latest/notify/)).

### Pattern 5: Tauri command boundary = DTO in/out

**What:** Commands accept/return serde structs (`RepoDto`, `SearchResultDto`), not internal domain types. Core services return domain; Tauri layer maps.

**When to use:** Every `#[tauri::command]`.

**Trade-offs:** Small mapping boilerplate; keeps frontend ignorant and CLI free of IPC shapes.

## Data Flow

### Request Flow (open repo from tray)

```
User types filter → Enter on row
    ↓
WebView: invoke('open_repo', { path })
    ↓
Tauri command handler (src-tauri)
    ↓
AppContext::open_repo(path)
    ↓
CursorLauncher::spawn(path)     // `cursor path` or `open -a Cursor path`
    ↓
Optional: CatalogService::touch_recent(path)
    ↓
SQLite UPDATE last_opened_at
    ↓
emit('repo-opened') → UI may close popup
```

### Index refresh flow

```
Trigger: startup | CLI `index` | watcher debounce | manual menu
    ↓
RefreshService::enqueue(job)
    ↓
[Worker] Discovery::scan(watch_roots) → candidate paths
    ↓
Filter: excludes, manual removals, non-git
    ↓
For each repo (parallel with limit): GitClient::snapshot(path)
    ↓
Store::upsert_repos(batch) in transaction
    ↓
emit('index-updated') → tray UI reloads list
```

### Search / filter flow (tray filter-as-you-type)

```
Keystroke in WebView
    ↓
invoke('search', { query, limit })  [debounced 50–100ms in UI]
    ↓
SearchService::query(query)
    ↓
SQLite: coarse filter (tags, branch prefix) optional
    ↓
In-memory fuzzy rank on name/path + boost pinned/dirty/recent
    ↓
Return Vec<RepoDto> ordered by score
```

### State Management

```
┌─────────────────────────────────────────┐
│ SQLite (repos, tags, recipes meta)      │  ← durable
└─────────────────┬───────────────────────┘
                  │ read on search; write via refresh worker only
┌─────────────────▼───────────────────────┐
│ AppContext (owned by Tauri setup / CLI) │
│  + in-memory: last search cache (opt)   │
└─────────────────┬───────────────────────┘
                  │ Tauri events: index-updated, repo-opened
┌─────────────────▼───────────────────────┐
│ WebView UI state (query string, selection)│  ← ephemeral only
└─────────────────────────────────────────┘
```

Do **not** mirror repo list in frontend global state as source of truth — always refetch after `index-updated` or open.

### Key Data Flows

1. **Cold start:** Load config → open DB + migrate → enqueue full refresh → show tray → popup reads DB (may show stale until first refresh completes).
2. **CLI `workpot open foo`:** Parse fuzzy name → `SearchService` → `Launcher` (no UI).
3. **Recipe run:** Resolve repo → `RecipeRunner` loads definition → sequential steps with `current_dir(repo)` → stream stdout to CLI or log file; tray shows toast on completion.

## Suggested Build Order

Build vertically by **proving the core loop in CLI first**, then wrap with Tauri. This de-risks git/DB before tray/macOS polish.

| Phase | Build | Delivers | Depends on |
|-------|--------|----------|------------|
| **1** | `workpot-core` skeleton + config paths + SQLite schema/migrations | Persistent store, empty CLI boots | — |
| **2** | Discovery + git snapshot + refresh worker | `workpot index` lists repos with branch/dirty | 1 |
| **3** | Catalog (manual add/exclude, tags, pins) + search/rank | `workpot search`, meaningful ordering | 2 |
| **4** | Cursor launcher + `workpot open` | Core value without UI | 3 |
| **5** | `workpot-cli` complete (index, search, open, JSON output) | Power-user loop shippable | 4 |
| **6** | Tauri shell: tray, popup window, IPC wiring | Tray shows same data as CLI | 5 |
| **7** | Tray UI: filter-as-you-type, keyboard nav, positioner | Daily loop in menu bar | 6 |
| **8** | Recipes (schema + runner + CLI/tray trigger) | “Open and run” workflows | 4 |
| **9** | FS watcher + incremental refresh | Fresher dirty state without full scan | 2 |

**Rationale:** Phases 1–5 validate PROJECT.md core value without WebView complexity. Phase 6–7 add macOS-specific concerns (`ActivationPolicy::Accessory`, tray-relative window via `tauri-plugin-positioner` ([system tray docs](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/learn/system-tray.mdx))). Recipes after open works so steps can call launcher. Watcher last — correctness already guaranteed by manual/scheduled refresh.

## Scaling Considerations

Workpot is **single-user, single-machine**. “Scale” means repo count and watch-tree size, not multi-tenant load.

| Scale | Repos / watch size | Architecture adjustments |
|-------|-------------------|---------------------------|
| **Personal (v1)** | 10–80 repos, 1–3 watch roots | Monolith core; full fuzzy in memory; single refresh worker |
| **Heavy user** | 200–500 repos | Cap parallel git status (e.g. 8 workers); SQLite indexes on `is_dirty`, `last_opened_at`; consider FTS5 on `name` only |
| **Pathological** | 1000+ under one root | Narrow watch roots; don’t watch `$HOME`; batch discovery; optional “lazy” git status (only pinned + recently opened) |

### Scaling Priorities

1. **First bottleneck:** Git status on hundreds of repos blocks refresh — parallelize with semaphore; skip bare repos; cache `ahead/behind` only when `origin` exists.
2. **Second bottleneck:** FS watcher fanout — debounce 500ms–2s; map events to repo roots; fall back to hourly full scan.

## Anti-Patterns

### Anti-Pattern 1: Business logic in Tauri commands or React

**What people do:** Call `git2` and SQL directly from `#[tauri::command]` or compute rank scores in the frontend.

**Why it's wrong:** CLI diverges; untestable without WebView; violates shared-core goal.

**Do this instead:** Commands call `AppContext` methods only; UI sends query strings and renders `RepoDto` list.

### Anti-Pattern 2: SQLite as source of truth for git state

**What people do:** Manually update `is_dirty` on FS events without re-running git status.

**Why it's wrong:** Drift from actual git state; erodes trust in “glance at tray” UX.

**Do this instead:** FS events **invalidate** and enqueue refresh; git commands recompute snapshot.

### Anti-Pattern 3: Deduping repos by remote URL

**What people do:** Collapse `~/work/foo` and `~/personal/foo` because same `origin`.

**Why it's wrong:** Conflicts with path-as-identity and local branch/dirty semantics ([repoindex design](https://github.com/queelius/repoindex/blob/master/DESIGN.md)).

**Do this instead:** One row per path; optional display of remote for disambiguation in UI only.

### Anti-Pattern 4: Monolithic `src-tauri/src/lib.rs`

**What people do:** 2000-line Tauri setup with indexing, recipes, and SQL inline.

**Why it's wrong:** Can't ship CLI; slow compile/test cycle; Tauri coupling everywhere.

**Do this instead:** `workpot-core` + thin IPC layer from day one.

### Anti-Pattern 5: Watching entire home directory

**What people do:** Single watch root `~` for “convenience.”

**Why it's wrong:** notify limits, noise, battery; refresh storms on unrelated changes.

**Do this instead:** Explicit watch roots (`~/dev`, `~/work`); manual `workpot add` for outliers.

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| **Cursor IDE** | Subprocess: `cursor <path>` | User must install shell command ([Cursor docs](https://cursor.com/docs/cli/using)); detect failure and surface clear error |
| **macOS `open`** | Fallback launcher | `open -a Cursor <path>` if CLI not in PATH |
| **Shell (recipes)** | `std::process::Command` with `sh -c` or explicit argv | No arbitrary root; recipes are user-defined local files |
| **Git** | libgit2 via `git2` crate | Prefer in-process over parsing `git` CLI for status; optional `git` CLI fallback for edge cases (LOW confidence — validate in phase research) |

No cloud APIs in v1 (per PROJECT.md).

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| **UI ↔ Tauri** | `invoke` + `listen` events | DTOs only; debounce search |
| **Tauri ↔ core** | Direct Rust calls | `AppContext` in `tauri::State` |
| **CLI ↔ core** | Direct Rust calls | Same `AppContext::new()` from config path |
| **Refresh worker ↔ store** | Channel jobs → single writer | Never open second write connection from UI thread |
| **Watcher ↔ refresh** | `enqueue(Paths)` | Debounced; never block FS callback |

## Sources

- [Tauri v2 project structure](https://v2.tauri.app/start/project-structure/) — HIGH
- [Tauri system tray](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/learn/system-tray.mdx) — HIGH
- [Tauri process model](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/concept/process-model.md) — HIGH
- [Tauri workspace + lib.rs convention](https://github.com/tauri-apps/tauri/issues/4232) — MEDIUM
- [rusqlite WAL / transactions](https://context7.com/rusqlite/rusqlite/llms.txt) — HIGH
- [git2 repository status / ahead-behind](https://docs.rs/git2/latest/git2/struct.Repository.html) — HIGH
- [notify + debouncer](https://github.com/notify-rs/notify) — HIGH
- [repoindex DESIGN — local-first catalog, SQLite as cache](https://github.com/queelius/repoindex/blob/master/DESIGN.md) — MEDIUM (analogous domain, Python)
- [Cursor CLI usage](https://cursor.com/docs/cli/using) — MEDIUM (launch path; verify `cursor .` flags in implementation phase)
- [commitmux — SQLite + git2 indexer pattern](https://github.com/blackwell-systems/commitmux) — MEDIUM (commit history, not repo launcher — structural reference only)

---
*Architecture research for: Workpot — macOS multi-repo git workspace launcher*
*Researched: 2026-05-28*
