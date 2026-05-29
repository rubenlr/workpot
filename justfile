
clean:
    cargo clean

build:
    cargo build -p workpot-cli

install: build
    cargo install --path crates/workpot-cli

check:
    cargo fmt --all -q
    cargo clippy --workspace --fix --allow-dirty --allow-staged --all-targets -- -D warnings
    cargo test --workspace --all-targets -q
    bash scripts/check-no-network-deps.sh
    cargo deny check

precommit: build check

# Rust components and cargo binaries required by `just check`.
install-deps:
    rustup component add rustfmt clippy
    cargo fetch
    cargo install cargo-deny cargo-audit --locked
