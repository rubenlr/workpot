# Workflow: merge-dependabot

TodoWrite (`merge: false`) with these ids before step 1. Mark each completed before advancing.

| todo_id           | Step                                                                                                 | Done when                                                               |
| ----------------- | ---------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| `md-preflight`    | Clean tree; `gh auth status`; on intended branch                                                     | Preflight JSON ok or stop fatal                                         |
| `md-list`         | `bash scripts/recipe-github/list-dependabot-branches.sh`                                             | Branch list JSON; empty → success stop                                  |
| `md-branch-loop`  | For each short branch name below                                                                     | All branches processed or escalated                                     |
| `md-merge`        | `bash scripts/recipe-github/merge-one.sh <branch>`                                                   | Exit 0 (merged) or 2 (conflicts) or 1 stop                              |
| `md-conflict`     | If exit 2: load `conflict-policy.md` + spawn agent per `agents/conflict-resolve.md`                  | `resolved` → finish merge commit; `escalate` → skip delete, next branch |
| `md-commit1`      | Ensure merge commit exists (`--no-ff`); no deprecation edits in this commit                          | Merge SHA recorded                                                      |
| `md-deprecations` | `capture-deprecation-signals.sh`; spawn `agents/deprecation-migrate.md` if `deprecated`>0 or symbols | Commit 2 created or skipped; escalate stops delete                      |
| `md-verify`       | `bash scripts/recipe-github/verify-full.sh`                                                          | All steps pass; on fail escalate (keep remote)                          |
| `md-metrics`      | `measure-trigger-sites.sh --deprecation-sites <agent TRIGGER_SITES> [--build-sites N]`               | Per-branch `deprecated` + `trigger_sites` (not LoC)                     |
| `md-delete`       | On verify pass: `bash scripts/recipe-github/delete-remotes.sh <branch>`                              | Remote deleted or delete error reported once                            |
| `md-summary`      | Aggregate per-branch JSON; emit table per `refs/report-table.md`                                     | Table shown (incl. deprecated + trigger sites); Next Up                 |

## Per-branch order (strict)

```
merge-one → [conflict agent?] → commit1 → [deprecation agent?] → verify-full → metrics → [delete?]
```

- Commit 1 = plain merge only
- Commit 2 = API deprecation migrations only (optional) — methods/functions/classes, not package notices
- `md-metrics` after verify (or after escalate decision): sum agent `TRIGGER_SITES` (call sites) + any build-break sites fixed — **not** LoC
- Delete remote **only** after verify PASS for that branch
- Do not auto-push the integration branch
- Branch-delete may auto-close Dependabot PRs; do not require `gh pr merge`

## Smoke / live checklist

Follow `refs/smoke-checklist.md`. Dry-run `list-dependabot-branches.sh` before the first live batch.
