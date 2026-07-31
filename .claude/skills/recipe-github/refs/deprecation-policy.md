# Deprecation migration policy (commit 2)

After plain merge (**commit 1**), migrate newly introduced deprecations in a **separate** commit. Never amend or fold into the merge commit.

## Detect

Prefer mechanical signals from `capture-deprecation-signals.sh` and verify output:

- Rust compiler / clippy `deprecated` notes
- JS install notices for deprecated packages; eslint deprecation if configured
- Summarized changelog / release “deprecated” / BREAKING for the bumped package (`gh` — compact, not full HTML)

Compare to pre-merge baseline when available; otherwise treat signals that **block verify** or are clearly from the bumped package as in-scope.

## Migrate rules

1. **Only new** deprecations from this bump (not unrelated pre-existing debt), unless they fail the verify gate
2. Prefer **official replacement** from that package’s docs/changelog — no speculative rewrites
3. **Fix call sites** — no `#[allow(deprecated)]` / eslint-disable unless human approves temporary
4. Commit 2 scope: deprecation migrations only (no drive-by refactors, no extra version bumps)
5. Unclear behavior / API → **escalate**; leave commit 1; do not invent migration
6. After commit 2 (or skip if none), run `verify-full.sh`; only then mark branch `merged_ok` for delete

## Commit message

```
chore(deps): migrate off deprecations after <branch-name>
```
