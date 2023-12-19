#!/bin/bash

# This script collect logs for running Miri in each crate

# Install nightly Miri
rustup toolchain install nightly --profile minimal --component miri

# Create the folder for logs
mkdir -p .logs

# Run clean to ensure that we get all miri errors
cargo clean

for path in crates/* ; do
    echo "Running miri in '$path'"
    package=$(basename "$path")
    OUTPUT_FILE=.logs/log_$package
    MIRIFLAGS="\
    -Zmiri-disable-stacked-borrows \
    -Zmiri-backtrace=full \
    -Zmiri-disable-isolation" \
    # Log to stdout and a file - for future reference
    cargo +nightly miri test --package $package --no-fail-fast 2>&1 | tee $OUTPUT_FILE
done
