# Run report table (md-summary)

Emit one markdown table at Close. One row per Dependabot branch processed (including escalated).

## Columns (fixed order)

| Column        | Source                                          | Meaning                                                                                         |
| ------------- | ----------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Branch        | merge-one `branch`                              | Short `dependabot/…` name                                                                       |
| Merge         | merge-one `merge` + short SHA                   | `ok` / `already` / `conflicts`→resolved / `fatal`                                               |
| Conflicts     | merge-one / conflict agent                      | `—` or brief resolve note                                                                       |
| Deprecations  | commit 2 present?                               | `none` / migrate SHA / `escalate`                                                               |
| Verify        | verify-full                                     | `pass` / failed step                                                                            |
| Deleted       | delete-remotes                                  | `yes` / `no`                                                                                    |
| deprecated    | `capture-deprecation-signals.sh` → `deprecated` | Count of **API symbols** (methods / functions / classes) newly deprecated in **our call sites** |
| trigger sites | `measure-trigger-sites.sh` → `trigger_sites`    | Count of **trigger sites** updated (see semantics) — **not** LoC                                |

## Semantics (non-negotiable)

- **deprecated** is about **code usage of dependency APIs** (names / signatures), not “this npm/crate package is deprecated.”
- **trigger sites** = number of **discrete trigger sites** updated because of the bump:
  - Each **call / usage** of a deprecated method/function/class that was migrated counts as **1**, whether the same symbol or different symbols.
  - Each **build-break site** fixed (one diagnostic / one broken usage that had to change for the tree to compile/typecheck) counts as **1**.
  - Example: one deprecated method used in two places → `deprecated` may be `1`, `trigger sites` is `2`. Class length before/after is irrelevant.
- **Never** use lines-of-code, hunk size, or file count as `trigger_sites`.
- Integers only; source of truth is agent-reported site counts aggregated by `measure-trigger-sites.sh`. If unknown, put `?` and escalate — do not guess from diffs.

## Example

```markdown
| Branch                            | Merge        | Conflicts | Deprecations      | Verify | Deleted | deprecated | trigger sites |
| --------------------------------- | ------------ | --------- | ----------------- | ------ | ------- | ---------- | ------------- |
| dependabot/npm_and_yarn/foo-1.2.3 | ok `abc1234` | —         | migrate `def5678` | pass   | yes     | 1          | 2             |
```
