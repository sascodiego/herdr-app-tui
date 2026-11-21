default:
    @just --list

# Debug build for everyday use. Release is only for cutting a release.
build:
    cargo build

release:
    cargo build --release

test:
    cargo test

check: test
    cargo clippy --all-targets -- -D warnings

# Depends on the release recipe because it copies target/release/herdr-app-tui.
install: release
    cp target/release/herdr-app-tui ~/.local/bin/

clean:
    cargo clean