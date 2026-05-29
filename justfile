
clean:
    cargo clean

build:
    cargo build -p workpot-cli

install: build
    cargo install --path crates/workpot-cli

check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    bash scripts/check-no-network-deps.sh
    cargo deny check
