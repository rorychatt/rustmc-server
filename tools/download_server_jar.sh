#!/usr/bin/env bash
# Downloads the Minecraft server JAR matching VERSION_NAME from version.rs
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CACHE_DIR="$PROJECT_ROOT/.cache"

VERSION=$(grep 'VERSION_NAME' "$PROJECT_ROOT/server/src/protocol/version.rs" \
  | head -1 | sed 's/.*"\(.*\)".*/\1/')

if [[ -z "$VERSION" ]]; then
    echo "ERROR: Could not parse VERSION_NAME from version.rs" >&2
    exit 1
fi

JAR_PATH="$CACHE_DIR/server-${VERSION}.jar"

if [[ -f "$JAR_PATH" ]]; then
    echo "Server JAR already cached: $JAR_PATH"
    echo "$JAR_PATH"
    exit 0
fi

mkdir -p "$CACHE_DIR"

echo "Fetching Mojang version manifest..."
MANIFEST=$(curl -sf https://launchermeta.mojang.com/mc/game/version_manifest_v2.json)
PACKAGE_URL=$(echo "$MANIFEST" | jq -r --arg v "$VERSION" '.versions[] | select(.id == $v) | .url')

if [[ -z "$PACKAGE_URL" || "$PACKAGE_URL" == "null" ]]; then
    echo "ERROR: Version $VERSION not found in Mojang manifest" >&2
    exit 1
fi

echo "Fetching package metadata for $VERSION..."
PACKAGE=$(curl -sf "$PACKAGE_URL")
SERVER_URL=$(echo "$PACKAGE" | jq -r '.downloads.server.url')
SERVER_SHA1=$(echo "$PACKAGE" | jq -r '.downloads.server.sha1')

echo "Downloading server JAR..."
curl -fo "$JAR_PATH" "$SERVER_URL"

if command -v sha1sum >/dev/null 2>&1; then
    ACTUAL_SHA1=$(sha1sum "$JAR_PATH" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
    ACTUAL_SHA1=$(shasum "$JAR_PATH" | awk '{print $1}')
else
    echo "ERROR: Neither sha1sum nor shasum command found" >&2
    exit 1
fi

if [[ "$ACTUAL_SHA1" != "$SERVER_SHA1" ]]; then
    rm -f "$JAR_PATH"
    echo "ERROR: SHA1 mismatch (expected $SERVER_SHA1, got $ACTUAL_SHA1)" >&2
    exit 1
fi

echo "Downloaded and verified: $JAR_PATH"
echo "$JAR_PATH"
