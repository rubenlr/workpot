---
phase: 01-core-persistence
reviewed: 2026-05-30T12:00:00Z
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
  warning: 5
  info: 2
  total: 7
status: issues_found
---

# Phase 1: Code Review Report

**Reviewed:** 2026-05-30T12:00:00Z  
**Depth:** standard  
**Files Reviewed:** 23  
**Status:** issues_found

## Summary

Re-review of Phase 1 core-persistence files after the 2026-05-29 fix pass. **All 10 prior findings (CR-01, WR-01–WR-05, IN-01–IN-04) remain fixed** in the current tree: macOS D-01/D-02 paths, git fixture validation, busy timeout, bare-repo support, `anyhow` removed from core, excluded-row filtering, stable sort, and remove-repo integration test.

The codebase has since absorbed Phase 2+ surface area inside the same files (`remove_repo_with_exclude`, `git_common_dir` resolution, extra migrations). Seven **new** issues were found — none critical, but several affect catalog correctness or D-12 CI contract.

Security posture remains appropriate: parameterized SQL, path canonicalization, no shell execution, no banned network crates in core/CLI trees.

## Prior Fix Verification

| ID | Status | Evidence |
|----|--------|----------|
| CR-01 | ✅ Fixed | `paths.rs:7-11` uses `~/.config` on macOS; `paths_test.rs` asserts D-01/D-02 |
| WR-01 | ✅ Fixed | `is_git_worktree()` requires `.git/HEAD` for directory markers |
| WR-02 | ✅ Fixed | `store.rs:11` sets 5s `busy_timeout` |
| WR-03 | ✅ Fixed | `catalog.rs:10-14` distinct "path does not exist" error |
| WR-04 | ✅ Fixed | `is_bare_repo()` + CLI help mentions bare repos |
| WR-05 | ✅ Fixed | `anyhow` absent from `workpot-core/Cargo.toml` |
| IN-01 | ✅ Fixed | `list_repos` filters `WHERE excluded = 0` |
| IN-02 | ✅ Fixed | `config.rs:34-37` documents Phase 2 consumption |
| IN-03 | ✅ Fixed | `ORDER BY registered_at, path` |
| IN-04 | ✅ Fixed | `remove_repo_deletes_and_not_found` test present |

## Narrative Findings (AI reviewer)

## Warnings

### WR-01: Gitdir-file worktrees accept any `gitdir:` prefix without validating target

**File:** `crates/workpot-core/src/services/catalog.rs:158-161`  
**Issue:** When `.git` is a file, registration succeeds if the file contents start with `gitdir:` — the linked directory is never opened or checked for `HEAD`. A `.git` file pointing at a non-existent or non-repo path registers successfully. The prior WR-01 fix hardened directory-style `.git` but left gitfile worktrees weak.  
**Fix:** Resolve and validate the gitdir target:

```rust
if marker.is_file() {
    let content = std::fs::read_to_string(&marker).ok()?;
    let rest = content.strip_prefix("gitdir:")?.trim();
    let gitdir = PathBuf::from(rest);
    let gitdir = if gitdir.is_absolute() {
        gitdir
    } else {
        path.join(gitdir)
    };
    return gitdir.join("HEAD").is_file();
}
```

### WR-02: `repo remove` fails when the directory no longer exists

**File:** `crates/workpot-core/src/services/catalog.rs:122-125` (also `102-105`)  
**Issue:** Both `remove_repo` and `remove_repo_with_exclude` call `path.canonicalize()` before DELETE. If the user deleted the repo directory, `canonicalize` fails with `InvalidPath` and the SQLite row remains — `repo list` shows a stale entry with no CLI path to remove it.  
**Fix:** Fall back to deleting by the user-supplied path string when canonicalize fails with `NotFound`, or add a `--force` flag that deletes by normalized path key without requiring the directory to exist:

```rust
let path_key = match path.canonicalize() {
    Ok(c) => c.display().to_string(),
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
        // try stored key as-is or via parent+file_name normalization
        path.display().to_string()
    }
    Err(e) => return Err(WorkpotError::InvalidPath(format!("{}: {e}", path.display()))),
};
```

### WR-03: Non-atomic config writes risk corruption on crash

**File:** `crates/workpot-core/src/lib.rs:156`, `crates/workpot-core/src/lib.rs:182`  
**Issue:** `ensure_default_config` and `save_config` use `fs::write` directly. A crash or kill mid-write can leave `config.toml` truncated/invalid; the next `AppContext::open()` fails with `Config` error and blocks all CLI commands until manual repair.  
**Fix:** Write to a temp file in the same directory, `fsync`, then `rename` atomically (same pattern ai-memory uses for wiki writes).

### WR-04: D-12 offline enforcement not on macOS CI matrix

**File:** `.github/workflows/ci.yml:39-52`, `.github/workflows/ci.yml:69-76`  
**Issue:** Locked decision D-12 requires macOS CI to run `cargo test --offline` plus dependency policy. The `test (macos-latest)` job runs `cargo test --workspace --all-targets` without `--offline` and without `check-no-network-deps.sh`. Offline + ban checks run only on `ubuntu-latest` in the `coverage` job. A macOS-only dependency regression could slip through the matrix job that actually targets the v1 platform.  
**Fix:** Add to the `macos-latest` test job (after `cargo fetch`):

```yaml
- run: bash scripts/check-no-network-deps.sh
- run: cargo test --offline --workspace --all-targets
```

Or merge offline/ban checks into the macOS matrix step per D-12.

### WR-05: `remove_repo_with_exclude` commits DELETE before config save

**File:** `crates/workpot-core/src/services/catalog.rs:127-149`  
**Issue:** The DB row is deleted via autocommit `execute`, then `save_config` writes excludes. If `save_config` fails (disk full, permissions, validation), the repo is gone but exclude globs are not persisted — a subsequent `workpot index` can rediscover the path. In-memory `config.excludes` is mutated even when the file write fails.  
**Fix:** Persist config first, or wrap both in a SQLite transaction plus atomic config write; on config failure, do not DELETE (or roll back).

## Info

### IN-01: `catalog::remove_repo` is dead code

**File:** `crates/workpot-core/src/services/catalog.rs:102-113`  
**Issue:** `AppContext::remove_repo` delegates to `remove_repo_with_exclude`; nothing calls the plain `remove_repo` function. Dead surface adds maintenance noise.  
**Fix:** Remove `remove_repo` or make `remove_repo_with_exclude` call it for the DELETE half after exclude logic is extracted.

### IN-02: `register_manual` silently stores empty `git_common_dir` when git2 open fails

**File:** `crates/workpot-core/src/services/catalog.rs:43-45`  
**Issue:** `resolve_git_common_dir` errors are swallowed with `unwrap_or_default()`. Filesystem-valid but git2-unopenable paths (minimal test fixtures, corrupt repos) register with `git_common_dir = ""`, weakening Phase 2 dedup/merge keyed on common dir.  
**Fix:** Log a warning at minimum; optionally surface a soft warning in CLI output, or fail registration when git2 cannot resolve common dir for non-bare repos.

---

_Reviewed: 2026-05-30T12:00:00Z_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
