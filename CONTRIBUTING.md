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

CI enforces the checks in `.github/workflows/ci.yml`. Locally:

```bash
just precommit   # build (CLI release + tray bundle), check, fmt-check
```

Or step by step:

```bash
just fmt         # rewrite formatting first
just check       # clippy (may fix), tests, frontend format/lint/check/test
just fmt-check   # strict fmt — same as CI rustfmt job
just build       # macOS: release CLI + `npm run tauri build`
```

Frontend formatting and lint (also run in CI on macOS):

```bash
npm run format      # Prettier (Svelte + Tailwind plugins)
npm run format:check
npm run lint        # eslint-plugin-svelte (flat config in eslint.config.js)
npm run lint:fix
npm run check       # svelte-check (compiler types)
npm test
```

Rust style config at the repo / crate root:

- `rustfmt.toml` — edition 2024; import grouping uses unstable rustfmt options (`unstable_features = true`; needs `cargo +nightly fmt` for full effect on stable you may see warnings only).
- `crates/workpot-core/clippy.toml` — MSRV and `disallowed-methods` for `unwrap`/`expect` in **library** code; integration tests allow those via crate attribute. `workpot-cli` and `workpot-tray` are not restricted.

Install optional policy tools (for manual runs while gates are disabled):

```bash
cargo install cargo-deny cargo-audit
```

### Dependency audits (disabled until Tauri 3)

Tauri 2.x pulls an unmaintained Linux GTK3 stack and build-time `unic-*` crates via `urlpattern`. Those dependencies are not linked into the macOS `Workpot.app` (`cargo tree --target aarch64-apple-darwin -i gtk` is empty).

**Disabled in CI and `just check`:** `cargo deny` (advisories) and `cargo audit`. Re-enable after adopting stable Tauri 3 in `src-tauri/Cargo.toml` and `package.json`.

**Manual run (optional):**

```bash
cargo deny check --config .github/ci-assist/deny.toml
cargo audit
```

Expect informational unmaintained/advisory warnings until upstream fixes land.

**Re-enable checklist (Tauri 3 stable):**

1. Bump `tauri` / `@tauri-apps/*` to Tauri 3; full tray smoke on macOS.
2. Uncomment `cargo deny` / `cargo audit` in `justfile` (`check` recipe) and `.github/workflows/ci.yml` (ubuntu `fmt` job).
3. Run `cargo audit` — goal: zero or minimal unmaintained warnings without expanding `.github/ci-assist/deny.toml` ignores.
4. Trim `[advisories].ignore` in `.github/ci-assist/deny.toml` to only what remains (may add `RUSTSEC-2024-0429` for `glib` if still needed).
5. Verify `cargo tree -i gtk` and `cargo tree -i unic-ucd-ident` are clean or shrunk; run `just precommit` and confirm CI green.

References: [Tauri #11928](https://github.com/tauri-apps/tauri/issues/11928), [Tauri GTK4 PR #14684](https://github.com/tauri-apps/tauri/pull/14684).

## SonarCloud (zero-issues gate)

Analysis targets [SonarCloud project `workpot`](https://sonarcloud.io/project/configuration?id=workpot) (`rubenlr` org). Config: `.github/ci-assist/sonar-project.properties`; CI job `sonarcloud` in `.github/workflows/ci.yml` (runs after `coverage`, fails if the quality gate fails).

### One-time setup (repo admin)

1. **GitHub ↔ SonarCloud** — In [project configuration](https://sonarcloud.io/project/configuration?id=workpot), bind this repository (`rubenlr/workpot`) so PR decoration and analysis run under `workpot` (not the duplicate `rubenlr_workpot` project).
2. **`SONAR_TOKEN`** — [Account → Security](https://sonarcloud.io/account/security) → generate token → GitHub repo **Settings → Secrets → Actions** → `SONAR_TOKEN`.
3. **Quality gate “zero issues”** — Default [Sonar way](https://sonarcloud.io/organizations/rubenlr/quality_gates) allows ratings/coverage slack, not a hard zero. Create a custom gate and assign it to `workpot`:

   | Scope | Metric | Operator | Threshold |
   | --- | --- | --- | --- |
   | Overall Code | Issues | is greater than | 0 |
   | New Code | Issues | is greater than | 0 |
   | Overall Code | Vulnerabilities | is greater than | 0 |
   | New Code | Vulnerabilities | is greater than | 0 |
   | New Code | Security Hotspots Reviewed | is less than | 100 |

   Assign: [Quality Gate for workpot](https://sonarcloud.io/project/quality_gate?id=workpot) → your custom gate.

4. **Branch protection** — Ruleset `.github/rulesets/ci.json` already requires the `sonarcloud` status check on `master` PRs.

Until the first successful PR analysis, SonarCloud shows no quality gate; after that, any open issue fails CI.

## Pull requests and releases

- **Squash merge only** into `master` (branch ruleset).
- **PR title + description → squash commit** on `master` (repo setting, not ruleset). One-time setup (admin):

  ```bash
  bash scripts/configure-github-merge-defaults.sh
  ```

  Or: **Settings → General → Pull requests** → _Allow squash merging_ → **Default to pull request title and description**.

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
