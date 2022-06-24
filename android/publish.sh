#!/bin/sh
echo "publish module lib-ezlog"
publish="./gradlew :lib-ezlog:publishMavenPublicationToMavenCentralRepository"
$publish