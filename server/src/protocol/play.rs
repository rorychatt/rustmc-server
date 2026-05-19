use super::packet::Packet;
use super::types::{write_string, VarInt};
use std::io::{self, Cursor, Read};

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
    Ok(Packet::new(0x77, data))
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
    Packet::new(0x2B, data)
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
    Packet::new(0x46, data)
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
    Packet::new(0x23, data)
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
    Ok(Packet::new(0x30, data))
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
        assert_eq!(packet.id, 0x2B);
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
        assert_eq!(packet.id, 0x23);
        assert_eq!(packet.data.len(), 5); // 1 byte event + 4 bytes float
    }

    #[test]
    fn test_login_play_protocol_775() {
        let packet = encode_login_play(1).unwrap();
        assert_eq!(packet.id, 0x30);
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
        assert_eq!(packet.id, 0x46);
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
}
