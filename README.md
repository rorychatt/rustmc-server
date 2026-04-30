# RustMC Server

A multi-threaded Rust-based Minecraft server with Paper plugin compatibility.

## Features

- **Async I/O**: Built on Tokio for efficient concurrent client handling
- **Multi-threaded world ticking**: Chunks are processed in parallel using a work-stealing thread pool
- **Paper plugin support**: JNI bridge layer enables loading and running Paper/Bukkit plugins
- **Minecraft 1.20.4 protocol**: Implements the Minecraft Java Edition protocol

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
