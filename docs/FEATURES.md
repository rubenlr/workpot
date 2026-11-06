# Workpot features

Workpot is a macOS menu-bar assistant for engineers who juggle many git repositories. This page lists what it does today (v0.0.7). For install steps see [INSTALL.md](../INSTALL.md); for every config key see [SETTINGS.md](../SETTINGS.md).

Everything is local: the catalog lives in SQLite on disk and config is a TOML file. Nothing needs an account; the only network use is the `git fetch` you already run and, if you configure remote sources, the `gh` / `glab` CLIs you already use.

## Contents

- [Surfaces](#surfaces)
- [Discovery and catalog](#discovery-and-catalog)
- [Remote catalog](#remote-catalog)
- [Git awareness](#git-awareness)
- [Finding a repo](#finding-a-repo)
- [Launching](#launching)
- [Organizing repos](#organizing-repos)
- [Branches and worktrees](#branches-and-worktrees)
- [Bare / worktree conversion](#bare--worktree-conversion)
- [Configuration](#configuration)
- [Keyboard reference](#keyboard-reference)
- [Not yet available](#not-yet-available)

## Surfaces

| Surface  | What it is                                                                                                      |
| -------- | --------------------------------------------------------------------------------------------------------------- |
| Tray app | `Workpot.app`, a Tauri 2 menu-bar app. Click the icon to open the repo finder panel.                            |
| CLI      | `workpot`, bundled inside the app and symlinked onto your PATH by Homebrew. Shares one Rust core with the tray. |

Both read and write the same catalog, so a repo you tag in the tray shows its tags in `workpot list`.

### Tray menu and icon

The tray context menu has **Sync**, **Preferences…**, **About Workpot** and **Quit Workpot**. The icon changes state:

- **Syncing**: animated while a sync runs.
- **Stale dirty**: a distinct icon when a repo has been dirty longer than `stale_dirty_days` (default 7), so forgotten uncommitted work is visible without opening the panel.

## Discovery and catalog

- **Watch roots**: directories scanned for git repositories. On first run `~/code` and `~/dev` are added if they exist. Manage them with `workpot roots add|list|remove`. Adding a root scans it immediately; removing one prunes the repos under it.
- **Manual registration**: `workpot repo add <path>` registers a worktree or bare repo anywhere on disk. Manual entries stay manual even if a watch root later finds the same path.
- **Excludes**: glob patterns (for example `**/node_modules/**`) skipped during scans, on top of built-in excludes such as `target`. See `workpot excludes list|remove`.
- **Bare repos and linked worktrees** are detected. Descent stops at the first repo found, so nested repos are not cataloged separately.
- **Projects**: checkouts are grouped under a remote-rooted project. The root remote is picked as `upstream`, then `origin`, then the first remote by name; forks and aliases are tracked, and projects whose remotes overlap are merged. Repos with no remote get a `local:` identity.
- **Limits**: `limits.max_watch_roots` (100) and `limits.max_repos` (1000). A sync that would exceed the repo cap aborts before changing the catalog.

### Sync

Sync is explicit; Workpot does not rescan on its own. These all run the same pipeline:

- `workpot sync` (and `workpot sync local` / `workpot sync remote` for one scope)
- Tray **Sync** menu item or `Cmd+R` in the panel
- Opening the panel

Remote sources (if any) are indexed first; panel open respects `remote_sync_ttl_minutes`, explicit sync always lists. Local pipeline: discover watch roots, merge into the catalog and attach projects, optionally `git fetch`, refresh git state in parallel, then persist worktrees and branches. A failure on one repo is counted and logged without aborting the rest. Overlapping runs are skipped. Details: [docs/sync.md](sync.md).

## Remote catalog

Point Workpot at a GitHub org/user or GitLab group and every repo under it (GitLab subgroups included) is indexed without cloning:

- `workpot remotes add|list|set|remove` manages sources; they live in `config.toml` as `[[remotes]]`. Access goes through your logged-in `gh` / `glab`, so Workpot stores no credentials.
- Indexed repos that are not cloned appear in the tray under **Not cloned** and in `workpot list` / `search` as `☁️ … (not cloned)`, with a **Clone** button (tray) or `workpot clone` (CLI).
- Per source, `clone_policy` chooses what is automatic: `manual`, `on_open` (opening it clones first) or `auto` (everything is cloned on sync). The Clone button always works.
- `prune_removed` optionally deletes local clones whose repo disappeared upstream, only when they are clean, fully pushed and the host confirms a 404; otherwise they are flagged "removed upstream".

Details: [docs/sync.md](sync.md#remote-catalog-sync-sync-remote-catalog).

## Git awareness

Per repo, read through libgit2 (no `git` process per repo) and refreshed on sync:

- Current branch
- Dirty or clean
- Ahead and behind counts against the upstream, when one is configured
- Last refresh time

Before each fetch, Workpot checks that every remote has a multi-branch fetch refspec and repairs it if not (typical after a bare conversion). Set `fetch = ""` to turn fetching off.

## Finding a repo

The tray panel is a filter-as-you-type finder.

- **Fuzzy match** over name, path, branch, tags, and notes. Aliases are shown as the display name.
- **Priority order** in four sections: **Pinned**, **Dirty**, **Recent**, then the rest. Manual pins win over the automatic signals. `max_recent_days` (14) and `min_recent_count` (3) tune the Recent section; `max_visible_rows` (15) controls panel height before scrolling.
- **Detail pane** for the selected repo: branches, tags, alias, notes, and actions.

The CLI mirrors the finder:

```bash
workpot list              # full list, Pinned > Dirty > Recent > Rest
workpot search "api"      # fuzzy filter, same ordering and row format
workpot open my-repo      # open by name, alias, path key or canonical path
```

`workpot search` treats `#` as plain text; use `workpot tag list <repo>` to inspect tags. It exits 0 even with no matches, so it works in scripts.

## Launching

- **Open in your editor** from the panel with `Enter`, double-click, or the tray actions. Workpot is not tied to an IDE: any editor works, as long as a command can open a folder with it. The command is the `launch_cmd` template, and `{path}` is replaced with the repo path. It is split into a program and arguments and run directly, not through a shell. The default is Cursor (`cursor --new-window {path}`). Examples:

  ```toml
  launch_cmd = "code --new-window {path}"   # VS Code
  launch_cmd = "zed {path}"                  # Zed
  launch_cmd = "idea {path}"                 # IntelliJ IDEA
  launch_cmd = "open -a \"Sublime Text\" {path}"  # any app, no CLI needed
  ```

- **Open in Finder** from the detail pane.
- For bare repos, Workpot opens the linked worktree whose branch matches the catalog branch.
- `workpot open` exits 1 if the repo is not found or the name is ambiguous (with numbered matches), and 2 if the launch fails. In the tray, launch failures show in an error banner.

## Organizing repos

| Feature | Tray                                              | CLI                                    |
| ------- | ------------------------------------------------- | -------------------------------------- |
| Tags    | `#` autocomplete in the detail pane, context menu | `workpot tag add\|remove\|list <repo>` |
| Pin     | Pin from the context menu; drag to reorder pinned | not available                          |
| Alias   | Set in the detail pane                            | accepted by `workpot open`             |
| Notes   | Free text in the detail pane, searchable          | not available                          |

Pinned repos are capped by `max_pinned` (default 5). Right-clicking a list row opens a context menu with Pin, Add/Remove tag, and Convert.

## Branches and worktrees

The detail pane lists each repo's branches with an indicator for where they exist: checked out, in a worktree, or remote-only. From there you can:

- **Switch branch** (checkout).
- **Pull and push** one branch against its upstream. These run `pull_cmd` and `push_cmd` (defaults `git -C {path} pull origin {branch}` and `... push origin {branch}`), then refresh git state. They are separate from catalog sync.
- **Hide or show branches** per repo. Hidden branches are filtered out by default, with a Show all / Show less toggle.

## Bare / worktree conversion

Convert a repo between a normal checkout and a bare repo plus linked worktree layout, keeping git remotes:

```bash
workpot repo convert <path> --to bare --dry-run   # preview paths and preflight
workpot repo convert <path> --to bare
workpot repo convert <path> --to local
```

A preflight gate blocks the conversion on dirty worktrees, unborn or detached HEAD, local branches with no upstream or unpushed commits, stashes, and (when `delete_original` is on) untracked files. The original directory is renamed to `<name>.temp` and kept unless `migration.delete_original = true`, so an interrupted run is recoverable. Target paths come from the `bare_repo_template` and `worktree_template` settings.

The tray shows **Convert to bare** / **Convert to local** according to the `migration.allow_conversion_*` and `default_repo_layout` settings. Full rules and recovery steps are in [SETTINGS.md](../SETTINGS.md).

## Configuration

- Config: `~/.config/workpot/config.toml`, created on first run. `workpot settings init` writes a commented default; `workpot settings --add-comments` backfills comments into an existing file.
- Database: `~/Library/Application Support/workpot/workpot.db`. `workpot db reset` deletes it (quit the tray first).
- `workpot paths` prints the resolved locations.
- Logging uses `RUST_LOG`, for example `RUST_LOG=workpot_core=debug`.

## Keyboard reference

| Key              | Action                                     |
| ---------------- | ------------------------------------------ |
| Type             | Filter the list                            |
| `↑` / `↓`, `Tab` | Move the selection                         |
| `→`              | Open the detail pane for the selection     |
| `←`              | Close the detail pane                      |
| `Enter`          | Open the selected repo in the editor       |
| `Cmd+R`          | Sync                                       |
| `Esc`            | Close the detail pane, then hide the panel |

## Not yet available

Planned, not implemented (see the [changelog](../CHANGELOG.md)):

- Configurable recipes (shell steps, multi-step workflows)
- Editing remote sources from the tray (use `workpot remotes` or `config.toml`)
- Providers other than GitHub and GitLab
- Cross-repo content search; search is metadata-only
- Platforms other than macOS
