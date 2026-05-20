# RustMC Server

A multi-threaded Rust-based Minecraft server with Paper plugin compatibility.

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
