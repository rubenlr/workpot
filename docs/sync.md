# Sync

Workpot sync keeps the local catalog and git metadata up to date. Sync is not automatic when Workpot starts — you run it when the catalog should catch up with the disk.

## Scopes

| Scope                 | Meaning                                                    | Status                              |
| --------------------- | ---------------------------------------------------------- | ----------------------------------- |
| `sync-local-catalog`  | Watch-root scan + catalog merge + project attach + git     | Implemented (`workpot sync local`)  |
| `sync-remote-catalog` | Index GitHub orgs / GitLab groups (no clone required)      | Implemented (`workpot sync remote`) |
| `fetch-repo`          | Per-repo fetch via `Config.fetch` before git-state refresh | Implemented                         |
| `sync` (full)         | Orchestrator: remote catalog, then local catalog           | Implemented (`workpot sync`)        |

Push/pull via tray/`repo_sync` is **not** a catalog sync scope — those commands move commits for one branch.

```bash
workpot sync         # full orchestrator: remote catalog, then local catalog
workpot sync local   # local catalog sync only
workpot sync remote  # remote catalog only (index, auto-clone, prune per source policy)
```

Tray **panel open**, tray **Sync** / Cmd+R, and `workpot sync` all run the **same** full catalog sync (not a git-only refresh). Concurrent runs are guarded so overlapping triggers skip.

## Local catalog sync (`sync-local-catalog`)

### Pipeline

Phased run (locks released between phases):

1. **Discover** — walk watch roots; plan upserts/removes
2. **Merge** — write locations into the catalog, then **attach** each location to a project and **merge** projects whose remote sets overlap
3. **Fetch + git refresh** — optional `Config.fetch`, then parallel HEAD/dirty/ahead/behind
4. **Persist** — write git columns on locations; replace per-location **worktrees** and **branches**; prune empty projects

### When to run it

Run a local/full sync when:

- You cloned, moved, or deleted repos under a watch root outside Workpot
- The tray or `workpot list` looks stale
- You changed `excludes` and want the catalog to match

When you are only adding a **new watch root**, prefer `workpot roots add <path>`. That command syncs the local catalog as part of add, so you usually do not need a separate full sync just for that root.

### What it walks

Workpot walks each path in `watch_roots` (see [SETTINGS.md](../SETTINGS.md#discovery-settings)).

- Roots that cannot be read are skipped with a warning; the rest of the run continues.
- Built-in exclude globs and your `excludes` list are applied during the walk (for example `**/node_modules/**`, `**/target/**`).
- Descent stops when a git worktree or bare repository is found. Nested repos inside another repo are **not** cataloged as separate entries.
- Bare repositories also pull in their linked worktrees as candidates.

### What gets into the catalog

After the walk, Workpot merges candidates into `locations`:

- **New** scan paths are added.
- **Existing** scan paths are updated (name / git common dir).
- **Manual** registrations (`workpot repo add`) stay manual even if the same path is rediscovered under a watch root.
- Rows are removed when a path has vanished from disk, when a scan-sourced repo is no longer under any configured watch root (orphan after editing roots), or when it was under a scannable root but was not seen this run (deleted, excluded, or no longer a git repo).
- Manual repos **outside** all watch roots are kept if they still exist and are valid git; otherwise they are cleaned up.

Then each location is attached to a **project** (remote-rooted identity). See [Projects](#projects-and-schema) below.

### Fetch + git refresh

After the catalog merge commits, Workpot optionally fetches remotes (`Config.fetch`, see below), then re-reads branch, dirty, ahead, and behind for all non-excluded locations (in parallel). Persist then replaces `worktrees` / `branches` for each location.

- Empty `fetch` disables the fetch step.
- A failure on one repo counts as a git error in the summary and does **not** abort the rest of the refresh.
- The `limits.max_repos` cap is checked **before** any catalog writes for that run. If the projected count would exceed the cap, sync aborts with no catalog mutation (an audit row may still record the cap failure).

### Output and exit codes

On success, stdout looks like:

```text
sync: +N -M skipped K / git: R refreshed, E errors
```

(`workpot sync local` prints `sync local:` with the same fields.)

| Field         | Meaning                                              |
| ------------- | ---------------------------------------------------- |
| `+N`          | Locations newly added                                |
| `-M`          | Locations removed                                    |
| `skipped K`   | Candidates skipped (for example git unavailable)     |
| `R refreshed` | Locations whose git state was refreshed successfully |
| `E errors`    | Per-location git refresh failures                    |

Exit codes:

- **0** — success, including empty watch roots
- **1** — projected repo count exceeds `limits.max_repos` (catalog unchanged for that run)
- **non-zero** — other failures

If the catalog merge succeeds but a later git-persist step fails, the merge may already be committed while git columns stay as they were. Re-run `workpot sync` after fixing the problem.

## Projects and schema

Catalog identity is **project + locations**, not one row collapsed by remote URL.

| Table             | Role                                                                 |
| ----------------- | -------------------------------------------------------------------- |
| `projects`        | Remote-rooted identity (`id` = sha256 of root remote; name, created) |
| `project_remotes` | Root + fork/alias URLs (`role` = `root` \| `fork`)                   |
| `locations`       | Checkout paths (path-as-identity; git HEAD columns; `project_id`)    |
| `worktrees`       | Linked worktrees per location (`path`, `head_branch`)                |
| `branches`        | Local/remote refs per location (`kind`, `tip_oid`, `remote_name`)    |

### Root election and merge

On attach (create):

- **Root election:** named remote `upstream` → `origin` → first by remote name. No usable remotes → `local:{git_common_dir}`.
- Non-root remotes are stored as **forks** on `project_remotes`.
- Attach matches if any location remote equals a project's root or any known fork/alias. Multi-project conflict picks a winner; it does **not** merge during attach.

After all attaches:

- Projects whose remote URL sets **overlap** are merged (connected components). Upstream-rooted survivors keep their root sticky across fork overlap.

## Fetch-repo (`fetch-repo`)

`Config.fetch` defaults to `git -C {path} fetch --prune --no-tags`. During batch git refresh (local catalog sync / tray background sync), Workpot runs this template per location before querying git state.

- Set `fetch = ""` to disable.
- Non-empty values must include `{path}`.
- Before each fetch, Workpot verifies every remote has a multi-branch fetch refspec (`+refs/heads/*:refs/remotes/<name>/*`) and rewrites missing or single-branch mappings (typical after bare conversion).
- Failures are soft: logged and counted toward git errors without aborting the batch.

## Full sync (`sync`)

`workpot sync`, tray Sync / Cmd+R, and panel open call the same orchestrator: the remote catalog runs first (so fresh clones are picked up), then the local catalog sync. A remote failure never blocks the local pass; it is reported per source.

Panel open honours `remote_sync_ttl_minutes` (default 15) so opening the panel does not hit the network every time. `workpot sync`, tray **Sync** and `Cmd+R` always list remotes.

## Remote catalog sync (`sync-remote-catalog`)

Point Workpot at a GitHub org/user or a GitLab group and every repo under it appears in the catalog **without being cloned**. GitLab groups are indexed recursively, subgroups included.

Workpot never handles credentials: it shells out to the `gh` / `glab` CLIs you already have, so you must be logged in for the host (`gh auth login`, `glab auth login --hostname <host>`). A missing CLI or login is reported as a soft per-source error. The tray app looks for the CLIs in `/opt/homebrew/bin`, `/usr/local/bin` and `~/.local/bin` as well as `PATH`.

```bash
workpot remotes add acme/platform --provider gitlab --host gitlab.example.com \
  --clone-root ~/code/acme --policy on-open
workpot remotes list
workpot remotes set platform --policy auto --prune-removed true
workpot remotes remove platform          # local clones stay as ordinary repos
workpot clone acme/platform/billing-api  # clone one indexed repo (any policy)
```

Sources live in `config.toml` as `[[remotes]]` (see [SETTINGS.md](../SETTINGS.md#remote-catalog-remotes)).

### Pipeline (per source)

1. **List** — `gh api` / `glab api` with pagination; no database lock is held while talking to the network.
2. **Index** — upsert into `remote_repos`; repos missing from a _complete_ listing are marked gone. An empty listing, a listing above `limits.max_remote_repos`, or any CLI error leaves the existing index untouched.
3. **Prune** (when `prune_removed = true`) — see below.
4. **Auto-clone** (when `clone_policy = "auto"`) — up to 4 clones in parallel; repos matching your `excludes` are skipped so `workpot repo remove` is respected.

### Clone policies

| Policy    | What happens automatically                                             |
| --------- | ---------------------------------------------------------------------- |
| `manual`  | Nothing. Clone from the tray or with `workpot clone`.                  |
| `on_open` | Opening a not-cloned repo (tray row / `workpot open`) clones it first. |
| `auto`    | Every indexed repo is cloned during sync.                              |

The tray **Clone** button and `workpot clone` work under every policy. Clones land in `<clone_root>/<remote path relative to the source path>`, are registered in the catalog, and use the transport (`ssh` / `https`) configured in the CLI for that host (`gh|glab config get git_protocol`) unless the source sets `protocol`.

### Cloned vs. not cloned

A repo counts as cloned when a cataloged checkout has the same normalized remote URL (SSH, HTTPS and `.git` forms match), so a repo you cloned by hand is recognised too. Not-cloned repos show in the tray under **Not cloned** and in `workpot list` / `search` as `☁️ … (not cloned)`.

### Removing local repos that disappeared upstream (`prune_removed`)

Off by default. A local clone is deleted only when **all** of these hold; otherwise it is kept and shown as "removed upstream":

- it is absent from a complete listing **and** a direct lookup of that repo returns 404 (an API error never leads to deletion);
- it is a plain checkout under the source's `clone_root` (bare layouts and repos with linked worktrees are kept);
- no uncommitted changes, no untracked files, no stash, and no merge/rebase/cherry-pick in progress or conflicted files;
- every local branch has an upstream and nothing is ahead of it;
- HEAD is not detached at a commit missing from every remote-tracking branch, and no local tag points at such a commit;
- no submodules (`.gitmodules` or a non-empty `.git/modules`).

Any check that cannot be completed (for example an unreadable linked worktree entry) keeps the clone.

Git-ignored files (build output, a local `.env`) are deleted together with the clone. Repos that were _moved_ upstream appear as a new not-cloned repo, and the old clone is pruned only if it passes the checks above.

## Related commands

- `workpot roots add|remove` — roots decide what a local sync scans. Add syncs as part of the operation; remove may prune scan-sourced repos under that root (unless you pass `--skip-prune`).
- `workpot repo add|remove` — manual add registers a path even when excludes would skip it; remove deletes the catalog row and adds exclude globs so a later rescan does not re-add it.
- `workpot excludes` — manage scan-only exclude globs (list / remove).
- Tray push/pull — per-branch `repo_sync`, not catalog sync.

## Where data lives

Run `workpot paths` for the resolved locations on your machine. Settings live in `config.toml`; the catalog is SQLite. Details and defaults are in [SETTINGS.md](../SETTINGS.md).

Remote listings keep an audit row per source in `remote_sync_runs` (status, counts, error); `workpot remotes list` and the tray show the last result.

Each successful (and many failed) local catalog sync runs leave an audit trail in the database (`local_catalog_sync_runs` and per-path `local_catalog_sync_changes`) so you can see what a run added, removed, or skipped without inspecting the schema yourself.

### Database wipe (schema bootstrap)

After a catalog schema bootstrap change (e.g. `repos` → `projects` / `locations`), wipe the local DB so the next start recreates tables:

```bash
workpot db reset   # deletes workpot.db + WAL/SHM; quit tray first if locked
just db-reset      # same via justfile
```

Idempotent if the DB is already absent. Then re-run `workpot sync` (or open the tray) to rebuild the catalog.
