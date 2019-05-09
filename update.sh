#!/bin/bash

BASEDIR=$(dirname "$0")
cd ${BASEDIR};

rustup self update \
    && rustup update stable \
    && cargo update \
    && cargo fmt \
    && cargo build \
    && cargo test \
    && cargo clippy \
    && cargo outdated
