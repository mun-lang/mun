#!/usr/bin/env bash

cargo afl build
cargo afl fuzz -i in -x dict/keywords.dict -o out target/debug/fuzz
