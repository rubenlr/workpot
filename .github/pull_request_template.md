## What changed

<!-- One paragraph or bullet list: the observable behaviour before vs. after. -->

## Why

<!-- The motivation: bug fix, new feature, performance, correctness. Link related issue if any. -->

## Release (squash → master)

Squash merges are configured to use **this PR title + description** as the commit on `master` (run `scripts/configure-github-merge-defaults.sh` once per repo if not set yet).

| | Convention |
|---|------------|
| **PR title** | [Conventional Commits](https://www.conventionalcommits.org/) subject — e.g. `feat: add repo fuzzy rank`, `fix: handle bare repos`, `feat!: drop legacy index` |
| **PR description** | Details, test notes; put `BREAKING CHANGE: …` here for majors |

GitHub appends ` (#123)` to the title in the squash commit; Release Please accepts that.

Do not edit `CHANGELOG.md` or `Cargo.toml` on feature PRs — **release-please** opens a separate Release PR.

## Test plan

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] `cargo test --workspace --all-targets` passes
- [ ] `bash scripts/check-no-network-deps.sh` passes
- [ ] `cargo deny check` passes (if installed)
- [ ] Manual test: <!-- describe what you ran and what you observed -->

## Notes for reviewers

<!-- Anything tricky, a design decision you made, or areas you'd like extra scrutiny on. -->
