# Phase 5: Tags & prioritization - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-30
**Phase:** 5-Tags & prioritization
**Areas discussed:** Tag editing UX, Pinned section layout, Prioritization model, Notes UX

---

## Tag editing UX

| Option | Description | Selected |
|--------|-------------|----------|
| CLI-only | workpot tag add <repo> <tag> — tray just displays + filters | |
| Tray + CLI | Right-arrow detail pane editable, Cmd+Click removes, right-click menu | ✓ |
| Config file | Manual TOML editing only | |

**User's choice:** Tray + CLI — right-arrow opens full detail pane (branches read-only, tags editable, notes, pin toggle); Cmd+Click on a tag removes it; right-click context menu has Add/Remove tag actions.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Type #tag in filter bar | Integrates with existing filter-as-you-type | |
| Tag chips below filter bar | Clickable chips, separate from text search | |
| Both — type or click chip | #tag syntax + visible chips (hidden until # typed) | ✓ |

**User's choice:** Both — type #tag or click chip. Chips hidden until user types #.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Hidden until #typed | Chips only appear when user types # | ✓ |
| Always visible below filter bar | Always shown, takes persistent vertical space | |
| Show only active/used tags | Chips for tags on at least one indexed repo | |

**User's choice:** Hidden until #typed.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Separate repo_tags table | Normalized, queryable, supports rename/delete | ✓ |
| JSON array column on repos | Simpler migration, harder to filter with SQL | |
| Claude decides | | |

**User's choice:** Separate repo_tags table.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Branches + tags | Branch list + editable tags | |
| Branches only | Right arrow = branch navigator, tags separate | |
| Full detail: branches, tags, notes, pin | Rich detail panel with all org metadata | ✓ |

**User's choice:** Full detail — branches (read-only), tags (editable), notes, pin toggle.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Read-only navigation | No git writes, v1 read-only | ✓ |
| Out of scope | Branch list not included in Phase 5 | |
| Checkout on select | Out of scope per PROJECT.md | |

**User's choice:** Read-only navigation.

---

| Option | Description | Selected |
|--------|-------------|----------|
| AND | Repo must have all active tags | ✓ |
| OR | Repo must have any active tag | |
| Claude decides | | |

**User's choice:** AND.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — dropdown of existing tags | Shows on # typed | ✓ |
| No autocomplete | | |
| Claude decides | | |

**User's choice:** Yes — dropdown autocomplete on #.

---

## Pinned section layout

| Option | Description | Selected |
|--------|-------------|----------|
| Separate labeled 'Pinned' section at top | Visual divider + header | ✓ |
| Pins float to top, no divider | No visual separator | |
| Pin badge on row only, no reordering | Stays in organic position | |

**User's choice:** Separate labeled Pinned section at top.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Via right-arrow detail pane only | Already decided | |
| Right-click context menu + detail pane | Both mechanisms | ✓ |
| CLI only | | |

**User's choice:** Right-click context menu + detail pane toggle.

---

| Option | Description | Selected |
|--------|-------------|----------|
| No cap — show all pinned | | |
| Cap at N (configurable) | max_pinned in config.toml | ✓ |
| Claude decides | | |

**User's choice:** Configurable cap (max_pinned).

---

| Option | Description | Selected |
|--------|-------------|----------|
| Pin order = last_pinned_at desc | Most recently pinned at top | |
| Manual drag-to-reorder | User sets custom order | ✓ |
| Same signals as rest | Dirty/recent within Pinned section | |

**User's choice:** Manual drag-to-reorder within Pinned section.

---

| Option | Description | Selected |
|--------|-------------|----------|
| No — pins follow the filter | Pins hidden if they don't match active tag | ✓ |
| Yes — Pinned section ignores tag filters | Pins always visible | |

**User's choice:** Pins follow the filter.

---

| Option | Description | Selected |
|--------|-------------|----------|
| pin_order INTEGER column on repos | Simple column, sort by | ✓ |
| Separate pins table with order | More normalized, extra join | |
| Claude decides | | |

**User's choice:** pin_order INTEGER column on repos table.

---

| Option | Description | Selected |
|--------|-------------|----------|
| CLI too — workpot pin <repo> | Phase 5 includes CLI pin | |
| Tray-only in Phase 5 | CLI pin deferred to Phase 6 | ✓ |

**User's choice:** Tray-only in Phase 5.

---

## Prioritization model

| Option | Description | Selected |
|--------|-------------|----------|
| Fixed tier sections: Dirty / Recent / Rest | Simple, predictable, glanceable | ✓ |
| Numeric score (configurable weights) | Flexible but opaque | |
| Single sorted list, no sections | No labeled sections | |

**User's choice:** Fixed tier sections.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Configurable window, default 7 days | | |
| Top N most recently opened | | |
| Configurable days with min/max floor | max_recent_days + min_recent_count | ✓ |

**User's choice (freeform):** "recent list with configurable days; list has min and max configuration; the query list last repos with max, then trim above max days without leaving the list with less than the minimum; minimum doesn't respect last N days." — Algorithm: query within max_recent_days, pad to min_recent_count if below minimum.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — subtle section headers | Gray labels for all sections | ✓ |
| No labels — visual spacing only | | |
| Only Pinned gets a label | | |

**User's choice:** Yes — all four sections (Pinned, Dirty, Recent, Rest) get subtle gray section headers.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Dirty section wins | Dirty + recent → goes to Dirty | ✓ |
| Recent section wins | | |
| Claude decides | | |

**User's choice:** Dirty wins.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Rest section | NULL last_opened_at → Rest | ✓ |
| Recent with NULL as oldest | | |
| Separate 'New' section | | |

**User's choice:** Rest section.

---

## Notes UX

| Option | Description | Selected |
|--------|-------------|----------|
| Detail pane — already decided | Right-arrow pane | ✓ |
| CLI + detail pane | Phase 5 adds CLI note command too | |
| CLI only in Phase 5 | | |

**User's choice:** Detail pane (already decided from full-detail pane selection).

---

| Option | Description | Selected |
|--------|-------------|----------|
| Inline textarea — click to edit | Blur to save | ✓ |
| Read-only with Edit button | Two-step | |
| Always editable textarea | | |

**User's choice:** Inline textarea, click to edit.

---

| Option | Description | Selected |
|--------|-------------|----------|
| notes TEXT column on repos | Simple nullable column | ✓ |
| Separate notes table | Overkill for one-note-per-repo | |
| Claude decides | | |

**User's choice:** notes TEXT column on repos table.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Notes included in fuzzy filter automatically | No special syntax | ✓ |
| Explicit note: prefix | | |
| Claude decides | | |

**User's choice:** Included in fuzzy filter automatically.

---

**User's freeform notes constraints:** "no markdown support; 3 lines min of notes; 5max; max length 500 chars; when lose focus it saves; no button necessary; no rollback"

---

## Claude's Discretion

- `max_pinned` default value (suggest 5)
- `max_recent_days` default (suggest 14) and `min_recent_count` default (suggest 3)
- Within-section sort order (suggest `last_opened_at DESC`; `registered_at DESC` for Rest)
- Drag-to-reorder implementation library choice (Svelte)
- `pin_order` re-sequencing strategy after drag (sparse gaps ok)
- Tailwind color tokens for section headers
- Tag chip styling in autocomplete dropdown
- Detail pane slide-in animation

## Deferred Ideas

- Full right-click context menu scope — user flagged "this right click menu on repo needs to be more explored later, it might have other features to add there." Phase 5 minimum: Pin/Unpin + Add/Remove tag.
- CLI pin/unpin commands — Phase 6.
- CLI tag/note commands — Phase 6.
- View mode submenu (from Phase 4 deferred) — partially addressed by section model; may not be needed.
