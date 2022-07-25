#!/bin/bash
set -e

echo ios publish
pushd ios
# pod lib lint EZLog.podspec
pod trunk push EZLog.podspec --allow-warnings
popd