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
    Ok(Packet::new(0x69, data))
}

pub fn encode_keep_alive(id: i64) -> Packet {
    let data = id.to_be_bytes().to_vec();
    Packet::new(0x26, data)
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
    data.extend_from_slice(&x.to_be_bytes());
    data.extend_from_slice(&y.to_be_bytes());
    data.extend_from_slice(&z.to_be_bytes());
    data.extend_from_slice(&yaw.to_be_bytes());
    data.extend_from_slice(&pitch.to_be_bytes());
    data.push(flags);
    VarInt(teleport_id).write(&mut data).unwrap();
    Packet::new(0x3E, data)
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
    write_string(&mut data, "minecraft:overworld")?; // Dimension type
    write_string(&mut data, "minecraft:overworld")?; // Dimension name
    data.extend_from_slice(&0i64.to_be_bytes()); // Hashed seed
    data.push(1); // Game mode: creative
    data.push(0xFF); // Previous game mode: -1
    data.push(0); // Is debug: false
    data.push(1); // Is flat: true
    data.push(0); // Has death location
    VarInt(0).write(&mut data)?; // Portal cooldown
    Ok(Packet::new(0x2B, data))
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
        assert_eq!(packet.id, 0x26);
        assert_eq!(packet.data.len(), 8);
    }
}
