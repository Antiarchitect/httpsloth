#!/bin/bash

BASEDIR=$(dirname "$0")
cd ${BASEDIR};

rustup self update \
    && rustup update stable \
    && cargo update \
    && cargo fmt \
    && cargo build \
    && cargo test \
    && cargo install cargo-audit || true \
    && cargo audit \
    && cargo clippy \
    && cargo install cargo-outdated || true \
    && cargo outdated
