default:
    @just --list

build:
    cargo build --release

test:
    cargo test

check: test
    cargo clippy --deny-warnings
    cargo fmt --check

install: build
    cp target/release/herdr-app-tui ~/.local/bin/

clean:
    cargo clean