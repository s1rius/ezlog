#!/bin/sh
set -e
echo "publish android"
# config doc see 
# https://github.com/vanniktech/gradle-maven-publish-plugin/blob/master/plugin/src/main/kotlin/com/vanniktech/maven/publish/MavenPublishBaseExtension.kt#L76

pushd android
# https://oss.sonatype.org/
./gradlew :lib-ezlog:publish
./gradlew closeAndReleaseRepository

popd