# Stack Research

**Domain:** macOS menu-bar git workspace launcher (tray + CLI, local-only)
**Researched:** 2026-05-28
**Confidence:** HIGH (core stack verified via official Tauri docs, crates.io, GitHub releases)

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| **Tauri** | `2.11.2` | Desktop shell, tray, webview popup, IPC | Default choice for small native macOS utilities in 2025–2026: native WebView, Rust core, `tray-icon` API in v2 (`TrayIconBuilder`), hide-on-close pattern. MSRV `1.77.2` per [crates.io/tauri](https://crates.io/crates/tauri). |
| **Rust (stable)** | `≥ 1.77.2` (pin `1.85+` in `rust-toolchain.toml`) | Shared engine for tray + CLI | One codebase for indexing, git state, search, recipes. Align with Tauri MSRV; pin toolchain for reproducible CI. |
| **SolidJS** | `^1.9` | Tray popup UI (list + filter-as-you-type) | Fine-grained reactivity without VDOM churn on every keystroke—better fit than React for a Raycast-style filter list. Official Tauri scaffolds support multiple frontends; Solid keeps bundle small. **Confidence: MEDIUM** (ecosystem choice; Tauri is agnostic). |
| **Vite** | `^6.0` | Frontend build/dev | Standard Tauri 2 frontend toolchain; fast HMR for popup UX iteration. |
| **TypeScript** | `^5.8` | Frontend types | Matches Tauri 2 templates and `@tauri-apps/api` typings. |
| **Node.js** | `22 LTS` | Tooling only | Tauri CLI/plugin install; not in runtime bundle. |
| **pnpm** | `10.x` | JS package manager | Efficient monorepo for `ui/` + Tauri app; matches common 2026 Tauri project layout. |

### Rust Workspace (`workpot-core` + binaries)

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| **git2** | `0.21.0` | Local git status (dirty, branch, ahead/behind) | Battle-tested libgit2 bindings; status/branch APIs match user mental model of `git status`. Pure-Rust **gix** (`0.84+`) is faster but higher integration cost—defer unless full reindex proves too slow. **Confidence: HIGH** for git2 choice on v1 scope. |
| **rusqlite** | `0.39.0` + feature `bundled` | Repo index, tags, notes, recipe metadata, refresh timestamps | Single embedded DB for tray + CLI; `bundled` avoids macOS SQLite drift. Official crate docs recommend `bundled` for app-owned databases. |
| **nucleo-matcher** | `0.3.1` | Fuzzy metadata search | Helix-grade matcher; ~6× faster than `skim`/`fuzzy-matcher` on typical patterns; Unicode-safe. Use matcher only in core (CLI + Tauri invoke), not full TUI `nucleo` crate. |
| **ignore** | `0.4.25` | Watch-root traversal | Ripgrep’s walker: respects `.gitignore`, fast parallel walks—critical when scanning `~/src` trees. |
| **notify** | `8.2.0` | Filesystem events | Cross-platform watcher; on macOS uses FSEvents via `macos_fsevent` feature path. |
| **notify-debouncer-full** | `0.7.0` | Debounced re-index triggers | Coalesces burst saves/git ops into one refresh; macOS rename stitching. Pairs with `notify ^8.2`. |
| **clap** | `4.6.1` | CLI (`workpot` binary) | Standard Rust CLI; derive API for `search`, `open`, `index refresh`, `recipe run`. |
| **tokio** | `1.52.3` | Background indexing / watcher tasks | Async worker pool for scan + git status without blocking UI; Tauri already depends on tokio. |
| **serde** / **serde_json** | `1.0` / `1.0.150` | Config + IPC DTOs | De facto serialization; recipe definitions as JSON in SQLite or sidecar files. |
| **toml** | `0.8` | User config (`watch_roots`, excludes) | Human-editable local config; fits “power user” CLI editing. |
| **directories** | `6.0.0` | `~/.config/workpot`, data dir | XDG-style paths on macOS without hardcoding `HOME`. |
| **thiserror** | `2.0` | Library error types | Clean error surfaces from `workpot-core` to CLI/Tauri. |
| **tracing** + **tracing-subscriber** | `0.1` | Debug/logging | Structured logs for index refresh and recipe execution (no telemetry). |

### Tauri Plugins (official v2)

| Plugin | Version | Purpose | Why Recommended |
|--------|---------|---------|-----------------|
| **tauri-plugin-shell** | `2.x` (match Tauri minor) | Recipe shell steps | Official, capability-scoped process spawn; required for “open terminal / run script” recipes. |
| **tauri-plugin-global-shortcut** | `2.3.2` | Global hotkey to open finder popup | Table stakes for launcher UX (Raycast/Alfred pattern). [Official plugin docs](https://v2.tauri.app/plugin/global-shortcut/). |
| **tauri-plugin-store** | `2.4.3` | UI prefs only (window position, theme) | Small key-value prefs; **do not** duplicate repo index here—keep index in SQLite via `workpot-core`. |

### Cursor Integration

| Mechanism | Version | Purpose | Why Recommended |
|-----------|---------|---------|-----------------|
| **`cursor` CLI** | User-installed (Cursor app) | Open repo in Cursor | v1 requirement: `cursor /path/to/repo` or `cursor -r` for reuse. Detect via `which cursor` / `Command::new("cursor")`; surface clear error if missing. Not a Cargo dep. **Confidence: HIGH** ([Cursor forum/docs](https://forum.cursor.com/t/how-to-open-cursor-from-terminal/3757)). |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| **@tauri-apps/cli** `2.11.x` | Build/dev/sign | Pin to same minor as `tauri` crate (`2.11.2`). |
| **cargo-nextest** | Rust tests | Faster than `cargo test` for core crate. |
| **rustfmt** + **clippy** | Rust quality | Workspace `clippy.toml`; deny `unwrap` in core paths. |
| **Biome** or **ESLint** | Frontend lint | Pick one; Biome is lighter for small Solid UI. |
| **GitHub Actions** | macOS-only CI | `macos-latest` build + `cargo test` + `pnpm tauri build` on tags. |

## Workspace Layout (architecture-driving)

```
workpot/
├── crates/
│   └── workpot-core/      # index, git, search, recipes (no Tauri deps)
├── crates/
│   └── workpot-cli/       # clap binary → depends on workpot-core
├── src-tauri/             # Tauri app → depends on workpot-core
└── ui/                    # Solid + Vite
```

**Why:** CLI and tray must share identical indexing/search/git logic. Tauri commands are thin wrappers over `workpot-core`—never duplicate git or SQL in the frontend.

## Installation

```bash
# Prerequisites (macOS)
# Rust ≥ 1.77.2, Node 22, Xcode CLT
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
npm install -g pnpm@10

# Scaffold (greenfield)
pnpm create tauri-app@latest workpot --template solid-ts
cd workpot

# Pin Tauri/Rust deps (src-tauri/Cargo.toml excerpt)
# tauri = { version = "2.11.2", features = ["tray-icon"] }
# tauri-plugin-shell = "2"
# tauri-plugin-global-shortcut = "2.3"
# tauri-plugin-store = "2.4"

# Core crate (crates/workpot-core/Cargo.toml excerpt)
# git2 = "0.21"
# rusqlite = { version = "0.39", features = ["bundled"] }
# nucleo-matcher = "0.3"
# ignore = "0.4"
# notify = "8.2"
# notify-debouncer-full = "0.7"
# tokio = { version = "1.52", features = ["rt-multi-thread", "macros", "sync"] }
# clap = { version = "4.6", features = ["derive"] }
# serde = { version = "1", features = ["derive"] }
# directories = "6"
# thiserror = "2"
# tracing = "0.1"

# Frontend (ui/package.json excerpt)
# solid-js ^1.9, vite ^6, typescript ^5.8
# @tauri-apps/api ^2.11, @tauri-apps/plugin-global-shortcut ^2.3

# Add plugins
pnpm tauri add shell global-shortcut store
```

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| Tauri 2.11 | **Electron** | Only if team is 100% TS and rejects Rust—rejected: bundle size, RAM, security surface for a personal launcher. |
| Tauri 2.11 | **Native Swift (MenuBarExtra)** | If UI never needs web tech and team is Swift-only—rejected: PROJECT already chose Tauri; recipes + shared CLI logic favor Rust core. |
| git2 0.21 | **gix 0.84+** | Reindex of 500+ repos exceeds budget (e.g. >3s cold refresh)—migrate hot paths first (status-only), keep git2 for edge cases during transition. |
| rusqlite bundled | **sled** / **redb** | Never for this domain—SQLite is the standard for queryable metadata + tags. |
| nucleo-matcher | **skim** / **fuzzy-matcher** | Legacy codebase already on skim—otherwise no; skim is slower and weak on Unicode. |
| Solid + Vite | **Svelte 5 + Vite** | Team preference or existing Svelte skill—equally valid for Tauri 2; pick one, don’t mix. |
| workpot-core + CLI | **git subprocess parsing** | Emergency fallback only (`git -C … status --porcelain`); avoid as primary—parsing fragility and process spawn cost on 50+ repos. |
| SQLite index | **tauri-plugin-sql** | If all persistence is UI-driven only—rejected: CLI must read same DB without Tauri runtime. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| **Tauri 1.x** | `SystemTray` API removed/replaced; dead end for new projects. | Tauri **2.11.x** + `tray-icon` feature |
| **Electron** | ~100MB+ bundles, high idle RAM for a menu-bar utility | Tauri 2 |
| **git CLI as primary index API** | Slow at scale; brittle porcelain parsing | **git2** in `workpot-core` |
| **gix for v1** unless perf proven | Larger API surface; slows v1 ship | **git2** first; benchmark before switch |
| **skim / fuzzy-matcher** | Superseded on perf and Unicode | **nucleo-matcher** |
| **walkdir alone** | Ignores `.gitignore`; over-scans `node_modules`, etc. | **ignore** |
| **Raw notify without debouncer** | Index thrash on save bursts | **notify-debouncer-full** |
| **Cloud DB (Supabase, etc.)** | Violates local-only constraint | **rusqlite** on disk |
| **VS Code / `code` CLI** | Out of v1 scope (Cursor-only) | **`cursor` CLI** |
| **sea-orm / Diesel** | ORM overhead for one embedded DB | **rusqlite** + hand-written queries |
| **React for popup only** | Heavier rerenders on each keypress vs Solid/Svelte | **Solid** or **Svelte** |
| **Tauri JS-side git/status** | Duplicates logic; CLI can’t share | **Rust `workpot-core`** |
| **Keyring / OAuth crates** | No accounts in v1 | N/A until remote features |

## Stack Patterns by Variant

**If tray popup must feel native (no web chrome):**
- Frameless, transparent, small NSPanel-style window (`decorations: false`, `alwaysOnTop`, `skipTaskbar` in `tauri.conf.json`)
- `TrayIconBuilder::on_tray_icon_event` → show/focus popup; `CloseRequested` → `hide()` + `prevent_close()`
- macOS: `icon_as_template(true)` for menu bar icon

**If CLI-only scripting is first-class:**
- Ship `workpot-cli` as separate binary in same repo; both binaries link `workpot-core`
- Optional: `tauri-plugin-cli` for dev-only—**not** a substitute for real `clap` binary

**If watch roots are huge (10k+ dirs):**
- `ignore::WalkParallel` for discovery; cap depth; persist “is repo” in SQLite
- Debounce filesystem events ≥ 500ms; incremental status refresh (mtime check before `git2` open)

**If recipes need isolation:**
- Run shell recipes via `tauri-plugin-shell` with explicit cwd = repo root
- Never `shell:allow-execute` without scoped capabilities per recipe ID

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `tauri 2.11.2` | `tauri-build 2.6.2`, `tauri-cli 2.11.x` | Lockstep from [tauri-v2.11.2 release](https://github.com/tauri-apps/tauri/releases/tag/tauri-v2.11.2) |
| `tauri 2.11.2` | `@tauri-apps/api ^2.11` | Mismatch causes IPC/schema errors |
| `notify 8.2.0` | `notify-debouncer-full 0.7.0` | Debouncer crate pins `notify ^8.2` |
| `tauri-plugin-* 2.x` | `tauri 2.11.x` | Use `pnpm tauri add` to align versions |
| `rusqlite 0.39 bundled` | SQLite 3.51.x (bundled) | No system SQLite on macOS required |
| `git2 0.21` | libgit2 1.9.x (bundled via crate) | Requires Xcode CLT on macOS for build |
| `workpot-core` | No Tauri dependency | Keeps CLI build fast and testable without webview |

## Confidence by Recommendation

| Area | Level | Notes |
|------|-------|-------|
| Tauri 2.11 + tray | **HIGH** | Official v2 tray docs + release tags verified |
| git2 for v1 git state | **HIGH** | crates.io + docs.rs; matches PROJECT requirements |
| rusqlite + bundled | **HIGH** | Official crate guidance for desktop apps |
| nucleo-matcher | **HIGH** | crates.io + Helix production use |
| ignore + notify stack | **HIGH** | Standard ripgrep/fsevent pattern on macOS |
| Solid over React | **MEDIUM** | Valid DX tradeoff; Svelte equally defensible |
| gix deferral | **MEDIUM** | Performance hypothesis; validate with benchmark phase |
| Vite 6 / Node 22 | **MEDIUM** | Ecosystem standard; exact minor pins low risk |

## Sources

- [/websites/v2_tauri_app](https://v2.tauri.app/) — system tray (`TrayIconBuilder`), plugins (shell, global-shortcut), security headers — **HIGH**
- [tauri v2.11.2 release](https://github.com/tauri-apps/tauri/releases/tag/tauri-v2.11.2) — version pins — **HIGH**
- [crates.io: tauri](https://crates.io/crates/tauri), [git2](https://crates.io/crates/git2), [rusqlite](https://crates.io/crates/rusqlite), [nucleo-matcher](https://crates.io/crates/nucleo-matcher), [ignore](https://crates.io/crates/ignore), [notify](https://crates.io/crates/notify), [clap](https://crates.io/crates/clap), [tokio](https://crates.io/crates/tokio) — versions verified 2026-05-28 — **HIGH**
- [docs.rs/gix 0.84.0](https://docs.rs/crate/gix/latest) — alternative git stack — **HIGH**
- [helix-editor/nucleo](https://github.com/helix-editor/nucleo) — fuzzy matcher benchmarks — **HIGH**
- [Cursor forum: shell command](https://forum.cursor.com/t/how-to-open-cursor-from-terminal/3757) — IDE launch — **MEDIUM**
- [tauri-plugin-store 2.4.3](https://crates.io/crates/tauri-plugin-store) — prefs only — **HIGH**

---
*Stack research for: Workpot — macOS multi-repo git workspace launcher*
*Researched: 2026-05-28*
