---
name: recipe-github
description: Orchestrates GitHub maintenance for this repo — batch-merge origin/dependabot branches with safe conflict resolution, full local verify gates, two-commit merge+deprecation migration, and remote cleanup. Use when adopting Dependabot updates or other recipe-github actions listed in disambiguation.
---

# GitHub maintenance (recipe-github)

## Purpose

Orchestrate GitHub maintenance for Workpot via **scripts** (mechanical loops + JSON summaries), **workflow refs** (step order), and **bounded subagents** (conflict resolve, deprecation migrate). Keep the orchestrator thin: run a script, read ≤50 lines of JSON, branch on exit code — never invent multi-step `gh`/`git` loops in chat.

## When to use

- Batch-merge `origin/dependabot/**` (nested, e.g. `dependabot/npm_and_yarn/eslint-10.8.0`) into the current branch with verify + deprecation cleanup
- Future actions listed in `refs/disambiguation.md`

## When not to use

- Shipping a single feature PR (`gsd-ship` / review-and-ship)
- Rewriting `.github/dependabot.yml`
- Force-push or history rewrite
- Non-GitHub remotes
- Ignoring verify failures to “just delete” remotes

## Procedure

1. **Intake** — Classify action via `refs/disambiguation.md`. Done when action id is known.
2. **Load workflow** — Read `refs/workflows/<action>.md`. Create TodoWrite from its step ids (`merge: false`).
3. **Execute** — Follow the workflow. Run scripts under `scripts/recipe-github/`; load policy/agent refs only when the step requires them.
4. **Escalate** — On `escalate` from conflict/deprecation agent or verify FAIL: stop that branch, keep remote, report JSON + reason. Do not guess.
5. **Close** — Emit final run summary table per `refs/report-table.md` (includes `deprecated` + `trigger sites`). Offer Next Up (push / open PR). Do **not** auto-push unless the user asked.

## Progressive disclosure

| Subtask                  | Load                                                               |
| ------------------------ | ------------------------------------------------------------------ |
| Intake / routing         | `refs/disambiguation.md`                                           |
| Script I/O + exit codes  | `refs/scripts-contract.md`                                         |
| merge-dependabot steps   | `refs/workflows/merge-dependabot.md`                               |
| Conflict handling        | `refs/conflict-policy.md`, `refs/agents/conflict-resolve.md`       |
| Deprecation commit 2     | `refs/deprecation-policy.md`, `refs/agents/deprecation-migrate.md` |
| Final run table          | `refs/report-table.md`                                             |
| Dry-run / live checklist | `refs/smoke-checklist.md`                                          |

## Orchestration

- Multi-step: TodoWrite ids from the active workflow ref before step 1.
- Spawn conflict-resolve / deprecation-migrate agents only when the workflow says so; pass script JSON + policy path — not the whole repo.
- Prefer `scripts/recipe-github/*.sh` over ad-hoc shell.

## Stop rules

- Dirty working tree at preflight
- `gh` auth / not a git repo / script exit `1` (fatal)
- Conflict markers left in tree
- Conflict or deprecation agent returns `escalate`
- `verify-full.sh` FAIL after commit 1+2 for a branch
- User abort

## Validity

Frontmatter `name` equals folder name. Body stays thin; policies live in refs.
