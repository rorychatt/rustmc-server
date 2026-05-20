# Default: list available targets
default:
    @just --list

# Build the workspace
build:
    cargo build --workspace

# Run all tests
test:
    cargo test --workspace

# Run clippy lints
lint:
    cargo clippy --workspace -- -D warnings

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Auto-format code
fmt:
    cargo fmt --all

# Run the server
run:
    cargo run -p rustmc-server

# Download the Minecraft server JAR for the current protocol version
download-jar:
    ./tools/download_server_jar.sh

# Regenerate block_states.json from the Minecraft server JAR (requires Java 21+)
regenerate-blocks:
    ./tools/regenerate_block_states.sh

# Generate block_states.json from an existing blocks.json report
generate-blocks blocks_json="":
    ./tools/generate_block_states.sh {{blocks_json}}

# Validate packet IDs against official Minecraft reports
validate-packets packets_json="":
    ./tools/validate_packet_ids.sh {{packets_json}}

# Run full CI checks locally (build + test + clippy + fmt)
ci:
    cargo build --workspace
    cargo test --workspace
    cargo clippy --workspace -- -D warnings
    cargo fmt --all -- --check
