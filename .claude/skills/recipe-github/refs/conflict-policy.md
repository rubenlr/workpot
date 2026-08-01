# Safe conflict-resolve policy

Used when `merge-one.sh` exits `2`. Cap: **8** conflicted files; above that → escalate.

## Auto (apply without asking)

- **Lockfiles only** (`pnpm-lock.yaml`, `Cargo.lock`): do not hand-merge markers. Checkout one side, regenerate (`pnpm install`, cargo build / lock refresh). Never commit markers in locks.
- **Identical / whitespace-only**: take either; normalize later with `just fmt`.
- **Manifest union (non-overlapping)**: `package.json` / `Cargo.toml` hunks bump **different** packages — keep both; validate parse.

## Agent may apply if clearly matched

- Import-order / formatting-only after semantic union
- Changelog/docs: concatenate unique lines; no invented notes

## Must escalate (stop branch; keep remote)

- Application logic conflicts (Rust / TS / Svelte) with behavioral or same-symbol divergence
- Security-sensitive paths (auth, crypto, CI secrets, entitlements, signing)
- Same dependency bumped to **two different versions**
- Lock regenerate still broken / verify cannot start
- Agent not highly confident — default escalate
- > 8 conflicted files, or generated + hand-written without clear regen path

## Hard stops

- Never leave conflict markers in a commit
- Never blanket `--ours` / `--theirs` on source trees
- Unresolved markers in tree = fatal
