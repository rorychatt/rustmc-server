use super::packet::Packet;
use super::types::{read_string, write_string, VarInt};
use std::io::{self, Cursor, Read, Write};

#[derive(Debug, Clone)]
pub struct PlayerPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub on_ground: bool,
}

impl PlayerPosition {
    pub fn decode(data: &[u8]) -> io::Result<Self> {
        let mut cursor = Cursor::new(data);
        let x = read_f64(&mut cursor)?;
        let y = read_f64(&mut cursor)?;
        let z = read_f64(&mut cursor)?;
        let mut buf = [0u8; 1];
        cursor.read_exact(&mut buf)?;
        let on_ground = buf[0] != 0;
        Ok(Self { x, y, z, on_ground })
    }
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub message: String,
}

impl ChatMessage {
    pub fn decode(data: &[u8]) -> io::Result<Self> {
        let mut cursor = Cursor::new(data);
        let message = super::types::read_string(&mut cursor)?;
        Ok(Self { message })
    }
}

pub fn encode_system_chat_message(message: &str) -> io::Result<Packet> {
    let json = serde_json::json!({"text": message}).to_string();
    let mut data = Vec::new();
    write_string(&mut data, &json)?;
    data.push(0); // overlay = false (chat, not action bar)
    Ok(Packet::new(0x79, data))
}

pub fn encode_set_center_chunk(chunk_x: i32, chunk_z: i32) -> Packet {
    let mut data = Vec::new();
    VarInt(chunk_x).write(&mut data).unwrap();
    VarInt(chunk_z).write(&mut data).unwrap();
    Packet::new(0x58, data)
}

pub fn encode_player_info_update(uuid: uuid::Uuid, name: &str, game_mode: i32) -> Packet {
    let mut data = Vec::new();
    // Actions bitmask: 0x01 (Add Player) | 0x04 (Update Game Mode) | 0x08 (Update Listed)
    data.push(0x01 | 0x04 | 0x08);
    VarInt(1).write(&mut data).unwrap(); // Number of players
                                         // UUID (128-bit, big-endian)
    data.extend_from_slice(uuid.as_bytes());
    // Action 0x01: Add Player
    write_string(&mut data, name).unwrap(); // Player name
    VarInt(0).write(&mut data).unwrap(); // Number of properties (0)
                                         // Action 0x04: Update Game Mode
    VarInt(game_mode).write(&mut data).unwrap();
    // Action 0x08: Update Listed
    data.push(1); // listed = true
    Packet::new(0x40, data)
}

pub fn encode_keep_alive(id: i64) -> Packet {
    let data = id.to_be_bytes().to_vec();
    Packet::new(0x2C, data)
}

pub fn encode_player_position_and_look(
    x: f64,
    y: f64,
    z: f64,
    yaw: f32,
    pitch: f32,
    flags: u8,
    teleport_id: i32,
) -> Packet {
    let mut data = Vec::new();
    VarInt(teleport_id).write(&mut data).unwrap();
    data.extend_from_slice(&x.to_be_bytes());
    data.extend_from_slice(&y.to_be_bytes());
    data.extend_from_slice(&z.to_be_bytes());
    // Velocity (new in protocol 775)
    data.extend_from_slice(&0.0f64.to_be_bytes()); // vel_x
    data.extend_from_slice(&0.0f64.to_be_bytes()); // vel_y
    data.extend_from_slice(&0.0f64.to_be_bytes()); // vel_z
    data.extend_from_slice(&yaw.to_be_bytes());
    data.extend_from_slice(&pitch.to_be_bytes());
    data.extend_from_slice(&(flags as i32).to_be_bytes()); // Flags (Int in 775)
    Packet::new(0x48, data)
}

pub fn encode_unload_chunk(chunk_x: i32, chunk_z: i32) -> Packet {
    let mut data = Vec::new();
    data.extend_from_slice(&chunk_z.to_be_bytes());
    data.extend_from_slice(&chunk_x.to_be_bytes());
    Packet::new(0x25, data)
}

pub fn encode_chunk_batch_start() -> Packet {
    Packet::new(0x0C, Vec::new())
}

pub fn encode_chunk_batch_finished(batch_size: i32) -> io::Result<Packet> {
    let mut data = Vec::new();
    VarInt(batch_size).write(&mut data)?;
    Ok(Packet::new(0x0B, data))
}

pub fn encode_game_event(event: u8, value: f32) -> Packet {
    let mut data = Vec::new();
    data.push(event);
    data.extend_from_slice(&value.to_be_bytes());
    Packet::new(0x26, data)
}

pub fn encode_login_play(entity_id: i32) -> io::Result<Packet> {
    let mut data = Vec::new();
    data.extend_from_slice(&entity_id.to_be_bytes()); // Entity ID
    data.push(0); // Is hardcore: false
    VarInt(1).write(&mut data)?; // Dimension count
    write_string(&mut data, "minecraft:overworld")?; // Dimension name
    VarInt(20).write(&mut data)?; // Max players
    VarInt(8).write(&mut data)?; // View distance
    VarInt(8).write(&mut data)?; // Simulation distance
    data.push(0); // Reduced debug info
    data.push(1); // Enable respawn screen
    data.push(0); // Do limited crafting
    VarInt(0).write(&mut data)?; // Dimension Type (VarInt registry ID, 0 = overworld)
    write_string(&mut data, "minecraft:overworld")?; // Dimension name
    data.extend_from_slice(&0i64.to_be_bytes()); // Hashed seed
    data.push(1); // Game mode: creative
    data.push(0xFF); // Previous game mode: -1
    data.push(0); // Is debug: false
    data.push(1); // Is flat: true
    data.push(0); // Has death location
    VarInt(0).write(&mut data)?; // Portal cooldown
    VarInt(63).write(&mut data)?; // Sea Level
    data.push(0); // Enforces Secure Chat: false
    Ok(Packet::new(0x31, data))
}

pub fn encode_play_cookie_request(key: &str) -> io::Result<Packet> {
    let mut data = Vec::new();
    write_string(&mut data, key)?;
    Ok(Packet::new(0x18, data))
}

pub fn encode_play_store_cookie(key: &str, payload: &[u8]) -> io::Result<Packet> {
    let mut data = Vec::new();
    write_string(&mut data, key)?;
    data.write_all(payload)?;
    Ok(Packet::new(0x74, data))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayCookieResponse {
    pub key: String,
    pub payload: Option<Vec<u8>>,
}

impl PlayCookieResponse {
    pub fn decode(data: &[u8]) -> io::Result<Self> {
        let mut cursor = Cursor::new(data);
        let key = read_string(&mut cursor)?;
        let mut has_payload_buf = [0u8; 1];
        cursor.read_exact(&mut has_payload_buf)?;
        let has_payload = has_payload_buf[0] != 0;
        let payload = if has_payload {
            let length = VarInt::read(&mut cursor)?.0 as usize;
            if length > 5120 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Cookie payload too large",
                ));
            }
            let mut buf = vec![0u8; length];
            cursor.read_exact(&mut buf)?;
            Some(buf)
        } else {
            None
        };
        Ok(Self { key, payload })
    }
}

pub fn encode_transfer(host: &str, port: i32) -> io::Result<Packet> {
    let mut data = Vec::new();
    write_string(&mut data, host)?;
    VarInt(port).write(&mut data)?;
    Ok(Packet::new(0x73, data))
}

fn read_f64(reader: &mut impl Read) -> io::Result<f64> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    Ok(f64::from_be_bytes(buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_position_decode() {
        let mut data = Vec::new();
        data.extend_from_slice(&100.0f64.to_be_bytes());
        data.extend_from_slice(&64.0f64.to_be_bytes());
        data.extend_from_slice(&200.0f64.to_be_bytes());
        data.push(1); // on_ground = true

        let pos = PlayerPosition::decode(&data).unwrap();
        assert_eq!(pos.x, 100.0);
        assert_eq!(pos.y, 64.0);
        assert_eq!(pos.z, 200.0);
        assert!(pos.on_ground);
    }

    #[test]
    fn test_keep_alive_encode() {
        let packet = encode_keep_alive(42);
        assert_eq!(packet.id, 0x2C);
        assert_eq!(packet.data.len(), 8);
    }

    #[test]
    fn test_chunk_batch_start() {
        let packet = encode_chunk_batch_start();
        assert_eq!(packet.id, 0x0C);
        assert!(packet.data.is_empty());
    }

    #[test]
    fn test_chunk_batch_finished() {
        let packet = encode_chunk_batch_finished(17).unwrap();
        assert_eq!(packet.id, 0x0B);
        assert!(!packet.data.is_empty());
    }

    #[test]
    fn test_game_event() {
        let packet = encode_game_event(13, 0.0);
        assert_eq!(packet.id, 0x26);
        assert_eq!(packet.data.len(), 5); // 1 byte event + 4 bytes float
    }

    #[test]
    fn test_login_play_protocol_775() {
        let packet = encode_login_play(1).unwrap();
        assert_eq!(packet.id, 0x31);
        assert!(!packet.data.is_empty());
    }

    #[test]
    fn test_unload_chunk() {
        let packet = encode_unload_chunk(5, 10);
        assert_eq!(packet.id, 0x25);
        assert_eq!(packet.data.len(), 8);
    }

    #[test]
    fn test_player_position_and_look() {
        let packet = encode_player_position_and_look(0.0, 64.0, 0.0, 0.0, 0.0, 0, 0);
        assert_eq!(packet.id, 0x48);
        // teleport_id(VarInt 1) + x,y,z(24) + vel_x,vel_y,vel_z(24) + yaw,pitch(8) + flags(4)
        assert!(packet.data.len() > 50);
    }

    #[test]

    fn test_set_center_chunk() {
        let packet = encode_set_center_chunk(3, -5);
        assert_eq!(packet.id, 0x58);
        // Two VarInts
        assert!(!packet.data.is_empty());

        // Verify roundtrip of VarInt values
        let mut cursor = Cursor::new(&packet.data);
        let x = VarInt::read(&mut cursor).unwrap().0;
        let z = VarInt::read(&mut cursor).unwrap().0;
        assert_eq!(x, 3);
        assert_eq!(z, -5);
    }

    #[test]
    fn test_player_info_update() {
        let uuid = uuid::Uuid::from_bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
        let packet = encode_player_info_update(uuid, "TestPlayer", 1);
        assert_eq!(packet.id, 0x40);
        assert!(!packet.data.is_empty());

        // Verify actions bitmask
        assert_eq!(packet.data[0], 0x01 | 0x04 | 0x08);
    }

    #[test]
    fn test_encode_play_cookie_request() {
        let packet = encode_play_cookie_request("minecraft:velocity_data").unwrap();
        assert_eq!(packet.id, 0x18);
        assert!(!packet.data.is_empty());
    }

    #[test]
    fn test_encode_play_store_cookie() {
        let payload = b"play_session_info";
        let packet = encode_play_store_cookie("minecraft:session", payload).unwrap();
        assert_eq!(packet.id, 0x74);
        assert!(!packet.data.is_empty());
    }

    #[test]
    fn test_play_cookie_response_decode_with_payload() {
        let mut data = Vec::new();
        write_string(&mut data, "minecraft:play_cookie").unwrap();
        data.push(1); // has_payload
        let payload = b"some_data";
        VarInt(payload.len() as i32).write(&mut data).unwrap();
        data.extend_from_slice(payload);

        let response = PlayCookieResponse::decode(&data).unwrap();
        assert_eq!(response.key, "minecraft:play_cookie");
        assert_eq!(response.payload, Some(b"some_data".to_vec()));
    }

    #[test]
    fn test_play_cookie_response_decode_without_payload() {
        let mut data = Vec::new();
        write_string(&mut data, "minecraft:empty").unwrap();
        data.push(0);

        let response = PlayCookieResponse::decode(&data).unwrap();
        assert_eq!(response.key, "minecraft:empty");
        assert_eq!(response.payload, None);
    }

    #[test]
    fn test_encode_transfer() {
        let packet = encode_transfer("play.example.com", 25565).unwrap();
        assert_eq!(packet.id, 0x73);
        assert!(!packet.data.is_empty());
    }

    #[test]
    fn test_encode_transfer_different_port() {
        let packet = encode_transfer("localhost", 19132).unwrap();
        assert_eq!(packet.id, 0x73);

        // Verify we can decode the host and port back
        let mut cursor = Cursor::new(&packet.data);
        let host = read_string(&mut cursor).unwrap();
        let port = VarInt::read(&mut cursor).unwrap().0;
        assert_eq!(host, "localhost");
        assert_eq!(port, 19132);
    }

    #[test]
    fn test_play_cookie_response_payload_too_large() {
        let mut data = Vec::new();
        write_string(&mut data, "minecraft:toobig").unwrap();
        data.push(1);
        VarInt(5121).write(&mut data).unwrap();
        data.extend(vec![0u8; 5121]);

        let result = PlayCookieResponse::decode(&data);
        assert!(result.is_err());
    }

    mod proptest_tests {
        use super::*;
        use proptest::prelude::*;

        fn identifier_strategy() -> impl Strategy<Value = String> {
            "[a-z][a-z0-9_]{0,15}:[a-z][a-z0-9_/]{0,30}"
        }

        fn hostname_strategy() -> impl Strategy<Value = String> {
            "[a-z][a-z0-9.]{0,30}"
        }

        proptest! {
            #[test]
            fn test_play_cookie_response_roundtrip(
                key in identifier_strategy(),
                has_payload in any::<bool>(),
                payload_data in proptest::collection::vec(any::<u8>(), 0..512)
            ) {
                let mut data = Vec::new();
                write_string(&mut data, &key).unwrap();
                if has_payload {
                    data.push(1);
                    VarInt(payload_data.len() as i32).write(&mut data).unwrap();
                    data.extend_from_slice(&payload_data);
                } else {
                    data.push(0);
                }

                let response = PlayCookieResponse::decode(&data).unwrap();
                prop_assert_eq!(&response.key, &key);
                if has_payload {
                    prop_assert_eq!(response.payload, Some(payload_data));
                } else {
                    prop_assert_eq!(response.payload, None);
                }
            }

            #[test]
            fn test_transfer_roundtrip(
                host in hostname_strategy(),
                port in 1i32..65535i32
            ) {
                let packet = encode_transfer(&host, port).unwrap();
                prop_assert_eq!(packet.id, 0x73);

                let mut cursor = Cursor::new(&packet.data);
                let decoded_host = read_string(&mut cursor).unwrap();
                let decoded_port = VarInt::read(&mut cursor).unwrap().0;
                prop_assert_eq!(decoded_host, host);
                prop_assert_eq!(decoded_port, port);
            }
        }
    }
}
