## What changed

<!-- One paragraph or bullet list: the observable behaviour before vs. after. -->

## Why

<!-- The motivation: bug fix, new feature, performance, correctness. Link related issue if any. -->

## Release (squash → master)

Squash merges use **this PR title + description** as the commit on `master`.

|                    | Convention                                                                                                                |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------- |
| **PR title**       | [Conventional Commits](https://www.conventionalcommits.org/) — e.g. `feat: add repo fuzzy rank`, `fix: handle bare repos` |
| **PR description** | Details, test notes; put `BREAKING CHANGE: …` here when needed                                                            |

### Shipping a release (optional — same PR as your work)

Only when you intend to cut a release:

1. Bump repo-root [`version`](../version) (must exceed latest `v*` tag and `master`).
2. Add `## [X.Y.Z]` with bullets to [`CHANGELOG.md`](../CHANGELOG.md).
3. Run `just version` and commit synced manifests.

CI **release-check** validates this; feature PRs without version/changelog changes skip it.

## Test plan

- [ ] `just precommit` passes (or equivalent CI-green commands on your OS).
- [ ] Manual test: <!-- describe what you ran and what you observed -->

## Notes for reviewers

<!-- Anything tricky, a design decision you made, or areas you'd like extra scrutiny on. -->
