[![CI](https://github.com/rubenlr/workpot/actions/workflows/ci.yml/badge.svg)](https://github.com/rubenlr/workpot/actions/workflows/ci.yml)

# Workpot

Workpot is a macOS menu-bar assistant for engineers who juggle many git repositories. It keeps a local catalog of your repos, shows their git state at a glance, and lets you jump into the right one in your preferred editor in seconds, without keeping dozens of windows open.

- **Find fast**: a menu-bar panel with fuzzy filtering over name, path, branch, tags and notes. Repos are ordered Pinned, Dirty, Recent, then the rest.
- **See git state first**: branch, dirty/clean and ahead/behind for every repo, so you know where your uncommitted work is before you switch.
- **Stay in sync**: watch roots and manual registration keep the catalog current, including bare repos and linked worktrees.
- **Act from the panel**: open in your editor, switch branches, pull and push, tag, pin, and convert between normal and bare+worktree layouts.
- **CLI and tray share one core**: `workpot list`, `search`, `open`, `sync` and more work against the same catalog.
- **Local only**: SQLite and a TOML config on your disk. No accounts, no cloud.

```bash
brew tap rubenlr/workpot
brew install rubenlr/workpot/workpot

workpot roots add ~/code   # scan a directory for repos
workpot search api         # fuzzy-find a repo
workpot open my-api        # open it in your editor (Cursor by default)
```

Click the menu-bar icon to open the finder panel. Cursor is the default editor; set `launch_cmd` in `config.toml` to use any editor or IDE: VS Code, Zed, JetBrains, Sublime Text, or anything that can open a folder from a command (see [SETTINGS.md](SETTINGS.md#launch-settings)). macOS only for now.

Built with Rust (shared `workpot-core` crate, `git2`, SQLite), Tauri 2 and Svelte 5.

## Documentation

| Doc                                              | What's in it                                         |
| ------------------------------------------------ | ---------------------------------------------------- |
| [docs/FEATURES.md](docs/FEATURES.md)             | Feature overview for the tray and the CLI            |
| [INSTALL.md](INSTALL.md)                         | Install, update, uninstall, PATH troubleshooting     |
| [SETTINGS.md](SETTINGS.md)                       | Every `config.toml` key                              |
| [docs/sync.md](docs/sync.md)                     | How catalog sync, fetch and projects work            |
| [CONTRIBUTING.md](CONTRIBUTING.md)               | PR gates and semver policy                           |
| [CHANGELOG.md](CHANGELOG.md)                     | Release history                                      |

## Installing Workpot

Use [`INSTALL.md`](INSTALL.md) for end-user install, update, uninstall, DMG, and PATH guidance.

## Releasing

See [CONTRIBUTING.md](CONTRIBUTING.md) for PR gates and semver policy. [Release Please](https://github.com/googleapis/release-please) manages version and changelog from conventional commits; merging its Release PR tags the release and GitHub Actions uploads macOS tarballs to [GitHub Releases](https://github.com/rubenlr/workpot/releases). Details: [docs/releasing.md](docs/releasing.md).
