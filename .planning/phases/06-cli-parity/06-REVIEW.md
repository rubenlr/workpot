---
phase: 06-cli-parity
reviewed: 2026-05-31T00:00:00Z
depth: deep
files_reviewed: 12
files_reviewed_list:
  - crates/workpot-cli/src/list_display.rs
  - crates/workpot-cli/src/main.rs
  - crates/workpot-cli/tests/cli_smoke.rs
  - crates/workpot-core/Cargo.toml
  - crates/workpot-core/src/lib.rs
  - crates/workpot-core/src/services/launch.rs
  - crates/workpot-core/src/services/mod.rs
  - crates/workpot-core/src/services/repo_fuzzy.rs
  - crates/workpot-core/src/services/repo_priority.rs
  - crates/workpot-core/tests/repo_fuzzy_test.rs
  - crates/workpot-core/tests/repo_priority_test.rs
  - src-tauri/src/launch.rs
findings:
  critical: 2
  warning: 4
  info: 3
  total: 9
status: issues_found
---

# Phase 06: Code Review Report

**Reviewed:** 2026-05-31
**Depth:** deep
**Files Reviewed:** 12
**Status:** issues_found

## Summary

This phase introduces CLI parity for `workpot list`, `workpot search`, `workpot open`, and related
subcommands. The implementation is structurally sound: the shared-core design is preserved, error
handling is generally explicit, and the fuzzy algorithm ports cleanly. However, two correctness bugs
exist — a byte-count vs char-count mismatch in the DoS guard and a duplicated, divergent
sorting implementation — plus several quality issues that will quietly produce wrong output or
confusing UX.

---

## Critical Issues

### CR-01: `fuzzy_score` DoS guard compares byte length to a char-count constant

**File:** `crates/workpot-core/src/services/repo_fuzzy.rs:80`

**Issue:** The guard `if q.len() > MAX_QUERY_LEN` uses Rust's `str::len()`, which returns the
*byte* length, not the Unicode scalar count. `MAX_QUERY_LEN` is `256`. For a query composed of
2-byte characters (e.g., accented Latin, Greek, Cyrillic), a 129-character query has a byte length
of 258 and is silently rejected even though it is well under the 256-*character* limit. Conversely,
a 256-byte string that consists entirely of 1-byte ASCII is accepted, which matches the intent but
makes the contract unclear and fragile when multi-byte scripts are involved.

The TS original measures `query.length` which is UTF-16 code-unit length; the Rust port is
inconsistent with both the TS source and with the tag validation in `main.rs:323`, which correctly
uses `.chars().count()`.

**Fix:**
```rust
// repo_fuzzy.rs line 80 — replace
if q.len() > MAX_QUERY_LEN {
// with
if q.chars().count() > MAX_QUERY_LEN {
```

---

### CR-02: Two independent sort implementations with diverging `Rest` sort order

**File:** `crates/workpot-cli/src/list_display.rs:124` vs `crates/workpot-core/src/services/repo_priority.rs:137`

**Issue:** `list_display::flat_tray_ordered_with_icons` (used by the CLI's `list` and `search`
commands) sorts the Rest section case-insensitively:
```rust
rest.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
```
`repo_priority::section_sort` (used by the tray and exported from the core) sorts it
case-sensitively:
```rust
rest.sort_by(|a, b| a.name.cmp(&b.name));
```

With repos whose names mix upper and lower case (e.g., `Zoo`, `apple`, `Banana`), the CLI and the
tray will produce different orderings. This directly violates the CLI-03 parity requirement and
means the golden-vector tests for `repo_priority` do not cover the CLI output path at all; the CLI
uses `list_display`, not `repo_priority`. Neither module re-uses the other's logic.

Additionally, `by_last_opened_desc` in `list_display.rs:149` tie-breaks by lowercase name, while
`cmp_last_opened_desc` in `repo_priority.rs:46` tie-breaks by raw name. These will diverge on
mixed-case repos with the same timestamp.

**Fix:** One of the two implementations must be deleted. The CLI should call into
`repo_priority::section_sort` + `flat_tray_ordered`, adding icon assignment as a thin wrapper
rather than re-implementing the full sectioning. Example:

```rust
// list_display.rs — replace flat_tray_ordered_with_icons body
pub fn flat_tray_ordered_with_icons(
    repos: Vec<RepoRecord>,
    config: &Config,
    now_secs: i64,
) -> Vec<(RepoRecord, &'static str)> {
    let sectioned = workpot_core::services::repo_priority::section_sort(&repos, config, now_secs);
    let mut result = Vec::new();
    for r in sectioned.pinned   { result.push((r, priority_icon(PrioritySection::Pinned)));  }
    for r in sectioned.dirty    { result.push((r, priority_icon(PrioritySection::Dirty)));   }
    for r in sectioned.recent   { result.push((r, priority_icon(PrioritySection::Recent)));  }
    for r in sectioned.rest     { result.push((r, priority_icon(PrioritySection::Rest)));    }
    result
}
```

---

## Warnings

### WR-01: `pin_order` sentinel value differs between `list_display` and `repo_priority`

**File:** `crates/workpot-cli/src/list_display.rs:75` vs `crates/workpot-core/src/services/repo_priority.rs:62`

**Issue:** `list_display` uses `i64::MAX` as the sentinel for `pin_order = None`:
```rust
pinned.sort_by_key(|r| r.pin_order.unwrap_or(i64::MAX));
```
`repo_priority` uses `999`:
```rust
pinned.sort_by_key(|r| r.pin_order.unwrap_or(999));
```

With `i64::MAX` as sentinel, any repo with `pin_order = None` will always sort after repos with
explicit `pin_order` values, which is the correct intent. However `999` allows a repo with
`pin_order = Some(1000)` to sort *before* a None-order repo in `repo_priority` but *after* it in
`list_display`. This is a consistency defect between the two surfaces, but is unlikely to be
triggered in practice (max_pinned defaults to 5).

**Fix:** Align both to use `i64::MAX` (or define a shared `PIN_ORDER_NONE_SENTINEL` constant in
`domain`).

---

### WR-02: `resolve_launch_program` short-circuit condition is logically vacuous

**File:** `crates/workpot-core/src/services/launch.rs:28`

**Issue:**
```rust
if program != "cursor" || !is_unqualified_program(program) {
    return program.to_string();
}
```
If `program == "cursor"` is true and `is_unqualified_program("cursor")` is also true (since
`"cursor"` contains neither `/` nor `\`), the condition reduces to `false || false = false`. This
is correct as written — the guard does nothing for bare `"cursor"`.

However, if `program == "cursor"` is **false** (i.e. any other program name), the first clause is
true and the function returns early — even for a different unqualified name like `"code"`. This is
the desired behavior. The problem is that the second clause `!is_unqualified_program(program)` is
dead code whenever the first clause is true. If someone sets `launch_cmd = "cursor-nightly {path}"`
(starts with `cursor` but is not literally `"cursor"`), the guard correctly returns early via the
`!=` check. The logic is correct but the second sub-expression is never reachable and creates
confusion. If the intent was "only resolve bare `cursor`", the `is_unqualified_program` check is
redundant because the `== "cursor"` check already implies it.

**Fix:** Simplify to express the actual intent:
```rust
pub fn resolve_launch_program(program: &str) -> String {
    if program != "cursor" {
        return program.to_string();
    }
    // ... rest unchanged
```

---

### WR-03: Spawned child process is never reaped — potential zombie accumulation

**File:** `crates/workpot-core/src/services/launch.rs:76-80`

**Issue:**
```rust
Command::new(&program)
    .args(&args)
    .spawn()
    .map_err(|e| format!("failed to launch {program}: {e}"))?;
ctx.touch_last_opened_at(&repo_path).map_err(|e| e.to_string())?;
```
`spawn()` returns a `Child` handle that is immediately dropped. On Unix, dropping a `Child` without
calling `.wait()` or `.kill()` leaves the process as a zombie entry in the process table until the
parent (workpot CLI) itself exits. For a long-running tray process that opens many repos, zombie
accumulation is a real concern.

For IDE launches the intent is clearly fire-and-forget. The standard safe pattern is to call
`child.wait()` in a detached thread or to configure the process so that a double-fork effectively
daemonizes it.

**Fix:**
```rust
let mut child = Command::new(&program)
    .args(&args)
    .spawn()
    .map_err(|e| format!("failed to launch {program}: {e}"))?;
// Reap in background thread; ignore exit status (fire-and-forget IDE launch).
std::thread::spawn(move || { let _ = child.wait(); });
```

---

### WR-04: `run_open` prints an error to stderr *and* then calls `exit(2)` — double-print risk if caller also logs

**File:** `crates/workpot-cli/src/main.rs:310-314`

**Issue:**
```rust
launch_repo(&ctx, &path_key).map_err(|e| {
    eprintln!("error: {e}");
    exit(2);
})
```
`launch_repo` returns `Result<(), String>`. The closure calls `eprintln!` then `exit(2)`. Because
`exit(2)` diverges, Rust accepts the closure return type as `!` coerced to `anyhow::Error`.
The `Err` value is never actually returned to `run()` or `main()`, so the `main()` error handler
is bypassed entirely — only the `eprintln!` inside the closure fires. This is technically correct
(no double printing occurs) but the pattern is fragile: it bypasses the unified error pipeline in
`main()`, makes the code harder to test (the `exit(2)` path cannot be asserted on without process
inspection), and mixes control flow and error value concerns.

It also means this is the *only* error path in the CLI that prints with a bare `"error: "` prefix
rather than the `"{e:#}"` anyhow chain format used everywhere else.

**Fix:** Return a proper `anyhow::Error` with an exit-code annotation, or use a dedicated error
variant that the top-level `main()` can dispatch on, eliminating the inline `exit()` call:
```rust
fn run_open(identifier: &str) -> anyhow::Result<()> {
    let ctx = AppContext::open().context("failed to open workpot")?;
    let path_key = resolve_repo_identifier(&ctx, identifier)?;
    println!("opening: {path_key}");
    launch_repo(&ctx, &path_key)
        .map_err(|e| anyhow::anyhow!("launch failed: {e}"))?;
    Ok(())
}
// Handle the launch error with exit code 2 in main()
```

---

## Info

### IN-01: `validate_tag_for_add` in `main.rs` is a redundant pre-check that can silently diverge from core validation

**File:** `crates/workpot-cli/src/main.rs:317-332`

**Issue:** The CLI validates tag emptiness, length, and `#` character before calling `ctx.add_tag`.
The core's `org::normalize_tag` performs identical checks. The CLI's version uses `exit(1)` for all
three cases; the core returns `WorkpotError::InvalidInput`. There are now two places where the
validation rules live, and they can drift. For example, if the core ever adds another disallowed
character, the CLI will silently pass it to the core which will then return an error with a
different message format.

**Fix:** Remove `validate_tag_for_add` from `main.rs` and handle the `WorkpotError::InvalidInput`
from `ctx.add_tag` in the error pipeline instead.

---

### IN-02: `match_repo_path_key` re-implements path comparison using `display().to_string()` instead of `Path` equality

**File:** `crates/workpot-cli/src/main.rs:370-375`

**Issue:**
```rust
fn match_repo_path_key(repos: &[RepoRecord], identifier: &str) -> Option<String> {
    repos
        .iter()
        .find(|r| r.path.display().to_string() == identifier)
        .map(|r| r.path.display().to_string())
}
```
`Path::display()` on non-UTF-8 paths uses replacement characters. Comparing the `display()` string
to `identifier` can fail silently if the stored path has non-UTF-8 bytes. Additionally, `display()`
is called twice per matched repo. Using `r.path.to_string_lossy()` is no worse but using
`r.path.as_os_str() == OsStr::new(identifier)` would be more correct on POSIX filesystems where
paths are raw byte sequences.

**Fix:**
```rust
fn match_repo_path_key(repos: &[RepoRecord], identifier: &str) -> Option<String> {
    repos
        .iter()
        .find(|r| r.path.to_str().map_or(false, |s| s == identifier))
        .map(|r| r.path.display().to_string())
}
```

---

### IN-03: `list_display` is a dead re-export — `repo_priority` exports exist but are unused by the CLI

**File:** `crates/workpot-core/src/lib.rs:24-27` and `crates/workpot-cli/src/list_display.rs`

**Issue:** `workpot_core` re-exports `flat_tray_ordered`, `flat_tray_ordered_repos`, `section_sort`,
and `SectionedRepos` at the crate root:
```rust
pub use crate::services::repo_priority::{
    flat_tray_ordered, flat_tray_ordered_repos, section_sort, SectionedRepos,
};
```
But the CLI's `run_list` and `run_search` bypass all of these and call
`list_display::flat_tray_ordered_with_icons` — a private re-implementation. The exported symbols
from `repo_priority` are presently unused by any Rust consumer (only the test files import them
directly). The public API surface is larger than necessary and the duplication creates the divergence
documented in CR-02.

**Fix:** After fixing CR-02 (CLI delegates to `repo_priority`), remove the duplicate
`flat_tray_ordered_with_icons` sorting logic from `list_display`. Keep `list_display` solely for
formatting (icon assignment, row formatting, `shorten_parent_dir`).

---

_Reviewed: 2026-05-31_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
