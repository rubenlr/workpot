# Contributing to Workpot

## Dev setup

```bash
git clone https://github.com/rubenlr/workpot
cd workpot
cargo build --workspace
cargo test --workspace
```

Rust 1.96 is required (pinned in `rust-toolchain.toml`).

## Required gates before every PR

All must pass — CI enforces them:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
bash scripts/check-no-network-deps.sh
cargo deny check --config .github/ci-assist/deny.toml
```

Install optional policy tools:

```bash
cargo install cargo-deny cargo-audit
```

Run the full local check recipe:

```bash
just check
```

## Pull requests and releases

- **Squash merge only** into `master` (branch ruleset).
- **PR title + description → squash commit** on `master` (repo setting, not ruleset). One-time setup (admin):

  ```bash
  bash scripts/configure-github-merge-defaults.sh
  ```

  Or: **Settings → General → Pull requests** → *Allow squash merging* → **Default to pull request title and description**.

- Write the **PR title** in [Conventional Commits](https://www.conventionalcommits.org/) form (`feat:`, `fix:`, `feat!:`, …). That title becomes the squash commit subject Release Please parses. Put `BREAKING CHANGE:` in the PR body when needed.
- CI **semantic-pr** checks the PR title before merge.
- Do not bump `Cargo.toml` or edit `CHANGELOG.md` on feature PRs. **release-please** opens a Release PR for the version bump and changelog.

See [docs/releasing.md](docs/releasing.md).

## Versioning and deprecation policy

This project follows [Semantic Versioning](https://semver.org/):

- **Patch** (`x.y.Z`): `fix:` commits.
- **Minor** (`x.Y.0`): `feat:` commits (while on `0.y.z`, Release Please uses pre-1.0 rules from `.github/ci-assist/release-please-config.json`).
- **Major** (`X.0.0`): breaking changes (`feat!:`, `fix!:`, or `BREAKING CHANGE:` in the body).

Breaking changes only ship in major (or pre-1.0 minor, per Release Please) releases. Call out deprecations in commit bodies; remove deprecated APIs no sooner than the following major release.
