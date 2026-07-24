# Indexing

`workpot index` rescans your configured watch roots and refreshes git metadata for every indexed repository. The tray **Refresh index** action runs the same pipeline. Indexing is not automatic when Workpot starts — you run it when the catalog should catch up with the disk.

```bash
workpot index
```

## When to run it

Run a full index when:

- You cloned, moved, or deleted repos under a watch root outside Workpot
- The tray or `workpot list` looks stale
- You changed `excludes` and want the catalog to match

When you are only adding a **new watch root**, prefer `workpot roots add <path>`. That command indexes as part of add, so you usually do not need a separate full `workpot index` just for that root.

## What it walks

Workpot walks each path in `watch_roots` (see [SETTINGS.md](../SETTINGS.md#discovery-settings)).

- Roots that cannot be read are skipped with a warning; the rest of the run continues.
- Built-in exclude globs and your `excludes` list are applied during the walk (for example `**/node_modules/**`, `**/target/**`).
- Descent stops when a git worktree or bare repository is found. Nested repos inside another repo are **not** indexed as separate entries.
- Bare repositories also pull in their linked worktrees as candidates.

Config keys for roots, excludes, and caps are defined in SETTINGS; this guide describes runtime behavior only.

## What gets into the catalog

After the walk, Workpot merges candidates into the local catalog:

- **New** scan repos are added.
- **Existing** scan repos are updated (path metadata such as name / git common dir).
- **Manual** registrations (`workpot repo add`) stay manual even if the same path is rediscovered under a watch root.
- Rows are removed when a path has vanished from disk, when a scan-sourced repo is no longer under any configured watch root (orphan after editing roots), or when it was under a scannable root but was not seen this run (deleted, excluded, or no longer a git repo).
- Manual repos **outside** all watch roots are kept if they still exist and are valid git; otherwise they are cleaned up.

## Git refresh

After the catalog merge commits, Workpot re-reads branch, dirty, ahead, and behind for all non-excluded repos (in parallel).

- A failure on one repo counts as a git error in the summary and does **not** abort the rest of the refresh.
- The `limits.max_repos` cap is checked **before** any catalog writes for that run. If the projected count would exceed the cap, indexing aborts with no catalog mutation (an audit row may still record the cap failure).

## Output and exit codes

On success, stdout looks like:

```text
index: +N -M skipped K / git: R refreshed, E errors
```

| Field         | Meaning                                          |
| ------------- | ------------------------------------------------ |
| `+N`          | Repos newly added                                |
| `-M`          | Repos removed                                    |
| `skipped K`   | Candidates skipped (for example git unavailable) |
| `R refreshed` | Repos whose git state was refreshed successfully |
| `E errors`    | Per-repo git refresh failures                    |

Exit codes:

- **0** — success, including empty watch roots
- **1** — projected repo count exceeds `limits.max_repos` (catalog unchanged for that run)
- **non-zero** — other failures

If the catalog merge succeeds but a later git-persist step fails, the merge may already be committed while git columns stay as they were. Re-run `workpot index` after fixing the problem.

## Related commands

- `workpot roots add|remove` — roots decide what a full index scans. Add indexes as part of the operation; remove may prune scan-sourced repos under that root (unless you pass `--skip-prune`).
- `workpot repo add|remove` — manual add registers a path even when excludes would skip it; remove deletes the catalog row and adds exclude globs so a later rescan does not re-add it.
- `workpot excludes` — manage scan-only exclude globs (list / remove).
- Tray **Refresh index** — same indexing pipeline as the CLI.

## Where data lives

Run `workpot paths` for the resolved locations on your machine. Settings live in `config.toml`; the catalog is SQLite. Details and defaults are in [SETTINGS.md](../SETTINGS.md).

Each successful (and many failed) index runs leave an audit trail in the database (`index_runs` and per-path `index_changes`) so you can see what a run added, removed, or skipped without inspecting the schema yourself.
