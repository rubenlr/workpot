# Action routing

Load at intake. Pick the narrowest action id.

| Signal                                                                          | Action id          | Workflow                                |
| ------------------------------------------------------------------------------- | ------------------ | --------------------------------------- |
| Merge / adopt dependabot branches, batch dep updates, “merge origin/dependabot” | `merge-dependabot` | `workflows/merge-dependabot.md`         |
| (stub) Close stale PRs, watch checks, rebase dependabot                         | —                  | Not implemented in v1 — say so and stop |

## Default assumptions

- Remote name: `origin`
- Branch glob: `dependabot/**` on `origin` (after `git fetch --prune`) — nested ecosystems, e.g. `dependabot/npm_and_yarn/eslint-10.8.0`
- Target: current HEAD branch
- No auto-push of the integration branch

## Ambiguous

If user says “update deps” without naming Dependabot → assume `merge-dependabot` and state it.

## Unimplemented

If they ask for a stub action → stop: “Not in recipe-github v1; only merge-dependabot is available.”
