#!/bin/bash
# usage ./bump.sh 2.0.0 2.0.1

set -e

EXCLUDE_DIR='./target'

# Replace oldstring with the first argument to the script in all files in the current directory and its subdirectories
find . -path "$EXCLUDE_DIR" -prune -o -type f \( -name "*.toml" -o -name "*.kt" -o -name "*.podspec" \) -exec sed -i '' "s/$1/$2/g" {} \;

# clippy check
cargo clippy --all --all-features -- -D warnings

# dinghy test
cargo dinghy -vvv -d android test -p ezlog
cargo dinghy -vvv -d ios test -p ezlog

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