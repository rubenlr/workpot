# Phase 2: Repo discovery - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-29
**Phase:** 2-Repo discovery
**Areas discussed:** Nested repos & worktrees, Exclude semantics, Rescan merge policy, Watch root UX, Worktree data model, Remove→exclude glob, Index output & history

---

## Nested repos & worktrees

| Option | Description | Selected |
|--------|-------------|----------|
| Index every .git | Each checkout is its own row | |
| Skip nested under parent | Don't index deeper .git inside indexed tree | ✓ |
| Dedupe by common-dir | Group worktrees in one row | |

**User's choice:** Skip nested; index bare + normal; all worktree paths as separate rows; bare row + worktree rows.
**Notes:** Future bare vs normal modes affect clone/launch. Launch = repo + branch. Bare worktree default path `repo.git/wt/{name}` later. Normal repos: no Workpot worktree creation in v1. Skip symlinks; multiple watch roots for org separation.

---

## Exclude semantics

| Option | Description | Selected |
|--------|-------------|----------|
| Glob patterns | `excludes` in config.toml | ✓ |
| Path prefixes only | Simpler matching | |
| Both | Prefixes + globs | |

**User's choice:** Globs; built-in defaults (node_modules, .Trash, build dirs); manual add overrides excludes.
**Notes:** `repo remove` deletes row and adds exclude glob (user edits config to undo). Not separate soft-exclude-only command.

---

## Rescan merge policy

| Option | Description | Selected |
|--------|-------------|----------|
| `workpot index` | Top-level rescan command | ✓ |
| `workpot repo scan` | Nested under repo | |
| Keep manual source | Manual rows stay manual on rescan | ✓ |
| Auto-remove missing | Delete rows when path gone | ✓ |
| Validate manual-only | Check repos outside watch roots | ✓ |

**User's choice:** As above; cap hit stops with error (exit 1).

---

## Watch root UX

| Option | Description | Selected |
|--------|-------------|----------|
| CLI + config | `roots add|list|remove` and hand-edit | ✓ |
| Index on `roots add` | Immediate scan | ✓ |
| Prune on `roots remove` | Default prune repos under root | ✓ |
| `--skip-prune` | Optional keep rows | ✓ |

**User's choice:** Max watch roots 100 default / 5000 hard max; max repos 1000 default / 20000 hard max. First-run `~` suggestion wizard deferred.

---

## Worktree data model

| Option | Description | Selected |
|--------|-------------|----------|
| Path-only rows | Group later via git | |
| `git_common_dir` key | Stable identity across paths | ✓ |
| Move handling: re-key | Update path on same git dir | |
| Move handling: stale + new | Remove old, discover new | ✓ |

**User's choice:** User asked for git-native id; settled on canonical `git_common_dir`. Bare path + worktree rows.

---

## Remove → exclude glob

| Option | Description | Selected |
|--------|-------------|----------|
| Exact path glob | Only that directory | |
| Parent + name glob | e.g. `{parent}/foo/**` | ✓ |
| `workpot excludes list|remove` | CLI for exclude management | ✓ |

---

## Index output & history

| Option | Description | Selected |
|--------|-------------|----------|
| Summary line | Default terminal output | ✓ |
| Scan history tables | Per-run log + add/remove/skip detail | ✓ |
| Exit 1 at cap | Stop, no partial index | ✓ |

**Notes:** Git “update” entries in scan log deferred to Phase 3.

## Claude's Discretion

Built-in default glob list, migration shape for `git_common_dir` + history tables, walk crate choice.

## Deferred Ideas

- First-run watch-root suggestion wizard
- FS watcher auto-rescan
- Bare/normal mode product split, clone, branch launch
- Worktree creation by Workpot
- Path re-key on move (same git dir)
