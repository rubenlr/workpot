<!-- GSD:project-start source:PROJECT.md -->

## Project

**Workpot**

Workpot is a macOS-native assistant for engineers who juggle many git repositories. It indexes local repos, surfaces git state and metadata at a glance, and makes it fast to find the right project and launch Cursor or run recipes—without keeping dozens of IDE windows open.

Built on Tauri with a CLI and menu-bar tray. v1 is a prioritized fuzzy finder with git awareness; content search and remote integrations come later.

**Core Value:** You always know which repo you need and can open it in Cursor in seconds, with git context visible before you switch contexts.

### Constraints

- **Platform**: macOS only for v1 — Tauri tray + CLI
- **IDE**: Cursor launch integration required in v1
- **Privacy**: Local-only — index and config stay on disk
- **Search**: Metadata-first in v1; no cross-repo code index until v2
- **Discovery**: Watch roots auto-index + manual register/exclude

<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->

## Technology Stack

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| Tauri | 2.x | macOS tray app + native APIs | Official path for menu-bar apps; shares Rust core with CLI |
| Rust | 1.85+ (2024 edition) | Indexer, git orchestration, shared library | One core crate used by Tauri + CLI; avoids duplicating git logic |
| SQLite (rusqlite) | 0.32+ | Local repo index, tags, recipes, settings | Embedded, fast, backup-friendly; fits local-only constraint |
| git2 (libgit2) | 0.19+ | Read git state without shelling out | Stable ahead/behind, dirty detection; batch-friendly for many repos |
| notify / notify-debouncer-full | 5.x / 0.4+ | Filesystem watch on roots | Debounced events; re-index on clone/pull outside Workpot |
| clap | 4.x | CLI (`workpot` binary) | Standard Rust CLI; subcommands mirror tray actions |
| serde + toml | 1.x / 0.8+ | Config (watch roots, defaults, recipes) | Human-editable; version in repo dotfile `~/.config/workpot/` |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tauri-plugin-shell | 2.x | Run recipes / `cursor` CLI | Recipe execution, IDE launch |
| fuzzy-matcher / nucleo | 0.3+ / 0.4+ | Tray filter-as-you-type | Sub-10ms filter on 100+ repos |
| chrono | 0.4+ | last_seen, last_commit_at | Prioritization signals |
| directories | 5.x | XDG config/data paths | macOS `~/Library/Application Support/workpot` |
| thiserror + anyhow | 1.x / 1.x | Error handling in core | User-facing CLI errors vs internal |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| cargo-nextest | Fast Rust tests | Run in CI for core crate |
| cargo-deny | License/advisory gate | Before distributing .app |
| Xcode CLT | macOS signing/notarization | Required for tray app distribution |

## Installation

# Prerequisites

# Tauri CLI

# Project bootstrap (when scaffold exists)

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| Tauri 2 tray | Electron + menubar | Only if team is JS-only; heavier RAM, worse fit for git subprocess orchestration |
| git2 | `git` subprocess per repo | Simpler v0 spike; does not scale past ~30 repos (process spawn cost) |
| SQLite | JSON index file | Spike only; loses query performance for tags/search |
| Rust shared core | Swift menu bar only | If abandoning CLI; loses cross-surface reuse |
| nucleo fuzzy | fuse.js in webview | If tray UI is fully web-driven; Rust-side filter keeps IPC minimal |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| Periodic full `find` scans | CPU + battery on large trees | Watch roots + incremental index |
| Embedding libgit2 in Node (isomorphic-git) | Slower on large repo sets; duplicate logic vs CLI | Rust git2 in shared core |
| Cloud sync / Firebase | Violates local-only; adds auth | SQLite + optional export file |
| Spotlight-only discovery | Unreliable for bare repos, worktrees, custom layouts | Explicit watch roots + `.git` detection |
| Multiple IDE integrations in v1 | Scope creep | Cursor via `cursor` CLI only |

## Version Verification

- Tauri 2 tray: https://v2.tauri.app/learn/system-tray/
- git2 crate: https://docs.rs/git2/latest/git2/
- Cursor CLI: `cursor --help` (ships with Cursor.app)

<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->

## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->

## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->

## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->

## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:

- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->

## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
