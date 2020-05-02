#!/bin/bash

cargo +nightly fuzz build
cargo +nightly fuzz run compiler_fuzz ../afl/in
