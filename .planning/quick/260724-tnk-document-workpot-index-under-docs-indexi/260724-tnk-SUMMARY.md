---
quick_id: 260724-tnk
status: complete
completed: 2026-07-24
---

# Quick Summary: Document `workpot index`

## Outcome

Operator guide at `docs/indexing.md` covering index flow, walk/catalog edges, git refresh, CLI output/exit codes, related commands, and data location. Discovery settings in `SETTINGS.md` now link to that guide.

## Decisions

- Plain English operator doc (not agent-guide); no YAML frontmatter; no Rust phase/module names in the body
- Behavior verified against `index.rs` / `discovery.rs` / `catalog.rs` (nested stop, bare linked worktrees, orphan/manual preserve, cap-before-write)

## Key files

- `docs/indexing.md` (created)
- `SETTINGS.md` (Discovery → See also link)

## Verification

- Sections match plan outline (what / when / walk / catalog / git / output / related / data)
- Relative link `docs/indexing.md` from SETTINGS.md resolves
- No code or test changes

## Self-Check: PASSED
