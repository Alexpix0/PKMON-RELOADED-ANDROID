#!/bin/bash
# Runs the Gradle build reusing an already-downloaded Gradle distribution from the local cache,
# so no wrapper jar or network download is needed.
set -e
cd "$(dirname "$0")"

GRADLE_DIST="${GRADLE_DIST:-9.4.1}"
GRADLE=$(ls -d "$HOME"/.gradle/wrapper/dists/gradle-"$GRADLE_DIST"-bin/*/gradle-"$GRADLE_DIST"/bin/gradle 2>/dev/null | head -1)
if [ -z "$GRADLE" ]; then
  echo "no encuentro Gradle $GRADLE_DIST en ~/.gradle/wrapper/dists; define GRADLE=/ruta/a/gradle"
  exit 1
fi

exec "$GRADLE" "$@"
