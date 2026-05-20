#!/usr/bin/env bash
# Orchestrates the full block state regeneration pipeline:
# 1. Downloads the correct server JAR (from Mojang, based on VERSION_NAME)
# 2. Runs the server JAR data generator to produce blocks.json
# 3. Runs generate_block_states.sh to produce server/data/block_states.json
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

JAR_PATH=$("$SCRIPT_DIR/download_server_jar.sh" | tail -1)

TEMP_DIR=$(mktemp -d)
trap 'rm -rf "$TEMP_DIR"' EXIT

echo "Running server data generator..."
java -DbundlerMainClass=net.minecraft.data.Main -jar "$JAR_PATH" --reports --output "$TEMP_DIR" 2>/dev/null

BLOCKS_JSON="$TEMP_DIR/reports/blocks.json"
if [[ ! -f "$BLOCKS_JSON" ]]; then
    echo "ERROR: Data generator did not produce reports/blocks.json" >&2
    exit 1
fi

echo "Generating block_states.json..."
"$SCRIPT_DIR/generate_block_states.sh" "$BLOCKS_JSON"
