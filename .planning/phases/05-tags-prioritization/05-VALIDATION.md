---
phase: 5
slug: tags-prioritization
status: draft
nyquist_compliant: false
wave_0_complete: true
created: 2026-05-31
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Vitest 3.x (frontend) + cargo test / cargo-nextest (Rust) |
| **Config file** | `vite.config.ts` (test section), `Cargo.toml` workspaces |
| **Quick run command** | `npm test && cargo test -p workpot-core` |
| **Full suite command** | `npm test && cargo test --workspace` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `npm test && cargo test -p workpot-core`
- **After every plan wave:** Run `npm test && cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 05-01-01 | 01 | 0 | ORG-01,04 | — | N/A | infra | `cargo test -p workpot-core org` | ✅ `tests/org_test.rs` | ✅ green |
| 05-01-02 | 01 | 0 | ORG-01 | — | N/A | infra | `npm test -- tagFilter` | ✅ `src/lib/tagFilter.test.ts` | ✅ green |
| 05-01-03 | 01 | 0 | ORG-02 | — | N/A | infra | `npm test -- pinOrder` | ✅ `src/lib/pinOrder.test.ts` | ✅ green |
| 05-02-01 | 02 | 1 | ORG-01 | — | N/A | unit | `cargo test -p workpot-core --test org_test` | ✅ `tests/org_test.rs` | ✅ green |
| 05-02-02 | 02 | 1 | ORG-02 | — | N/A | unit | `cargo test -p workpot-core --test org_test` | ✅ `tests/org_test.rs` | ✅ green |
| 05-02-03 | 02 | 1 | ORG-04 | — | N/A | unit | `cargo test -p workpot-core --test org_test` | ✅ `tests/org_test.rs` | ✅ green |
| 05-03-01 | 03 | 1 | ORG-03 | — | N/A | unit | `npm test -- sort` | ✅ `sort.test.ts` | ✅ green |
| 05-03-02 | 03 | 1 | ORG-02 | — | N/A | unit | `npm test -- trayList` | ✅ `trayList.test.ts` | ✅ green |
| 05-03-03 | 03 | 1 | ORG-01 | — | N/A | unit | `npm test -- tagFilter` | ✅ `tagFilter.test.ts` | ✅ green |
| 05-04-01 | 04 | 3 | ORG-01 | — | N/A | unit | `npm test -- fuzzy` | ✅ extend `fuzzy.test.ts` | ✅ green |
| 05-04-02 | 04 | 3 | ORG-04 | — | N/A | unit | `npm test -- fuzzy` | ✅ extend `fuzzy.test.ts` | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `crates/workpot-core/tests/org_test.rs` — stubs for ORG-01 (tag CRUD), ORG-02 (pin mutations), ORG-04 (notes CRUD)
- [x] `src/lib/tagFilter.test.ts` — stubs for `#tag` parse and AND filter logic
- [x] `src/lib/pinOrder.test.ts` — stubs for `pinOrder` drag-reorder array helper

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Right-arrow detail pane opens/closes with keyboard | ORG-01, ORG-02, ORG-04 | DOM focus/keyboard interaction | Open tray, arrow-key to repo, press right arrow, verify detail pane appears; press Esc/left arrow, verify closes |
| Drag-to-reorder pins updates order visually | ORG-02 | HTML5 drag requires real pointer events | Pin 3 repos, drag middle repo to top, verify `pin_order` update in DB |
| Context menu (right-click) pin and tag actions | ORG-01, ORG-02 | Tauri MenuEvent + webview wiring | Right-click repo in tray, select "Add tag", verify tag appears; select "Pin", verify moves to Pinned section |
| Tag autocomplete dropdown on `#` keystroke | ORG-01 | UI interaction with dropdown | Type `#` in filter bar, verify dropdown shows existing tags; arrow-key to select |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
