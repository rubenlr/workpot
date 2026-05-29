# Phase 2: Repo discovery - Pattern Map

**Mapped:** 2026-05-29
**Files analyzed:** 18 new/modified targets
**Analogs found:** 14 / 18

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/workpot-core/src/services/discovery.rs` | service | file-I/O, batch | `services/catalog.rs` (git detect only) | partial |
| `crates/workpot-core/src/services/index.rs` | service | batch, CRUD merge | `services/catalog.rs` + `lib.rs` | role-match |
| `crates/workpot-core/src/services/roots.rs` | service | CRUD (config) | `lib.rs` (config bootstrap/write) | role-match |
| `crates/workpot-core/src/services/excludes.rs` | service | CRUD (config) | `lib.rs` + `domain/config.rs` | role-match |
| `crates/workpot-core/src/services/catalog.rs` | service | CRUD | `services/catalog.rs` (self) | exact |
| `crates/workpot-core/src/domain/config.rs` | model | transform | `domain/config.rs` | exact |
| `crates/workpot-core/src/domain/repo.rs` | model | transform | `domain/repo.rs` | exact |
| `crates/workpot-core/src/infra/migrations/002_discovery.sql` | migration | transform | `infra/migrations/001_init.sql` | exact |
| `crates/workpot-core/src/infra/migrations.rs` | config | transform | `infra/migrations.rs` | exact |
| `crates/workpot-core/src/lib.rs` | provider | request-response | `lib.rs` (`AppContext`) | exact |
| `crates/workpot-core/src/services/mod.rs` | config | — | `services/mod.rs` | exact |
| `crates/workpot-core/src/error.rs` | utility | — | `error.rs` | exact |
| `crates/workpot-core/Cargo.toml` | config | — | `workpot-core/Cargo.toml` | exact |
| `crates/workpot-cli/src/main.rs` | route | request-response | `workpot-cli/src/main.rs` | exact |
| `crates/workpot-core/tests/discovery_test.rs` | test | batch | `tests/catalog_test.rs` | exact |
| `crates/workpot-core/tests/index_test.rs` | test | batch | `tests/catalog_test.rs` | exact |
| `crates/workpot-core/tests/roots_test.rs` | test | CRUD | `tests/bootstrap_test.rs` | role-match |
| `crates/workpot-core/tests/excludes_test.rs` | test | CRUD | `tests/bootstrap_test.rs` | role-match |
| `crates/workpot-core/src/infra/git.rs` (optional) | utility | request-response | — | none |
| `crates/workpot-cli/tests/cli_smoke.rs` (extend) | test | request-response | `cli_smoke.rs` | exact |

## Pattern Assignments

### `crates/workpot-core/src/services/discovery.rs` (service, file-I/O / batch)

**Analog (git detection):** `crates/workpot-core/src/services/catalog.rs`  
**Analog (walk — no codebase match):** use RESEARCH.md Pattern 1 (`walkdir` + `skip_current_dir`)

**Imports pattern** — mirror catalog + new deps:

```1:6:crates/workpot-core/src/services/catalog.rs
use crate::domain::RepoRecord;
use crate::error::{Result, WorkpotError};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
```

Phase 2 adds: `walkdir::WalkDir`, `globset::{Glob, GlobSet, GlobSetBuilder}`, `std::process::Command` only if git helpers live here (prefer `infra/git.rs`).

**Core git-detect pattern** (reuse, do not duplicate logic):

```93:108:crates/workpot-core/src/services/catalog.rs
fn is_git_worktree(path: &Path) -> bool {
    let marker = path.join(".git");
    if marker.is_dir() {
        return marker.join("HEAD").is_file();
    }
    if marker.is_file() {
        return std::fs::read_to_string(&marker)
            .map(|s| s.starts_with("gitdir:"))
            .unwrap_or(false);
    }
    false
}

fn is_bare_repo(path: &Path) -> bool {
    path.join("HEAD").is_file() && path.join("objects").is_dir()
}
```

**Planner note:** Export `is_git_worktree` / `is_bare_repo` as `pub(crate)` from `catalog` or move to `domain/git_detect.rs` so discovery and catalog share one implementation (INDEX-04).

**Walk + prune pattern** — no analog; RESEARCH prescribes:

```rust
// From 02-RESEARCH.md Pattern 1 — implement in discovery.rs
let mut walk = WalkDir::new(root).follow_links(false).into_iter();
// exclude_set.is_match → skip_current_dir on dirs
// is_repo_root → push candidate, skip_current_dir (D-01)
```

---

### `crates/workpot-core/src/services/index.rs` (service, batch / CRUD merge)

**Analog:** `crates/workpot-core/src/services/catalog.rs` (per-row SQL) + `crates/workpot-core/src/lib.rs` (orchestration facade target)

**Catalog upsert/insert pattern** (extend for `source='scan'` + `git_common_dir`):

```41:59:crates/workpot-core/src/services/catalog.rs
    let rows = conn.execute(
        "INSERT INTO repos (path, name, registered_at, source) VALUES (?1, ?2, ?3, 'manual')",
        params![path_key, name, registered_at],
    );

    match rows {
        Ok(_) => Ok(RepoRecord {
            path: canonical,
            name,
            registered_at,
            source: "manual".to_string(),
        }),
        Err(rusqlite::Error::SqliteFailure(err, _))
            if err.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            Err(WorkpotError::AlreadyRegistered(path_key))
        }
        Err(e) => Err(WorkpotError::Database(e)),
    }
```

**Stale removal pattern** (reuse for D-15):

```80:90:crates/workpot-core/src/services/catalog.rs
pub fn remove_repo(conn: &Connection, path: &Path) -> Result<()> {
    let canonical = path
        .canonicalize()
        .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", path.display())))?;
    let path_key = canonical.display().to_string();

    let deleted = conn.execute("DELETE FROM repos WHERE path = ?1", params![path_key])?;
    if deleted == 0 {
        return Err(WorkpotError::NotFound(path_key));
    }
    Ok(())
}
```

**Path canonicalization before DB keys** (D-13 Phase 1, still required):

```25:29:crates/workpot-core/src/services/catalog.rs
    let canonical = path
        .canonicalize()
        .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", path.display())))?;

    let path_key = canonical.display().to_string();
```

**Transaction / cap (no analog):** First use of `conn.transaction()` in codebase. Follow RESEARCH Pattern 4: discover → count → `BEGIN` → upsert/delete/history → `COMMIT` or rollback on cap (D-18). Parameterized SQL only (Phase 1 ASVS pattern).

**AppContext delegation target** — add methods beside existing catalog wrappers:

```76:86:crates/workpot-core/src/lib.rs
    pub fn register_manual(&self, path: &Path) -> Result<RepoRecord> {
        catalog::register_manual(&self.conn, path)
    }

    pub fn list_repos(&self) -> Result<Vec<RepoRecord>> {
        catalog::list_repos(&self.conn)
    }

    pub fn remove_repo(&self, path: &Path) -> Result<()> {
        catalog::remove_repo(&self.conn, path)
    }
```

Add `run_index(&mut self) -> Result<IndexSummary>` that reloads config if needed, calls `index::run_full(&mut self.conn, &self.config, ...)`, and may mutate in-memory `config` after exclude/roots writes.

---

### `crates/workpot-core/src/services/roots.rs` (service, CRUD config)

**Analog:** `crates/workpot-core/src/lib.rs` (config load + first-write) + `domain/config.rs`

**Config shape** (extend, do not break defaults):

```4:12:crates/workpot-core/src/domain/config.rs
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    /// Watch roots for auto-discovery (consumed in Phase 2).
    #[serde(default)]
    pub watch_roots: Vec<PathBuf>,
    /// Path patterns excluded from indexing (consumed in Phase 2).
    #[serde(default)]
    pub excludes: Vec<String>,
}
```

**Config persist pattern** (only write site in repo today — extract `save_config` helper in `lib.rs` and call from roots/excludes/catalog remove):

```89:103:crates/workpot-core/src/lib.rs
fn ensure_default_config(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() {
        return Ok(());
    }
    let home = directories::BaseDirs::new()
        .map(|b| b.home_dir().to_path_buf())
        .ok_or(WorkpotError::PathsUnavailable)?;
    let default = default_config(&home);
    let contents = toml::to_string_pretty(&default)
        .map_err(|e| crate::error::WorkpotError::Config(e.to_string()))?;
    fs::write(path, contents)?;
    Ok(())
}
```

**Watch-root seeding pattern** (for validation, not auto-add in roots service):

```23:32:crates/workpot-core/src/lib.rs
pub fn default_config(home: &Path) -> Config {
    let mut config = Config::default();
    for name in ["code", "dev"] {
        let candidate = home.join(name);
        if candidate.is_dir() {
            config.watch_roots.push(candidate);
        }
    }
    config
}
```

**roots add → immediate scan (D-20):** After append + `save_config`, call `discovery::scan_root` or `index::run_partial(root)` from `AppContext` — same entry style as `register_manual` delegation.

---

### `crates/workpot-core/src/services/excludes.rs` (service, CRUD config)

**Analog:** `roots.rs` (same config R/W) + `domain/config.rs` `excludes` field

**Glob build** — no analog; RESEARCH Pattern 2 (`GlobSetBuilder` + built-in defaults chained with `config.excludes`).

**repo remove → exclude glob (D-10)** — extend `catalog::remove_repo` or new `remove_repo_with_exclude` in catalog; persist via shared `save_config` from `lib.rs`.

---

### `crates/workpot-core/src/services/catalog.rs` (service, CRUD — extend)

**Analog:** self (Phase 1 implementation)

**New public functions (planner sketch):** `upsert_scan`, `remove_repo_and_exclude` (or split), `validate_manual_rows`, `list_all_paths_for_prune`. Keep `register_manual` ignoring scan excludes (D-11).

**List query pattern** (preserve `excluded = 0` filter):

```62:78:crates/workpot-core/src/services/catalog.rs
pub fn list_repos(conn: &Connection) -> Result<Vec<RepoRecord>> {
    let mut stmt = conn.prepare(
        "SELECT path, name, registered_at, source FROM repos WHERE excluded = 0 ORDER BY registered_at, path",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(RepoRecord {
            path: PathBuf::from(row.get::<_, String>(0)?),
            name: row.get(1)?,
            registered_at: row.get(2)?,
            source: row.get(3)?,
        })
    })?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(WorkpotError::Database)
}
```

Extend `SELECT` and `RepoRecord` with `git_common_dir`; on rescan upsert, `UPDATE` rows where `source='manual'` must not flip to `scan` (D-14).

---

### `crates/workpot-core/src/domain/config.rs` (model, transform)

**Analog:** `domain/config.rs`

Add nested `Limits` with serde defaults matching D-22/D-23; validate hard max on load in `load_config` or dedicated `Config::validate()`.

```4:12:crates/workpot-core/src/domain/config.rs
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    #[serde(default)]
    pub watch_roots: Vec<PathBuf>,
    #[serde(default)]
    pub excludes: Vec<String>,
}
```

---

### `crates/workpot-core/src/domain/repo.rs` (model, transform)

**Analog:** `domain/repo.rs`

```3:9:crates/workpot-core/src/domain/repo.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRecord {
    pub path: PathBuf,
    pub name: String,
    pub registered_at: i64,
    pub source: String,
}
```

Add `pub git_common_dir: String` (default empty until backfill / index).

---

### `crates/workpot-core/src/infra/migrations/002_discovery.sql` (migration)

**Analog:** `infra/migrations/001_init.sql`

```1:9:crates/workpot-core/src/infra/migrations/001_init.sql
CREATE TABLE repos (
  path TEXT NOT NULL PRIMARY KEY,
  name TEXT NOT NULL,
  registered_at INTEGER NOT NULL,
  source TEXT NOT NULL DEFAULT 'manual' CHECK (source IN ('manual', 'scan')),
  excluded INTEGER NOT NULL DEFAULT 0 CHECK (excluded IN (0, 1))
);

CREATE INDEX idx_repos_registered_at ON repos(registered_at);
```

Follow same style: `CHECK` constraints on enums, explicit indexes. RESEARCH sketch for `index_runs` / `index_changes` / `ALTER TABLE repos ADD COLUMN git_common_dir`.

---

### `crates/workpot-core/src/infra/migrations.rs` (config)

**Analog:** `infra/migrations.rs`

```5:10:crates/workpot-core/src/infra/migrations.rs
pub fn apply_migrations(conn: &mut Connection) -> Result<()> {
    static MIGRATION_UP: &str = include_str!("migrations/001_init.sql");
    let steps = [M::up(MIGRATION_UP)];
    let migrations = Migrations::from_slice(&steps);
    migrations.to_latest(conn)?;
    Ok(())
}
```

Add second step: `static MIGRATION_002: &str = include_str!("migrations/002_discovery.sql");` and `M::up(MIGRATION_002)` in `steps` array. Update `bootstrap_test::migrations_apply` expected `user_version` to `2`.

---

### `crates/workpot-core/src/lib.rs` (provider)

**Analog:** `lib.rs`

**Module exports:**

```3:6:crates/workpot-core/src/lib.rs
pub mod domain;
pub mod error;
pub mod infra;
pub mod services;
```

**Open path** (unchanged for production):

```44:61:crates/workpot-core/src/lib.rs
    pub fn open() -> Result<Self> {
        let config_path = paths::config_file()?;
        let db_path = paths::database_file()?;
        Self::open_with_paths(config_path, db_path)
    }

    pub fn open_with_paths(config_path: PathBuf, db_path: PathBuf) -> Result<Self> {
        ensure_default_config(&config_path)?;
        let config = load_config(&config_path)?;
        let conn = store::open_connection(&db_path)?;
        Ok(Self {
            config_path,
            db_path,
            config,
            conn,
        })
    }
```

Add `save_config(&self) -> Result<()>` using `config_path` + `toml::to_string_pretty(&self.config)`.

---

### `crates/workpot-core/src/services/mod.rs` (config)

**Analog:** `services/mod.rs`

```1:1:crates/workpot-core/src/services/mod.rs
pub mod catalog;
```

Add: `pub mod discovery; pub mod index; pub mod roots; pub mod excludes;`

---

### `crates/workpot-core/src/error.rs` (utility)

**Analog:** `error.rs`

```4:32:crates/workpot-core/src/error.rs
#[derive(Debug, Error)]
pub enum WorkpotError {
    #[error("config error: {0}")]
    Config(String),
    // ...
    #[error("invalid path: {0}")]
    InvalidPath(String),
}
```

Add variants as needed: `IndexCapExceeded { max: u32, found: u32 }`, `GitUnavailable`, `WatchRootNotFound(PathBuf)`, `LimitsExceeded(String)`. Keep `thiserror` + `Result<T>` alias.

---

### `crates/workpot-core/Cargo.toml` (config)

**Analog:** `workpot-core/Cargo.toml`

```8:14:crates/workpot-core/Cargo.toml
[dependencies]
directories = "6.0.0"
rusqlite = { version = "0.39.0", features = ["bundled"] }
rusqlite_migration = "2.5.0"
serde = { version = "1", features = ["derive"] }
thiserror = "2"
toml = "0.8"
```

Add per RESEARCH: `walkdir = "2.5.0"`, `globset = "0.4.18"`.

---

### `crates/workpot-cli/src/main.rs` (route)

**Analog:** `main.rs`

**Top-level CLI structure:**

```6:29:crates/workpot-cli/src/main.rs
#[derive(Parser)]
#[command(name = "workpot", about = "Local git repo workspace launcher", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Paths,
    #[command(subcommand)]
    Repo(RepoCommands),
}
```

**Handler pattern** (repeat for `Index`, `Roots`, `Excludes`):

```39:56:crates/workpot-cli/src/main.rs
        Commands::Repo(sub) => match sub {
            RepoCommands::Add { path } => {
                let ctx = AppContext::open().context("failed to open workpot")?;
                let record = ctx.register_manual(&path).context("repo add failed")?;
                println!("registered: {}", record.path.display());
            }
            // ...
        },
```

`workpot index` → one summary line (D-17); propagate `WorkpotError::IndexCapExceeded` to `anyhow` and exit code 1 (D-18). Use `#[command(subcommand)]` nesting like `Repo`.

---

### `crates/workpot-core/tests/discovery_test.rs` | `index_test.rs` (test, batch)

**Analog:** `tests/catalog_test.rs`

**Fixture helpers** (copy/adapt):

```6:14:crates/workpot-core/tests/catalog_test.rs
fn git_fixture() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().join("sample-repo");
    fs::create_dir_all(&repo).expect("repo dir");
    let git_dir = repo.join(".git");
    fs::create_dir_all(git_dir.join("objects")).expect("objects");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("HEAD");
    (dir, repo)
}
```

**Isolation pattern:**

```40:59:crates/workpot-core/tests/catalog_test.rs
#[test]
fn repo_persists_across_reopen() {
    let (dir, repo_path) = git_fixture();
    let config_path = dir.path().join("config.toml");
    let db_path = dir.path().join("workpot.db");

    {
        let ctx = AppContext::open_with_paths(config_path.clone(), db_path.clone())
            .expect("first open");
        // ...
    }

    {
        let ctx = AppContext::open_with_paths(config_path, db_path).expect("second open");
        // ...
    }
}
```

Add watch-root tree fixtures under `tempdir`: nested repo + `vendor/nested/.git` for D-01. Prefer real `git init` in fixtures if fake `.git` dirs insufficient for `git rev-parse` tests.

---

### `crates/workpot-core/tests/roots_test.rs` | `excludes_test.rs` (test, CRUD)

**Analog:** `tests/bootstrap_test.rs`

```29:39:crates/workpot-core/tests/bootstrap_test.rs
fn open_does_not_overwrite_existing_config() {
  // ...
    let marker = "watch_roots = []\nexcludes = [\"/custom/exclude\"]\n";
    fs::write(&config_path, marker).expect("seed config");

    let _ctx = AppContext::open_with_paths(config_path.clone(), db_path).expect("open");
    let contents = fs::read_to_string(&config_path).expect("read config");
    assert_eq!(contents, marker);
}
```

After `roots add` / `excludes remove`, assert TOML round-trip via `fs::read_to_string` + `toml::from_str::<Config>`.

---

### `crates/workpot-cli/tests/cli_smoke.rs` (test, extend)

**Analog:** `cli_smoke.rs`

```15:19:crates/workpot-cli/tests/cli_smoke.rs
fn workpot_cmd(home: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("workpot").expect("workpot binary");
    cmd.env("HOME", home);
    cmd
}
```

Add smoke: `workpot index`, `workpot roots list`, cap failure exit code 1.

---

## Shared Patterns

### Path canonicalization + string DB keys

**Source:** `services/catalog.rs`  
**Apply to:** discovery candidates, all catalog upserts/removes, index stale detection

```25:29:crates/workpot-core/src/services/catalog.rs
    let canonical = path
        .canonicalize()
        .map_err(|e| WorkpotError::InvalidPath(format!("{}: {e}", path.display())))?;

    let path_key = canonical.display().to_string();
```

### Typed errors → CLI context

**Source:** `workpot-cli/src/main.rs` + `error.rs`  
**Apply to:** All new CLI subcommands

```41:42:crates/workpot-cli/src/main.rs
                let ctx = AppContext::open().context("failed to open workpot")?;
                let record = ctx.register_manual(&path).context("repo add failed")?;
```

### SQLite access via `AppContext` only (hosts)

**Source:** Phase 1 D-10 (`01-CONTEXT.md`)  
**Apply to:** CLI, future tray — no raw `Connection::open` in `workpot-cli`

### Parameterized SQL

**Source:** `catalog.rs` (`params!`)  
**Apply to:** `index_runs`, `index_changes`, bulk DELETE/INSERT in index transaction

### Config load (no save until Phase 2)

**Source:** `lib.rs`

```106:111:crates/workpot-core/src/lib.rs
fn load_config(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }
    let contents = fs::read_to_string(path)?;
    toml::from_str(&contents).map_err(|e| WorkpotError::Config(e.to_string()))
}
```

Extract symmetric `save_config(path, &Config)`.

### DB open + migrations

**Source:** `infra/store.rs`

```6:14:crates/workpot-core/src/infra/store.rs
pub fn open_connection(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
    migrations::apply_migrations(&mut conn)?;
    Ok(conn)
}
```

### Integration test layout

**Source:** `catalog_test.rs` + `bootstrap_test.rs`  
**Apply to:** All new `workpot-core/tests/*_test.rs` — `tempfile::tempdir`, isolated `config.toml` + `workpot.db`, `AppContext::open_with_paths`.

---

## No Analog Found

| File / concern | Role | Data Flow | Reason |
|----------------|------|-----------|--------|
| `walkdir` traversal + `skip_current_dir` | service | file-I/O | No filesystem walk in repo; follow RESEARCH Pattern 1 |
| `globset` exclude matching | utility | transform | No glob code; RESEARCH Pattern 2 |
| `git rev-parse` / `git worktree list` subprocess | utility | request-response | Phase 1 explicitly no git2/subprocess; RESEARCH Pattern 3 |
| `conn.transaction()` index merge | service | batch | No transactions in codebase yet |
| `infra/git.rs` (if split) | utility | request-response | Greenfield; isolate `Command::new("git")` with fixed args (no shell) |

---

## Metadata

**Analog search scope:** `crates/workpot-core/src/**`, `crates/workpot-core/tests/**`, `crates/workpot-cli/**`, `.planning/phases/01-core-persistence/`  
**Files scanned:** 22  
**Pattern extraction date:** 2026-05-29

## PATTERN MAPPING COMPLETE

**Phase:** 02 - Repo discovery  
**Files classified:** 18  
**Analogs found:** 14 / 18

### Coverage
- Files with exact analog: 10
- Files with role-match / partial analog: 4
- Files with no analog: 4 (walk, globset, git subprocess, SQL transactions)

### Key Patterns Identified
- Git repo detection is centralized in `catalog.rs` (`is_git_worktree`, `is_bare_repo`); discovery must reuse, not reimplement.
- All persistence flows through `AppContext` + parameterized `catalog` SQL with canonical path strings as keys.
- Config R/W follows `ensure_default_config` / `load_config` in `lib.rs`; Phase 2 needs extracted `save_config` for roots/excludes/remove.
- CLI adds nested `clap` subcommands mirroring `Repo`, with `anyhow::Context` on `WorkpotError`.
- Tests use `tempfile` + `open_with_paths` and hand-built `.git` fixtures from `catalog_test.rs`.

### File Created
`.planning/phases/02-repo-discovery/02-PATTERNS.md`

### Ready for Planning
Pattern mapping complete. Planner can reference analog patterns in PLAN.md task actions.
