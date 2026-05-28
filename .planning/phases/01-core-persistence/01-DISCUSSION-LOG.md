# Phase 1: Core & persistence - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-28
**Phase:** 1-Core & persistence
**Areas discussed:** macOS paths (config vs data)

---

## macOS paths (config vs data)

### Config file location

| Option | Description | Selected |
|--------|-------------|----------|
| ~/.config/workpot/config.toml | XDG-style; dotfile-manageable | ✓ (split) |
| ~/Library/Application Support/workpot/config.toml | Single Apple-native tree | |
| Split | Config in ~/.config, DB in Application Support | ✓ |

**User's choice:** Split — config in `~/.config/workpot/`

### Database location

| Option | Description | Selected |
|--------|-------------|----------|
| ~/Library/Application Support/workpot/workpot.db | Standard macOS app data | ✓ |
| ~/.local/share/workpot/workpot.db | XDG data dir | |
| Same as config | One folder to backup | |

**User's choice:** `~/Library/Application Support/workpot/workpot.db`

### Environment overrides

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — data and optionally config | Override via env for tests/power users | |
| Yes — data dir only | Config stays fixed | |
| No in Phase 1 | Fixed paths only | ✓ |

**User's choice:** No env overrides in Phase 1

### First-run behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Create defaults | Default config.toml + empty migrated DB | ✓ |
| Create DB only | Config on first config command | |
| Require init | Explicit `workpot init` required | |

**User's choice:** Create default config + empty DB on first launch

---

## Claude's Discretion

Bootstrap layout, CLI surface, and schema width left to planner per ARCHITECTURE.md and ROADMAP success criteria.

## Deferred Ideas

- Env path overrides — future phase when tests need them
- Bootstrap / CLI / schema gray areas — not selected for discussion this session
