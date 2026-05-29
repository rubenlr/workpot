# Phase 3: Git state - Research

**Researched:** 2026-05-29
**Domain:** git2 (libgit2 Rust bindings), rayon parallel processing, SQLite migration
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**git2 adoption**
- D-01: Add `git2` crate with `vendored-libgit2` feature to `workpot-core` — no system libgit2 required, hermetic CI build.
- D-02: Migrate ALL existing subprocess calls in `crates/workpot-core/src/infra/git.rs` to git2 equivalents — no `Command::new("git")` remains in the core crate after Phase 3.
- D-03: Use `rayon` for parallel git2 repository opens across all indexed repos — required to meet success criterion 4 (<500ms for 50+ repos).
- D-04: Repos with no configured upstream → omit ahead/behind from display (store `ahead = NULL`, `behind = NULL`); do not show a "—" placeholder.

**Schema: git state columns on repos table**
- D-05: Add new columns to `repos` via a migration (`003_git_state.sql`): `branch TEXT`, `is_dirty INTEGER`, `ahead INTEGER NULL`, `behind INTEGER NULL`, `git_refreshed_at INTEGER NULL`, `git_state_error TEXT NULL`.
- D-06: Repos where `git_refreshed_at IS NULL` (never refreshed) → display `?` for branch/dirty/ahead-behind in list output.
- D-07: `workpot repo list` shows a staleness age indicator alongside git state (e.g. `branch=main dirty=yes  5m ago`).
- D-08: Stale path removal (Phase 2 D-15) unchanged — row is deleted entirely; no tombstone state.
- D-09: `git_state_error TEXT NULL` captures the last failure reason when a repo fails to refresh; surfaced in list output for that repo.

**Dirty detection scope**
- D-10: `is_dirty` = true iff the repo has staged or unstaged changes to tracked files (git2 `INDEX_*` + `WT_MODIFIED` flags on tracked paths). Untracked files are excluded.
- D-11: Repos with only untracked files → show as clean (no secondary indicator). One binary `is_dirty` flag only.
- D-12: git2 status checks respect `.gitignore` (default git2 behavior — ignored files never contribute to dirty).
- D-13: Bare repos → skip dirty check entirely; store `is_dirty = NULL`, display `N/A` in list.
- D-14: Each worktree path row gets its own `is_dirty` check independently (aligns with Phase 2 per-path indexing).

**Refresh trigger**
- D-15: Git state refresh is piggybacked on `workpot index`: discovery walk + DB merge runs first (existing behavior), then git2 parallel refresh of all indexed repos runs as a second pass in the same command.
- D-16: Git refresh continues on individual repo failure — store error in `git_state_error`, refresh remaining repos (do not abort the batch).
- D-17: `workpot index` output includes git refresh stats: e.g. `42 added, 0 removed / git: 47 refreshed, 2 errors`.
- D-18: Phase 3 ships a `refresh_git_state(path: &Path) -> Result<GitState>` function in `workpot-core` for Phase 4 tray to call on a single repo without running a full index.

### Claude's Discretion
- Exact rayon thread pool sizing (default rayon pool is fine for this use case).
- git2 `StatusOptions` bitfield composition (implement using `INCLUDE_UNTRACKED=false`, `RECURSE_UNTRACKED_DIRS=false`, `EXCLUDE_SUBMODULES=true`).
- Exact migration file name and number.
- Output format details for the age indicator (relative time string).
- Whether `list_worktree_paths` uses git2 `worktrees()` API or the parsed porcelain format.

### Deferred Ideas (OUT OF SCOPE)
- Separate `workpot git refresh` / `workpot git sync` standalone command.
- Filesystem watcher (`notify`) for automatic git state re-index — Phase 9 / post-v1.
- Ahead/behind with "—" placeholder for no-upstream repos.
- Structured git stats in index history table (`index_runs`).
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| GIT-01 | User sees current branch name per indexed repository | git2 `repo.head()?.shorthand()` + detached HEAD handling (store short OID hash) |
| GIT-02 | User sees whether the working tree is dirty per repository | git2 `repo.statuses()` with `include_untracked(false)` + INDEX_* + WT_MODIFIED flags; bare repos store NULL |
| GIT-03 | User sees ahead/behind counts relative to upstream when configured | git2 `repo.graph_ahead_behind(local_oid, upstream_oid)`; upstream() returning Err → store NULL (D-04) |
| GIT-04 | Git state refreshes without blocking the tray UI | rayon `par_iter` over repo paths; each path opens its own `Repository`; `Repository: Send` is confirmed |
</phase_requirements>

---

## Summary

Phase 3 replaces the two existing `Command::new("git")` subprocess functions in `infra/git.rs` with git2 bindings, and adds a new parallel batch refresh service that populates six new columns on the `repos` table. The core data flow is: `workpot index` calls discovery (existing Phase 2 pass), then calls a new git-refresh second pass that opens every indexed repo path via `git2::Repository::open`, queries branch/dirty/ahead-behind, and writes results back to SQLite. Individual repo failures are captured per-row and never abort the batch.

The git2 crate (v0.21.0) is the only new external dependency. rayon (v1.12.0) is added to enable the parallel open-and-query pattern. `humantime` (v2.3.0) is used for the staleness age indicator in CLI output. `Repository` implements `unsafe impl Send`, making it safe to open one per rayon thread. The critical threading constraint is that a `Repository` instance is NOT `Sync` — each rayon thread must open its own `Repository` from the path; instances cannot be shared across threads.

The performance target (500ms for 50+ repos) is achievable: each `Repository::open` + status + ahead/behind is typically 2–10ms on local SSD; rayon with the default thread pool parallelizes across cores, giving an expected wall-clock time of 20–50ms for 50 repos with a 4-core machine.

**Primary recommendation:** Add `git2 = { version = "0.21", features = ["vendored-libgit2"] }` and `rayon = "1"` to workpot-core's `[dependencies]`. Implement `refresh_git_state(path) -> Result<GitState>` as the single-repo primitive, then build the batch pass in `services/git_state.rs` using `paths.par_iter().map(|p| (p, refresh_git_state(p))).collect()`.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Git state read (branch/dirty/ahead-behind) | workpot-core library | — | Single source of truth; Phase 4 tray and CLI both call same function |
| Parallel batch refresh | workpot-core service | — | Lives in services/git_state.rs; orchestrated by index.rs second pass |
| Schema migration | workpot-core infra | — | 003_git_state.sql adds columns to repos table |
| CLI output (list + index stats) | workpot-cli | workpot-core (data) | CLI formats data; core provides structs |
| Subprocess elimination | workpot-core infra | — | infra/git.rs rewritten; no Command::new("git") after Phase 3 |

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| git2 | 0.21.0 | libgit2 Rust bindings — branch, status, ahead/behind | Official Rust binding to libgit2; threadsafe; no system libgit2 required with vendored feature |
| rayon | 1.12.0 | Data parallelism for batch repo refresh | Industry-standard work-stealing parallel iterator; zero unsafe code in calling code |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| humantime | 2.3.0 | Format `git_refreshed_at` as "5m ago" for CLI | Age indicator on `workpot repo list` (D-07); no chrono needed for formatting only |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| git2 vendored-libgit2 | system libgit2 (default) | System libgit2 may not be present or may be wrong version on CI; vendored is hermetic |
| rayon par_iter | tokio spawn_blocking | rayon is synchronous CPU-parallel; tokio adds async complexity not needed here |
| humantime | chrono | chrono is heavier; humantime sufficient for relative time display only |

**Installation:**
```bash
# In crates/workpot-core/Cargo.toml [dependencies]:
git2 = { version = "0.21", features = ["vendored-libgit2"] }
rayon = "1"
humantime = "2"
```

**Version verification:**
```
git2: 0.21.0 [VERIFIED: cargo search git2]
rayon: 1.12.0 [VERIFIED: cargo search rayon]
humantime: 2.3.0 [VERIFIED: cargo search humantime]
```

---

## Package Legitimacy Audit

> slopcheck was not installable in this environment. All packages below are tagged `[ASSUMED]` per graceful degradation policy. The planner must gate each install behind a `checkpoint:human-verify` task before adding to Cargo.toml.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| git2 | crates.io | ~10 yrs | Very high (cargo itself uses it) | github.com/rust-lang/git2-rs | [ASSUMED] | Approved — official Rust org maintained |
| rayon | crates.io | ~9 yrs | Very high (rayon-rs org) | github.com/rayon-rs/rayon | [ASSUMED] | Approved — official rayon-rs org maintained |
| humantime | crates.io | ~7 yrs | High (systemd, cargo ecosystem) | github.com/tailhook/humantime | [ASSUMED] | Approved — stable, widely used |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

*slopcheck was unavailable at research time — all packages tagged `[ASSUMED]`. The planner must add a `checkpoint:human-verify` before each `cargo add` task. Note: git2 and rayon are maintained by the official Rust organization and rayon-rs org respectively — risk is effectively zero, but protocol applies.*

---

## Architecture Patterns

### System Architecture Diagram

```
workpot index
    │
    ├── Pass 1: Discovery (existing Phase 2 behavior)
    │   └── scan_root → upsert repos → delete stale → commit tx
    │
    └── Pass 2: Git refresh (new Phase 3)
        │
        ├── SELECT path FROM repos WHERE excluded = 0
        │
        ├── paths.par_iter()  [rayon — opens one Repository per thread]
        │   │
        │   ├── Repository::open(path)?
        │   │   ├── is_bare() → skip dirty; store branch if readable
        │   │   │
        │   │   ├── head()? → shorthand() → branch name
        │   │   │   └── detached HEAD → short OID hex (7 chars)
        │   │   │
        │   │   ├── statuses(opts)? → any INDEX_* or WT_MODIFIED → is_dirty
        │   │   │   └── opts: include_untracked=false, exclude_submodules=true
        │   │   │
        │   │   └── head_branch.upstream()? → graph_ahead_behind(local, upstream)
        │   │       └── Err (no upstream) → ahead=NULL, behind=NULL
        │   │
        │   └── Err → GitState { error: Some(msg), rest: NULL }
        │
        └── Vec<(path, GitState)> → batch UPDATE repos SET branch=?, is_dirty=?, ...
            └── git_refreshed_at = now()
```

### Recommended Project Structure
```
crates/workpot-core/src/
├── domain/
│   ├── repo.rs          # RepoRecord: add branch, is_dirty, ahead, behind, git_refreshed_at, git_state_error
│   └── git_state.rs     # NEW: GitState struct (branch, is_dirty, ahead, behind, error)
├── infra/
│   ├── git.rs           # REWRITE: subprocess → git2; resolve_git_common_dir, list_worktree_paths
│   └── migrations/
│       └── 003_git_state.sql  # NEW: ALTER TABLE repos ADD COLUMN ...
├── services/
│   ├── git_state.rs     # NEW: refresh_git_state(), refresh_all_git_states()
│   └── index.rs         # EXTEND: run_full() calls git second pass; IndexSummary adds git_refreshed/git_errors
```

### Pattern 1: Single-Repo Git State Refresh
**What:** Open a repo by path and read branch, dirty flag, and ahead/behind.
**When to use:** Called per-repo in the rayon parallel pass, and exposed as public API for Phase 4.

```rust
// Source: git2 docs.rs/git2/0.21.0 + examples/status.rs
use git2::{Repository, StatusOptions, Status};
use std::path::Path;

pub struct GitState {
    pub branch: Option<String>,     // None = detached HEAD (store short OID)
    pub is_dirty: Option<bool>,     // None = bare repo
    pub ahead: Option<i64>,         // None = no upstream
    pub behind: Option<i64>,        // None = no upstream
    pub error: Option<String>,      // per-repo failure message
}

pub fn refresh_git_state(path: &Path) -> Result<GitState, git2::Error> {
    let repo = Repository::open(path)?;

    // Bare repos: skip dirty detection (D-13)
    if repo.is_bare() {
        let branch = head_name(&repo).ok();
        return Ok(GitState { branch, is_dirty: None, ahead: None, behind: None, error: None });
    }

    let branch = head_name(&repo).ok();
    let is_dirty = Some(detect_dirty(&repo)?);
    let (ahead, behind) = detect_ahead_behind(&repo).unwrap_or((None, None));

    Ok(GitState { branch, is_dirty, ahead, behind, error: None })
}

fn head_name(repo: &Repository) -> Result<String, git2::Error> {
    let head = repo.head()?;
    if head.is_branch() {
        Ok(head.shorthand().unwrap_or("HEAD").to_string())
    } else {
        // Detached HEAD: store short OID
        let oid = head.target().ok_or(git2::Error::from_str("no HEAD target"))?;
        Ok(oid.to_string()[..7].to_string())
    }
}

fn detect_dirty(repo: &Repository) -> Result<bool, git2::Error> {
    // D-10: staged + unstaged tracked changes only; no untracked, no ignored
    let mut opts = StatusOptions::new();
    opts.include_untracked(false)
        .recurse_untracked_dirs(false)
        .exclude_submodules(true);
    let statuses = repo.statuses(Some(&mut opts))?;
    let dirty_flags = Status::INDEX_NEW
        | Status::INDEX_MODIFIED
        | Status::INDEX_DELETED
        | Status::INDEX_RENAMED
        | Status::INDEX_TYPECHANGE
        | Status::WT_MODIFIED
        | Status::WT_DELETED
        | Status::WT_RENAMED
        | Status::WT_TYPECHANGE;
    Ok(statuses.iter().any(|e| e.status().intersects(dirty_flags)))
}

fn detect_ahead_behind(repo: &Repository) -> Result<(Option<i64>, Option<i64>), git2::Error> {
    let head = repo.head()?;
    if !head.is_branch() {
        return Ok((None, None)); // detached HEAD has no upstream
    }
    let branch_name = head.shorthand().unwrap_or("");
    let branch = repo.find_branch(branch_name, git2::BranchType::Local)?;
    let upstream = match branch.upstream() {
        Ok(u) => u,
        Err(_) => return Ok((None, None)), // no upstream configured (D-04)
    };
    let local_oid = head.target().ok_or(git2::Error::from_str("no local OID"))?;
    let upstream_oid = upstream.get().target()
        .ok_or(git2::Error::from_str("no upstream OID"))?;
    let (ahead, behind) = repo.graph_ahead_behind(local_oid, upstream_oid)?;
    Ok((Some(ahead as i64), Some(behind as i64)))
}
```

### Pattern 2: Parallel Batch Refresh (rayon)
**What:** Refresh all indexed repo paths in parallel, collecting per-path results.
**When to use:** Second pass in `index::run_full`.

```rust
// Source: rayon docs.rs/rayon/1.12.0 + training knowledge
use rayon::prelude::*;
use std::path::PathBuf;

pub struct GitRefreshResult {
    pub path: String,
    pub state: GitState,
}

pub fn refresh_all(paths: Vec<PathBuf>) -> Vec<GitRefreshResult> {
    // Repository: Send (confirmed), NOT Sync — one per thread is correct
    paths
        .into_par_iter()
        .map(|path| {
            let state = refresh_git_state(&path)
                .unwrap_or_else(|e| GitState {
                    branch: None,
                    is_dirty: None,
                    ahead: None,
                    behind: None,
                    error: Some(e.message().to_string()),
                });
            GitRefreshResult {
                path: path.display().to_string(),
                state,
            }
        })
        .collect()
}
```

### Pattern 3: Subprocess Migration — resolve_git_common_dir
**What:** Replace `Command::new("git")` with git2.
**When to use:** Direct replacement in `infra/git.rs`.

```rust
// Source: git2 docs.rs/git2/0.21.0
use git2::Repository;
use std::path::{Path, PathBuf};

pub fn resolve_git_common_dir(path: &Path) -> Result<PathBuf> {
    let repo = Repository::open(path)
        .map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))?;
    let common = repo.commondir();
    std::fs::canonicalize(common)
        .map_err(|_| WorkpotError::GitUnavailable(path.to_path_buf()))
}
```

### Pattern 4: list_worktree_paths via git2 worktrees() API
**What:** Replace subprocess porcelain parsing with git2 native worktrees API.
**When to use:** Direct replacement in `infra/git.rs`.

```rust
// Source: git2 docs.rs/git2/0.21.0 Repository::worktrees()
pub fn list_worktree_paths(repo_path: &Path) -> Result<Vec<PathBuf>> {
    let repo = Repository::open(repo_path)
        .map_err(|_| WorkpotError::GitUnavailable(repo_path.to_path_buf()))?;
    let names = repo.worktrees()
        .map_err(|_| WorkpotError::GitUnavailable(repo_path.to_path_buf()))?;
    let mut paths = Vec::new();
    for name in names.iter().flatten() {
        if let Ok(wt) = repo.find_worktree(name) {
            if let Ok(path) = wt.path().canonicalize() {
                // Skip the main worktree (bare) — only linked worktrees
                if path != repo_path {
                    paths.push(path);
                }
            }
        }
    }
    Ok(paths)
}
```

### Pattern 5: IndexSummary Extended with Git Stats
```rust
#[derive(Debug, Clone, Default)]
pub struct IndexSummary {
    pub added: u32,
    pub removed: u32,
    pub skipped: u32,
    pub git_refreshed: u32,   // NEW
    pub git_errors: u32,      // NEW
}
// CLI output: "42 added, 0 removed / git: 47 refreshed, 2 errors"
```

### Pattern 6: Staleness Age Formatting
```rust
// Source: humantime 2.3.0 (cargo search humantime)
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn format_age(git_refreshed_at: i64) -> String {
    let refreshed = UNIX_EPOCH + Duration::from_secs(git_refreshed_at as u64);
    let elapsed = SystemTime::now()
        .duration_since(refreshed)
        .unwrap_or_default();
    humantime::format_duration(Duration::from_secs(elapsed.as_secs())).to_string()
    // Output: "5m", "2h 3m", "1d 4h"
}
```

### Anti-Patterns to Avoid

- **Sharing a Repository across threads:** `Repository` is `Send` but NOT `Sync`. Never wrap in `Arc`; each rayon thread opens its own instance from the path.
- **Using `collect::<Result<Vec<_>,_>>()`  on the parallel refresh:** This aborts on first error. The design requires continuing on failure (D-16). Always collect to `Vec<(path, GitState)>` and embed the error in GitState.
- **Calling `include_untracked(true)` in dirty check:** This makes every repo with new files appear dirty. Per D-10, only tracked-file changes count.
- **Calling `include_ignored(true)` in dirty check:** Build artifacts and ignored files must not pollute dirty flag. Default behavior excludes them — do not override.
- **Truncating error messages before storing:** Store the full `git2::Error::message()` string in `git_state_error`; truncate only at display time if needed.
- **Falling back to subprocess if git2 fails:** D-02 prohibits `Command::new("git")` anywhere in core. If git2 fails, store the error and continue; do not shell out.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Ahead/behind calculation | Custom commit-walk counting | `repo.graph_ahead_behind()` | libgit2 handles merge-base finding correctly; manual commit walk mis-counts on non-linear history |
| Dirty detection | Custom index diff | `repo.statuses()` with flags | libgit2 handles stat-cache, symlinks, case-insensitive FS, submodules, gitignore — none of which a hand-rolled diff handles correctly |
| Parallel work stealing | Custom thread pool | rayon `par_iter` | rayon handles work-stealing, panic propagation, thread count tuning automatically |
| Age formatting | Custom duration → string | `humantime::format_duration` | Edge cases: singular/plural, correct unit selection (don't say "60 minutes", say "1h") |
| subprocess → git2 migration for commondir | Keep both code paths | Replace entirely (D-02) | Having both paths creates test surface ambiguity and CI uncertainty about which is exercised |

**Key insight:** git2's status API has 7+ years of edge-case fixes for platform-specific path handling, CRLF, symlinks, and submodule boundary conditions. Any hand-rolled dirty check will miss these.

---

## Common Pitfalls

### Pitfall 1: `Repository` is Send, not Sync
**What goes wrong:** Wrapping one `Repository` in `Arc<Mutex<_>>` and sharing across rayon threads causes deadlocks or panics because the mutex can't guarantee libgit2's internal assumptions.
**Why it happens:** Send means "safe to send to another thread"; Sync means "safe to share a reference across threads." libgit2 is documented as thread-safe when each thread has its own handle.
**How to avoid:** Each rayon thread calls `Repository::open(path)` independently. Do not try to reuse a single Repository instance.
**Warning signs:** Seeing `unsafe impl Sync` attempts; wrapping in `Arc`; using `Mutex<Repository>` across rayon threads.

### Pitfall 2: Detached HEAD not handled
**What goes wrong:** `repo.head()?.shorthand()` returns `"HEAD"` (the ref name), not the branch name, when detached.
**Why it happens:** `shorthand()` on the HEAD reference when not pointing to a branch returns the literal string "HEAD".
**How to avoid:** Check `head.is_branch()` first. If false, use `head.target()` to get the OID and format the first 7 hex chars as the "branch" display value.
**Warning signs:** All detached-HEAD repos showing `branch = "HEAD"` in list output.

### Pitfall 3: Unborn branch (empty repo, no commits)
**What goes wrong:** `repo.head()` returns `Err(ErrorCode::UnbornBranch)` on a freshly initialized repo with no commits.
**Why it happens:** HEAD points to `refs/heads/main` which doesn't exist yet (no commits).
**How to avoid:** Match on the git2 error code `ErrorCode::UnbornBranch` and store `branch = "unborn"` (or `None`). Do not panic or propagate as a fatal error.
**Warning signs:** Test repos created with `git init` but no commits failing the whole batch refresh.

### Pitfall 4: ahead/behind reversed
**What goes wrong:** `graph_ahead_behind(local, upstream)` returns `(behind, ahead)` — the argument order maps to `(commits_in_local_not_in_upstream, commits_in_upstream_not_in_local)` which is `(ahead, behind)`.
**Why it happens:** libgit2 documentation is confusing; a known upstream issue (#2501) documents this was historically ambiguous.
**How to avoid:** The return is `(ahead, behind)` where ahead = commits local has that upstream doesn't. Verify with a test: create one local commit past origin → `ahead=1, behind=0`.
**Warning signs:** Ahead and behind counts appearing swapped vs `git status` output.

### Pitfall 5: `vendored-libgit2` vs `bundled` feature name
**What goes wrong:** The CLAUDE.md recommended stack mentions `bundled` feature, but the correct feature name in git2 0.19+ is `vendored-libgit2`.
**Why it happens:** Old documentation or other crates (rusqlite) use the name `bundled`; git2 uses `vendored-libgit2`.
**How to avoid:** Use `git2 = { version = "0.21", features = ["vendored-libgit2"] }`. [VERIFIED: github.com/rust-lang/git2-rs README]
**Warning signs:** Build failing with "feature `bundled` not found in crate git2".

### Pitfall 6: Git refresh not using a transaction
**What goes wrong:** If the process crashes mid-batch, some repos have updated git state and others don't, creating inconsistent data.
**Why it happens:** Individual `UPDATE` statements outside a transaction are committed one-by-one.
**How to avoid:** Wrap all batch UPDATE statements in a single SQLite transaction. Either use the existing `unchecked_transaction` pattern from `index.rs`, or collect all results first and write in one transactional batch.
**Warning signs:** Partial refresh visible in `repo list` with some repos having `git_refreshed_at` and others NULL after an interrupted `workpot index`.

### Pitfall 7: `worktrees()` returns only linked worktrees, not the main one
**What goes wrong:** `repo.worktrees()` returns the *names* of linked worktrees only; the main worktree is not included.
**Why it happens:** libgit2 `git_worktree_list` lists linked worktrees by name; the main worktree path is the repo path itself.
**How to avoid:** `list_worktree_paths` should use `repo.worktrees()` for linked worktrees and separately include `repo_path` as the main worktree if the repo is bare (Phase 2 D-04 semantics).
**Warning signs:** Bare repo rows missing; `list_worktree_paths` returning empty Vec for bare repos with linked worktrees.

---

## Code Examples

### Migration 003_git_state.sql
```sql
-- Source: existing migration pattern from 002_discovery.sql
ALTER TABLE repos ADD COLUMN branch TEXT;
ALTER TABLE repos ADD COLUMN is_dirty INTEGER;
ALTER TABLE repos ADD COLUMN ahead INTEGER;
ALTER TABLE repos ADD COLUMN behind INTEGER;
ALTER TABLE repos ADD COLUMN git_refreshed_at INTEGER;
ALTER TABLE repos ADD COLUMN git_state_error TEXT;
```

### Extending migrations.rs
```rust
// Source: existing infra/migrations.rs pattern
static MIGRATION_003: &str = include_str!("migrations/003_git_state.sql");
let steps = [M::up(MIGRATION_001), M::up(MIGRATION_002), M::up(MIGRATION_003)];
```

### Batch UPDATE after parallel refresh
```rust
// Collect all results first (outside any DB borrow), then write in one transaction
let results = refresh_all(paths); // Vec<GitRefreshResult>
let tx = conn.unchecked_transaction()?;
for r in &results {
    tx.execute(
        "UPDATE repos SET branch=?1, is_dirty=?2, ahead=?3, behind=?4,
                          git_refreshed_at=?5, git_state_error=?6
         WHERE path=?7",
        rusqlite::params![
            r.state.branch, r.state.is_dirty.map(|b| b as i64),
            r.state.ahead, r.state.behind,
            now_secs(), r.state.error, r.path
        ],
    )?;
}
tx.commit()?;
```

### repo list output with git state (D-07)
```
myapp          /Users/rub/code/myapp          main  dirty  ↑2↓1  5m ago
workpot        /Users/rub/code/workpot        feature/repo-discovery  clean  4h ago
bare-mono.git  /Users/rub/code/bare-mono.git  main  N/A   2d ago
old-thing      /Users/rub/code/old-thing      ?                            (never refreshed)
broken-repo    /Users/rub/code/broken-repo    ERROR: not a git repository
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| subprocess `git -C path rev-parse --git-common-dir` | `git2::Repository::open(path).commondir()` | Phase 3 | Eliminates process spawn overhead; works in sandboxed envs |
| subprocess `git -C path worktree list --porcelain` | `git2::Repository::worktrees()` | Phase 3 | Native API; no text parsing; handles edge cases correctly |
| `Command::new("git")` subprocess for git state | git2 direct | Phase 3 | D-02; needed for Phase 4 tray (no subprocess in async context) |

**Deprecated/outdated:**
- `Command::new("git")` in `infra/git.rs`: prohibited after Phase 3 (D-02); replaced entirely by git2 equivalents.
- The `bundled` feature name for git2: the correct name is `vendored-libgit2` per current git2-rs README.

---

## Runtime State Inventory

> Not applicable — this is not a rename/refactor/migration phase. Phase 3 adds new columns (no existing column renamed) and new code. Existing DB rows remain valid; new columns default to NULL.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `git2::Repository` implements `unsafe impl Send {}` (confirmed via WebSearch citing source code) | Architecture Patterns, Pitfall 1 | If wrong, rayon par_iter would not compile; would need Arc<Mutex> or spawn_blocking |
| A2 | The correct Cargo feature flag for hermetic git2 build is `vendored-libgit2` (not `bundled`) | Standard Stack, Pitfall 5 | Build fails with feature not found error |
| A3 | `repo.worktrees()` returns only linked worktrees, not the main worktree | Pattern 4 | list_worktree_paths would either duplicate or miss the main worktree |
| A4 | humantime 2.3.0 `format_duration` produces short human-readable strings ("5m", "2h") | Pattern 6 | Output format may differ; cosmetic issue only |
| A5 | `graph_ahead_behind(local, upstream)` return order is (ahead, behind) — not (behind, ahead) | Pattern 1 | Counts displayed swapped; affects user trust |

**Notes on A1:** Multiple WebSearch results citing the git2-rs source code confirmed `unsafe impl Send for Repository {}` with comment "It is the current belief that a Repository can be sent among threads." [CITED: github.com/rust-lang/git2-rs/blob/master/src/repo.rs]

**Notes on A2:** git2-rs README explicitly lists `vendored-libgit2` as the static-link feature. [CITED: github.com/rust-lang/git2-rs README]

---

## Open Questions

1. **Does `repo.commondir()` exist as a method, or is it `path()` + `git_dir()`?**
   - What we know: The C libgit2 API has `git_repository_commondir()`; the Rust binding should expose it.
   - What's unclear: Exact method name on `Repository` in git2 0.21.0 (docs.rs page cut off).
   - Recommendation: Implementer should check `Repository::commondir()` at compile time; fallback is `repo.path()` (which for worktrees returns the `.git/worktrees/N` path, not the common dir — must test).

2. **`repo.worktrees()` return type is `StringArray` (names), not `Vec<PathBuf>`**
   - What we know: The API returns names, and you call `repo.find_worktree(name)?.path()` for each.
   - What's unclear: Whether `path()` on `Worktree` returns an absolute path or relative.
   - Recommendation: Test with `Repository::init` on a tempdir + add a linked worktree; verify path is absolute or canonicalize defensively.

3. **Batch UPDATE strategy: single transaction or per-row auto-commit?**
   - What we know: Existing index.rs uses `unchecked_transaction`; this pattern works.
   - What's unclear: Whether `rayon::collect` result set can be large enough to hit SQLite lock timeout.
   - Recommendation: Collect all rayon results into a Vec first (outside any DB connection borrow), then write in a single transaction. This is both safe (no DB borrow across thread boundary) and fast.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | Building git2 with vendored-libgit2 | ✓ | rustc 1.85+ (workspace.rust-version) | — |
| C compiler (cc/clang) | vendored-libgit2 build (compiles C source) | ✓ | Xcode CLT on macOS | — |
| cargo-nextest | Fast test runner (CLAUDE.md dev tool) | Not checked | — | `cargo test` |
| libgit2 (system) | NOT required | N/A | — | vendored-libgit2 feature provides it |

**Missing dependencies with no fallback:** None — vendored-libgit2 compiles libgit2 from source; only requires a C compiler which is present on macOS (Xcode CLT).

**Note on compile time:** vendored-libgit2 adds ~15–30s to a clean build (compiles libgit2 C source). Incremental rebuilds are fast. This is acceptable for CI.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test + integration tests |
| Config file | none (uses cargo test) |
| Quick run command | `cargo test --package workpot-core` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| GIT-01 | `refresh_git_state` returns correct branch name for a normal repo | unit | `cargo test --package workpot-core git_state` | ❌ Wave 0 — new file |
| GIT-01 | Detached HEAD returns short OID (7 chars), not "HEAD" | unit | `cargo test --package workpot-core detached_head` | ❌ Wave 0 |
| GIT-01 | Unborn branch (empty repo) returns gracefully (no panic) | unit | `cargo test --package workpot-core unborn_branch` | ❌ Wave 0 |
| GIT-02 | Unstaged tracked modification detected as dirty | unit | `cargo test --package workpot-core dirty_unstaged` | ❌ Wave 0 |
| GIT-02 | Staged change detected as dirty | unit | `cargo test --package workpot-core dirty_staged` | ❌ Wave 0 |
| GIT-02 | Untracked-only repo shows clean (D-10) | unit | `cargo test --package workpot-core untracked_is_clean` | ❌ Wave 0 |
| GIT-02 | Bare repo returns `is_dirty = None` (D-13) | unit | `cargo test --package workpot-core bare_no_dirty` | ❌ Wave 0 |
| GIT-03 | Repo with upstream shows correct ahead=1, behind=0 | unit | `cargo test --package workpot-core ahead_behind` | ❌ Wave 0 |
| GIT-03 | Repo without upstream returns ahead=None, behind=None (D-04) | unit | `cargo test --package workpot-core no_upstream` | ❌ Wave 0 |
| GIT-04 | `refresh_all` on 50 repos completes in < 500ms | integration | `cargo test --package workpot-core refresh_50_repos -- --ignored` | ❌ Wave 0 |
| D-02 | No `Command::new("git")` in workpot-core after migration | static audit | `grep -r "Command::new" crates/workpot-core/src --include="*.rs"` | ✓ (manual check) |
| D-15 | `workpot index` output shows "git: N refreshed, M errors" | integration | `cargo test --package workpot-cli index_git_stats` | ❌ Wave 0 |

**Note on GIT-04 performance test:** The `--ignored` flag marks it as opt-in. It requires creating 50 real git repos via `git2::Repository::init`, which is slow in test environments. Mark `#[ignore]` and run explicitly to avoid slowing normal `cargo test`.

### Sampling Rate
- **Per task commit:** `cargo test --package workpot-core`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/workpot-core/tests/git_state_test.rs` — covers GIT-01, GIT-02, GIT-03 (branch, dirty, ahead/behind, bare, unborn, detached)
- [ ] `crates/workpot-core/tests/git_state_perf_test.rs` — covers GIT-04 (50-repo timing, `#[ignore]`)
- [ ] `crates/workpot-core/src/domain/git_state.rs` — `GitState` struct definition
- [ ] `crates/workpot-core/src/services/git_state.rs` — `refresh_git_state` and `refresh_all` functions

*(If no gaps: "None — existing test infrastructure covers all phase requirements")*

---

## Security Domain

> `security_enforcement: true` in `.planning/config.json`. ASVS level 1 applies.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Phase 3 is local CLI only; no auth surface |
| V3 Session Management | no | No sessions |
| V4 Access Control | no | All data is local user's own repos |
| V5 Input Validation | yes | Repo paths from DB must be canonicalized before `Repository::open()` to prevent path traversal |
| V6 Cryptography | no | No secrets or encryption |

### Known Threat Patterns for git2 + local filesystem

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal via crafted repo path | Tampering | Always canonicalize paths before `Repository::open`; existing pattern from Phase 1 D-13 |
| Symlink following into sensitive dirs | Information Disclosure | git2 `Repository::open` follows symlinks; paths are always canonicalized from DB (stored canonical), so risk is low unless a watch root itself is a malicious symlink |
| Resource exhaustion (git LFS pointer files, huge index) | Denial of Service | StatusOptions with `exclude_submodules(true)` reduces scope; no per-file content read; risk is low for local use |
| git2 C library CVEs | Tampering | `vendored-libgit2` pins to a specific version; monitor libgit2 advisories; `cargo-deny` enforces advisory gate (CLAUDE.md dev tool) |

**Note:** Phase 3 is read-only git access. No writes to any git repo occur. Attack surface is minimal.

---

## Project Constraints (from CLAUDE.md)

| Directive | Impact on Phase 3 |
|-----------|------------------|
| macOS-only for v1 | No cross-platform concerns; `vendored-libgit2` builds cleanly with Xcode CLT |
| Local-only — no network calls | git2 is used read-only; no `Repository::clone` or remote fetch; vendored-libgit2 links SSH/HTTPS features only if enabled (they are not needed here — do not add `https` or `ssh` features) |
| Cursor launch integration required in v1 | Not relevant to Phase 3 (data layer only) |
| Rust 1.85+ (2024 edition) | git2 0.21 is compatible |
| cargo-nextest for CI | Use `cargo nextest run` in CI; `cargo test` locally |
| cargo-deny for license/advisory gate | Add git2, rayon, humantime to `deny.toml` allowlist before CI |
| No git2 until Phase 3 (Phase 1 D-08) | Phase 3 is the designated phase for git2 adoption — this is correct |
| No `Command::new("git")` in core after Phase 3 | D-02; verified by grep check in tests |

---

## Sources

### Primary (HIGH confidence)
- [git2 crate docs.rs 0.21.0](https://docs.rs/git2/0.21.0/git2/) — Repository methods: open, head, statuses, graph_ahead_behind, worktrees, is_bare
- [git2-rs examples/status.rs](https://github.com/rust-lang/git2-rs/blob/master/examples/status.rs) — StatusOptions pattern, branch name from HEAD, Status flags
- [git2-rs README.md](https://github.com/rust-lang/git2-rs/blob/master/README.md) — vendored-libgit2 feature flag documentation
- [rayon docs.rs 1.12.0](https://docs.rs/rayon/latest/rayon/) — par_iter, into_par_iter, collect
- [humantime docs.rs 2.3.0](https://docs.rs/humantime/latest/humantime/) — format_duration
- [cargo search git2](https://crates.io/crates/git2) — version 0.21.0 confirmed
- [cargo search rayon](https://crates.io/crates/rayon) — version 1.12.0 confirmed
- [cargo search humantime](https://crates.io/crates/humantime) — version 2.3.0 confirmed

### Secondary (MEDIUM confidence)
- [git2-rs issue #194](https://github.com/rust-lang/git2-rs/issues/194) — `unsafe impl Send for Repository` confirmed via WebSearch citing source code
- [libgit2 graph_ahead_behind docs](https://libgit2.org/docs/reference/main/graph/git_graph_ahead_behind.html) — API semantics for (ahead, behind) return order
- [WebSearch: Repository Send confirmation](https://github.com/rust-lang/git2-rs/blob/master/src/repo.rs) — comment in source: "It is the current belief that a Repository can be sent among threads"

### Tertiary (LOW confidence)
- Training knowledge for rayon parallel collect + error partitioning pattern (standard Rust idiom, not verified via specific source)

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — versions confirmed via cargo search; git2 is the established libgit2 Rust binding
- Architecture: HIGH — all patterns derived from official git2 examples and docs
- Pitfalls: HIGH — vendored feature name from README; Repository Send from source; detached HEAD from official example
- Performance claim (500ms for 50 repos): MEDIUM — derived from typical SSD latency + rayon parallelism reasoning; no benchmark run in this session

**Research date:** 2026-05-29
**Valid until:** 2026-08-29 (90 days — git2 and rayon are stable, slow-moving crates)
