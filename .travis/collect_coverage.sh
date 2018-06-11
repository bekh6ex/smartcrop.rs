#!/usr/bin/env bash
set -ex

wget https://github.com/SimonKagstrom/kcov/archive/master.tar.gz
tar xzf master.tar.gz
cd kcov-master
mkdir build
cd build
cmake ..
make
make install DESTDIR=../../kcov-build
cd ../..
rm -rf kcov-master

for file in target/debug/smartcrop-*[^\.d]; do
    mkdir -p "target/cov/$(basename $file)"
    # Have to reduce PROPTEST_CASES. It doesn't matter that much for the code coverage, but speeds up the build
    PROPTEST_CASES=1 travis_wait ./kcov-build/usr/local/bin/kcov --exclude-pattern=/.cargo,/usr/lib --verify \
        "target/cov/$(basename $file)" "$file"
done

bash <(curl -s https://codecov.io/bash)
echo "Uploaded code coverage"