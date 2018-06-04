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

function compare_with_js () {
    docker build --tag smartcrop-js build/compare-to-js

    cargo test --features "compare-to-js" produces_the_same_result_as_js_version

}