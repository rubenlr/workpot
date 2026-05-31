---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: Ready to plan
last_updated: "2026-05-31T21:00:00.000Z"
progress:
  total_phases: 9
  completed_phases: 7
  total_plans: 42
  completed_plans: 37
  percent: 79
---

# Project State

## Project Reference

See: `.planning/PROJECT.md` (updated 2026-05-28)

**Core value:** Know which repo you need and open it in Cursor in seconds, with git context visible first.

**Current focus:** Phase 06.2 — tray-ux-polish

## Phase Status

| Phase | Name | Status |
|-------|------|--------|
| 1 | Core & persistence | Complete (2026-05-28) |
| 2 | Repo discovery | Complete — 5/5 plans (2026-05-29) |
| 3 | Git state | Complete — 3/3 plans, UAT 10/10 (2026-05-30) |
| 4 | Tray finder MVP | Complete — 4/4 plans (2026-05-30) |
| 5 | Tags & prioritization | Shipped — PR https://github.com/rubenlr/workpot/pull/4 (2026-05-31) |
| 6 | CLI parity | Complete — 5/5 plans, UAT 5/6 auto (2026-05-31) |
| 06.1 | Release & distribution | Not started — inserted 2026-05-31 |
| 06.2 | Tray UX polish | In progress — 3/9 plans (2026-05-31) |

## Session Notes

- Phase 06.2 plan 06.2-02 (2026-05-31): has_stale_dirty + 16-case tests; TDD RED `9252f6d`, GREEN `6e1aefc`
- Phase 06.2 plan 06.2-01 (2026-05-31): migration 007 alias, org::set_alias, RepoDto.alias, TrayConfigDto.stale_dirty_days; commits `5767d80`, `a64345a`, `49ff05f`, `61bf197`
- Phase 6 UAT auto (2026-05-31): `cargo test -p workpot-core -p workpot-cli` green; list/search/open CLI smoke verified
- Phase 5 shipped (2026-05-31): PR https://github.com/rubenlr/workpot/pull/4
- Phase 5 gap 05-09 (2026-05-31): tag blur-save, duplicate feedback, allTags refresh; commits `dbacbbb`, `e359e42`, `a01eb99`
- Phase 5 plan 05-08 (2026-05-31): `allow-org-commands` — commits `1070e7a`, `ffd36e4`
- Phase 5 plan 05-06 (2026-05-31): +page.svelte four-section tray, detail pane, tags, context menu, pin reorder; commit `ac9ac08`
- Phase 5 plan 05-07 (2026-05-31): CLI `workpot tag` add/remove/list; commit `6ac7b8e`
- Phase 5 plan 05-04 (2026-05-31): Tauri org IPC, RepoDto fields, context menu event; commits `abfe95c`, `a7aa876`
- Phase 5 plan 05-05 (2026-05-31): DetailPane, TagChip, TagAutocomplete, SectionHeader; commits `b50fa4d`, `f4d65f5`, `c97f574`
- Phase 5 wave 1 (2026-05-31): 05-02 Rust org layer; 05-03 TS sectionSort; commits `045b6a5`, `7304a9e`, `a7570b3`
- Phase 4 shipped (2026-05-30): PR https://github.com/rubenlr/workpot/pull/3
- Phase 3 shipped (2026-05-30): PR https://github.com/rubenlr/workpot/pull/2
- Phase 2 shipped (2026-05-29): PR https://github.com/rubenlr/workpot/pull/1

## Decisions

- Index tests use `git init`; discovery uses ignore `filter_entry` with `Arc<Mutex>` (02-02)
- repo remove persists exact path + `/**` tree glob for reliable discovery skip (02-04)
- Cap pre-check before transaction; cap_exceeded audit only in `run_full` outer match (03-UAT / CR-01)
- Git refresh: rayon batch outside DB tx; `open_and_query` expects pre-canonicalized path (03-02/03)
- Migration 005 for `last_opened_at` (04-01)
- Tauri org IPC requires explicit `allow-org-commands` capability (05-08; same class as 04 `get_tray_config`)
- [Phase ?]: Updater failures map to Network=2 and Install=1 for deterministic CLI exits.
- [Phase ?]: Checksum verification is enforced before any CLI or tray replacement step.
- [Phase 06.1]: Installer now enforces checksum-first staging for CLI tarball and DMG before install writes.
- [Phase 06.1]: Installer smoke tests use local fixture metadata/assets for deterministic default, flag, global, and checksum-failure verification.
- [Phase 06.2-01]: Alias is user-only (scan upsert does not touch alias); Config.stale_dirty_days default 7 added with plan 01 for TrayConfigDto IPC.
- [Phase 06.2-02]: has_stale_dirty uses injectable now_secs; never-opened dirty repos use i64::MAX age (immediate stale).

## Accumulated Context

### Roadmap Evolution

- Phase 06.1 inserted after Phase 6: Release distribution and install: GitHub tarballs, install.sh, DMG, workpot update, INSTALL.md (URGENT)
- Phase 06.2 inserted after Phase 6: Tray UX polish — icons, panel chrome, alias, list/detail interaction, Storybook (2026-05-31 explore)
- Phase 7 (Recipes) deferred to backlog as 999.1 (2026-05-31) — prioritize 06.1 release path first

### Pending Todos

None — install/update and DMG scope live in [Phase 06.1](phases/06.1-release-distribution-and-install-github-release-tarballs-sta/06.1-CONTEXT.md).

## Blockers

None.
