#!/usr/bin/env bash
TOOLCHAIN="${1:-nightly}"

set -e

run_cargo build
run_cargo test
if [ "$TOOLCHAIN" == "nightly" ]; then
            run_cargo bench
fi