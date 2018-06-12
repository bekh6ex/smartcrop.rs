#!/usr/bin/env bash
TOOLCHAIN="${1:-nightly}"

set -ex

echo ${TRAVIS_EVENT_TYPE};

cargo build --features 'image clap'
PROPTEST_CASES=5 cargo test
if [ "$TOOLCHAIN" == "nightly" ]; then
    cargo bench
fi