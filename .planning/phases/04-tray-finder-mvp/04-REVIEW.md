---
phase: 04-tray-finder-mvp
reviewed: 2026-05-30T12:00:00Z
depth: standard
files_reviewed: 25
files_reviewed_list:
  - .github/workflows/ci.yml
  - Cargo.toml
  - crates/workpot-core/src/domain/config.rs
  - crates/workpot-core/src/domain/repo.rs
  - crates/workpot-core/src/infra/migrations.rs
  - crates/workpot-core/src/infra/migrations/005_tray.sql
  - crates/workpot-core/src/lib.rs
  - crates/workpot-core/src/services/catalog.rs
  - crates/workpot-core/tests/tray_migration_test.rs
  - package.json
  - src-tauri/capabilities/default.json
  - src-tauri/permissions/tray-commands.toml
  - src-tauri/src/commands.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/main.rs
  - src-tauri/src/tray.rs
  - src-tauri/tauri.conf.json
  - src/lib/fuzzy.ts
  - src/lib/sort.ts
  - src/lib/types.ts
  - src/routes/+page.svelte
  - src/routes/+layout.svelte
  - src/routes/+layout.ts
  - vite.config.ts
  - svelte.config.js
findings:
  critical: 0
  warning: 7
  info: 5
  total: 12
status: issues_found
---

# Phase 4: Code Review Report

**Reviewed:** 2026-05-30T12:00:00Z  
**Depth:** standard  
**Files Reviewed:** 25  
**Status:** issues_found

## Summary

Reviewed Phase 4 tray finder work through plans **04-01** and **04-02** (scaffold, IPC, migration `005_tray`, fuzzy filter, keyboard chrome). Core persistence and IPC boundaries are sound; capabilities are wired for current commands. No security-critical injection paths exist yet because launch is still a stub.

Main gaps are **stale panel data** (list/config loaded only once), **keyboard navigation asymmetry** while filtering, **CI not linting the tray crate**, and **missing `launch_cmd` validation** ahead of plan 04-04. Plans 04-03 (background git refresh) and 04-04 (Cursor launch) are not in this file set — noted as scope boundaries, not defects in reviewed code.

## Narrative Findings (AI reviewer)

### WR-01: Repo list never refreshes after first mount

**File:** `src/routes/+page.svelte:141-149`  
**Issue:** `list_repos` runs only in `onMount`. A long-lived tray app will show an empty or stale list after `workpot index`, manual register, or remove — until the process restarts. `panel-opened` only refocuses the filter input.  
**Fix:** Reload on panel open (and after future git/index events):

```typescript
async function loadRepos() {
  try {
    repos = await invoke<RepoDto[]>("list_repos");
    error = null;
  } catch (e) {
    error = String(e);
  }
}

listen("panel-opened", () => {
  void loadRepos();
  focusFilter();
});
```

---

### WR-02: ArrowUp blocked in filter field while query is non-empty

**File:** `src/routes/+page.svelte:94-122`  
**Issue:** `onFilterKeydown` handles `ArrowUp` only when `filterQuery.length === 0`. `onPanelKeydown` explicitly bails when the event target is `#repo-filter`. Result: with text in the filter, the user cannot move selection up from the input — only down (when caret is at end) or via global handlers that never run for the input.  
**Fix:** Mirror `ArrowDown` logic for `ArrowUp` (caret at start or empty query), or delegate up/down to `moveSelection` whenever the list has items:

```typescript
} else if (e.key === "ArrowUp") {
  const input = e.currentTarget as HTMLInputElement;
  const atStart =
    input.selectionStart === 0 && input.selectionEnd === 0;
  if (atStart || filterQuery.length === 0) {
    e.preventDefault();
    moveSelection(-1);
  }
}
```

---

### WR-03: `get_tray_config` failures are silently ignored

**File:** `src/routes/+page.svelte:151-157`  
**Issue:** The `.catch(() => { maxVisibleRows = 15 })` handler masks capability misconfiguration, DB lock errors, and other IPC failures — same symptom as a valid default, so production misconfig is hard to detect.  
**Fix:** Log or surface a non-blocking warning; only fall back when the error indicates missing optional config:

```typescript
.catch((e) => {
  console.warn("get_tray_config failed", e);
  maxVisibleRows = 15;
});
```

---

### WR-04: CI Clippy does not cover `workpot-tray`

**File:** `.github/workflows/ci.yml:34`  
**Issue:** `cargo clippy` runs only for `workpot-core` and `workpot-cli`. Tray-specific Rust (`src-tauri/src/*.rs`) is built on macOS but never Clippy-checked in CI — warnings and correctness issues in commands/tray can slip through.  
**Fix:** On macOS build job (or a dedicated job):

```yaml
- name: Clippy tray crate
  if: matrix.os == 'macos-latest'
  run: cargo clippy -p workpot-tray --all-targets -- -D warnings
```

---

### WR-05: `launch_cmd` not validated before plan 04-04 execution

**File:** `crates/workpot-core/src/domain/config.rs:72-98`  
**Issue:** `Config::validate` checks limits and `max_visible_rows` but not `launch_cmd`. Empty strings, missing `{path}`, or shell-metacharacter-heavy templates will only fail at spawn time (or execute unexpectedly once `launch.rs` lands).  
**Fix:** Add validation before 04-04 ships:

```rust
if self.launch_cmd.trim().is_empty() {
    return Err("launch_cmd must not be empty".into());
}
if !self.launch_cmd.contains("{path}") {
    return Err("launch_cmd must contain {path} placeholder".into());
}
```

Pair with `shell-words` parsing tests in plan 04-04.

---

### WR-06: Null CSP in tray webview config

**File:** `src-tauri/tauri.conf.json:25-27`  
**Issue:** `"csp": null` disables Content-Security-Policy for the panel webview. Acceptable for local-only MVP dev; increases blast radius if untrusted content is ever loaded (XSS → full IPC surface allowed by capabilities).  
**Fix:** Set a restrictive CSP for production builds, e.g. `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'` (adjust for Vite hashed assets).

---

### WR-07: List rows are not mouse-interactive

**File:** `src/routes/+page.svelte:201-233`  
**Issue:** Selection and open are keyboard-only. `<li>` elements have no `onclick` / `onpointerdown`. Plan 04-04 expects Cmd+Click behavior; without click handlers, basic mouse selection is missing for a finder UI.  
**Fix:** Add handlers on each row:

```svelte
onclick={() => { selectedIndex = i; }}
ondblclick={() => { selectedIndex = i; void openSelected(); }}
```

Wire modifier keys when implementing 04-04 (Cmd+Enter / Cmd+Click per D-36).

---

## Info

### IN-01: `openSelected` is an intentional stub until plan 04-04

**File:** `src/routes/+page.svelte:85-92`  
**Issue:** Enter runs `console.debug` only — no Cursor launch, no `touch_last_opened_at`. Documented in `04-02-SUMMARY.md` but user-visible no-op.  
**Fix:** Complete in plan 04-04 (`open_in_cursor`, `launch.rs`). Until then, consider disabling Enter or showing “Launch coming soon” to avoid false confidence.

---

### IN-02: Git branch/dirty display is SQLite-cached only

**File:** `src-tauri/src/commands.rs:72-78`, `src/routes/+page.svelte:141-149`  
**Issue:** Panel shows `branch` / `is_dirty` from last index/refresh. No background refresh in this slice (plan 04-03). Stale badges until index runs — expected for 04-01/02, not a bug in reviewed code.  
**Fix:** Implement 04-03 `refresh_all_git_state` + `git-refresh-complete` listener.

---

### IN-03: `console.debug` left in production UI path

**File:** `src/routes/+page.svelte:91`  
**Issue:** Debug logging on every Enter in stub path.  
**Fix:** Remove when `openSelected` is implemented.

---

### IN-04: `directories` crate version differs between core and tray

**File:** `src-tauri/Cargo.toml:21`, `crates/workpot-core/Cargo.toml` (workspace)  
**Issue:** Tray uses `directories = "6.0.0"` while core likely uses 5.x — duplicate resolution, minor API drift risk in `parent_dir_display`.  
**Fix:** Align on one workspace dependency version.

---

### IN-05: Tray icon `expect` panics if bundle icon missing

**File:** `src-tauri/src/tray.rs:37-40`  
**Issue:** `expect("bundled default window icon")` aborts startup if icons are misconfigured — acceptable for release bundles, harsh for partial dev setups.  
**Fix:** Map to `tauri::Result` and log error instead of panic in `setup_tray`.

---

## Critical Issues

None in the reviewed 04-01/04-02 scope. Security-sensitive `launch_cmd` execution is not implemented yet; address WR-05 before 04-04 merges.

---

_Reviewed: 2026-05-30T12:00:00Z_  
_Reviewer: Claude (gsd-code-reviewer)_  
_Depth: standard_
