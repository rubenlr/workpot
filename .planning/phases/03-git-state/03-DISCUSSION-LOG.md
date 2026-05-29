# Phase 3: Git state - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-29
**Phase:** 3-Git state
**Areas discussed:** git2 vs subprocess, State persistence, Dirty detection scope, Refresh trigger

---

## git2 vs subprocess

| Option | Description | Selected |
|--------|-------------|----------|
| Add git2 (Recommended) | Use git2/libgit2 for all git reads; eliminates fork overhead for 50+ repos | ✓ |
| Extend subprocess | Run git commands via shell per repo; simpler, no C dep | |
| Mixed: git2 for state, subprocess for discovery | Add git2 only for new state reads; keep existing subprocess for rev-parse/worktree | |

**User's choice:** Add git2 (full adoption, bundled)

| Option | Description | Selected |
|--------|-------------|----------|
| Migrate all git calls to git2 | Replace subprocess rev-parse + worktree list with git2 equivalents | ✓ |
| Phase 3 adds git2 for new state reads only | Keep existing subprocess calls; add git2 only for branch/dirty/ahead-behind | |
| You decide | Claude picks based on risk | |

**User's choice:** Migrate all — full replacement of subprocess in git.rs

| Option | Description | Selected |
|--------|-------------|----------|
| Bundled (Recommended) | Hermetic build; no system libgit2 required | ✓ |
| System libgit2 | Smaller binary; requires brew install | |
| You decide | | |

**User's choice:** Bundled

| Option | Description | Selected |
|--------|-------------|----------|
| Show None / omit ahead-behind | No upstream configured → omit field entirely | ✓ |
| Show — placeholder | Explicit dash for unconfigured upstream | |
| You decide | | |

**User's choice:** Omit ahead-behind when no upstream configured (store NULL)

| Option | Description | Selected |
|--------|-------------|----------|
| Parallel with rayon (Recommended) | Open N repo handles concurrently; fits 50+ repo success criterion | ✓ |
| Sequential | Simpler; won't scale past ~20 repos | |
| You decide | | |

**User's choice:** Parallel with rayon

---

## State persistence

| Option | Description | Selected |
|--------|-------------|----------|
| Persist in DB (Recommended) | Columns on repos table with refreshed_at timestamp | ✓ |
| Compute on demand | No schema change; 50+ git2 opens per list call | |
| Separate git_state table | New table with FK; adds JOIN complexity | |

**User's choice:** Persist in DB

| Option | Description | Selected |
|--------|-------------|----------|
| Columns on repos table (Recommended) | ALTER TABLE repos ADD COLUMN branch, is_dirty, etc. | ✓ |
| Separate git_state table | Cleaner schema separation; more migration work | |

**User's choice:** Columns on repos table

| Option | Description | Selected |
|--------|-------------|----------|
| Show '?' placeholder | Honest about unrefreshed state | ✓ |
| Trigger refresh inline before listing | Always fresh on list; slower first call | |
| You decide | | |

**User's choice:** Show '?' for NULL git_refreshed_at

| Option | Description | Selected |
|--------|-------------|----------|
| Show age indicator (e.g. '5m ago') | User knows data staleness | ✓ |
| Raw values only | Cleaner output; staleness implicit | |
| You decide | | |

**User's choice:** Show age indicator alongside git state in list

| Option | Description | Selected |
|--------|-------------|----------|
| Remove the row entirely (current behavior) | Phase 2 D-15 unchanged | ✓ |
| Keep row with status=missing | Tombstone state; more complex to query | |
| You decide | | |

**User's choice:** Remove row (Phase 2 behavior unchanged)

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — store last error string | git_state_error TEXT NULL column | ✓ |
| No — skip silently (log to stderr) | Keep schema simple | |
| You decide | | |

**User's choice:** Add git_state_error column

---

## Dirty detection scope

| Option | Description | Selected |
|--------|-------------|----------|
| Staged + unstaged tracked changes (Recommended) | INDEX_* + WT_MODIFIED on tracked files; no untracked | ✓ |
| Staged + unstaged + untracked files | WT_NEW included; more complete, noisier | |
| Staged only | Very narrow; ignores working tree mods | |

**User's choice:** Tracked changes only (staged + unstaged, no untracked)

| Option | Description | Selected |
|--------|-------------|----------|
| Same 'clean' status (Recommended) | Untracked only = clean; one binary is_dirty flag | ✓ |
| Separate 'has untracked' indicator | Secondary visual indicator; more complexity | |
| You decide | | |

**User's choice:** Untracked-only repos show as clean

| Option | Description | Selected |
|--------|-------------|----------|
| Respect .gitignore (Recommended) | Default git2 behavior | ✓ |
| You decide | | |

**User's choice:** Respect .gitignore

| Option | Description | Selected |
|--------|-------------|----------|
| Always clean (bare repos have no working tree) | is_dirty=false | |
| Skip dirty check, show N/A | Explicit that dirty is not applicable for bare repos | ✓ |
| You decide | | |

**User's choice:** Skip dirty check for bare repos; show N/A

| Option | Description | Selected |
|--------|-------------|----------|
| Each worktree separately | Per-row is_dirty; aligns with Phase 2 per-path indexing | ✓ |
| Shared git dir (aggregate) | One flag for whole git_common_dir | |
| You decide | | |

**User's choice:** Per-worktree dirty detection

---

## Refresh trigger

| Option | Description | Selected |
|--------|-------------|----------|
| workpot git refresh (new command) | Explicit CLI command; clean separation from discovery | |
| Piggybacked on workpot index | One command does discovery + git refresh | ✓ |
| Automatic on workpot repo list | Always fresh; adds list latency | |

**User's choice:** Piggyback on workpot index

| Option | Description | Selected |
|--------|-------------|----------|
| Continue on error (Recommended) | Store error; refresh remaining repos | ✓ |
| Stop on first git error | Consistent with D-18 cap behavior; blocks all others | |

**User's choice:** Continue on individual repo error

| Option | Description | Selected |
|--------|-------------|----------|
| Include git refresh stats in output (Recommended) | 'git: 47 refreshed, 2 errors' | ✓ |
| Discovery stats only | Keep index output focused on changes | |
| You decide | | |

**User's choice:** Include git refresh stats in workpot index output

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — ship refresh_git_state(path) in core (Recommended) | Phase 4 tray can call per-repo; builds GIT-04 contract | ✓ |
| No — batch refresh only in Phase 3 | Phase 4 adds per-repo refresh later | |
| You decide | | |

**User's choice:** Ship refresh_git_state(path) in core

| Option | Description | Selected |
|--------|-------------|----------|
| One pass: discovery then git refresh sequentially (Recommended) | Single command, single summary output | ✓ |
| Two separate internal phases, same command | Verbose per-phase output | |
| You decide | | |

**User's choice:** One pass: discovery walk first, then git2 parallel refresh

---

## Claude's Discretion

- Exact rayon thread pool sizing
- git2 `StatusOptions` bitfield composition
- Migration file number and name
- Output format details for age indicator (relative time string)
- Whether `list_worktree_paths` uses git2 `worktrees()` API or parsed porcelain format

## Deferred Ideas

- Separate `workpot git refresh` standalone command — user chose to piggyback on index
- Filesystem watcher for automatic git re-index — post-v1
- Ahead/behind "—" placeholder for no-upstream — user chose NULL/omit; revisit in Phase 4 UI
- Git stats in index_runs history table — only CLI output stats in Phase 3
