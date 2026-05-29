---
phase: 02-repo-discovery
reviewed: 2026-05-30T12:00:00Z
depth: standard
files_reviewed: 18
files_reviewed_list:
  - crates/workpot-cli/src/main.rs
  - crates/workpot-cli/tests/cli_smoke.rs
  - crates/workpot-core/src/domain/config.rs
  - crates/workpot-core/src/error.rs
  - crates/workpot-core/src/infra/git.rs
  - crates/workpot-core/src/lib.rs
  - crates/workpot-core/src/services/catalog.rs
  - crates/workpot-core/src/services/discovery.rs
  - crates/workpot-core/src/services/excludes.rs
  - crates/workpot-core/src/services/index.rs
  - crates/workpot-core/src/services/mod.rs
  - crates/workpot-core/src/services/roots.rs
  - crates/workpot-core/tests/catalog_test.rs
  - crates/workpot-core/tests/discovery_test.rs
  - crates/workpot-core/tests/excludes_test.rs
  - crates/workpot-core/tests/index_test.rs
  - crates/workpot-core/tests/roots_test.rs
  - crates/workpot-core/src/infra/migrations/002_discovery.sql
findings:
  critical: 0
  warning: 5
  info: 2
  total: 7
status: issues_found
---

# Phase 2: Code Review Report

**Reviewed:** 2026-05-30T12:00:00Z  
**Depth:** standard  
**Files Reviewed:** 18  
**Status:** issues_found

## Summary

Fresh adversarial pass on Phase 02 repo discovery after the 2026-05-29 fix iteration. Prior findings WR-01–WR-05 and IN-01–IN-03 were verified as landed: `repo_under_root` no longer aborts on missing paths, `Config::validate` enforces soft watch-root count, `roots add` runs index before save, bare worktree paths are canonicalized or skipped, error index runs are recorded, manual register resolves `git_common_dir`, and CLI cap handling is consolidated in `main`.

No critical security or data-loss defects found in normal happy paths. Five warnings remain around limit enforcement gaps, partial-failure consistency, and orphan scan-row hygiene. SQL and git access remain parameterized and canonicalize-first.

## Narrative Findings (AI reviewer)

## Warnings

### WR-01: `register_manual` bypasses `max_repos`

**File:** `crates/workpot-core/src/services/catalog.rs:9-50`  
**Issue:** `index::run_full` enforces `limits.max_repos` before merge, but `register_manual` inserts directly with no count check. A user can exceed the configured cap indefinitely via `workpot repo add`, undermining D-14/D-23 for manually registered repos.  
**Fix:** Before insert, count non-excluded rows and return `WorkpotError::IndexCapExceeded` (or a dedicated limit error) when `count >= config.limits.max_repos`. Thread `&Config` into `register_manual` or check via a small helper on `Connection`.

### WR-02: `roots add` — index committed but `save_config` failure leaves orphan scan rows

**File:** `crates/workpot-core/src/services/roots.rs:25-29`  
**Issue:** The WR-03 fix runs index before save, but if `save_config` fails after a successful `run_full`, the watch root is not written to disk while scan-sourced repos are already in SQLite. On the next CLI invocation the root is absent from config, so `collect_stale_scan_paths` never targets those rows (they are not under any configured root). Repos persist until manual `repo remove`.  
**Fix:** On `save_config` failure, pop the in-memory root (mirror the index-failure rollback) and either (a) run a compensating prune for repos discovered in that index pass, or (b) write config first to a temp file and atomically rename only after index succeeds.

### WR-03: `roots remove` prune skips deleted paths under removed root

**File:** `crates/workpot-core/src/services/roots.rs:88-92`  
**Issue:** The WR-01 fix correctly avoids aborting the whole prune when `canonicalize` fails, but it also skips deletion for missing directories. After a user deletes repo folders and removes a watch root, scan rows remain in the DB until `workpot index` runs `collect_missing_paths`. `repo list` shows stale entries in the interim.  
**Fix:** When `canonicalize` fails, fall back to prefix match on stored path keys (both sides are canonical strings from upsert) or delete scan rows whose stored `path` is under `root_canon` regardless of filesystem presence:

```rust
fn repo_under_root(repo_path: &Path, root_canon: &Path) -> Result<bool> {
    match repo_path.canonicalize() {
        Ok(repo_canon) => Ok(repo_canon.starts_with(root_canon)),
        Err(_) => Ok(repo_path.starts_with(root_canon)),
    }
}
```

### WR-04: Index never prunes scan repos outside all configured watch roots

**File:** `crates/workpot-core/src/services/index.rs:259-281`  
**Issue:** `collect_stale_scan_paths` only considers scan repos still under a *current* watch root. Scan rows whose directories exist but are no longer covered by any root (e.g. `roots remove --skip-prune`, hand-edited config, or WR-02 failure) are never removed by `run_full`. Only `collect_missing_paths` handles deleted directories. Operability gap is partially documented for `--skip-prune` but applies more broadly.  
**Fix:** Add a pass that removes `source = 'scan'` rows not under any canonical watch root (unless `--skip-prune`-style opt-out is intentional), or document that `workpot index` will not clean these and require explicit `repo remove`.

### WR-05: `path_under_root` skips canonicalization unlike `repo_under_root`

**File:** `crates/workpot-core/src/services/index.rs:401-403`, `crates/workpot-core/src/services/roots.rs:88-92`  
**Issue:** Stale-scan and manual-outside-root checks use `path.starts_with(root)` on the raw DB path string, while root prune uses `canonicalize` + `starts_with`. If any stored path or root ever diverges in representation (e.g. `/tmp` vs `/private/tmp` on macOS before full canonicalization), stale detection and prune behavior can disagree.  
**Fix:** Extract a shared helper that canonicalizes when possible and falls back to component-wise prefix match, used by both `index.rs` and `roots.rs`.

## Info

### IN-01: Manual register still allows empty `git_common_dir` on git failure

**File:** `crates/workpot-core/src/services/catalog.rs:43-45`  
**Issue:** IN-02 fix resolves `git_common_dir` when git is available, but `unwrap_or_default()` silently stores an empty string when `resolve_git_common_dir` fails. Row is incomplete until a successful index backfill.  
**Fix:** Return `WorkpotError::GitUnavailable` when resolution fails (consistent with scan skip behavior), or document that empty `git_common_dir` is expected until index.

### IN-02: Config validation errors always map to `LimitsExceeded`

**File:** `crates/workpot-core/src/lib.rs:166-168`  
**Issue:** `config.validate()` returns `Result<(), String>` for both hard-cap violations and watch-root count violations, but `load_config` / `save_config` map all failures to `WorkpotError::LimitsExceeded`. Misleading for operators debugging invalid TOML limits vs count mismatch.  
**Fix:** Map validation failures to `WorkpotError::Config(msg)` or split error variants.

---

_Reviewed: 2026-05-30T12:00:00Z_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
