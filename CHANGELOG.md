# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned

- CLI parity with the tray (`workpot list`, `workpot search`, `workpot open`) — Phase 6
- Configurable recipes (shell steps, Cursor launch, multi-step workflows) — Phase 7
- macOS `.app` / DMG distribution and install/update UX

## [0.0.3] - 2026-06-15

Tray detail UX, branch operations, and bare↔normal repo migration.

- Tray UI refresh: repo detail components, unified error banner, and panel height sync with the webview
- Branch checkout and sync from the tray detail pane (switch branch, pull/push against upstream when configured)
- Migrate repositories between normal checkout and bare+worktree layouts via `workpot repo convert` and tray settings
- Branch list with presence indicators (checkout, worktree, remote-only) and improved git state refresh
- Tag autocomplete and repo list mouse/keyboard interaction polish
- `SETTINGS.md` reference for user-facing config, including migration templates
- Storybook build recipes (`just sbook`) for isolated tray component development

## [0.0.2] - 2026-06-09

Maintenance release: tray error handling cleanup and release pipeline fixes for Homebrew distribution.

- Simplified tray list error handling — repo load failures surface via `setListError` instead of split error paths
- Added `clean:all` npm script for full frontend artifact and lockfile cleanup
- Version sync validates pnpm manifests only (dropped stale `package-lock.json` checks)

## [0.0.1] - 2026-05-31

First public preview: local-only repo index, git-aware menu bar finder, and Cursor launch on macOS.

### Added

- **Core & privacy** — Shared Rust core (`workpot-core`) with SQLite migrations, TOML config under standard macOS app data paths, and lazy first-run bootstrap (default watch roots `~/code` and `~/dev`); all indexing and metadata stay on disk with no accounts or network calls for core use
- **Repository discovery** — Watch roots with `workpot roots add|list|remove`, full rescan via `workpot index`, manual `workpot repo add|list|remove`, path excludes (`workpot excludes`), built-in discovery excludes, bare-repo and linked worktree detection, transactional index merge with run history and configurable caps
- **Git state** — Per-repo branch, dirty/clean, ahead/behind vs upstream (when configured), and last-refresh timestamps via libgit2; background refresh in the tray without blocking the panel; CLI shows git summary on `workpot repo list` and index runs
- **Menu bar tray (Tauri 2)** — macOS tray icon opens a prioritized repo finder: real-time fuzzy filter (name, path, branch, tags, notes), keyboard navigation, four-section ordering (pinned → dirty → recent → rest), detail pane with branch picker, and tray context menu (refresh index, preferences, about, quit)
- **Cursor launch** — Open selected repo in Cursor from Enter, double-click, or tray actions; configurable `launch_cmd`; visible error banner when launch fails
- **Organization** — Tags (`workpot tag add|remove|list` and tray UI with `#` autocomplete), pin/unpin with drag-to-reorder among pinned repos, free-text notes searchable from the filter bar, and ranking signals (pinned, dirty, recently opened) with manual overrides winning
- **CLI** — `workpot paths`, `workpot index`, and subcommands for repos, roots, excludes, and tags
- **CI & releases** — GitHub Actions CI (Rust + frontend), SonarCloud, PR release-check for `version` + changelog, and macOS tarball artifacts (`aarch64` and `x86_64`) published on version bump to `master`

### Fixed

- Index run records error status when a full index fails instead of leaving a stale success row
- Manual repo add resolves `git_common_dir` correctly for linked worktrees
- Watch-root prune treats missing paths as not under the root; rejects config with more than `max_watch_roots`
- Bare and worktree paths canonicalized consistently during discovery
- Unified CLI message when the repository index cap is exceeded

[0.0.3]: https://github.com/rubenlr/workpot/releases/tag/v0.0.3
[0.0.2]: https://github.com/rubenlr/workpot/releases/tag/v0.0.2
[0.0.1]: https://github.com/rubenlr/workpot/releases/tag/v0.0.1
[Unreleased]: https://github.com/rubenlr/workpot/compare/v0.0.3...HEAD
