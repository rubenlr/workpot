---
phase: 01-core-persistence
reviewed: 2026-05-29T12:00:00Z
depth: standard
files_reviewed: 22
files_reviewed_list:
  - .github/workflows/ci.yml
  - .gitignore
  - Cargo.toml
  - crates/workpot-cli/Cargo.toml
  - crates/workpot-cli/src/main.rs
  - crates/workpot-core/Cargo.toml
  - crates/workpot-core/src/domain/config.rs
  - crates/workpot-core/src/domain/mod.rs
  - crates/workpot-core/src/domain/repo.rs
  - crates/workpot-core/src/error.rs
  - crates/workpot-core/src/infra/migrations.rs
  - crates/workpot-core/src/infra/migrations/001_init.sql
  - crates/workpot-core/src/infra/mod.rs
  - crates/workpot-core/src/infra/paths.rs
  - crates/workpot-core/src/infra/store.rs
  - crates/workpot-core/src/lib.rs
  - crates/workpot-core/src/services/catalog.rs
  - crates/workpot-core/src/services/mod.rs
  - crates/workpot-core/tests/bootstrap_test.rs
  - crates/workpot-core/tests/catalog_test.rs
  - rust-toolchain.toml
  - scripts/check-no-network-deps.sh
findings:
  critical: 1
  warning: 5
  info: 4
  total: 10
status: issues_found
---

# Phase 1: Code Review Report

**Reviewed:** 2026-05-29T12:00:00Z  
**Depth:** standard  
**Files Reviewed:** 22  
**Status:** issues_found

## Summary

Phase 1 delivers a coherent Rust workspace: SQLite migrations, WAL mode, parameterized catalog SQL, typed errors, and thin CLI wiring. Security posture is appropriate for a local-only tool (no string-concatenated SQL, no shell execution, no network deps in core).

**Primary defect:** `paths.rs` uses `BaseDirs::config_dir()` for D-01, but on macOS `directories` 6.0 resolves that to `~/Library/Application Support`, not `~/.config`. Locked decision D-01 (`~/.config/workpot/config.toml`) is not met on the v1 platform. Manual UAT in `01-VALIDATION.md` will fail `workpot paths` on macOS.

Secondary gaps: weak `.git` presence check, no SQLite busy timeout, unused `anyhow` in core, and missing integration test for `remove_repo`.

## Critical Issues

### CR-01: macOS config path violates locked D-01

**File:** `crates/workpot-core/src/infra/paths.rs:5-8`  
**Issue:** `config_file()` uses `BaseDirs::config_dir()`. On macOS, `directories` 6.0 maps `config_dir()` to `~/Library/Application Support`, not `~/.config`. Resolved path is `~/Library/Application Support/workpot/config.toml`, while D-01 requires `~/.config/workpot/config.toml`. `data_dir()` correctly lands under Application Support for D-02, but config and DB both sit under the same macOS base today instead of the split specified in CONTEXT.md.  
**Fix:**

```rust
use std::path::PathBuf;

pub fn config_file() -> Result<PathBuf> {
    let base = BaseDirs::new().ok_or(WorkpotError::PathsUnavailable)?;
    #[cfg(target_os = "macos")]
    let config_root = base.home_dir().join(".config");
    #[cfg(not(target_os = "macos"))]
    let config_root = base.config_dir().to_path_buf();
    Ok(config_root.join("workpot").join("config.toml"))
}

pub fn database_file() -> Result<PathBuf> {
    BaseDirs::new()
        .map(|b| b.data_dir().join("workpot").join("workpot.db"))
        .ok_or(WorkpotError::PathsUnavailable)
}
```

Add a macOS integration test (or CI smoke step) asserting `workpot paths` prints the D-01/D-02 paths.

## Warnings

### WR-01: `.git` check accepts non-repository directories

**File:** `crates/workpot-core/src/services/catalog.rs:15-18`  
**Issue:** Registration only tests that `.git` exists as a file or directory. An empty `.git` directory (as in tests) passes; a mistaken `mkdir .git` registers a non-repo. Users get a persisted row with no real git metadata until Phase 3.  
**Fix:** Minimum hardening for Phase 1: require `.git/HEAD` (file) for directory-style `.git`, or parse `gitdir:` for gitfile worktrees:

```rust
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
```

Defer full `git2` open until Phase 3 if desired, but do not treat bare `.git` presence as sufficient.

### WR-02: No SQLite busy timeout for concurrent CLI invocations

**File:** `crates/workpot-core/src/infra/store.rs:10-13`  
**Issue:** WAL is enabled, but default `busy_timeout` is 0. Two simultaneous `workpot` processes (e.g. tray + CLI later) can hit `SQLITE_BUSY` immediately instead of brief retry.  
**Fix:** After opening the connection:

```rust
conn.busy_timeout(std::time::Duration::from_secs(5))?;
```

### WR-03: Misleading error when register path does not exist

**File:** `crates/workpot-core/src/services/catalog.rs:8-12`  
**Issue:** `path.is_dir()` is false for missing paths, so users see `invalid path: not a directory: …` instead of a clear “path does not exist”.  
**Fix:**

```rust
if !path.exists() {
    return Err(WorkpotError::InvalidPath(format!("path does not exist: {}", path.display())));
}
if !path.is_dir() {
    return Err(WorkpotError::InvalidPath(format!("not a directory: {}", path.display())));
}
```

### WR-04: Bare repositories cannot be registered

**File:** `crates/workpot-core/src/services/catalog.rs:15-18`  
**Issue:** Bare repos expose `HEAD` at the repo root; there is no `path/.git` child. `register_manual` always rejects them. Acceptable for Phase 1 manual UX if documented; conflicts with eventual bare/worktree layouts in STACK.md unless only scan discovers them.  
**Fix:** Document limitation in CLI help, or detect bare repo via `path/HEAD` + `path/objects` (still filesystem-only) until `git2` lands.

### WR-05: `anyhow` declared in `workpot-core` but unused

**File:** `crates/workpot-core/Cargo.toml:9`  
**Issue:** Research locks `anyhow` to the CLI edge; core uses `thiserror` only. Unused dependency adds noise to `cargo tree` and violates stated layering.  
**Fix:** Remove `anyhow = "1"` from `workpot-core/Cargo.toml`; keep it only in `workpot-cli`.

## Info

### IN-01: `excluded` column never read or written

**File:** `crates/workpot-core/src/infra/migrations/001_init.sql:6`, `crates/workpot-core/src/services/catalog.rs:57-60`  
**Issue:** Schema includes `excluded` for Phase 2, but catalog queries ignore it. Not a bug today; risk of listing excluded repos once Phase 2 sets the flag without updating `list_repos`.  
**Fix:** When exclude UX ships, add `WHERE excluded = 0` to list queries (or document that Phase 2 must update catalog in the same change).

### IN-02: `watch_roots` / `excludes` loaded but unused

**File:** `crates/workpot-core/src/domain/config.rs:6-9`, `crates/workpot-core/src/lib.rs:60-62`  
**Issue:** Config fields are persisted and defaulted but not consumed. Expected for Phase 1; no action until Phase 2 scanning.  
**Fix:** None until watch-root indexing; optional `#[allow(dead_code)]` on fields is unnecessary if accessed via `config()`.

### IN-03: `list_repos` sort is unstable for identical timestamps

**File:** `crates/workpot-core/src/services/catalog.rs:58-59`  
**Issue:** `ORDER BY registered_at` only; bulk registrations in the same second have undefined order.  
**Fix:** `ORDER BY registered_at, path` for deterministic output.

### IN-04: No automated test for `remove_repo`

**File:** `crates/workpot-core/tests/catalog_test.rs`  
**Issue:** Add/list/duplicate/non-git are covered; remove path (success, not-found, canonicalization) is not. Does not block Phase 1 but leaves regression risk.  
**Fix:** Add `remove_repo_deletes_and_not_found` integration test mirroring register persistence patterns.

---

_Reviewed: 2026-05-29T12:00:00Z_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
