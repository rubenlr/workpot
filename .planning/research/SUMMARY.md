# Project Research Summary

**Project:** Workpot
**Domain:** macOS local multi-repo git launcher
**Researched:** 2026-05-28
**Confidence:** HIGH

## Executive Summary

Workpot fits the "developer launcher" category (Raycast repos, GitHub Desktop list, custom `proj` scripts) but narrows to **local git awareness + Cursor launch** without cloud or IDE sprawl. Experts build this as a **thin native shell (Tauri tray) over a Rust core** with SQLite index and libgit2 for scalable status reads.

Recommended approach: ship **vertical MVP slices** — index → git status → tray finder → launch — before recipes and advanced prioritization. Biggest risks are over-scanning filesystems, subprocess-per-repo git, and duplicating logic between CLI and tray; all mitigated by `.git`-only discovery, git2, and a shared `workpot-core` crate.

## Key Findings

### Recommended Stack

Tauri 2 + Rust shared core + SQLite + git2 + clap CLI. Tray webview stays thin; fuzzy filter and ranking live in Rust.

**Core technologies:**
- **Tauri 2:** macOS menu bar / tray host
- **git2:** Batch-friendly git state without shelling out
- **SQLite:** Tags, pins, recipes, repo metadata

### Expected Features

**Must have (table stakes):**
- Index repos under watch roots + manual add/exclude
- Branch + dirty (+ ahead/behind) per repo
- Fuzzy metadata search
- Open in Cursor from tray
- Background refresh

**Should have (competitive):**
- Signal-based prioritization + manual pins/tags
- CLI parity
- Recipes (shell + launch + multi-step)

**Defer (v2+):**
- Cross-repo code search
- PR/CI integrations
- Non-Cursor IDEs

### Architecture Approach

Single `workpot-core` library consumed by Tauri and CLI. Indexer + GitReader write SQLite; Ranker serves tray/CLI queries; Launcher/RecipeRunner handle actions.

**Major components:**
1. **Indexer** — discovery and watch
2. **GitReader** — status refresh
3. **RepositoryStore** — SQLite persistence

### Critical Pitfalls

1. Scanning non-repo directories — use `.git` detection only
2. `git` subprocess per repo — use libgit2
3. CLI/tray logic drift — shared core crate
4. Cursor not on PATH — detect app bundle
5. Unsafe recipes — explicit cwd, no silent destructive defaults

## Implications for Roadmap

| Finding | Roadmap Impact |
|---------|----------------|
| Shared Rust core | Phase 1 establishes crate + DB before any UI |
| Tray MVP is v1 daily driver | Phase 4 delivers finder+launch before recipes |
| git2 batching | Git phase before tray relies on fresh status |
| Recipes are high complexity | Last phase; TOML-defined, no GUI editor v1 |
| macOS-only | No cross-platform phases in v1 |

## Sources

- Tauri 2 system tray docs
- Prior art: Raycast, Alfred workflows, `ghq`/`mise` patterns
- PROJECT.md user decisions (2026-05-28)

---
*Research synthesis for Workpot initialization*
