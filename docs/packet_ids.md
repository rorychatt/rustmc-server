# Packet ID Validation

## Source of Truth

Packet IDs are validated against the official Minecraft server JAR's generated report.

- **Server version:** Minecraft 26.1.2
- **Protocol version:** 775
- **Validation date:** 2026-05-19
- **Report generated via:** `java -DbundlerMainClass=net.minecraft.data.Main -jar server.jar --reports`

## Validation Script

Run `tools/validate_packet_ids.sh <path/to/packets.json>` to compare the source code against an official report.

## Implemented Packets

### Status Phase

| Direction    | Packet Name      | ID   |
|-------------|------------------|------|
| Clientbound | Status Response  | 0x00 |
| Clientbound | Pong Response    | 0x01 |
| Serverbound | Status Request   | 0x00 |
| Serverbound | Ping Request     | 0x01 |

### Login Phase

| Direction    | Packet Name       | ID   |
|-------------|-------------------|------|
| Clientbound | Login Finished    | 0x02 |
| Clientbound | Login Compression | 0x03 |
| Serverbound | Hello (Login Start)| 0x00 |

### Configuration Phase

| Direction    | Packet Name            | ID   |
|-------------|------------------------|------|
| Clientbound | Finish Configuration   | 0x03 |
| Clientbound | Registry Data          | 0x07 |
| Clientbound | Update Tags            | 0x0D |
| Clientbound | Select Known Packs     | 0x0E |
| Serverbound | Finish Configuration   | 0x03 |
| Serverbound | Select Known Packs     | 0x07 |

### Play Phase

| Direction    | Packet Name                    | ID   |
|-------------|--------------------------------|------|
| Clientbound | Chunk Batch Finished           | 0x0B |
| Clientbound | Chunk Batch Start              | 0x0C |
| Clientbound | Forget Level Chunk             | 0x25 |
| Clientbound | Game Event                     | 0x26 |
| Clientbound | Keep Alive                     | 0x2C |
| Clientbound | Level Chunk With Light          | 0x2D |
| Clientbound | Login (Play)                   | 0x31 |
| Clientbound | Player Position                | 0x48 |
| Clientbound | System Chat                    | 0x79 |
| Serverbound | Accept Teleportation           | 0x00 |
| Serverbound | Chat Command                   | 0x07 |
| Serverbound | Chat                           | 0x09 |
| Serverbound | Chunk Batch Received           | 0x0B |
| Serverbound | Client Tick End                | 0x0D |
| Serverbound | Keep Alive                     | 0x1C |
| Serverbound | Move Player Pos                | 0x1E |
| Serverbound | Move Player Pos Rot            | 0x1F |
| Serverbound | Player Loaded                  | 0x2C |

## Intentional Deviations

None. All packet IDs match the official 26.1.2 server report exactly.
