#!/bin/sh

cd "$(dirname "$0")" || exit 1

rustup self update 2>/dev/null || true \
        && rustup update stable \
        && rustup component add clippy \
        && rustup component add rustfmt \
        && cargo update \
        && cargo fmt \
        && cargo build \
        && cargo test \
        && cargo install cargo-audit \
        && cargo audit \
        && cargo clippy \
        && cargo install cargo-outdated \
        && cargo outdated \
        && cargo install cargo-udeps \
        && cargo +nightly udeps
