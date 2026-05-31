# Local dev (fast, may auto-fix)
clean:
    cargo clean
    npm run clear

# CLI release + tray bundle (macOS)
build:
    cargo fetch
    cargo build --release -p workpot-cli
    npm ci
    npm run tauri:build

install: build
    cargo install --path crates/workpot-cli -q

launch: build
    npm run tauri dev

# Rewrite formatting (run before clippy / tests)
fmt:
    cargo fmt --all -q
    npm run lint
    npm run format

# Strict fmt — CI parity; run after fmt if you want to verify
fmt-check:
    cargo fmt --all -- --check

fix: build fmt coverage
    cargo clippy --workspace --fix --allow-dirty --allow-staged --all-targets -q -- -D warnings
    # Disabled until Tauri 3 stable — transitive GTK3/unic advisories (macOS v1). See CONTRIBUTING.md.
    # cargo deny check --config .github/ci-assist/deny.toml
    # cargo audit
    npm run check
    npm run test:coverage

# One-time: `just coverage-tools` (crate is cargo-llvm-cov; needs llvm-tools-preview)
coverage-tools:
    rustup component add llvm-tools-preview
    cargo install cargo-llvm-cov --locked

coverage:
    cargo llvm-cov test -q -p workpot-core -p workpot-cli --all-targets --lcov --output-path lcov-core-cli.info
    cargo llvm-cov test -q -p workpot-tray --all-targets --lcov --output-path lcov-tray.info

# precommit: build + check (no cargo deny/audit until Tauri 3 — see CONTRIBUTING.md) + fmt-check
precommit: build fix fmt-check
    ./target/release/workpot --version

# Sync version from repo-root version file into all manifests and lockfiles
version:
    bash scripts/sync-version.sh

# Verify manifests match version file (no writes)
version-check:
    bash scripts/sync-version.sh --check
