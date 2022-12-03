#!/bin/bash
set -e

pushd ios
sh b_ios.sh
popd

pushd android
sh b_android.sh
popd

cargo clippy --workspace --all-features
