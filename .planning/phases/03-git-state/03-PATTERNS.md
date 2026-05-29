# Phase 3: Git state - Pattern Map

**Mapped:** 2026-05-29
**Files analyzed:** 8 new/modified files
**Analogs found:** 8 / 8

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/workpot-core/src/domain/git_state.rs` | model | transform | `crates/workpot-core/src/domain/repo.rs` | role-match |
| `crates/workpot-core/src/services/git_state.rs` | service | batch + request-response | `crates/workpot-core/src/services/discovery.rs` | role-match |
| `crates/workpot-core/src/infra/git.rs` | infra utility | request-response | self (rewrite of existing file) | exact |
| `crates/workpot-core/src/infra/migrations/003_git_state.sql` | migration | transform | `crates/workpot-core/src/infra/migrations/002_discovery.sql` | exact |
| `crates/workpot-core/src/domain/repo.rs` | model | CRUD | self (extension of existing file) | exact |
| `crates/workpot-core/src/services/index.rs` | service | batch | self (extension of existing file) | exact |
| `crates/workpot-core/src/services/catalog.rs` | service | CRUD | self (extension of existing file) | exact |
| `crates/workpot-cli/src/main.rs` | CLI entrypoint | request-response | self (extension of existing file) | exact |

---

## Pattern Assignments

### `crates/workpot-core/src/domain/git_state.rs` (model, transform)

**Analog:** `crates/workpot-core/src/domain/repo.rs` (lines 1–10)

**Pattern: Plain data struct with optional fields.**
`repo.rs` defines a plain Rust struct with no trait derives beyond `Debug, Clone, PartialEq, Eq`. `GitState` follows the same shape but uses `Option<T>` for nullable fields (mirroring the SQL NULL semantics in D-04, D-13).

**Struct pattern** (analog `src/domain/repo.rs` lines 1–10):
```rust
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRecord {
    pub path: PathBuf,
    pub name: String,
    pub registered_at: i64,
    pub source: String,
    pub git_common_dir: String,
}
```

**New file must produce:**
```rust
/// Per-repo git state snapshot. All fields are Option to encode:
///   branch=None  → detached HEAD short OID stored as String (or unborn branch)
///   is_dirty=None → bare repo (D-13)
///   ahead=None, behind=None → no upstream configured (D-04)
///   error=Some  → git2 open/query failed (D-09, D-16)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitState {
    pub branch: Option<String>,
    pub is_dirty: Option<bool>,
    pub ahead: Option<i64>,
    pub behind: Option<i64>,
    pub error: Option<String>,
}
```

**Module registration:** `src/domain/mod.rs` (lines 1–5) must gain `pub mod git_state;` and `pub use git_state::GitState;` alongside the existing `pub mod repo;`.

---

### `crates/workpot-core/src/services/git_state.rs` (service, batch + request-response)

**Analog:** `crates/workpot-core/src/services/discovery.rs` — closest in structure: a service module that takes paths, applies a library operation, and returns results.

**Imports pattern** (analog `src/services/discovery.rs` lines 1–8):
```rust
use crate::domain::Config;
use crate::error::{Result, WorkpotError};
use crate::infra::git::list_worktree_paths;
use crate::services::catalog::{is_bare_repo, is_git_worktree};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
```

**New file imports must follow the same crate-relative path style:**
```rust
use crate::domain::git_state::GitState;
use crate::error::Result;
use crate::infra::git; // refresh_git_state lives in infra/git.rs
use rayon::prelude::*;
use std::path::{Path, PathBuf};
```

**Error handling pattern** (analog `src/services/discovery.rs` lines 44–98):
Errors in the discovery loop are not propagated fatally — they are collected with `eprintln!` warnings and the loop continues. The git_state batch refresh follows the same continue-on-error pattern (D-16), except instead of a warning print, errors are embedded in `GitState { error: Some(...) }`.

**Core batch pattern** (from RESEARCH.md Pattern 2, adapted to project conventions):
```rust
pub struct GitRefreshResult {
    pub path: String,
    pub state: GitState,
}

/// Refresh git state for all provided paths in parallel (rayon).
/// Never aborts on individual failure — embeds error in GitState.error (D-16).
pub fn refresh_all(paths: Vec<PathBuf>) -> Vec<GitRefreshResult> {
    paths
        .into_par_iter()
        .map(|path| {
            let state = refresh_git_state(&path)
                .unwrap_or_else(|e| GitState {
                    branch: None,
                    is_dirty: None,
                    ahead: None,
                    behind: None,
                    error: Some(e.to_string()),
                });
            GitRefreshResult {
                path: path.display().to_string(),
                state,
            }
        })
        .collect()
}
```

**Single-repo public API** (D-18 — must be public for Phase 4 tray):
```rust
/// Public API for Phase 4 tray to refresh a single repo without running full index.
pub fn refresh_git_state(path: &Path) -> Result<GitState> {
    // calls git::open_and_query(path)
    // maps git2::Error → WorkpotError::GitUnavailable
}
```

**Note on error mapping:** `git2::Error` is not in `WorkpotError` directly — map via `WorkpotError::GitUnavailable(path.to_path_buf())` for the public surface, consistent with the existing pattern in `src/infra/git.rs` lines 12–13.

---

### `crates/workpot-core/src/infra/git.rs` (infra utility, rewrite)

**Analog:** self — this is a full rewrite of the existing file (D-02). The existing file (lines 1–84) provides the function signatures and error mapping patterns to preserve.

**Existing function signatures to preserve** (`src/infra/git.rs` lines 6–31 and 34–84):
```rust
pub fn resolve_git_common_dir(path: &Path) -> Result<PathBuf>
pub fn list_worktree_paths(repo_path: &Path) -> Result<Vec<PathBuf>>
```

**Existing error mapping pattern to preserve** (`src/infra/git.rs` lines 12–13):
```rust
.map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))?;
```
All `git2::Error` returns in this file map to `WorkpotError::GitUnavailable(path)`. Do not expose `git2::Error` in the public return type.

**Existing canonicalization pattern to preserve** (`src/infra/git.rs` lines 30–31):
```rust
std::fs::canonicalize(&resolved).map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))
```
All returned `PathBuf` values must be canonicalized — consistent with Phase 1 D-13.

**Existing warning print pattern to preserve** (`src/infra/git.rs` lines 73–76):
```rust
eprintln!(
    "warning: skip worktree {}: {e}",
    resolved.display()
);
```

**New imports block (replaces `use std::process::Command`):**
```rust
use crate::error::{Result, WorkpotError};
use git2::Repository;
use std::path::{Path, PathBuf};
```

**New function to add** (for `services/git_state.rs` to call internally):
The git2 branch/dirty/ahead-behind logic lives in this infra module. `services/git_state.rs` calls `infra::git::open_and_query(path)` or similar — keeping all git2 usage inside infra, consistent with the project's existing layering (`infra/` owns external library calls; `services/` owns orchestration).

---

### `crates/workpot-core/src/infra/migrations/003_git_state.sql` (migration, transform)

**Analog:** `crates/workpot-core/src/infra/migrations/002_discovery.sql` (lines 1–22) — exact pattern match.

**Pattern: ALTER TABLE for additive column additions** (`002_discovery.sql` line 1):
```sql
ALTER TABLE repos ADD COLUMN git_common_dir TEXT NOT NULL DEFAULT '';
```

**New file must produce** (D-05):
```sql
ALTER TABLE repos ADD COLUMN branch TEXT;
ALTER TABLE repos ADD COLUMN is_dirty INTEGER;
ALTER TABLE repos ADD COLUMN ahead INTEGER;
ALTER TABLE repos ADD COLUMN behind INTEGER;
ALTER TABLE repos ADD COLUMN git_refreshed_at INTEGER;
ALTER TABLE repos ADD COLUMN git_state_error TEXT;
```

All six columns are nullable (`NULL` default, no `NOT NULL` constraint) — existing rows silently default to NULL, which is the "never refreshed" sentinel (D-06).

**Migration registration pattern** (`src/infra/migrations.rs` lines 1–12):
```rust
use crate::error::Result;
use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

pub fn apply_migrations(conn: &mut Connection) -> Result<()> {
    static MIGRATION_001: &str = include_str!("migrations/001_init.sql");
    static MIGRATION_002: &str = include_str!("migrations/002_discovery.sql");
    let steps = [M::up(MIGRATION_001), M::up(MIGRATION_002)];
    // EXTEND: add M::up(MIGRATION_003) here
    let migrations = Migrations::from_slice(&steps);
    migrations.to_latest(conn)?;
    Ok(())
}
```

Add `static MIGRATION_003: &str = include_str!("migrations/003_git_state.sql");` and append `M::up(MIGRATION_003)` to `steps`. The array literal must be extended — it is not a Vec.

---

### `crates/workpot-core/src/domain/repo.rs` (model, CRUD extension)

**Analog:** self — extend the existing 10-line file.

**Existing struct** (`src/domain/repo.rs` lines 1–10):
```rust
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRecord {
    pub path: PathBuf,
    pub name: String,
    pub registered_at: i64,
    pub source: String,
    pub git_common_dir: String,
}
```

**Add six fields** after `git_common_dir` (D-05):
```rust
pub branch: Option<String>,
pub is_dirty: Option<bool>,       // None = bare repo (D-13); false = clean; true = dirty
pub ahead: Option<i64>,           // None = no upstream (D-04)
pub behind: Option<i64>,          // None = no upstream (D-04)
pub git_refreshed_at: Option<i64>,// None = never refreshed (D-06)
pub git_state_error: Option<String>, // last failure message (D-09)
```

All six new fields use `Option<T>` — matches the nullable SQL columns in the migration.

---

### `crates/workpot-core/src/services/index.rs` (service, batch extension)

**Analog:** self — extend the existing 360-line file.

**Existing IndexSummary struct** (`src/services/index.rs` lines 10–15):
```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndexSummary {
    pub added: u32,
    pub removed: u32,
    pub skipped: u32,
}
```

**Extend with two git stats fields** (D-17):
```rust
pub git_refreshed: u32,
pub git_errors: u32,
```

**Existing transaction pattern** (`src/services/index.rs` lines 107–150) — the batch git UPDATE must use the same `unchecked_transaction` + `tx.commit()` pattern:
```rust
let tx = conn.unchecked_transaction()?;
// ... batch operations ...
tx.commit()?;
```

**Existing now_secs helper** (`src/services/index.rs` lines 22–27) — reuse for `git_refreshed_at`:
```rust
fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
```

**Existing continue-on-error pattern** (`src/services/index.rs` lines 68–85) — git refresh follows same shape: attempt per-item, log warning/record error, continue loop:
```rust
match resolve_git_common_dir(&path) {
    Ok(common) => { upserts.push((path, common.display().to_string())); }
    Err(_) => {
        eprintln!("warning: skip {}: git unavailable", path_key);
        pre_skipped += 1;
        changelog.push(ChangeEntry { path: path_key, action: "skipped" });
    }
}
```

**New second pass call site** — after `tx.commit()?;` at line 150 (or after the transaction block), call:
```rust
let all_paths: Vec<PathBuf> = /* SELECT path FROM repos WHERE excluded = 0 */;
let git_results = services::git_state::refresh_all(all_paths);
// batch UPDATE in a new transaction (separate from discovery transaction)
let git_tx = conn.unchecked_transaction()?;
for r in &git_results {
    // UPDATE repos SET branch=?, is_dirty=?, ahead=?, behind=?,
    //   git_refreshed_at=?, git_state_error=? WHERE path=?
    if r.state.error.is_some() { summary.git_errors += 1; }
    else { summary.git_refreshed += 1; }
}
git_tx.commit()?;
```

**Existing finish_index_run** (`src/services/index.rs` lines 331–354) — if the index_runs schema is extended to store git counts, pass `summary.git_refreshed` and `summary.git_errors` here. If not (RESEARCH defers structured git stats to post-Phase 3), pass them as part of the message string only.

---

### `crates/workpot-core/src/services/catalog.rs` (service, CRUD extension)

**Analog:** self — extend the existing `list_repos` function.

**Existing list_repos SELECT** (`src/services/catalog.rs` lines 69–86):
```rust
pub fn list_repos(conn: &Connection) -> Result<Vec<RepoRecord>> {
    let mut stmt = conn.prepare(
        "SELECT path, name, registered_at, source, git_common_dir FROM repos WHERE excluded = 0 ORDER BY registered_at, path",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(RepoRecord {
            path: PathBuf::from(row.get::<_, String>(0)?),
            name: row.get(1)?,
            registered_at: row.get(2)?,
            source: row.get(3)?,
            git_common_dir: row.get(4)?,
        })
    })?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(WorkpotError::Database)
}
```

**Must extend SELECT and query_map** to include the six new columns at positions 5–10:
```
SELECT path, name, registered_at, source, git_common_dir,
       branch, is_dirty, ahead, behind, git_refreshed_at, git_state_error
FROM repos WHERE excluded = 0 ORDER BY registered_at, path
```

**query_map field extraction for Option<bool>:** `is_dirty` is stored as `INTEGER NULL` in SQLite. Extract as `Option<i64>` then map to `Option<bool>`:
```rust
is_dirty: row.get::<_, Option<i64>>(6)?.map(|v| v != 0),
```

All other new fields map directly (`Option<String>` or `Option<i64>`).

**Existing upsert_scan** (`src/services/catalog.rs` lines 157–202) — does NOT need to set git state columns. Git state is set only by the git refresh pass in `services/git_state.rs` + `services/index.rs`. The `INSERT ... ON CONFLICT DO UPDATE` must explicitly leave git columns untouched (do not include them in the `DO UPDATE SET` clause).

---

### `crates/workpot-cli/src/main.rs` (CLI entrypoint, request-response extension)

**Analog:** self — extend two existing command handlers.

**Existing Index handler** (`src/main.rs` lines 87–93):
```rust
Commands::Index => {
    let ctx = AppContext::open().context("failed to open workpot")?;
    let summary = ctx.run_index()?;
    println!(
        "index: +{} -{} skipped {}",
        summary.added, summary.removed, summary.skipped
    );
}
```

**Must extend output** (D-17) — two-line or combined format:
```rust
println!(
    "index: +{} -{} skipped {} / git: {} refreshed, {} errors",
    summary.added, summary.removed, summary.skipped,
    summary.git_refreshed, summary.git_errors
);
```

**Existing repo list handler** (`src/main.rs` lines 101–106):
```rust
RepoCommands::List => {
    let ctx = AppContext::open().context("failed to open workpot")?;
    let repos = ctx.list_repos().context("repo list failed")?;
    for repo in repos {
        println!("{}  {}", repo.name, repo.path.display());
    }
}
```

**Must extend per-repo output** (D-06, D-07, D-09) — format function to add:
```rust
fn format_git_state(repo: &RepoRecord) -> String {
    // D-06: never refreshed → "?"
    let Some(refreshed_at) = repo.git_refreshed_at else {
        return "?".to_string();
    };
    // D-09: error → show error
    if let Some(ref err) = repo.git_state_error {
        return format!("ERROR: {err}");
    }
    let branch = repo.branch.as_deref().unwrap_or("?");
    let dirty = match repo.is_dirty {
        None => "N/A",       // bare repo (D-13)
        Some(true) => "dirty",
        Some(false) => "clean",
    };
    let ahead_behind = match (repo.ahead, repo.behind) {
        (Some(a), Some(b)) => format!(" ↑{a}↓{b}"),
        _ => String::new(),  // D-04: omit when no upstream
    };
    let age = format_age(refreshed_at); // D-07
    format!("{branch}  {dirty}{ahead_behind}  {age}")
}
```

**Age formatting helper** (uses `humantime` from RESEARCH.md Pattern 6):
```rust
fn format_age(git_refreshed_at: i64) -> String {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    let refreshed = UNIX_EPOCH + Duration::from_secs(git_refreshed_at as u64);
    let elapsed = SystemTime::now()
        .duration_since(refreshed)
        .unwrap_or_default();
    humantime::format_duration(Duration::from_secs(elapsed.as_secs())).to_string()
}
```

Add `humantime` to `workpot-cli/Cargo.toml` `[dependencies]` (or to `workpot-core` if `format_age` is placed in core). The RESEARCH.md locates it in the CLI output layer — keep it in `workpot-cli`.

**Existing error mapping pattern** (`src/main.rs` lines 152–158):
```rust
fn map_roots_error(err: WorkpotError) -> anyhow::Error {
    match err {
        WorkpotError::LimitsExceeded(msg) | WorkpotError::WatchRootNotFound(msg) => {
            anyhow::anyhow!(msg)
        }
        other => other.into(),
    }
}
```
No changes needed here for Phase 3.

---

## Shared Patterns

### Error Handling
**Source:** `crates/workpot-core/src/error.rs` lines 1–49
**Apply to:** all new and modified files in `workpot-core`

The project uses `thiserror` with a single `WorkpotError` enum. All functions in `workpot-core` return `crate::error::Result<T>` (a type alias for `std::result::Result<T, WorkpotError>`). The CLI boundary uses `anyhow::Context` to attach context strings.

```rust
// In workpot-core: always return Result<T> (= std::result::Result<T, WorkpotError>)
pub fn my_fn(path: &Path) -> Result<MyType> {
    something().map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))?;
    ...
}

// In workpot-cli: wrap with anyhow context at call boundary
let result = ctx.my_fn(&path).context("my_fn failed")?;
```

The `WorkpotError::GitUnavailable(PathBuf)` variant is the correct variant for all git2 failures — it already exists (`src/error.rs` line 34).

### Path Canonicalization
**Source:** `crates/workpot-core/src/services/catalog.rs` lines 27–31; `infra/git.rs` lines 24–31
**Apply to:** `services/git_state.rs`, `infra/git.rs` (rewrite)

All paths must be canonicalized before use as DB keys or before passing to `Repository::open`. The existing pattern:
```rust
let canonical = path
    .canonicalize()
    .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", path.display())))?;
let path_key = canonical.display().to_string();
```

### now_secs Helper
**Source:** `crates/workpot-core/src/services/index.rs` lines 22–27
**Apply to:** `services/git_state.rs` (or call through a shared location)

```rust
fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
```

For `git_refreshed_at` timestamp, use this same helper. It lives in `services/index.rs` today; if `services/git_state.rs` needs it too, either re-define it locally (it is 5 lines, no import complexity) or move it to a shared `infra/time.rs` — planner's discretion.

### SQLite Transaction Pattern
**Source:** `crates/workpot-core/src/services/index.rs` lines 107–150
**Apply to:** git batch UPDATE in `services/index.rs` second pass

```rust
let tx = conn.unchecked_transaction()?;
// ... batch operations using &tx ...
tx.commit()?;
```

The rayon collect must complete before any DB transaction is opened (cannot borrow `conn` across thread boundary). Pattern: collect `Vec<GitRefreshResult>` outside any DB scope, then open one transaction and write all results.

### Continue-on-Error Batch Pattern
**Source:** `crates/workpot-core/src/services/index.rs` lines 68–85
**Apply to:** git refresh second pass

Per-item failures must not abort the batch (D-16). Discovery uses `eprintln!` + `pre_skipped += 1` + continue. Git refresh uses the same approach but stores the error string in `GitState.error` and counts it in `summary.git_errors`.

### Test Fixture Pattern
**Source:** `crates/workpot-core/tests/index_test.rs` lines 9–41
**Apply to:** `tests/git_state_test.rs` (new)

Existing tests use:
- `tempfile::tempdir()` for isolation
- `Command::new("git").args(["init", "-q"])` to create real git repos
- `store::open_connection(&db_path)` for DB setup
- `AppContext::open_with_paths(config_path, db_path)` for integration tests

New git_state tests should use `git2::Repository::init(tempdir.path().join("repo"))` instead of `Command::new("git")` — consistent with D-02 (no subprocess in core) and the RESEARCH.md recommendation.

```rust
// Pattern for git_state_test.rs test helper:
fn init_git_repo(parent: &Path, name: &str) -> PathBuf {
    let repo_path = parent.join(name);
    git2::Repository::init(&repo_path).expect("git2::Repository::init");
    repo_path
}
```

---

## New Dependencies to Add

### `crates/workpot-core/Cargo.toml`
```toml
git2 = { version = "0.21", features = ["vendored-libgit2"] }
rayon = "1"
```

### `crates/workpot-cli/Cargo.toml`
```toml
humantime = "2"
```

**Planner note:** RESEARCH.md requires a `checkpoint:human-verify` before each `cargo add` task (all packages tagged `[ASSUMED]` per slopcheck graceful degradation). Add a verification checkpoint task before Cargo.toml edits.

---

## No Analog Found

None — all 8 files have sufficient analogs in the codebase. RESEARCH.md provides concrete code examples for the git2-specific patterns that have no codebase precedent.

---

## Metadata

**Analog search scope:** `crates/workpot-core/src/`, `crates/workpot-cli/src/`, `crates/workpot-core/tests/`
**Files scanned:** 21 Rust source files + 2 SQL migrations + 2 Cargo.toml files
**Pattern extraction date:** 2026-05-29
