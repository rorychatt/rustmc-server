# Configuration Phase Manual Verification

Verify that a real Minecraft 1.21.5 (protocol 775) client completes the configuration phase using spec-correct packet IDs.

## Prerequisites

- Minecraft Java Edition 1.21.5 client
- Rust toolchain (cargo)

## Steps

1. Start the server with debug logging for the network module:

   ```bash
   RUST_LOG=rustmc_server::network=debug cargo run
   ```

2. Connect the Minecraft client to `localhost:25565`.

3. Observe the server log output during the configuration phase.

## Expected Output

A successful configuration phase produces log lines like:

```
DEBUG Configuration serverbound packet: 0x03 (0 bytes)    <- Login Acknowledged
DEBUG Client acknowledged login, sending configuration data
DEBUG Configuration serverbound packet: 0x07 (N bytes)    <- Known Packs
DEBUG Received Known Packs response from client
DEBUG Configuration serverbound packet: 0x01 (N bytes)    <- Cookie Response (if server sent a cookie request)
DEBUG Configuration serverbound packet: 0x03 (0 bytes)    <- Acknowledge Finish Configuration
DEBUG Client acknowledged finish configuration, transitioning to Play
```

Key packet IDs to confirm (protocol 775 spec):
- `0x01` = Cookie Response
- `0x03` = Acknowledge Finish Configuration (also Login Acknowledged)
- `0x07` = Known Packs

## Failure Indicators

- Client disconnects during configuration phase
- Missing "transitioning to Play" log message
- Unexpected packet IDs in the debug output
- Server logs "Unhandled configuration packet" for expected IDs
