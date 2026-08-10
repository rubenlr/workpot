# Live merge smoke checklist

Run before the first real Dependabot batch on a throwaway branch.

## Dry-run (safe)

```bash
bash scripts/recipe-github/list-dependabot-branches.sh
# expect: valid JSON, count >= 0

bash scripts/recipe-github/capture-deprecation-signals.sh
# expect: valid JSON with deprecated (int) + symbols[]; exit 0

bash scripts/recipe-github/measure-trigger-sites.sh --deprecation-sites 2 --build-sites 0
# expect: trigger_sites=2 (not LoC); exit 0

# Do NOT run verify-full on every PR CI — it is the full local gate (fmt+pre+test).
# Optional once on a clean tree when validating the skill:
# bash scripts/recipe-github/verify-full.sh
```

## Live batch

1. Create/switch to integration branch; ensure clean tree
2. `list-dependabot-branches.sh` — confirm expected set
3. Invoke skill `recipe-github` / follow `refs/workflows/merge-dependabot.md`
4. Per branch: merge → conflict agent? → commit1 → deprecations → commit2? → verify-full (`just fmt`→`just pre`→`just test`) → metrics → delete?
5. If any delete failed: fix tree → `just pre` → re-run `delete-remotes.sh` for failed branches
6. Summary table must include `deprecated` + `trigger sites` (`refs/report-table.md`)
7. Confirm remotes gone: `git fetch --prune && list-dependabot-branches.sh`
8. Ship: `git push -u origin HEAD` + `gh pr create` (default; not optional)

## Abort

- Escalate / verify fail: leave commits; keep remote; fix manually or `git merge --abort` if still mid-merge
- Never `delete-remotes` without verify pass for that branch
- Delete recovery exhausted (`just pre` still red) → stop before push/PR
