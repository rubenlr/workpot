# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned

- CLI parity with the tray (`workpot list`, `workpot search`, `workpot open`) — Phase 6
- Configurable recipes (shell steps, Cursor launch, multi-step workflows) — Phase 7
- macOS `.app` / DMG distribution and install/update UX

## [0.0.5] - 2026-07-21

Repo conversion remotes, hidden branches, list context menu, and dependency maintenance.

- **Convert remotes** — bare ↔ local conversion snapshots and re-applies git remotes; partial-failure cleanup on remote reconcile errors
- **Hidden branches** — per-repo hide/show in the detail pane (filtered by default; Show all / Show less)
- **Repo context menu** — right-click list row for Pin, Add/Remove tag, and Convert (with volatile preflight block reason)
- **Deps** — tauri 2.11.5, `window-vibrancy` 0.7, `anyhow` 1.0.103, `log` 0.4.33, `env_logger` 0.11.11; Storybook/Vitest addon 10.4.6, svelte-check 4.7, testing-library/svelte 5.4, playwright 1.61.1; CI `taiki-e/install-action` 2.82.7; cookie constraint / lockfile hygiene for vulnerability cleanup

## [0.0.4] - 2026-06-17

Maintenance release: grouped dependency updates across Rust, frontend tooling, and CI.

- **Tauri** — tauri 2.11.3, tauri-build 2.6.3, `@tauri-apps/api` 2.11.1
- **Rust** — `toml_edit` 0.25, `window-vibrancy` 0.6
- **Frontend** — TypeScript 6.0, `@tailwindcss/vite` 4.3.1, prettier-plugin-svelte 4.1, prettier-plugin-tailwindcss 0.8
- **CI** — actions/checkout v6, upload-artifact v7, download-artifact v8

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

[0.0.5]: https://github.com/rubenlr/workpot/releases/tag/v0.0.5
[0.0.4]: https://github.com/rubenlr/workpot/releases/tag/v0.0.4
[0.0.3]: https://github.com/rubenlr/workpot/releases/tag/v0.0.3
[0.0.2]: https://github.com/rubenlr/workpot/releases/tag/v0.0.2
[0.0.1]: https://github.com/rubenlr/workpot/releases/tag/v0.0.1
[Unreleased]: https://github.com/rubenlr/workpot/compare/v0.0.5...HEAD
