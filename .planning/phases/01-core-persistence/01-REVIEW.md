---
phase: 01-core-persistence
reviewed: 2026-05-30T18:00:00Z
depth: standard
files_reviewed: 23
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
  - crates/workpot-core/tests/paths_test.rs
  - rust-toolchain.toml
  - scripts/check-no-network-deps.sh
findings:
  critical: 0
  warning: 1
  info: 2
  total: 3
status: issues_found
---

# Phase 1: Code Review Report

**Reviewed:** 2026-05-30T18:00:00Z  
**Depth:** standard  
**Files Reviewed:** 23  
**Status:** issues_found

## Summary

Third pass on Phase 1 core-persistence scope (current `feature/03-git-state` tree). All seven findings from the 2026-05-30 earlier review are **fixed** in code: gitdir `HEAD` validation, remove-after-delete (canonical path), atomic config writes, macOS D-12 offline + ban script in CI, config-before-DELETE ordering in `remove_repo_with_exclude`, and `max_repos` on manual register (Phase 2 landed in the same `catalog.rs`).

Security and DATA-02 posture remain sound: parameterized SQL, path canonicalization on register, no shell execution, banned network crates checked on macOS CI, 5s SQLite busy timeout, WAL mode.

Three minor items remain — one warning (stale row when removing with a non-canonical path after the directory is gone), two informational.

## Prior Finding Verification (2026-05-30 batch)

| ID | Status | Evidence |
|----|--------|----------|
| WR-01 | ✅ Fixed | `catalog.rs:179-194` resolves gitdir target and requires `HEAD` |
| WR-02 | ✅ Fixed | `resolve_repo_location` NotFound fallback; `remove_repo_succeeds_when_directory_deleted` |
| WR-03 | ✅ Fixed | `lib.rs:183-196` `write_atomic` for config create/save |
| WR-04 | ✅ Fixed | `ci.yml:52-60` macOS `cargo fetch`, ban script, `--offline` test |
| WR-05 | ✅ Fixed | `remove_repo_with_exclude` calls `save_config` before `remove_repo` |
| IN-01 | ✅ N/A | `remove_repo` used by `remove_repo_with_exclude` — not dead |
| IN-02 | ✅ Fixed | `register_manual` uses `resolve_git_common_dir(...)?` (hard error, not silent empty) |

## Narrative Findings (AI reviewer)

## Warnings

### WR-01: `repo remove` after directory delete fails unless path matches stored canonical key

**File:** `crates/workpot-core/src/services/catalog.rs:109-120`  
**Issue:** Registration stores `path.canonicalize()` as the SQLite key. When the directory is deleted, `resolve_repo_location` falls back to `path.display().to_string()` on `NotFound`. If the user passes a relative path (`./my-repo`) or a symlink path that differed at register time, the DELETE matches zero rows and returns `NotFound` even though the row still exists under the absolute canonical key. The integration test only covers remove with the canonical path after delete.  
**Fix:** On `NotFound` from `canonicalize`, look up by basename under known parents or scan `SELECT path FROM repos WHERE path LIKE '%' || ?1` with normalized `file_name`, or document that remove-after-delete requires the same absolute path. Minimal fix:

```rust
Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
    let name = path.file_name().and_then(|n| n.to_str());
    if let Some(name) = name {
        if let Ok(key) = conn.query_row(
            "SELECT path FROM repos WHERE path LIKE '%' || ?1",
            params![name],
            |row| row.get::<_, String>(0),
        ) {
            return Ok((path.to_path_buf(), key));
        }
    }
    Ok((path.to_path_buf(), path.display().to_string()))
}
```

(Prefer a single-row match guard to avoid ambiguous basenames.)

## Info

### IN-01: Orphan `config.toml.tmp` on crash mid-write

**File:** `crates/workpot-core/src/lib.rs:188-195`  
**Issue:** `write_atomic` writes `config.toml.tmp` then renames. Kill/crash after `fs::write` but before `rename` leaves a stale tmp next to config; harmless but can confuse support.  
**Fix:** On `AppContext::open`, ignore/remove stale `*.tmp` in the config directory, or use `tempfile` in the same directory with explicit cleanup.

### IN-02: First-run `default_config` seeds `~/code` and `~/dev` watch roots

**File:** `crates/workpot-core/src/lib.rs:26-34`  
**Issue:** Empty first config is not empty — users with those directories get implicit watch roots before they run `roots add`. Surprising for privacy-focused “local only” users who expected zero discovery until configured.  
**Fix:** Product call: document in README/CLI `paths` output, or gate seeding behind an explicit opt-in key in config.

---

_Reviewed: 2026-05-30T18:00:00Z_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
