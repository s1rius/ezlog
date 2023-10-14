#!/bin/bash
set -e

EXCLUDE_DIR='./target'

# Replace oldstring with the first argument to the script in all files in the current directory and its subdirectories
find . -path "$EXCLUDE_DIR" -prune -o -type f \( -name "*.toml" -o -name "*.kt" -o -name "*.podspec" \) -exec sed -i '' "s/$1/$2/g" {} \;

# clippy check
cargo clippy --all --all-features -- -D warnings

# build iOS
pushd ios
sh b_ios.sh
popd

# build android
pushd android
sh b_android.sh
popd

pushd docs
cat ./src/introduction.md  ./src/platform/*.md ./src/architecture.md ./src/benchmark.md ./src/build.md >> ./../README.md
popd