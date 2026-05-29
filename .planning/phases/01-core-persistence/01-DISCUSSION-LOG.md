# Phase 1: Core & persistence - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-29
**Phase:** 01-core-persistence
**Areas discussed:** Retroactive locks, UAT paths

---

## Retroactive locks

| Option | Description | Selected |
|--------|-------------|----------|
| core+CLI only | No Tauri stubs in Phase 1 | ✓ |
| add stubs now | Empty Tauri/ui in workspace | |
| minimal CLI | paths + repo add\|list\|remove only | ✓ |
| expand CLI | More verbs now | |
| lock schema | path PK, source, excluded for Phase 2 | ✓ |
| narrow schema | Strip forward columns | |
| fs git check | .git / bare only, no git2 | ✓ |
| git2 now | libgit2 in Phase 1 | |
| empty watch_roots | Minimal default config | |
| sample ~/code + ~/dev | Seed existing dirs on first run | ✓ |
| AppContext only | Production entry via open() | ✓ |
| split APIs | Expose raw store to hosts | |
| anyhow errors | Human messages, no exit taxonomy yet | ✓ |
| structured errors | Stable codes now | |
| CI gate only | DATA-02 via workflow, script optional | ✓ |
| lock script + CI | Permanent standalone script contract | |
| canonical path PK | Always canonicalize for DB key | ✓ |
| user path as stored | No canonicalization | |
| bare + worktree | Both registration forms | ✓ |
| worktree only | Reject bare | |
| hard DELETE remove | Phase 2 adds exclude-on-remove | ✓ |
| soft exclude now | excluded=1 on remove | |

**Notes:** Sample watch roots implemented in `default_config()`; aligns with D-09 and UAT test 2.

---

## UAT paths

| Option | Description | Selected |
|--------|-------------|----------|
| fix UAT | Expect ~/.config/workpot/config.toml | ✓ |
| fix code | Move config to Application Support | |
| yes DB path | Application Support/workpot.db | ✓ |
| yes bootstrap | workpot paths creates config+DB+migrations | ✓ |
| yes offline | Manual + CI sufficient for DATA-02 | ✓ |
| fix UAT in session | Update 01-UAT.md test 2 now | ✓ |
| assert seeded roots | UAT checks watch_roots when dirs exist | ✓ |
| exact path prefixes | Document expected printed paths | ✓ |
| UAT gate | All 7 tests must pass to close Phase 1 | ✓ |

**User's choice:** Align UAT with D-01; Phase 1 completion requires UAT pass.

---

## Claude's Discretion

(none in this session)

---

## Deferred Ideas

- Phase 2 handoff and completion-bar areas were not discussed in this update (user stopped after UAT paths).
