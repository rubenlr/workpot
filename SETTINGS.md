# Workpot settings reference

All settings live in `config.toml`. On first run, workpot creates the file with defaults at:

- macOS: `~/.config/workpot/config.toml`
- Database: `~/Library/Application Support/workpot/workpot.db`

Run `workpot paths` to print the resolved paths on your machine.

Explicit bootstrap and documentation backfill:

- `workpot settings init` — write a documented default `config.toml` (fails if the file already exists; use `--force` to overwrite)
- `workpot settings --add-comments` — add missing inline documentation to an existing install without changing values

## Configuration file structure

Inline comments in generated `config.toml` files are sourced from [`crates/workpot-core/src/infra/settings.template.toml`](crates/workpot-core/src/infra/settings.template.toml).

```toml
watch_roots = []
excludes = []

[limits]
max_watch_roots = 100
max_repos = 1000

launch_cmd = "cursor --new-window {path}"
push_cmd = "git -C {path} push origin {branch}"
pull_cmd = "git -C {path} pull origin {branch}"
max_visible_rows = 15
max_pinned = 5
max_recent_days = 14
min_recent_count = 3
stale_dirty_days = 7

[migration]
temp_suffix = ".temp"
delete_original = false
bare_repo_template = "{project}/bare.git"
worktree_template = "{project}/wtrees/{worktree}"
project_name_source = "folder_name"
```

## Discovery settings

| Key                      | Default | Description                                                                                                         |
| ------------------------ | ------- | ------------------------------------------------------------------------------------------------------------------- |
| `watch_roots`            | `[]`    | Directories scanned for git repositories. On first run, `~/code` and `~/dev` are added automatically if they exist. |
| `excludes`               | `[]`    | Glob patterns excluded from indexing (e.g. `**/node_modules/**`).                                                   |
| `limits.max_watch_roots` | `100`   | Maximum number of watch roots allowed.                                                                              |
| `limits.max_repos`       | `1000`  | Maximum number of indexed repositories.                                                                             |

## Tray settings

| Key                | Default | Description                                                  |
| ------------------ | ------- | ------------------------------------------------------------ |
| `max_visible_rows` | `15`    | Maximum repo rows shown in the tray panel before scrolling.  |
| `max_pinned`       | `5`     | Maximum pinned repositories in the tray.                     |
| `max_recent_days`  | `14`    | Recency window (days) for the Recent section.                |
| `min_recent_count` | `3`     | Minimum repos shown in Recent via padding.                   |
| `stale_dirty_days` | `7`     | Days before a dirty repo triggers the stale-dirty tray icon. |

## Launch settings

| Key          | Default                        | Description                                                                               |
| ------------ | ------------------------------ | ----------------------------------------------------------------------------------------- |
| `launch_cmd` | `"cursor --new-window {path}"` | Shell command template for opening a repo. `{path}` is replaced with the repository path. |

## Sync settings

| Key        | Default                                | Description                                                                                       |
| ---------- | -------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `push_cmd` | `"git -C {path} push origin {branch}"` | Shell command template for pushing a branch. `{path}` and `{branch}` are replaced at invoke time. |
| `pull_cmd` | `"git -C {path} pull origin {branch}"` | Shell command template for pulling a branch. `{path}` and `{branch}` are replaced at invoke time. |

`{path}` resolves to the indexed launch path (worktree for bare repos). `{branch}` is the branch name from the tray row. Both placeholders are required. Commands run synchronously and refresh git state on success.

## Repo migration settings (`[migration]`)

Migration settings control `workpot repo convert`, which switches a repository between a normal checkout and a bare repository plus linked worktree layout. Conversion is CLI-only in v1 (no tray UI).

Path templates are global only — there are no per-repo template overrides in v1.

### Keys and defaults

| Key                             | Default                         | Description                                                                                                                                                                                                                           |
| ------------------------------- | ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `migration.temp_suffix`         | `".temp"`                       | Suffix appended to the original directory name during conversion. Must not be empty.                                                                                                                                                  |
| `migration.delete_original`     | `false`                         | When `true`, the renamed `.temp` directory is deleted after a successful conversion (including any untracked files it contains). Conversion is blocked if untracked files exist. When `false`, `.temp` is kept for manual inspection. |
| `migration.bare_repo_template`  | `"{project}/bare.git"`          | Path template for the bare git repository. Must contain `{project}`.                                                                                                                                                                  |
| `migration.worktree_template`   | `"{project}/wtrees/{worktree}"` | Path template for the first worktree. Must contain `{project}` and `{worktree}`.                                                                                                                                                      |
| `migration.project_name_source` | `"folder_name"`                 | Source for `{project}`: `folder_name` uses the directory name; `alias` uses the workpot alias (falls back to folder name if unset).                                                                                                   |

### Template variables

- `{project}` — project name from `project_name_source`.
- `{worktree}` — sanitized branch name (slashes replaced with dots; a short hash suffix is appended on collision).

Resolved paths must stay under the parent directory of the source repository. Templates containing `../` are rejected.

### Template examples

**Example A** (nested, recommended):

```toml
[migration]
bare_repo_template = "{project}/bare.git"
worktree_template = "{project}/wtrees/{worktree}"
```

For project `my-project-test` on branch `feature/my-branch`:

- Bare repo: `my-project-test/bare.git`
- Worktree: `my-project-test/wtrees/feature.my-branch`

**Example B** (flat sibling):

```toml
[migration]
bare_repo_template = "{project}.git"
worktree_template = "{project}.{worktree}"
```

Same project and branch:

- Bare repo: `my-project-test.git`
- Worktree: `my-project-test.feature.my-branch`

### Conversion command

- Repository must be registered (`workpot repo add`) before conversion.

```bash
# Normal checkout → bare + worktree
workpot repo convert <path> --to bare

# Bare + worktree → normal checkout
workpot repo convert <path> --to normal

# Preview resolved paths and preflight without changes
workpot repo convert <path> --to bare --dry-run
```

### Preflight gate

Conversion is blocked unless all of the following are true:

- No dirty state in any worktree (for bare repos, all linked worktrees are checked).
- Repository must have at least one commit (unborn HEAD is rejected).
- Normal repositories must be on a named branch (detached HEAD is rejected).
- Every local branch has an upstream and is not ahead of it.
- No stash entries exist.
- When `migration.delete_original = true`, no untracked files may exist in any worktree (they would be deleted with the `.temp` directory).

Dry-run checks the same path collisions as a real run, including an existing `{original}{temp_suffix}` directory.

### Bare repo launch path

When opening a bare catalog entry, Workpot launches the linked worktree whose checked-out branch matches the catalog `branch` field. If `branch` is unset, the first linked worktree is used — for repos with multiple worktrees, ensure git state is refreshed (`workpot index`) so the catalog branch is current.

### Recovery from interrupted conversion

If the process crashes mid-conversion, the original directory is renamed to `{original}{temp_suffix}` (for example `my-project.temp`) and the catalog still points at the original path.

To recover:

1. Restore the original layout: `mv my-project.temp my-project`
2. Or register the renamed directory while you decide: `workpot repo add my-project.temp`

On successful conversion, the catalog is updated to the new path. On failure, the catalog is left unchanged and the `.temp` directory remains on disk.
