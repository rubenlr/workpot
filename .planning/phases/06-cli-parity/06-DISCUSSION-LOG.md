# Phase 6: CLI parity - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-31
**Phase:** 6-CLI parity
**Areas discussed:** list output format, search behavior, open matching & feedback, pin/unpin CLI scope

---

## list output format

| Option | Description | Selected |
|--------|-------------|----------|
| Section headers | Output groups with labels — `=== Pinned ===`, `=== Dirty ===`, etc. Mirrors tray visually. | |
| Flat ordered list | No section labels. Repos in priority order, no dividers. | ✓ |

**User's choice:** Flat ordered list with emoji symbol at start of each row.

**Notes:** User specified exact emoji icons: 📌 pinned, 🟡 dirty, 🔥 recent. Rest icon deferred to Claude's discretion. Row format specified as `[icon] [parent_dir] [name] [branch] [tags]`. Parent directory only (not full path), home-shortened. Emoji icons confirmed (macOS-only v1, all modern terminals support them).

---

## search behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Print-only | Filters repos by query, prints matches in priority order, exits. Composable with pipes. | ✓ |
| Interactive (fzf-style) | Live filter UI in terminal, arrow keys + Enter to open. | |

**User's choice:** Print-only.

**Fuzzy algorithm:**
| Option | Description | Selected |
|--------|-------------|----------|
| Same fuzzy algorithm as tray | True parity — same results for same query. Reuses fuzzy-matcher crate. | ✓ |
| Substring / contains match | Simpler. Results may differ from tray. | |

**#tag filter syntax:**
| Option | Description | Selected |
|--------|-------------|----------|
| Yes — support #tag syntax | `workpot search #backend` filters by tag. | |
| No — text search only | Tag filter stays tray-only for now. | ✓ |

**Notes:** Plain text search only. No #tag syntax in CLI. Same output format as `workpot list`.

---

## open matching & feedback

**Ambiguous match:**
| Option | Description | Selected |
|--------|-------------|----------|
| Error with numbered list | Print ambiguous count + numbered paths. Tell user to use full path. Exit 1. | ✓ |
| Open top-ranked match silently | Use priority order — open highest-ranked match. | |

**Success output:**
| Option | Description | Selected |
|--------|-------------|----------|
| Print confirmation | `opening: /path/to/repo` then exit 0. | ✓ |
| Silent exit 0 | No output on success. | |

**Notes:** On no match: `repo not found: <identifier>`, exit 1. On ambiguous: numbered list + "use the full path from 'workpot list'".

---

## pin/unpin CLI scope

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — ship pin/unpin in Phase 6 | D-18 deferred it here. Completes CLI parity for all org features. | |
| No — skip, Phase 6 is list/search/open only | Narrow scope. Pin stays tray-only for v1. | ✓ |

**User's choice:** Out of scope. "pin / unpin isn't necessary on CLI, out of scope."

---

## Claude's Discretion

- Rest-section emoji icon (suggest ⬜ or `·`)
- Exact column spacing / padding
- Tag display format (brackets or plain)
- Fuzzy-matcher crate selection (whichever is in workpot-core already)
- Exit code for launch failure in `workpot open`

## Deferred Ideas

- `workpot pin` / `workpot unpin` — confirmed out of scope for v1
- `workpot search` with #tag filter syntax — tray-only for now
- Interactive search TUI (fzf-style) — future v2 feature
- `workpot list --json` — not discussed; possible scripting add-on
