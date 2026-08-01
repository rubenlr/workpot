# Deprecation migration policy (commit 2)

After plain merge (**commit 1**), migrate newly introduced deprecations in a **separate** commit. Never amend or fold into the merge commit.

## Detect

Prefer mechanical signals from `capture-deprecation-signals.sh` (`deprecated` / `symbols`) and verify output:

- **In scope:** deprecated **API symbols** in our call sites — methods, functions, classes (and Rust structs/traits/types) — names and signatures flagged by clippy / tsc (or equivalent)
- **Out of scope:** “this npm/crate package is deprecated” / install-time package notices — those are not `deprecated` column counts
- Optional compact changelog peek for the bumped package only to find **replacement APIs** for symbols already flagged — not to invent package-level debt

Compare to pre-merge baseline when available; otherwise treat symbols that **block verify** or are clearly from the bumped package as in-scope.

## Migrate rules

1. **Only new** deprecations from this bump (not unrelated pre-existing debt), unless they fail the verify gate
2. Prefer **official replacement** from that package’s docs/changelog — no speculative rewrites
3. **Fix call sites** — no `#[allow(deprecated)]` / eslint-disable unless human approves temporary. Count each updated **call/usage site** as one `TRIGGER_SITES` unit (two calls to the same method → `2`). Do **not** count LoC or files.
4. Commit 2 scope: deprecation migrations only (no drive-by refactors, no extra version bumps)
5. Unclear behavior / API → **escalate**; leave commit 1; do not invent migration
6. After commit 2 (or skip if none), run `verify-full.sh`; only then mark branch `merged_ok` for delete

## Metrics

- `DEPRECATED` / capture `deprecated` = distinct symbols
- `TRIGGER_SITES` (deprecation portion) = call sites migrated
- Build-break fixes (if any, outside commit 2) add to `--build-sites` in `measure-trigger-sites.sh` the same way: one diagnostic/usage fixed = 1

## Commit message

```
chore(deps): migrate off deprecations after <branch-name>
```
