#!/usr/bin/env bash
# Validates packet IDs in Rust source files against the official Minecraft server report.
#
# Usage:
#   ./tools/validate_packet_ids.sh [path/to/packets.json]
#
# If no path is provided, generates the report from server.jar in the current directory.
# Requires: jq, grep

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PACKETS_JSON="${1:-}"

if [[ -z "$PACKETS_JSON" ]]; then
    if [[ -f "$PROJECT_ROOT/generated/reports/packets.json" ]]; then
        PACKETS_JSON="$PROJECT_ROOT/generated/reports/packets.json"
    else
        echo "Usage: $0 <path/to/packets.json>"
        echo ""
        echo "Generate packets.json by running:"
        echo "  java -DbundlerMainClass=net.minecraft.data.Main -jar server.jar --reports"
        exit 1
    fi
fi

if ! command -v jq &>/dev/null; then
    echo "ERROR: jq is required but not installed."
    exit 1
fi

ERRORS=0
CHECKED=0

check_id() {
    local file="$1"
    local hex_id="$2"
    local expected_dec="$3"
    local packet_name="$4"
    local phase="$5"
    local direction="$6"

    CHECKED=$((CHECKED + 1))
    local expected_hex
    expected_hex=$(printf "0x%02X" "$expected_dec")

    if [[ "$hex_id" != "$expected_hex" ]]; then
        echo "MISMATCH: $phase/$direction/$packet_name"
        echo "  File:     $file"
        echo "  Expected: $expected_hex (decimal $expected_dec)"
        echo "  Found:    $hex_id"
        echo ""
        ERRORS=$((ERRORS + 1))
    fi
}

echo "Validating packet IDs against: $PACKETS_JSON"
echo "Source root: $PROJECT_ROOT/server/src"
echo ""

# --- Status Phase ---
# Clientbound
status_response=$(jq '.status.clientbound["minecraft:status_response"].protocol_id' "$PACKETS_JSON")
pong_response=$(jq '.status.clientbound["minecraft:pong_response"].protocol_id' "$PACKETS_JSON")
# Serverbound
status_request=$(jq '.status.serverbound["minecraft:status_request"].protocol_id' "$PACKETS_JSON")
ping_request=$(jq '.status.serverbound["minecraft:ping_request"].protocol_id' "$PACKETS_JSON")

check_id "protocol/status.rs" "0x00" "$status_response" "status_response" "status" "clientbound"
check_id "protocol/status.rs" "0x01" "$pong_response" "pong_response" "status" "clientbound"

# --- Login Phase ---
login_finished=$(jq '.login.clientbound["minecraft:login_finished"].protocol_id' "$PACKETS_JSON")
login_compression=$(jq '.login.clientbound["minecraft:login_compression"].protocol_id' "$PACKETS_JSON")

check_id "protocol/login.rs" "0x02" "$login_finished" "login_finished" "login" "clientbound"
check_id "protocol/login.rs" "0x03" "$login_compression" "login_compression" "login" "clientbound"

# --- Configuration Phase ---
finish_config_cb=$(jq '.configuration.clientbound["minecraft:finish_configuration"].protocol_id' "$PACKETS_JSON")
registry_data=$(jq '.configuration.clientbound["minecraft:registry_data"].protocol_id' "$PACKETS_JSON")
update_tags_config=$(jq '.configuration.clientbound["minecraft:update_tags"].protocol_id' "$PACKETS_JSON")
select_known_packs_cb=$(jq '.configuration.clientbound["minecraft:select_known_packs"].protocol_id' "$PACKETS_JSON")

check_id "protocol/configuration.rs" "0x03" "$finish_config_cb" "finish_configuration" "configuration" "clientbound"
check_id "protocol/configuration.rs" "0x07" "$registry_data" "registry_data" "configuration" "clientbound"
check_id "protocol/configuration.rs" "0x0D" "$update_tags_config" "update_tags" "configuration" "clientbound"
check_id "protocol/configuration.rs" "0x0E" "$select_known_packs_cb" "select_known_packs" "configuration" "clientbound"

# --- Play Phase (Clientbound) ---
chunk_batch_finished=$(jq '.play.clientbound["minecraft:chunk_batch_finished"].protocol_id' "$PACKETS_JSON")
chunk_batch_start=$(jq '.play.clientbound["minecraft:chunk_batch_start"].protocol_id' "$PACKETS_JSON")
game_event=$(jq '.play.clientbound["minecraft:game_event"].protocol_id' "$PACKETS_JSON")
forget_level_chunk=$(jq '.play.clientbound["minecraft:forget_level_chunk"].protocol_id' "$PACKETS_JSON")
keep_alive_cb=$(jq '.play.clientbound["minecraft:keep_alive"].protocol_id' "$PACKETS_JSON")
level_chunk=$(jq '.play.clientbound["minecraft:level_chunk_with_light"].protocol_id' "$PACKETS_JSON")
login_play=$(jq '.play.clientbound["minecraft:login"].protocol_id' "$PACKETS_JSON")
player_position=$(jq '.play.clientbound["minecraft:player_position"].protocol_id' "$PACKETS_JSON")
system_chat=$(jq '.play.clientbound["minecraft:system_chat"].protocol_id' "$PACKETS_JSON")

check_id "protocol/play.rs" "0x0B" "$chunk_batch_finished" "chunk_batch_finished" "play" "clientbound"
check_id "protocol/play.rs" "0x0C" "$chunk_batch_start" "chunk_batch_start" "play" "clientbound"
check_id "protocol/play.rs" "0x26" "$game_event" "game_event" "play" "clientbound"
check_id "protocol/play.rs" "0x25" "$forget_level_chunk" "forget_level_chunk" "play" "clientbound"
check_id "protocol/play.rs" "0x2C" "$keep_alive_cb" "keep_alive" "play" "clientbound"
check_id "protocol/chunk_data.rs" "0x2D" "$level_chunk" "level_chunk_with_light" "play" "clientbound"
check_id "protocol/play.rs" "0x31" "$login_play" "login" "play" "clientbound"
check_id "protocol/play.rs" "0x48" "$player_position" "player_position" "play" "clientbound"
check_id "protocol/play.rs" "0x79" "$system_chat" "system_chat" "play" "clientbound"

# --- Play Phase (Serverbound) ---
accept_teleport=$(jq '.play.serverbound["minecraft:accept_teleportation"].protocol_id' "$PACKETS_JSON")
chat_command=$(jq '.play.serverbound["minecraft:chat_command"].protocol_id' "$PACKETS_JSON")
chat=$(jq '.play.serverbound["minecraft:chat"].protocol_id' "$PACKETS_JSON")
chunk_batch_recv=$(jq '.play.serverbound["minecraft:chunk_batch_received"].protocol_id' "$PACKETS_JSON")
client_tick_end=$(jq '.play.serverbound["minecraft:client_tick_end"].protocol_id' "$PACKETS_JSON")
keep_alive_sb=$(jq '.play.serverbound["minecraft:keep_alive"].protocol_id' "$PACKETS_JSON")
move_player_pos=$(jq '.play.serverbound["minecraft:move_player_pos"].protocol_id' "$PACKETS_JSON")
move_player_pos_rot=$(jq '.play.serverbound["minecraft:move_player_pos_rot"].protocol_id' "$PACKETS_JSON")
player_loaded=$(jq '.play.serverbound["minecraft:player_loaded"].protocol_id' "$PACKETS_JSON")

check_id "network/connection.rs" "0x00" "$accept_teleport" "accept_teleportation" "play" "serverbound"
check_id "network/connection.rs" "0x07" "$chat_command" "chat_command" "play" "serverbound"
check_id "network/connection.rs" "0x09" "$chat" "chat" "play" "serverbound"
check_id "network/connection.rs" "0x0B" "$chunk_batch_recv" "chunk_batch_received" "play" "serverbound"
check_id "network/connection.rs" "0x0D" "$client_tick_end" "client_tick_end" "play" "serverbound"
check_id "network/connection.rs" "0x1C" "$keep_alive_sb" "keep_alive" "play" "serverbound"
check_id "network/connection.rs" "0x1E" "$move_player_pos" "move_player_pos" "play" "serverbound"
check_id "network/connection.rs" "0x1F" "$move_player_pos_rot" "move_player_pos_rot" "play" "serverbound"
check_id "network/connection.rs" "0x2C" "$player_loaded" "player_loaded" "play" "serverbound"

echo "---"
echo "Checked: $CHECKED packet IDs"
if [[ $ERRORS -eq 0 ]]; then
    echo "Result: ALL PASS"
    exit 0
else
    echo "Result: $ERRORS MISMATCH(ES) FOUND"
    exit 1
fi
