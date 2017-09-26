#!/usr/bin/env bash
TOOLCHAIN="${1:-nightly}"

set -e

cargo build
cargo test
if [ "$TOOLCHAIN" == "nightly" ]; then
    cargo bench
fi