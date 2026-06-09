# Local dev (fast, may auto-fix)
clean:
    cargo clean
    pnpm run clean

# CLI release + tray bundle (macOS)
build:
    cargo fetch
    cargo build --release -p workpot-cli
    pnpm install --frozen-lockfile
    pnpm run tauri:build

install: build
    cargo install --path crates/workpot-cli -q

# Tray dev only (no release DMG — use `just build` for bundles).
# Git refresh loading is tray-icon only (no panel spinner).
# Trace tray: RUST_LOG=workpot_tray_lib=debug,workpot_core=debug just launch
# Trace CLI: RUST_LOG=workpot_core=debug,workpot_cli=debug workpot index
# Webview: right-click panel → Inspect → Console ([workpot-tray] lines)
launch:
    RUST_LOG=workpot_tray_lib=debug,workpot_core=debug pnpm run tauri dev

# Rewrite formatting (run before clippy / tests)
fmt-fix:
    cargo fmt --all -q
    cargo fix --workspace --allow-dirty --allow-staged --all-targets -q
    pnpm run lint
    pnpm run format

# Strict fmt — CI parity; run after fmt if you want to verify
fmt-check:
    cargo fmt --all -- --check
    pnpm run lint:check
    pnpm run format:check
    pnpm run check
    cargo clippy --workspace --fix --allow-dirty --allow-staged --all-targets -q -- -D warnings

# CI test-macos job — cargo/vitest/coverage/bundle only (`fmt-check` covers format/lint/svelte-check)
test:
    cargo fetch
    pnpm install --frozen-lockfile
    cargo test -p workpot-core -p workpot-cli -p workpot-tray --all-targets
    pnpm run test:coverage
    CI=true pnpm run tauri:build

fix: fmt-fix

alias fmt := fmt-fix

# One-time: `just coverage-tools` (crate is cargo-llvm-cov; needs llvm-tools-preview)
coverage-tools:
    rustup component add llvm-tools-preview
    cargo install cargo-llvm-cov --locked

coverage:
    cargo llvm-cov test -q -p workpot-core -p workpot-cli --all-targets --lcov --output-path lcov-core-cli.info
    cargo llvm-cov test -q -p workpot-tray --all-targets --lcov --output-path lcov-tray.info

# Pre-push: release build + fmt/clippy (CI `fmt` job on macOS). Tests: `just test` (CI `test-macos`).
# No cargo deny/audit until Tauri 3 — see CONTRIBUTING.md.
pre: build fix fmt-check
    ./target/release/workpot --version

alias precommit := pre

# Sync version from repo-root version file into all manifests and lockfiles
version:
    bash scripts/sync-version.sh

# Verify manifests match version file (no writes)
version-check:
    bash scripts/sync-version.sh --check
