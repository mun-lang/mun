#!/bin/bash

# This script installs the correct Rust release.

set -euo pipefail

rust_version=$(<rust-toolchain)

# rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain $rust_version

# Make rustup available to this script
source "$HOME/.cargo/env"

# Install nightly rustfmt
rustup toolchain install nightly --profile minimal --component rustfmt

