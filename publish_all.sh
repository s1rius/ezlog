#!/bin/bash
set -e

echo "publish version $1"

echo build rust
cargo build --workspace

echo build iOS
pushd ios
sh b_ios.sh
popd

echo build android
pushd android
sh b_android.sh
popd

echo git tag $1

git add --all
git commit -m "bump version $1"
git tag $1
git push origin $1

echo cargo publish
cargo +nightly publish -p ezlog
cargo +nightly publish -p ezlog-cli

echo ios publish
pushd ios
pod lib lint EZLog.podspec
pod trunk push EZLog.podspec --allow-warnings
popd

echo android publish
pushd android
sh publish.sh
popd

echo https://oss.sonatype.org/