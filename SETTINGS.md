# Workpot settings reference

All settings live in `config.toml`. On first run, workpot creates the file with defaults at:

- macOS: `~/.config/workpot/config.toml`
- Database: `~/.config/workpot/workpot.db`

Run `workpot paths` to print the resolved paths on your machine.

## Configuration file structure

```toml
watch_roots = []
excludes = []

[limits]
max_watch_roots = 100
max_repos = 1000

launch_cmd = "cursor --new-window {path}"
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

## Repo migration settings (`[migration]`)

Migration settings control `workpot repo convert`, which switches a repository between a normal checkout and a bare repository plus linked worktree layout. Conversion is CLI-only in v1 (no tray UI).

Path templates are global only — there are no per-repo template overrides in v1.

### Keys and defaults

| Key                             | Default                         | Description                                                                                                                          |
| ------------------------------- | ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `migration.temp_suffix`         | `".temp"`                       | Suffix appended to the original directory name during conversion. Must not be empty.                                                 |
| `migration.delete_original`     | `false`                         | When `true`, the renamed `.temp` directory is deleted after a successful conversion. When `false`, it is kept for manual inspection. |
| `migration.bare_repo_template`  | `"{project}/bare.git"`          | Path template for the bare git repository. Must contain `{project}`.                                                                 |
| `migration.worktree_template`   | `"{project}/wtrees/{worktree}"` | Path template for the first worktree. Must contain `{project}` and `{worktree}`.                                                     |
| `migration.project_name_source` | `"folder_name"`                 | Source for `{project}`: `folder_name` uses the directory name; `alias` uses the workpot alias (falls back to folder name if unset).  |

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
- Every local branch has an upstream and is not ahead of it.
- No stash entries exist.

### Recovery from interrupted conversion

If the process crashes mid-conversion, the original directory is renamed to `{original}{temp_suffix}` (for example `my-project.temp`) and the catalog still points at the original path.

To recover:

1. Restore the original layout: `mv my-project.temp my-project`
2. Or register the renamed directory while you decide: `workpot repo add my-project.temp`

On successful conversion, the catalog is updated to the new path. On failure, the catalog is left unchanged and the `.temp` directory remains on disk.
