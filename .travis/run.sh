#!/usr/bin/env bash
TOOLCHAIN="${1:-nightly}"

set -ex

echo ${TRAVIS_EVENT_TYPE};

cargo build
cargo test
if [ "$TOOLCHAIN" == "nightly" ]; then
    cargo bench
fi

# Check that cli compiles
cd cli
cargo build