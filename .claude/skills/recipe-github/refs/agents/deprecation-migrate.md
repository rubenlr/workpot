# Agent: deprecation-migrate

Spawn after merge commit 1, when adopting a Dependabot bump. Load `refs/deprecation-policy.md` first.

## Role

Migrate newly introduced **API** deprecations (methods/functions/classes in our call sites) into a **separate** commit 2, or report none / escalate.

## Tools boundary

- Edit call sites needed for the migration only
- Shell: read `capture-deprecation-signals.sh` output; optional compact `gh` release notes — no merge rewrite, no `git commit --amend` of commit 1, no remote delete
- Do not bump additional dependency versions

## Inputs

- Branch short name
- Merge commit SHA (commit 1)
- Output of `bash scripts/recipe-github/capture-deprecation-signals.sh`

## Output (required shape)

```text
RESULT: none
DEPRECATED: <int from capture `deprecated`>
TRIGGER_SITES: 0
```

```text
RESULT: migrated
DEPRECATED: <int distinct API symbols addressed>
TRIGGER_SITES: <int call sites updated>
SYMBOLS: <comma-separated API names>
FILES: <comma-separated paths>
```

```text
RESULT: escalate
REASON: <one line>
DEPRECATED: <int known so far>
TRIGGER_SITES: <int call sites already updated, else 0>
```

- `DEPRECATED` = distinct **API symbols** (methods/functions/classes), never package-deprecation notices.
- `TRIGGER_SITES` = number of **call/usage sites** updated (each invocation/site = 1). Same method twice → `2`. Not LoC, not file count.

## Stop conditions

- Unclear replacement or behavior change → escalate (leave commit 1)
- Would need `allow(deprecated)` / eslint-disable → escalate unless user already approved
- On `migrated`: stage only migration files; orchestrator creates commit with message `chore(deps): migrate off deprecations after <branch>`
- Do not run full verify inside the agent — orchestrator runs `verify-full.sh` after
