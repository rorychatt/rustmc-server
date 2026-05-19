#!/usr/bin/env bash
# Generates server/data/block_states.json from the Minecraft server JAR reports.
#
# Usage:
#   ./tools/generate_block_states.sh [path/to/blocks.json]
#
# If no path is provided, uses generated/reports/blocks.json if present.
# Otherwise prints instructions for generating the report from server.jar.
# Requires: jq

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

BLOCKS_JSON="${1:-}"

if [[ -z "$BLOCKS_JSON" ]]; then
    if [[ -f "$PROJECT_ROOT/generated/reports/blocks.json" ]]; then
        BLOCKS_JSON="$PROJECT_ROOT/generated/reports/blocks.json"
    else
        echo "Usage: $0 <path/to/blocks.json>"
        echo ""
        echo "Generate blocks.json by running:"
        echo "  java -DbundlerMainClass=net.minecraft.data.Main -jar server.jar --reports"
        echo ""
        echo "Then pass the generated file:"
        echo "  $0 generated/reports/blocks.json"
        exit 1
    fi
fi

if ! command -v jq &>/dev/null; then
    echo "ERROR: jq is required but not installed."
    exit 1
fi

if [[ ! -f "$BLOCKS_JSON" ]]; then
    echo "ERROR: File not found: $BLOCKS_JSON"
    exit 1
fi

OUTPUT="$PROJECT_ROOT/server/data/block_states.json"

echo "Generating block_states.json from: $BLOCKS_JSON"
echo "Output: $OUTPUT"

# Extract each block's default state ID from the server JAR report.
# The report format is:
#   { "minecraft:air": { "states": [{ "id": 0, "default": true }, ...] }, ... }
#
# For each block, find the state with "default": true and extract its "id".
jq '{
  protocol_version: 775,
  blocks: (
    [to_entries[] | {
      key: .key,
      value: { default_state_id: (.value.states[] | select(.default == true) | .id) }
    }] | from_entries
  )
}' "$BLOCKS_JSON" > "$OUTPUT"

BLOCK_COUNT=$(jq '.blocks | length' "$OUTPUT")
echo "Done. Generated $BLOCK_COUNT blocks."
