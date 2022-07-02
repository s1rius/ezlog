#!/bin/bash
set -e

echo "publish version $1"

echo build rust
cargo build --workspace

echo build iOS
cd ios
sh b_ios.sh
cd ..

echo build android
cd android
sh b_android.sh
cd ..

echo git tag $1

git add --all
git commit -m "bump version $1"
git tag $1
git push origin $1

echo cargo publish
cargo +nightly publish -p ezlog
cargo +nightly publish -p ezlog-cli

echo ios publish
cd ios
pod lib lint ./ios/EZLog.podspec
pod trunk push ./ios/EZLog.podspec --allow-warnings
cd ..

echo android publish
cd android
sh publish.sh
cd ..