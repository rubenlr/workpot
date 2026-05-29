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

All must pass — CI enforces them and so does `bin/release`:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
bash scripts/check-no-network-deps.sh
cargo deny check
```

Install optional policy tools:

```bash
cargo install cargo-deny cargo-audit
```

Run the full local check recipe:

```bash
just check
```

## Versioning and deprecation policy

This project follows [Semantic Versioning](https://semver.org/):

- **Patch** (`x.y.Z`): bug fixes that do not change public API or on-disk format.
- **Minor** (`x.Y.0`): additive changes; new CLI subcommands, new config keys.
  Existing behaviour is preserved.
- **Major** (`X.0.0`): breaking changes including on-disk format changes without
  a migration, removal of CLI subcommands, or incompatible config schema changes.

Breaking changes only ship in major releases. Deprecated items are documented in
the CHANGELOG under `### Deprecated` and removed no sooner than the following
major release.

## Releasing

1. Ensure `[Unreleased]` in `CHANGELOG.md` has entries for this release.
2. Run `bin/release X.Y.Z` (semver without the leading `v`).
3. Review the commit and annotated tag locally.
4. Push: `git push && git push origin vX.Y.Z`

The tag push triggers `.github/workflows/release.yml`, which validates the tag
against `Cargo.toml`, builds macOS aarch64/x86_64 tarballs, and creates a GitHub
Release with checksums.

See [docs/releasing.md](docs/releasing.md) for future Tauri tray app and code
signing steps when `src-tauri/` lands.
