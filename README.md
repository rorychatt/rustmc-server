# RustMC Server

A multi-threaded Rust-based Minecraft server with Paper plugin compatibility.

[![Documentation Website](https://img.shields.io/badge/docs-website-blue.svg)](https://rorychatt.github.io/rustmc-server/)

Documentation website: [https://rorychatt.github.io/rustmc-server/](https://rorychatt.github.io/rustmc-server/)

## Features

- **Async I/O**: Built on Tokio for efficient concurrent client handling
- **Multi-threaded world ticking**: Chunks are processed in parallel using a work-stealing thread pool
- **Paper plugin support**: JNI bridge layer enables loading and running Paper/Bukkit plugins
- **Minecraft 26.1.2 protocol**: Implements the Minecraft Java Edition protocol (protocol version 775)

## Architecture

```
Tokio Runtime (N threads)
├── Client Handlers (async per-connection)
├── World Tick Thread Pool (parallel chunk processing)
└── Plugin Main Thread (JNI, serial plugin callbacks)
```

## Building

```bash
cargo build --workspace
```

## Running

```bash
cargo run -p rustmc-server
```

The server binds to `0.0.0.0:25565` by default.

## Configuration

The server can be configured using a `server.yaml` (or `server.toml`) file. You can specify the configuration file path by setting the `RUSTMC_CONFIG` environment variable. If not set, the server will search for `server.yaml` (or `server.toml`) in the working directory, falling back to default values if not found.

Example `server.yaml`:

```yaml
server:
  bind: "0.0.0.0:25565"
  view_distance: 8

rate_limit:
  invalid_packet_threshold: 16
  invalid_packet_window_secs: 10

gameplay:
  motd: "RustMC Server - A Rust-powered Minecraft server"
  max_players: 20
  gamemode: "creative"        # Options: survival, creative, adventure, spectator
  difficulty: "normal"        # Options: peaceful, easy, normal, hard
  pvp: true
  allow_flight: false
  hardcore: false
  simulation_distance: 8
  sea_level: 63
  world_type: "normal"        # Options: normal, flat
  seed: 0                     # Seed for normal noise generation
  world_dir: "world"          # Directory to save chunks and level data
  save_interval_secs: 300     # Periodic background chunk autosave interval
  backup_interval_secs: 3600  # Periodic background backup interval (0 to disable)
  max_backups: 5              # Number of backup rotations to keep
```

## Testing

```bash
cargo test --workspace
```

## Development Tasks

This project uses [just](https://github.com/casey/just) as a task runner. List all available targets:

```bash
just --list
```

Common tasks:

```bash
just ci                  # Run full CI checks locally
just regenerate-blocks   # Regenerate block_states.json (requires Java 21+, jq)
just validate-packets    # Validate packet IDs against server reports
```

## Project Structure

```
├── server/              Core server implementation
│   └── src/
│       ├── main.rs      Entry point
│       ├── protocol/    Minecraft protocol handling
│       ├── network/     Connection and packet I/O
│       └── world/       World state and chunk management
├── plugin-bridge/       Paper plugin compatibility layer
│   └── src/
│       ├── lib.rs       Bridge API
│       ├── events.rs    Event system
│       ├── plugin.rs    Plugin loader and manager
│       └── scheduler.rs Task scheduler
└── tests/               Integration tests
```

## License

MIT
