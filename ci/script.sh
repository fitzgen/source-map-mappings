#!/usr/bin/env bash

set -eux

case "$JOB" in
    "test")
        cargo install -f cargo-readme
        cargo test
        ;;
    "bench")
        cargo bench
        ;;
    "wasm")
        rustup target add wasm32-unknown-unknown
        cargo build --target wasm32-unknown-unknown
        cargo build --release --target wasm32-unknown-unknown
        ;;
    *)
        echo "Unknown \$JOB = '$JOB'"
        exit 1
esac
