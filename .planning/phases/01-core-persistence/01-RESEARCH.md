# Phase 1: Core & persistence - Research

**Researched:** 2026-05-28
**Domain:** Rust Cargo workspace bootstrap, local TOML config, SQLite persistence (rusqlite), CLI proof surface
**Confidence:** HIGH (stack/migration choice); MEDIUM (macOS path edge cases, CI network isolation)

## Summary

Phase 1 is a **greenfield Rust workspace** with `workpot-core` (library) and `workpot-cli` (binary). No Tauri, no git2, no filesystem watcher, no HTTP clients. The product goal is proving **DATA-01** (durable local config + SQLite) and **DATA-02** (no network dependency) before any indexing or tray work.

**Primary recommendation:** Bootstrap **core + CLI only** (no `src-tauri` / `ui` stubs). Use **`rusqlite` 0.39 + `bundled` + `rusqlite_migration` 2.5** for schema versioning via `user_version`. Resolve paths with **`directories` 6.0 `BaseDirs`** (not `ProjectDirs` for config — locked D-01 requires `~/.config/workpot`). Lazy **`AppContext::open()`** on first CLI invocation creates default `config.toml`, DB directory, runs migrations, and serves `repo add` / `repo list`. Defer git snapshot columns to Phase 3; keep Phase 1 `repos` table to identity + registration metadata only.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Config load/save (`config.toml`) | **Rust core (`workpot-core`)** | CLI (thin) | Shared by future Tauri; human-editable TOML |
| Path resolution (config vs DB) | **Rust core (`infra/paths`)** | — | Locked macOS paths; no env overrides in Phase 1 |
| SQLite open + migrations | **Rust core (`infra/store`)** | — | Single writer; tray/CLI reuse later |
| Repo registration (manual path) | **Rust core (`services/catalog`)** | CLI subcommands | Phase 1 proves persistence; discovery is Phase 2 |
| CLI UX / exit codes | **CLI (`workpot-cli`)** | — | clap only; no business logic in `main` beyond wiring |
| Network / remote APIs | **— (forbidden)** | — | DATA-02; no deps that pull HTTP stacks in Phase 1 |

## Project Constraints (from CLAUDE.md)

- **Platform:** macOS only for v1 (Tauri tray later; not Phase 1).
- **Privacy:** Local-only — index and config stay on disk.
- **Stack direction:** Rust shared core, SQLite, TOML config, clap CLI; git2/notify/nucleo deferred to later phases.
- **GSD workflow:** Planning artifacts drive execution; this research feeds `01-PLAN.md`.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Primary config file at `~/.config/workpot/config.toml` (XDG-style; dotfile-friendly, terminal-editable).
- **D-02:** SQLite database at `~/Library/Application Support/workpot/workpot.db` (Apple-native app data location).
- **D-03:** No environment-variable path overrides in Phase 1 — fixed default paths only; defer `WORKPOT_HOME` / `WORKPOT_DATA_DIR` until integration tests or power-user needs justify it.
- **D-04:** First launch creates both artifacts: default `config.toml` (documented empty `watch_roots`, sensible defaults) and empty DB with migrations applied — no explicit `workpot init` gate required.

### Claude's Discretion

- Workspace bootstrap depth (core+CLI only vs empty Tauri stubs), Phase 1 CLI command set, and initial `repos` table column scope — not discussed; follow `.planning/research/ARCHITECTURE.md` Phase 1 slice and ROADMAP success criteria unless planner flags conflict.

### Deferred Ideas (OUT OF SCOPE)

- Workspace bootstrap: core+CLI only vs include empty `src-tauri` / `ui` stubs in Phase 1
- Phase 1 CLI verbs beyond what success criteria require
- Initial SQLite schema width (minimal vs forward-compatible nullable columns)
- Env-based path overrides (explicitly deferred past Phase 1 per D-03)

</user_constraints>

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DATA-01 | All index data, tags, and recipes persist locally on disk | `config.toml` + SQLite `workpot.db` with migrations; `repo add` persists path; `repo list` after restart proves durability |
| DATA-02 | No network calls or accounts required for core functionality | Phase 1 dependency set excludes HTTP clients; CI `cargo tree` + optional deny rules; tests use temp dirs only |

</phase_requirements>

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `rusqlite` | **0.39.0** + `features = ["bundled"]` | Embedded SQLite | [CITED: rusqlite README] recommends `bundled` for app-owned DBs; avoids macOS system SQLite drift |
| `rusqlite_migration` | **2.5.0** | Schema migrations via `user_version` | [CITED: docs.rs/rusqlite_migration] — no migration metadata table; fits sync desktop app; MSRV 1.84 |
| `serde` | **1.0** + `derive` | Config (de)serialization | Project standard |
| `toml` | **0.8** | `config.toml` parse/write | Human-editable; locked format |
| `directories` | **6.0.0** | OS-standard base dirs | [CITED: directories-rs] — `BaseDirs` for `~/.config` and Application Support data |
| `thiserror` | **2.0** | Typed errors in core | Clean CLI error mapping |
| `anyhow` | **1.0** | CLI error context | Acceptable at binary edge only |
| `clap` | **4.6.1** + `derive` | CLI subcommands | [VERIFIED: crates.io] — non-interactive, scriptable |

**Intentionally absent in Phase 1:** `git2`, `notify`, `tokio`, `reqwest`/`ureq`/`hyper`, Tauri, `sqlx`, `refinery` (see migration section).

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tempfile` | **3.x** | Integration tests | Isolated HOME/config/db per test |
| `cargo-nextest` | latest (dev) | Fast test runs | CI / local when installed |
| `cargo-deny` | latest (dev) | License + optional ban list | CI gate before distribution |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `rusqlite_migration` | **refinery 0.9.1** | Refinery maintains `_refinery_schema_history` table + embed macro/CLI; heavier than needed for a single embedded DB [CITED: docs.rs/refinery] |
| `rusqlite_migration` | **sqlx** | Async-first, larger dep graph; wrong fit for sync Phase 1 core with no server |
| `BaseDirs` path split | **`ProjectDirs::from(...)`** | On macOS, `ProjectDirs::config_dir()` is under **Application Support**, not `~/.config` — violates D-01 [CITED: directories-rs README] |
| core+CLI only | Empty Tauri/`ui` stubs | Adds Node/toolchain/CI cost with zero Phase 1 value |

**Installation (workspace root `Cargo.toml`):**

```toml
[workspace]
members = ["crates/workpot-core", "crates/workpot-cli"]
resolver = "2"

[workspace.package]
edition = "2024"
rust-version = "1.85"
```

Pin in `workpot-core/Cargo.toml`:

```toml
rusqlite = { version = "0.39.0", features = ["bundled"] }
rusqlite_migration = "2.5.0"
directories = "6.0.0"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
thiserror = "2"
```

**Version verification (2026-05-28, crates.io API):** `rusqlite_migration` 2.5.0 depends on `rusqlite ^0.39.0` (not 0.40). `rusqlite` latest is 0.40.0 — **pin 0.39.0** until migration crate widens its constraint.

## Package Legitimacy Audit

> slopcheck was unavailable in the research environment — all packages below require planner `checkpoint:human-verify` before lockfile commit unless slopcheck is run locally.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| rusqlite | crates.io | ~11 yrs | very high | github.com/rusqlite/rusqlite | n/a | Approved pending verify |
| rusqlite_migration | crates.io | ~5 yrs | ~1.6M total | github.com/cljoly/rusqlite_migration | n/a | Approved pending verify |
| directories | crates.io | ~8 yrs | very high | github.com/dirs/directories-rs | n/a | Approved pending verify |
| clap | crates.io | ~10 yrs | very high | github.com/clap-rs/clap | n/a | Approved pending verify |
| refinery | crates.io | ~10 yrs | ~7.8M total | github.com/rust-db/refinery | n/a | Not used Phase 1 |
| sqlx | crates.io | — | — | — | n/a | Not used Phase 1 |

**Packages removed due to slopcheck [SLOP] verdict:** none (slopcheck not run)
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│  workpot-cli (clap)                                          │
│  repo add | repo list | paths | (optional) repo remove       │
└───────────────────────────┬─────────────────────────────────┘
                            │ AppContext::open() once per process
┌───────────────────────────▼─────────────────────────────────┐
│  workpot-core                                                  │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────────────┐ │
│  │ paths       │  │ config       │  │ catalog (repo CRUD)    │ │
│  │ BaseDirs    │→ │ load/save    │  │ register_manual path │ │
│  └─────────────┘  └──────────────┘  └───────────┬────────────┘ │
│  ┌─────────────────────────────────────────────▼────────────┐ │
│  │ store: Connection + WAL pragma + rusqlite_migration        │ │
│  └──────────────────────────────────────────────────────────┘ │
└───────────────────────────┬─────────────────────────────────┘
                            │
         ┌──────────────────┴──────────────────┐
         ▼                                      ▼
 ~/.config/workpot/config.toml     ~/Library/Application Support/
                                    workpot/workpot.db
```

### Recommended Project Structure

```
workpot/
├── Cargo.toml                 # workspace
├── rust-toolchain.toml        # pin 1.85+
├── crates/
│   ├── workpot-core/
│   │   └── src/
│   │       ├── lib.rs         # AppContext, public API
│   │       ├── domain/        # Config, RepoRecord
│   │       ├── services/      # catalog.rs
│   │       └── infra/         # paths.rs, store.rs, migrations.rs
│   └── workpot-cli/
│       └── src/main.rs        # clap → AppContext
└── .cargo/
    └── config.toml            # optional: nextest, deny (Wave 0)
```

**Do not add `src-tauri/` or `ui/` in Phase 1** — ROADMAP Phase 4 introduces Tauri; empty stubs force Node CI and confuse “what ships now.”

### Pattern 1: Lazy first-launch bootstrap (D-04)

**What:** Any subcommand calls `AppContext::open()` which: ensures parent dirs exist → writes default config if missing → opens/creates DB → `PRAGMA journal_mode=WAL` → `MIGRATIONS.to_latest()`.

**When:** Every CLI invocation; future Tauri uses same entrypoint.

**Example:**

```rust
// Source: [CITED: docs.rs/rusqlite_migration] + [CITED: rusqlite context7 WAL pragma]
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

### Pattern 2: Split path resolution (D-01 vs D-02)

**What:** `config_path()` = `BaseDirs::config_dir()?.join("workpot/config.toml")`. `db_path()` = `BaseDirs::data_dir()?.join("workpot/workpot.db")`.

**Why not `ProjectDirs` for config:** On macOS, `ProjectDirs::config_dir()` resolves under Application Support, not `~/.config` [CITED: directories-rs README].

### Pattern 3: Path-as-primary-key catalog

**What:** `repos.path` TEXT PRIMARY KEY; canonicalize with `std::fs::canonicalize` on register (best-effort; document symlink edge cases).

**When:** All repo identity; git columns added in later migrations.

### Pattern 4: Phase 1 `.git` check without git2

**What:** `repo add` requires directory exists and (`path/.git` is dir OR file). No `git` subprocess, no libgit2 — satisfies “git repo path” for ROADMAP criterion 3 without network or git crate.

### Anti-Patterns to Avoid

- **`workpot init` as required step:** Violates D-04; bootstrap must be implicit.
- **`ProjectDirs` for config:** Violates D-01 on macOS.
- **JSON index file:** PITFALLS — no migration story; use SQLite from day one.
- **Nullable git columns in migration 001:** Creates dead columns until Phase 3; prefer additive migration `002_git_snapshot` later.
- **Tauri stub in workspace:** Pulls frontend toolchain into Phase 1 CI with no deliverable.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Schema versioning | Custom `schema_version` table only | `rusqlite_migration` + `user_version` | Atomic steps, battle-tested [CITED: docs.rs/rusqlite_migration] |
| macOS/XDG paths | Hardcoded `$HOME` strings | `directories::BaseDirs` | Correct OS conventions |
| CLI parsing | Manual `std::env::args` | `clap` derive | Help, validation, subcommands |
| SQL string concat for user paths | Dynamic SQL | `rusqlite` parameterized queries | Injection / escaping |

## Migration library choice (rusqlite_migration vs refinery vs sqlx)

| Criterion | rusqlite_migration 2.5 | refinery 0.9.1 | sqlx |
|-----------|------------------------|----------------|------|
| Sync `rusqlite::Connection` | Native | Supported via `rusqlite` driver | Async pool; blocking awkward |
| Migration tracking | SQLite `user_version` pragma | `_refinery_schema_history` table | `sqlx` migration table |
| Phase 1 deps | Minimal (rusqlite + log) | embed macro + optional CLI | tokio + drivers + larger graph |
| Aligns with ARCHITECTURE.md | Yes (named in doc) | Viable but heavier | Overkill |
| MSRV | 1.84 (fits 1.85 pin) | check at pin | higher |

**Prescription:** Use **`rusqlite_migration` 2.5.0** with SQL in `crates/workpot-core/src/infra/migrations/001_init.sql` included via `include_str!` in `M::up(...)`.

## Config schema (Phase 1)

```toml
# ~/.config/workpot/config.toml — created on first launch if missing

watch_roots = []
excludes = []   # glob strings; used Phase 2+, empty default documents intent

# Optional section for future phases (omit from default file if YAGNI):
# [defaults]
```

Serde shape:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub watch_roots: Vec<PathBuf>,
    #[serde(default)]
    pub excludes: Vec<String>,
}
```

Save with `toml::to_string_pretty` on explicit config mutation (Phase 1 may only write defaults once).

## Minimal `repos` table (forward-compatible)

**Recommendation: slim migration 001** — identity + registration only; git snapshot columns in **migration 002 (Phase 3)**.

```sql
-- 001_init.sql
CREATE TABLE repos (
  path TEXT NOT NULL PRIMARY KEY,
  name TEXT NOT NULL,
  registered_at INTEGER NOT NULL,  -- Unix seconds UTC
  source TEXT NOT NULL DEFAULT 'manual' CHECK (source IN ('manual', 'scan')),
  excluded INTEGER NOT NULL DEFAULT 0 CHECK (excluded IN (0, 1))
);

CREATE INDEX idx_repos_registered_at ON repos(registered_at);
```

| Column | Phase 1 use | Later phase |
|--------|---------------|-------------|
| `path` | PK, canonical path string | unchanged |
| `name` | `file_name()` or last segment for display | unchanged |
| `registered_at` | audit / future ranking | unchanged |
| `source` | always `'manual'` | `'scan'` in Phase 2 |
| `excluded` | always `0` | Phase 2 exclude UX |

**Future tables (not in 001):** `repo_tags`, recipe metadata — Phase 5/7 per ARCHITECTURE.md.

## Phase 1 CLI command set (ROADMAP success criteria)

| Command | Purpose | Success criterion |
|---------|---------|-------------------|
| `workpot paths` | Print resolved config + DB paths | #1 config paths visible |
| `workpot repo add <PATH>` | Register one git repo (`.git` check) | #3 persist repo |
| `workpot repo list` | List registered repos (path, name) | #3 visible after restart |
| `workpot repo remove <PATH>` | Optional; delete row | hygiene / tests |

**Implicit:** First run of any command triggers D-04 bootstrap (no `init`).

**Defer:** `index`, `search`, `open`, `refresh` — later phases.

**Restart proof (manual UAT / integration test):**

```bash
workpot repo add ~/dev/my-repo
workpot repo list
# new shell
workpot repo list   # same row present
```

## Verifying DATA-02 (no network)

### Design-time (primary)

1. **Dependency allowlist:** Phase 1 `workpot-core` / `workpot-cli` must not depend on `reqwest`, `ureq`, `hyper`, `curl`, `oauth2`, etc.
2. **CI `cargo tree` assertion:** Fail if tree contains banned crate names (script or `cargo-deny` ban).
3. **`cargo test --offline`** in CI after `cargo fetch` — proves build/test without registry access [CITED: Cargo book / Stack Overflow offline mode]; note this guards **Cargo**, not app sockets [CITED: Stack Overflow #79740512].

### Test-time (secondary)

4. **Hermetic integration tests:** `tempfile::TempDir` + env isolation pattern:
   - Set `HOME` (or inject path overrides **only in tests** via `AppContext::open_with_paths` test API — not production D-03).
   - Run `repo add` / `repo list` against temp config/db.
5. **No HTTP in test code:** Do not use `reqwest` test servers in Phase 1.

### Optional hardening (macOS CI)

6. **Custom test runner** with network namespace disabled on Linux CI (`target.'cfg(target_os = "linux")'.runner`) [CITED: Cargo config runner + unshare — MEDIUM confidence for Workpot macOS-first CI].

**Prescription for planner:** Wave 0 adds `scripts/check-no-network-deps.sh` (grep `cargo tree -p workpot-core`) + document that DATA-02 is **structural** in Phase 1 (no network crates), not runtime socket auditing.

## Common Pitfalls

### Pitfall 1: Using ProjectDirs for config (violates D-01)

**What goes wrong:** Config lands in `~/Library/Application Support/.../config/` instead of `~/.config/workpot/`.

**How to avoid:** `BaseDirs::config_dir().join("workpot/config.toml")` only.

### Pitfall 2: rusqlite 0.40 with rusqlite_migration 2.5

**What goes wrong:** Dependency resolution failure or subtle ABI mismatch.

**How to avoid:** Pin `rusqlite = "0.39.0"` until `rusqlite_migration` declares `^0.40` [VERIFIED: crates.io 2.5.0 deps].

### Pitfall 3: Manual `user_version` + migration table

**What goes wrong:** Double-tracking schema state; drift on downgrade.

**How to avoid:** Let `rusqlite_migration` own `user_version`; do not hand-set pragma in app code.

### Pitfall 4: Calling `workpot index` in Phase 1

**What goes wrong:** Scope creep into discovery/git (Phase 2–3).

**How to avoid:** `repo add` only; no watch-root scan.

### Pitfall 5: Single JSON persistence

**What goes wrong:** Corruption, no query layer [CITED: PITFALLS.md].

**How to avoid:** SQLite from migration 001.

## Code Examples

### Path resolution (macOS)

```rust
// Source: [CITED: directories-rs README] — BaseDirs platform table
use directories::BaseDirs;
use std::path::PathBuf;

pub fn config_file() -> Option<PathBuf> {
    BaseDirs::new().map(|b| b.config_dir().join("workpot").join("config.toml"))
}

pub fn database_file() -> Option<PathBuf> {
    BaseDirs::new().map(|b| b.data_dir().join("workpot").join("workpot.db"))
}
```

### clap subcommand skeleton

```rust
// Source: [CITED: context7 /clap-rs/clap Subcommand derive]
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

### Parameterized insert

```rust
// Source: [CITED: rusqlite context7]
conn.execute(
    "INSERT INTO repos (path, name, registered_at, source) VALUES (?1, ?2, ?3, 'manual')",
    params![path_display, name, registered_at],
)?;
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Custom migration tables | `user_version` via rusqlite_migration | 2020+ crate maturity | Faster open, less SQL metadata |
| `rusqlite` 0.32 (PROJECT.md) | 0.39–0.40 family | 2025–2026 | Pin 0.39 for migration compat |
| Required `init` command | Lazy bootstrap on first use | Workpot D-04 | Simpler UX |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `BaseDirs::data_dir()` on macOS resolves to `~/Library/Application Support` | Path resolution | If wrong, D-02 path drifts — verify on first macOS CI run |
| A2 | `canonicalize` is acceptable for repo PK | Catalog | Symlinked worktrees may need Phase 2 identity rules |
| A3 | `.git` file/dir presence is enough for “git repo” in Phase 1 | CLI validation | Bare repos / unusual layouts may need git2 later |

## Open Questions

1. **Test-only path injection API**
   - What we know: D-03 forbids env overrides in production.
   - What's unclear: Whether `AppContext::open_with_paths` in `#[cfg(test)]` is desired.
   - Recommendation: Add `open_with_paths` for tests only (not CLI flags) — planner discretion.

2. **Default `name` derivation**
   - Recommendation: Last path segment; allow future `repo rename` in ORG phase.

## Environment Availability

| Dependency | Required By | Available (research host) | Version | Fallback |
|------------|------------|---------------------------|---------|----------|
| Rust toolchain | build | ✗ (not in PATH) | pin **1.85+** in `rust-toolchain.toml` | Install via rustup on dev/CI macOS |
| cargo / rustc | build, test | ✗ | — | CI must install before execute |
| cargo-nextest | fast tests | ✗ | — | `cargo test` |
| slopcheck | package audit | ✗ | — | Manual `checkpoint:human-verify` |
| Xcode CLT | libgit2 later | [ASSUMED] present on macOS dev | — | Phase 1 does not need git2 |

**Missing dependencies with no fallback:** Rust toolchain on execute agents (blocking for implementation, not for planning).

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `cargo test` (optional `cargo-nextest` when installed) |
| Config file | none — Wave 0 may add `.config/nextest.toml` |
| Quick run command | `cargo test -p workpot-core` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| DATA-01 | Default config created on first open | integration | `cargo test -p workpot-core config_creates_defaults` | ❌ Wave 0 |
| DATA-01 | Migrations apply on fresh DB | unit/integration | `cargo test -p workpot-core migrations_apply` | ❌ Wave 0 |
| DATA-01 | `repo add` survives new `AppContext` | integration | `cargo test -p workpot-core repo_persists_across_reopen` | ❌ Wave 0 |
| DATA-02 | No HTTP crates in dependency graph | static | `scripts/check-no-network-deps.sh` or `cargo tree` grep | ❌ Wave 0 |
| DATA-02 | Tests pass offline | CI | `cargo test --offline --workspace` (after fetch) | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p workpot-core`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green + manual `repo add` / restart UAT per ROADMAP

### Wave 0 Gaps

- [ ] Workspace `Cargo.toml` + `workpot-core` / `workpot-cli` crates
- [ ] `rust-toolchain.toml` (1.85+)
- [ ] `workpot-core` tests with temp paths + migration smoke
- [ ] `scripts/check-no-network-deps.sh` for DATA-02
- [ ] `.github/workflows/ci.yml` (macOS): `cargo fetch`, `cargo test --offline`, deny script
- [ ] Framework install: rustup on CI — required before execute

## Security Domain

### Applicable ASVS Categories (ASVS L1)

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | N/A Phase 1 |
| V3 Session Management | no | N/A |
| V4 Access Control | no | Local single-user |
| V5 Input Validation | **yes** | Canonicalize paths; reject non-directories; parameterized SQL; validate TOML parse errors |
| V6 Cryptography | no | No secrets in Phase 1 |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| SQL injection via repo path | Tampering | `params![]` only |
| Path traversal on `repo add` | Elevation | `canonicalize` + ensure path stays user-intended; clear errors |
| TOML billion-laughs / huge file | DoS | Size cap on read [ASSUMED] — optional Phase 1 |
| Supply-chain typosquat | Spoofing | slopcheck + human verify at pin |

## Sources

### Primary (HIGH confidence)

- [/websites/rs_rusqlite_migration](https://docs.rs/rusqlite_migration) — `Migrations::from_slice`, `to_latest`, WAL before migrate
- [/rusqlite/rusqlite](https://github.com/rusqlite/rusqlite) — `bundled` feature guidance
- [crates.io API](https://crates.io/api/v1/crates/rusqlite_migration) — version 2.5.0, rusqlite ^0.39.0 dep
- [crates.io API](https://crates.io/api/v1/crates/rusqlite) — 0.40.0 latest (pin 0.39 for compat)
- [directories-rs README](https://github.com/dirs/directories-rs) — BaseDirs vs ProjectDirs macOS paths
- [/clap-rs/clap](https://github.com/clap-rs/clap) — Subcommand derive

### Secondary (MEDIUM confidence)

- [docs.rs/refinery](https://docs.rs/refinery/latest/refinery/) — comparison baseline
- [Stack Overflow: disable network in tests](https://stackoverflow.com/questions/79740512/how-to-disable-network-access-for-cargo-tests) — `--offline` vs socket isolation
- `.planning/research/ARCHITECTURE.md`, `STACK.md`, `PITFALLS.md`, `ROADMAP.md`

### Tertiary (LOW confidence)

- macOS `BaseDirs::data_dir()` exact suffix `workpot/` — verify in execute-phase smoke test (A1)

## Metadata

**Confidence breakdown:**
- Standard stack: **HIGH** — crates.io + official docs aligned with ARCHITECTURE.md
- Architecture: **HIGH** — greenfield, locked CONTEXT paths
- Pitfalls: **MEDIUM** — path/canonicalize edge cases need macOS validation

**Research date:** 2026-05-28
**Valid until:** 2026-06-28 (stable Rust/SQLite ecosystem)
