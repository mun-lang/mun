#!/bin/sh
#

set -eu

# Setting RUSTFLAGS env for clippy makes it not include custom rules
RUSTFLAGS=-Dwarnings cargo check --workspace --all-targets --profile bench --all-features
cargo clippy --all --all-targets --all-features -- -D warnings
cargo +nightly fmt --all -- --check
cargo test --doc --workspace --all-features
cargo test --workspace --all-targets --all-features
