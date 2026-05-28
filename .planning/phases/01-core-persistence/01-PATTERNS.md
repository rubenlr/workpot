# Phase 1: Core & persistence - Pattern Map

**Mapped:** 2026-05-28
**Files analyzed:** 18 (new/modified in Phase 1 scope)
**Analogs found:** 0 / 18 (greenfield — no application source in repo)

## Greenfield Status

The Workpot repository contains **planning artifacts only** (`.planning/`, `CLAUDE.md`, git metadata). A workspace search found **zero** `.rs` / `Cargo.toml` application files. There are no existing controllers, services, migrations, or tests to copy.

**Pattern contract for all Phase 1 files:** `.planning/research/ARCHITECTURE.md` (target layout, layer boundaries, `AppContext` shape) plus `.planning/phases/01-core-persistence/01-RESEARCH.md` (Phase 1–specific pins, paths, slim schema, CLI verbs). Locked paths from `01-CONTEXT.md` (D-01–D-04) override generic ARCHITECTURE examples where they differ.

**Planner instruction:** Treat each “Analog” below as **ARCHITECTURE.md + RESEARCH.md**, not a repo path. Copy structure and responsibilities from those docs; use RESEARCH code blocks for crate APIs (rusqlite_migration, `BaseDirs`, clap).

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `Cargo.toml` (workspace root) | config | batch | ARCHITECTURE.md § Recommended Project Structure | contract |
| `rust-toolchain.toml` | config | — | RESEARCH.md § Standard Stack | contract |
| `.cargo/config.toml` (optional) | config | — | RESEARCH.md § Wave 0 Gaps | contract |
| `crates/workpot-core/Cargo.toml` | config | — | RESEARCH.md § Installation | contract |
| `crates/workpot-cli/Cargo.toml` | config | — | RESEARCH.md § Installation | contract |
| `crates/workpot-core/src/lib.rs` | provider | request-response | ARCHITECTURE.md Pattern 1 (`AppContext`) | contract |
| `crates/workpot-core/src/domain/` (Config, RepoRecord) | model | transform | ARCHITECTURE.md § Domain | contract |
| `crates/workpot-core/src/services/catalog.rs` | service | CRUD | ARCHITECTURE.md § Catalog service | contract |
| `crates/workpot-core/src/infra/paths.rs` | utility | file-I/O | RESEARCH.md Pattern 2 + CONTEXT D-01/D-02 | contract |
| `crates/workpot-core/src/infra/store.rs` | service | CRUD | ARCHITECTURE.md § Store + Pattern 2 | contract |
| `crates/workpot-core/src/infra/migrations.rs` | migration | batch | RESEARCH.md Pattern 1 | contract |
| `crates/workpot-core/src/infra/migrations/001_init.sql` | migration | CRUD | RESEARCH.md § Minimal repos table | contract |
| `crates/workpot-core/src/error.rs` (or `lib.rs` re-exports) | utility | — | RESEARCH.md § Standard Stack (`thiserror`) | contract |
| `crates/workpot-cli/src/main.rs` | route | request-response | ARCHITECTURE.md § workpot-cli + RESEARCH clap skeleton | contract |
| `crates/workpot-core` integration tests | test | CRUD | RESEARCH.md § Validation Architecture | contract |
| `scripts/check-no-network-deps.sh` | utility | batch | RESEARCH.md § Verifying DATA-02 | contract |
| `.github/workflows/ci.yml` | config | batch | RESEARCH.md § Wave 0 Gaps | contract |

**Phase 1 explicitly excluded** (present in full ARCHITECTURE tree but not created now): `src-tauri/`, `ui/`, `services/refresh.rs`, `infra/git2`, `infra/watcher` — per RESEARCH.md and ARCHITECTURE.md build order Phase 1 slice.

---

## Pattern Assignments

### `Cargo.toml` (workspace root) — config, batch

**Analog:** `.planning/research/ARCHITECTURE.md` (workspace layout) + `01-RESEARCH.md` (workspace snippet)

**Workspace members pattern** (ARCHITECTURE.md lines 74–77, RESEARCH.md lines 102–110):

```toml
[workspace]
members = ["crates/workpot-core", "crates/workpot-cli"]
resolver = "2"

[workspace.package]
edition = "2024"
rust-version = "1.85"
```

**Do not include** `src-tauri` or `ui` in `members` for Phase 1 (RESEARCH.md lines 187–188).

---

### `crates/workpot-core/src/lib.rs` — provider, request-response

**Analog:** `.planning/research/ARCHITECTURE.md` Pattern 1

**Core `AppContext` shape** (ARCHITECTURE.md lines 118–135) — Phase 1 subset only (`config`, `store`, catalog; defer `refresh`, `search`, `launcher`, `recipes`):

```rust
// crates/workpot-core/src/lib.rs
pub struct AppContext {
    config: Config,
    store: SqliteStore,
    // Phase 1: catalog only; add refresh/search/launcher in later phases
}

impl AppContext {
    pub fn open() -> Result<Self> { /* lazy bootstrap D-04 */ }
    // pub fn open_with_paths(...) — tests only, RESEARCH open question
}
```

**Layer rule** (ARCHITECTURE.md lines 103–106): `domain/` pure types; `services/` orchestration; `infra/` SQLite, paths, migrations. **No Tauri/UI deps** in core.

**Lazy bootstrap** (RESEARCH.md Pattern 1, CONTEXT D-04): `AppContext::open()` ensures dirs, default config, DB + migrations — no separate `workpot init`.

---

### `crates/workpot-core/src/domain/` — model, transform

**Analog:** `.planning/research/ARCHITECTURE.md` § Domain + RESEARCH.md § Config schema

**Config type** (RESEARCH.md lines 276–284):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub watch_roots: Vec<PathBuf>,
    #[serde(default)]
    pub excludes: Vec<String>,
}
```

**Repo identity** (ARCHITECTURE.md Pattern 2, lines 145–159; Phase 1 slimmed in RESEARCH.md lines 292–303): path-as-primary-key; git columns deferred to Phase 3 migration `002_git_snapshot`.

---

### `crates/workpot-core/src/services/catalog.rs` — service, CRUD

**Analog:** `.planning/research/ARCHITECTURE.md` § Catalog service (lines 59–60)

**Responsibility contract:** Register/list/remove repos; enforce path-as-id; `source = 'manual'` in Phase 1; merge with scan results in Phase 2.

**Insert pattern** (RESEARCH.md lines 439–445):

```rust
conn.execute(
    "INSERT INTO repos (path, name, registered_at, source) VALUES (?1, ?2, ?3, 'manual')",
    params![path_display, name, registered_at],
)?;
```

**Validation** (RESEARCH.md Pattern 4): directory exists; `.git` is dir or file; `canonicalize` best-effort; no `git2`, no subprocess.

---

### `crates/workpot-core/src/infra/paths.rs` — utility, file-I/O

**Analog:** `01-CONTEXT.md` D-01/D-02 + RESEARCH.md Pattern 2

**Path resolution** (RESEARCH.md lines 394–406) — **`BaseDirs` only**, not `ProjectDirs` for config:

```rust
use directories::BaseDirs;
use std::path::PathBuf;

pub fn config_file() -> Option<PathBuf> {
    BaseDirs::new().map(|b| b.config_dir().join("workpot").join("config.toml"))
}

pub fn database_file() -> Option<PathBuf> {
    BaseDirs::new().map(|b| b.data_dir().join("workpot").join("workpot.db"))
}
```

**Locked paths:** config → `~/.config/workpot/config.toml`; DB → `~/Library/Application Support/workpot/workpot.db` on macOS. **No env overrides** in production (CONTEXT D-03).

---

### `crates/workpot-core/src/infra/store.rs` + `migrations.rs` — service / migration, CRUD / batch

**Analog:** `.planning/research/ARCHITECTURE.md` § Store (lines 63–64) + RESEARCH.md Pattern 1

**Open + WAL + migrate** (RESEARCH.md lines 197–214):

```rust
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

pub const MIGRATIONS: Migrations<'static> = Migrations::from_slice(&[
    M::up(include_str!("migrations/001_init.sql")),
]);

pub fn open_connection(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(path)?;
    conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
    MIGRATIONS.to_latest(&mut conn)?;
    Ok(conn)
}
```

**Materialized view principle** (ARCHITECTURE.md Pattern 2, lines 137–143): disk/git is truth later; Phase 1 DB holds registration metadata only.

**Anti-pattern:** Do not hand-roll `user_version` or parallel migration tables (RESEARCH.md Pitfall 3).

---

### `crates/workpot-core/src/infra/migrations/001_init.sql` — migration, CRUD

**Analog:** RESEARCH.md § Minimal repos table (not full ARCHITECTURE example with git columns)

```sql
CREATE TABLE repos (
  path TEXT NOT NULL PRIMARY KEY,
  name TEXT NOT NULL,
  registered_at INTEGER NOT NULL,
  source TEXT NOT NULL DEFAULT 'manual' CHECK (source IN ('manual', 'scan')),
  excluded INTEGER NOT NULL DEFAULT 0 CHECK (excluded IN (0, 1))
);

CREATE INDEX idx_repos_registered_at ON repos(registered_at);
```

---

### `crates/workpot-cli/src/main.rs` — route, request-response

**Analog:** `.planning/research/ARCHITECTURE.md` § workpot-cli (lines 68–69) + RESEARCH.md § Phase 1 CLI

**Thin host rule** (ARCHITECTURE.md Anti-Pattern 4, lines 325–331): clap parsing + `AppContext::open()` + delegate; **no business logic in `main`**.

**Subcommand skeleton** (RESEARCH.md lines 409–435):

```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "workpot", about = "Local git repo workspace launcher")]
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

#[derive(Subcommand)]
enum RepoCommands {
    Add { path: PathBuf },
    List,
    Remove { path: PathBuf },
}
```

**Phase 1 commands:** `paths`, `repo add`, `repo list`, `repo remove` (optional). **Defer:** `index`, `search`, `open`, `refresh`.

**Errors:** `thiserror` in core; `anyhow` at CLI edge only (RESEARCH.md § Standard Stack).

---

### `crates/workpot-core` tests — test, CRUD

**Analog:** RESEARCH.md § Validation Architecture

**Hermetic integration pattern:** `tempfile::TempDir`; inject paths via `AppContext::open_with_paths` (test-only API, not CLI flags).

**Required behaviors:** default config on first open; migrations on fresh DB; `repo add` survives new `AppContext`; no HTTP crates in `cargo tree`.

---

### `scripts/check-no-network-deps.sh` + `.github/workflows/ci.yml` — utility / config

**Analog:** RESEARCH.md § Verifying DATA-02

Structural DATA-02: ban `reqwest`, `ureq`, `hyper`, etc. in `workpot-core` / `workpot-cli` dependency trees; CI `cargo test --offline` after `cargo fetch`.

---

## Shared Patterns

### Layering: domain / services / infra

**Source:** `.planning/research/ARCHITECTURE.md` lines 101–106, 36–44

**Apply to:** All `workpot-core` modules

```
domain/     → pure types (Config, RepoRecord), no I/O
services/   → catalog orchestration
infra/      → paths, SQLite, migrations
```

### Shared core, multiple hosts (Phase 4+)

**Source:** `.planning/research/ARCHITECTURE.md` Pattern 1 (lines 110–116)

**Apply to:** `lib.rs` public API design now so Tauri and CLI both construct `AppContext` later without API churn.

### Path-as-primary-key

**Source:** `.planning/research/ARCHITECTURE.md` Pattern 2 (lines 137–159) + Anti-Pattern 3 (lines 317–323)

**Apply to:** `catalog`, SQL schema, CLI `repo add/remove`

### Error handling split

**Source:** RESEARCH.md § Standard Stack

**Apply to:** Core (`thiserror` typed errors) vs CLI (`anyhow` context at `main`)

### Input validation (ASVS V5)

**Source:** RESEARCH.md § Security Domain

**Apply to:** `repo add` (canonicalize, directory check), TOML parse errors, **parameterized SQL only** (`params![]`).

### Single-writer store (forward-compatible)

**Source:** `.planning/research/ARCHITECTURE.md` Pattern 3 (lines 161–167)

**Apply to:** Phase 1 may use synchronous single connection; design `store` API so a refresh worker channel can wrap it in Phase 2–3 without schema changes.

---

## No Analog Found

All Phase 1 application files are greenfield. No in-repo Rust analog exists.

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| *(all 18 classified files above)* | *various* | *various* | Repository has zero application source; only `.planning/` and `CLAUDE.md` |

**External structural references** (not copied into repo — for human context only):

| Reference | Relevance |
|-----------|-----------|
| [repoindex DESIGN](https://github.com/queelius/repoindex/blob/master/DESIGN.md) | service/domain/infra split (ARCHITECTURE.md line 104) |
| [commitmux](https://github.com/blackwell-systems/commitmux) | SQLite + git indexer shape (ARCHITECTURE.md line 375) |

Planner should **not** import these codebases into Workpot; use ARCHITECTURE.md + RESEARCH.md only.

---

## Metadata

**Analog search scope:** `/Users/rubenlr/c/workpot` (full tree); `**/*.rs`, `**/Cargo.toml`, `crates/**`, `src-tauri/**`
**Files scanned:** 39 paths (mostly `.planning/`, `.git/`)
**Application source files:** 0
**Pattern extraction date:** 2026-05-28
**Confidence:** HIGH for greenfield assessment; HIGH for ARCHITECTURE/RESEARCH as contract

---

## PATTERN MAPPING COMPLETE

**Phase:** 1 - Core & persistence
**Files classified:** 18
**Analogs found:** 0 / 18 (in-repo); 18 / 18 via ARCHITECTURE.md + RESEARCH.md contract

### Coverage
- Files with exact in-repo analog: 0
- Files with role-match in-repo analog: 0
- Files with architecture-contract analog: 18
- Files with no guidance: 0

### Key Patterns Identified
- **Greenfield:** No Rust code to copy; planner implements from ARCHITECTURE.md layout (`workpot-core` + `workpot-cli` only, no Tauri/ui).
- **AppContext hub:** Lazy `open()` bootstrap (config + DB + migrations); catalog service for manual repo CRUD.
- **Split paths:** `BaseDirs::config_dir()` vs `data_dir()` — never `ProjectDirs` for config (D-01).
- **SQLite:** `rusqlite` 0.39 bundled + `rusqlite_migration` 2.5; slim `001_init.sql`; WAL on open.
- **CLI:** Thin clap host; `paths` + `repo` subcommands; `thiserror`/`anyhow` split.

### File Created
`.planning/phases/01-core-persistence/01-PATTERNS.md`

### Ready for Planning
Pattern mapping complete. Planner should reference ARCHITECTURE.md sections and RESEARCH.md excerpts per file in `01-PLAN.md` tasks; no in-repo copy-paste sources exist.
