#!/bin/sh
#

set -eu

RUSTFLAGS=-Dwarnings cargo check --workspace --all-targets --all-features
cargo clippy --all --all-targets --all-features -- -D warnings
cargo +nightly fmt --all -- --check
cargo test --doc --workspace --all-features
cargo test --workspace --all-targets --all-features
