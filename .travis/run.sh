#!/usr/bin/env bash
TOOLCHAIN="${1:-nightly}"

set -e

echo ${TRAVIS_EVENT_TYPE};

cargo build
cargo test
if [ "$TOOLCHAIN" == "nightly" ]; then
    cargo bench
fi