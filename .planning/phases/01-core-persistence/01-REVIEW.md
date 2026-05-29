---
phase: 01-core-persistence
reviewed: 2026-05-30T20:00:00Z
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
  info: 1
  total: 2
status: issues_found
---

# Phase 1: Code Review Report

**Reviewed:** 2026-05-30T20:00:00Z  
**Depth:** standard  
**Files Reviewed:** 23  
**Status:** issues_found

## Summary

Fourth pass on Phase 1 core-persistence. All three findings from the third pass (2026-05-30) are **fixed** in code: basename fallback for remove-after-delete (`resolve_repo_path_key`), stale `config.tmp` cleanup on open, and first-run watch-root seeding documented in `workpot paths` output.

Security and persistence posture remain sound: parameterized SQL, path canonicalization on register, atomic config writes via temp+rename+fsync, WAL + busy timeout, macOS offline CI gate with banned-network script.

Two minor new items remain — one warning (SQL LIKE metacharacters in deleted-repo basename lookup), one informational (missing regression test for tmp cleanup).

## Prior Finding Verification (third pass → fourth pass)

| ID | Status | Evidence |
|----|--------|----------|
| WR-01 (non-canonical remove after delete) | ✅ Fixed | `catalog.rs:126-163` `resolve_repo_path_key` basename/suffix lookup; `catalog_test.rs:194-213` removes via relative path after directory delete |
| IN-01 (orphan config tmp) | ✅ Fixed | `lib.rs:55,145-149` `remove_stale_config_temp` before config load; same `with_extension("tmp")` as `write_atomic` (`lib.rs:197`) |
| IN-02 (default watch roots surprise) | ✅ Fixed | `main.rs:92` documents seeding in `paths` output; `lib.rs:25-34` comment on `default_config` |

## Narrative Findings (AI reviewer)

## Warnings

### WR-01: Basename lookup for deleted repos does not escape SQL LIKE metacharacters

**File:** `crates/workpot-core/src/services/catalog.rs:141-151`  
**Issue:** When the repo directory is gone, `resolve_repo_path_key` falls back to `path LIKE '%' || ?2` where `?2` is `/{basename}`. Basenames containing `%` or `_` are interpreted as LIKE wildcards, not literal characters. A single spurious match can resolve to the wrong `repos.path` key and delete the wrong catalog row.  
**Fix:** Escape LIKE metacharacters in the basename before building the suffix, or avoid LIKE entirely:

```rust
fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

// In resolve_repo_path_key, when building the query:
let suffix = format!("/{}", escape_like(name));
let mut stmt = conn.prepare(
    "SELECT path FROM repos WHERE path = ?1 OR path LIKE '%' || ?2 ESCAPE '\\'",
)?;
```

Prefer filtering candidates in Rust (`path.ends_with(&format!("/{name}"))`) if the table stays small in v1.

## Info

### IN-01: No regression test for stale `config.tmp` cleanup

**File:** `crates/workpot-core/src/lib.rs:145-149`  
**Issue:** `remove_stale_config_temp` is the fix for the prior IN-01 finding but has no integration test. A future refactor could drop the call without CI catching it.  
**Fix:** Add a bootstrap test: create `config.tmp` beside a valid `config.toml`, open `AppContext`, assert tmp is gone.

---

_Reviewed: 2026-05-30T20:00:00Z_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
