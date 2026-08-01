# Scripts contract

All scripts live under `scripts/recipe-github/`. Invoke from repo root.

## Shared rules

- `set -euo pipefail`; source `lib.sh`
- stdout: **one final JSON object** (and optional progress on stderr)
- Agent chat: paste ≤50 lines of that JSON; on verify fail, also last ~80 lines of the failing step log
- Do not dump full `gh`/`git`/`just` output into context on success

## Exit codes

| Code | Meaning                                                                        |
| ---- | ------------------------------------------------------------------------------ |
| `0`  | Success (including empty branch list)                                          |
| `1`  | Fatal (dirty tree, auth, not a repo, verify hard-fail setup)                   |
| `2`  | Partial / needs agent or human (conflicts, escalate, verify failed for branch) |

## Scripts

| Script                           | Role                                                                                                      |
| -------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `list-dependabot-branches.sh`    | `git fetch --prune`; list `origin/dependabot/**` short names (nested refs; not one-level `*`)             |
| `merge-one.sh <branch>`          | Attempt `git merge --no-ff origin/<branch>`; `0` merged, `2` conflicts + paths, `1` fatal                 |
| `verify-full.sh`                 | `just fmt` → `just pre` → `just test`; fail-fast                                                          |
| `delete-remotes.sh <branch>…`    | `git push origin --delete` for listed branches                                                            |
| `capture-deprecation-signals.sh` | Count deprecated **API symbols** in our usage (`deprecated` / `symbols`); not package-deprecation notices |
| `measure-trigger-sites.sh`       | Sum trigger-site counts: `--deprecation-sites N --build-sites N` → `trigger_sites` (**not** LoC)          |

## Example branch result shape

```json
{
  "action": "merge-dependabot",
  "branch": "dependabot/npm_and_yarn/foo-1.2.3",
  "merge": "ok",
  "conflict_paths": [],
  "commits": { "merge": "abc", "deprecations": "def" },
  "verify": { "fmt": "pass", "pre": "pass", "test": "pass" },
  "deleted": true,
  "deprecated": 1,
  "trigger_sites": 2,
  "escalate": null
}
```

`trigger_sites` = deprecation call sites updated + build-break sites fixed (see `refs/report-table.md`). Report table columns: see `refs/report-table.md`.
