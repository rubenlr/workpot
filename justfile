# Local dev (fast, may auto-fix)
clean:
    cargo clean
    npm run clear

# CLI release + tray bundle (macOS)
build:
    cargo fetch
    cargo build --release -p workpot-cli
    npm ci
    CI=true npm run tauri build

install: build
    cargo install --path crates/workpot-cli -q

launch:
    npm run tauri dev

# Rewrite formatting (run before clippy / tests)
fmt:
    cargo fmt --all -q

# Strict fmt — CI parity; run after fmt if you want to verify
fmt-check:
    cargo fmt --all -- --check

check: fmt
    cargo clippy --workspace --fix --allow-dirty --allow-staged --all-targets -q -- -D warnings
    cargo test --workspace --all-targets -q
    # Disabled until Tauri 3 stable — transitive GTK3/unic advisories (macOS v1). See CONTRIBUTING.md.
    # cargo deny check --config .github/ci-assist/deny.toml
    # cargo audit
    npm run format:check
    npm run lint
    npm run check
    npm test

coverage:
    cargo llvm-cov test -q --workspace --all-targets --lcov --output-path lcov.info

# precommit: build + check (no cargo deny/audit until Tauri 3 — see CONTRIBUTING.md) + fmt-check
precommit: build check fmt-check
    ./target/release/workpot --version
