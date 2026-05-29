
clean:
    cargo clean

build:
    cargo build -p workpot-cli

install: build
    cargo install --path crates/workpot-cli
