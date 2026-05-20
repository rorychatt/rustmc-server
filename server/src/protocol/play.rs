use super::packet::Packet;
use super::packet_ids::play::clientbound as ids;
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
    const MAX_HORIZONTAL: f64 = 30_000_000.0;
    const MIN_Y: f64 = -64.0;
    const MAX_Y: f64 = 320.0;

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

    pub fn is_valid(&self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.z.is_finite()
            && self.x.abs() <= Self::MAX_HORIZONTAL
            && self.z.abs() <= Self::MAX_HORIZONTAL
            && self.y >= Self::MIN_Y
            && self.y <= Self::MAX_Y
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
    Ok(Packet::new(ids::SYSTEM_CHAT_MESSAGE, data))
}

pub fn encode_set_center_chunk(chunk_x: i32, chunk_z: i32) -> Packet {
    let mut data = Vec::new();
    VarInt(chunk_x).write(&mut data).unwrap();
    VarInt(chunk_z).write(&mut data).unwrap();
    Packet::new(ids::SET_CENTER_CHUNK, data)
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
    Packet::new(ids::PLAYER_INFO_UPDATE, data)
}

pub fn encode_keep_alive(id: i64) -> Packet {
    let data = id.to_be_bytes().to_vec();
    Packet::new(ids::KEEP_ALIVE, data)
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
    Packet::new(ids::SYNCHRONIZE_PLAYER_POSITION, data)
}

pub fn encode_unload_chunk(chunk_x: i32, chunk_z: i32) -> Packet {
    let mut data = Vec::new();
    data.extend_from_slice(&chunk_z.to_be_bytes());
    data.extend_from_slice(&chunk_x.to_be_bytes());
    Packet::new(ids::UNLOAD_CHUNK, data)
}

pub fn encode_chunk_batch_start() -> Packet {
    Packet::new(ids::CHUNK_BATCH_START, Vec::new())
}

pub fn encode_chunk_batch_finished(batch_size: i32) -> io::Result<Packet> {
    let mut data = Vec::new();
    VarInt(batch_size).write(&mut data)?;
    Ok(Packet::new(ids::CHUNK_BATCH_FINISHED, data))
}

pub fn encode_game_event(event: u8, value: f32) -> Packet {
    let mut data = Vec::new();
    data.push(event);
    data.extend_from_slice(&value.to_be_bytes());
    Packet::new(ids::GAME_EVENT, data)
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
    Ok(Packet::new(ids::LOGIN_PLAY, data))
}

pub fn encode_play_cookie_request(key: &str) -> io::Result<Packet> {
    let mut data = Vec::new();
    write_string(&mut data, key)?;
    Ok(Packet::new(ids::COOKIE_REQUEST, data))
}

pub fn encode_play_store_cookie(key: &str, payload: &[u8]) -> io::Result<Packet> {
    let mut data = Vec::new();
    write_string(&mut data, key)?;
    data.write_all(payload)?;
    Ok(Packet::new(ids::STORE_COOKIE, data))
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

#[derive(Debug, Clone)]
pub struct PlayerPositionAndRotation {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

impl PlayerPositionAndRotation {
    const MAX_HORIZONTAL: f64 = 30_000_000.0;
    const MIN_Y: f64 = -64.0;
    const MAX_Y: f64 = 320.0;
    const MAX_PITCH: f32 = 90.0;

    pub fn decode(data: &[u8]) -> io::Result<Self> {
        let mut cursor = Cursor::new(data);
        let x = read_f64(&mut cursor)?;
        let y = read_f64(&mut cursor)?;
        let z = read_f64(&mut cursor)?;
        let yaw = read_f32(&mut cursor)?;
        let pitch = read_f32(&mut cursor)?;
        let mut buf = [0u8; 1];
        cursor.read_exact(&mut buf)?;
        let on_ground = buf[0] != 0;
        Ok(Self {
            x,
            y,
            z,
            yaw,
            pitch,
            on_ground,
        })
    }

    pub fn is_valid(&self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.z.is_finite()
            && self.yaw.is_finite()
            && self.pitch.is_finite()
            && self.x.abs() <= Self::MAX_HORIZONTAL
            && self.z.abs() <= Self::MAX_HORIZONTAL
            && self.y >= Self::MIN_Y
            && self.y <= Self::MAX_Y
            && self.pitch >= -Self::MAX_PITCH
            && self.pitch <= Self::MAX_PITCH
    }
}

#[derive(Debug, Clone)]
pub struct PlayerRotation {
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

impl PlayerRotation {
    const MAX_PITCH: f32 = 90.0;

    pub fn decode(data: &[u8]) -> io::Result<Self> {
        let mut cursor = Cursor::new(data);
        let yaw = read_f32(&mut cursor)?;
        let pitch = read_f32(&mut cursor)?;
        let mut buf = [0u8; 1];
        cursor.read_exact(&mut buf)?;
        let on_ground = buf[0] != 0;
        Ok(Self {
            yaw,
            pitch,
            on_ground,
        })
    }

    pub fn is_valid(&self) -> bool {
        self.yaw.is_finite()
            && self.pitch.is_finite()
            && self.pitch >= -Self::MAX_PITCH
            && self.pitch <= Self::MAX_PITCH
    }
}

#[derive(Debug, Clone)]
pub struct PlayerStatusOnly {
    pub on_ground: bool,
}

impl PlayerStatusOnly {
    pub fn decode(data: &[u8]) -> io::Result<Self> {
        if data.is_empty() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "empty packet"));
        }
        let on_ground = data[0] != 0;
        Ok(Self { on_ground })
    }
}

#[derive(Debug, Clone)]
pub struct PlayerCommand {
    pub entity_id: i32,
    pub action_id: i32,
    pub jump_boost: i32,
}

impl PlayerCommand {
    pub fn decode(data: &[u8]) -> io::Result<Self> {
        let mut cursor = Cursor::new(data);
        let entity_id = VarInt::read(&mut cursor)?.0;
        let action_id = VarInt::read(&mut cursor)?.0;
        let jump_boost = VarInt::read(&mut cursor)?.0;
        Ok(Self {
            entity_id,
            action_id,
            jump_boost,
        })
    }

    pub fn is_valid(&self) -> bool {
        (0..=6).contains(&self.action_id) && (0..=100).contains(&self.jump_boost)
    }
}

#[derive(Debug, Clone)]
pub struct Swing {
    pub hand: i32,
}

impl Swing {
    pub fn decode(data: &[u8]) -> io::Result<Self> {
        let mut cursor = Cursor::new(data);
        let hand = VarInt::read(&mut cursor)?.0;
        Ok(Self { hand })
    }

    pub fn is_valid(&self) -> bool {
        self.hand == 0 || self.hand == 1
    }
}

#[derive(Debug, Clone)]
pub struct SetCarriedItem {
    pub slot: i16,
}

impl SetCarriedItem {
    pub const HOTBAR_SLOT_MIN: i16 = 0;
    pub const HOTBAR_SLOT_MAX: i16 = 8;

    pub fn decode(data: &[u8]) -> io::Result<Self> {
        let mut cursor = Cursor::new(data);
        let slot = read_i16(&mut cursor)?;
        Ok(Self { slot })
    }

    pub fn is_valid_slot(&self) -> bool {
        self.slot >= Self::HOTBAR_SLOT_MIN && self.slot <= Self::HOTBAR_SLOT_MAX
    }
}

pub struct ChatCommand {
    pub command: String,
}

impl ChatCommand {
    pub fn decode(data: &[u8]) -> io::Result<Self> {
        let mut cursor = Cursor::new(data);
        let command = read_string(&mut cursor)?;
        Ok(Self { command })
    }
}

pub fn encode_transfer(host: &str, port: i32) -> io::Result<Packet> {
    let mut data = Vec::new();
    write_string(&mut data, host)?;
    VarInt(port).write(&mut data)?;
    Ok(Packet::new(ids::TRANSFER, data))
}

pub fn encode_entity_animation(entity_id: i32, animation: u8) -> Packet {
    let mut data = Vec::new();
    VarInt(entity_id).write(&mut data).unwrap();
    data.push(animation);
    Packet::new(0x03, data)
}

pub fn encode_set_entity_metadata(entity_id: i32, metadata: &[u8]) -> Packet {
    let mut data = Vec::new();
    VarInt(entity_id).write(&mut data).unwrap();
    data.extend_from_slice(metadata);
    data.push(0xFF);
    Packet::new(0x60, data)
}

pub fn encode_entity_base_flags_metadata(flags: u8) -> Vec<u8> {
    vec![0x00, 0x00, flags]
}

fn read_f64(reader: &mut impl Read) -> io::Result<f64> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    Ok(f64::from_be_bytes(buf))
}

fn read_f32(reader: &mut impl Read) -> io::Result<f32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(f32::from_be_bytes(buf))
}

fn read_i16(reader: &mut impl Read) -> io::Result<i16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(i16::from_be_bytes(buf))
}

#[cfg(test)]
mod tests {
    use super::ids;
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
        assert_eq!(packet.id, ids::KEEP_ALIVE);
        assert_eq!(packet.data.len(), 8);
    }

    #[test]
    fn test_chunk_batch_start() {
        let packet = encode_chunk_batch_start();
        assert_eq!(packet.id, ids::CHUNK_BATCH_START);
        assert!(packet.data.is_empty());
    }

    #[test]
    fn test_chunk_batch_finished() {
        let packet = encode_chunk_batch_finished(17).unwrap();
        assert_eq!(packet.id, ids::CHUNK_BATCH_FINISHED);
        assert!(!packet.data.is_empty());
    }

    #[test]
    fn test_game_event() {
        let packet = encode_game_event(13, 0.0);
        assert_eq!(packet.id, ids::GAME_EVENT);
        assert_eq!(packet.data.len(), 5); // 1 byte event + 4 bytes float
    }

    #[test]
    fn test_login_play_protocol_775() {
        let packet = encode_login_play(1).unwrap();
        assert_eq!(packet.id, ids::LOGIN_PLAY);
        assert!(!packet.data.is_empty());
    }

    #[test]
    fn test_unload_chunk() {
        let packet = encode_unload_chunk(5, 10);
        assert_eq!(packet.id, ids::UNLOAD_CHUNK);
        assert_eq!(packet.data.len(), 8);
    }

    #[test]
    fn test_player_position_and_look() {
        let packet = encode_player_position_and_look(0.0, 64.0, 0.0, 0.0, 0.0, 0, 0);
        assert_eq!(packet.id, ids::SYNCHRONIZE_PLAYER_POSITION);
        // teleport_id(VarInt 1) + x,y,z(24) + vel_x,vel_y,vel_z(24) + yaw,pitch(8) + flags(4)
        assert!(packet.data.len() > 50);
    }

    #[test]

    fn test_set_center_chunk() {
        let packet = encode_set_center_chunk(3, -5);
        assert_eq!(packet.id, ids::SET_CENTER_CHUNK);
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
        assert_eq!(packet.id, ids::PLAYER_INFO_UPDATE);
        assert!(!packet.data.is_empty());

        // Verify actions bitmask
        assert_eq!(packet.data[0], 0x01 | 0x04 | 0x08);
    }

    #[test]
    fn test_encode_play_cookie_request() {
        let packet = encode_play_cookie_request("minecraft:velocity_data").unwrap();
        assert_eq!(packet.id, ids::COOKIE_REQUEST);
        assert!(!packet.data.is_empty());
    }

    #[test]
    fn test_encode_play_store_cookie() {
        let payload = b"play_session_info";
        let packet = encode_play_store_cookie("minecraft:session", payload).unwrap();
        assert_eq!(packet.id, ids::STORE_COOKIE);
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
        assert_eq!(packet.id, ids::TRANSFER);
        assert!(!packet.data.is_empty());
    }

    #[test]
    fn test_encode_transfer_different_port() {
        let packet = encode_transfer("localhost", 19132).unwrap();
        assert_eq!(packet.id, ids::TRANSFER);

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

    #[test]
    fn test_player_position_and_rotation_decode() {
        let mut data = Vec::new();
        data.extend_from_slice(&100.0f64.to_be_bytes()); // x
        data.extend_from_slice(&64.0f64.to_be_bytes()); // y
        data.extend_from_slice(&200.0f64.to_be_bytes()); // z
        data.extend_from_slice(&45.0f32.to_be_bytes()); // yaw
        data.extend_from_slice(&(-30.0f32).to_be_bytes()); // pitch
        data.push(1); // on_ground = true

        let pos_rot = PlayerPositionAndRotation::decode(&data).unwrap();
        assert_eq!(pos_rot.x, 100.0);
        assert_eq!(pos_rot.y, 64.0);
        assert_eq!(pos_rot.z, 200.0);
        assert_eq!(pos_rot.yaw, 45.0);
        assert_eq!(pos_rot.pitch, -30.0);
        assert!(pos_rot.on_ground);
    }

    #[test]
    fn test_player_rotation_decode() {
        let mut data = Vec::new();
        data.extend_from_slice(&90.0f32.to_be_bytes()); // yaw
        data.extend_from_slice(&(-45.0f32).to_be_bytes()); // pitch
        data.push(0); // on_ground = false

        let rot = PlayerRotation::decode(&data).unwrap();
        assert_eq!(rot.yaw, 90.0);
        assert_eq!(rot.pitch, -45.0);
        assert!(!rot.on_ground);
    }

    #[test]
    fn test_player_status_only_decode() {
        let data = vec![1u8];
        let status = PlayerStatusOnly::decode(&data).unwrap();
        assert!(status.on_ground);

        let data = vec![0u8];
        let status = PlayerStatusOnly::decode(&data).unwrap();
        assert!(!status.on_ground);
    }

    #[test]
    fn test_player_command_decode() {
        let mut data = Vec::new();
        VarInt(42).write(&mut data).unwrap(); // entity_id
        VarInt(3).write(&mut data).unwrap(); // action_id (sprint)
        VarInt(0).write(&mut data).unwrap(); // jump_boost

        let cmd = PlayerCommand::decode(&data).unwrap();
        assert_eq!(cmd.entity_id, 42);
        assert_eq!(cmd.action_id, 3);
        assert_eq!(cmd.jump_boost, 0);
    }

    #[test]
    fn test_swing_decode() {
        let mut data = Vec::new();
        VarInt(0).write(&mut data).unwrap(); // main hand

        let swing = Swing::decode(&data).unwrap();
        assert_eq!(swing.hand, 0);

        let mut data = Vec::new();
        VarInt(1).write(&mut data).unwrap(); // off hand

        let swing = Swing::decode(&data).unwrap();
        assert_eq!(swing.hand, 1);
    }

    #[test]
    fn test_set_carried_item_decode() {
        let data = 5i16.to_be_bytes().to_vec();
        let item = SetCarriedItem::decode(&data).unwrap();
        assert_eq!(item.slot, 5);

        let data = 0i16.to_be_bytes().to_vec();
        let item = SetCarriedItem::decode(&data).unwrap();
        assert_eq!(item.slot, 0);
    }

    #[test]
    fn test_set_carried_item_valid_slots() {
        for slot in 0..=8i16 {
            let item = SetCarriedItem { slot };
            assert!(item.is_valid_slot(), "slot {} should be valid", slot);
        }
    }

    #[test]
    fn test_set_carried_item_invalid_slots() {
        let invalid_slots: &[i16] = &[-1, -100, 9, 10, 255, i16::MAX, i16::MIN];
        for &slot in invalid_slots {
            let item = SetCarriedItem { slot };
            assert!(!item.is_valid_slot(), "slot {} should be invalid", slot);
        }
    }
    #[test]
    fn test_encode_entity_animation_main_hand() {
        let packet = encode_entity_animation(42, 0);
        assert_eq!(packet.id, 0x03);
        let mut cursor = Cursor::new(&packet.data);
        let eid = VarInt::read(&mut cursor).unwrap().0;
        assert_eq!(eid, 42);
        let pos = cursor.position() as usize;
        assert_eq!(packet.data[pos], 0);
    }

    #[test]
    fn test_encode_entity_animation_offhand() {
        let packet = encode_entity_animation(7, 3);
        assert_eq!(packet.id, 0x03);
        let mut cursor = Cursor::new(&packet.data);
        let eid = VarInt::read(&mut cursor).unwrap().0;
        assert_eq!(eid, 7);
        let pos = cursor.position() as usize;
        assert_eq!(packet.data[pos], 3);
    }

    #[test]
    fn test_encode_set_entity_metadata() {
        let metadata = vec![0x00, 0x00, 0x02];
        let packet = encode_set_entity_metadata(99, &metadata);
        assert_eq!(packet.id, 0x60);
        let mut cursor = Cursor::new(&packet.data);
        let eid = VarInt::read(&mut cursor).unwrap().0;
        assert_eq!(eid, 99);
        let pos = cursor.position() as usize;
        assert_eq!(&packet.data[pos..pos + 3], &[0x00, 0x00, 0x02]);
        assert_eq!(packet.data[pos + 3], 0xFF);
    }

    #[test]
    fn test_encode_entity_base_flags_metadata_sneaking() {
        let result = encode_entity_base_flags_metadata(0x02);
        assert_eq!(result, vec![0x00, 0x00, 0x02]);
    }

    #[test]
    fn test_encode_entity_base_flags_metadata_sprinting() {
        let result = encode_entity_base_flags_metadata(0x08);
        assert_eq!(result, vec![0x00, 0x00, 0x08]);
    }

    #[test]
    fn test_encode_entity_base_flags_metadata_both() {
        let result = encode_entity_base_flags_metadata(0x02 | 0x08);
        assert_eq!(result, vec![0x00, 0x00, 0x0A]);
    }

    #[test]
    fn test_player_position_valid() {
        let pos = PlayerPosition {
            x: 100.0,
            y: 64.0,
            z: -200.0,
            on_ground: true,
        };
        assert!(pos.is_valid());
    }

    #[test]
    fn test_player_position_invalid_nan() {
        let pos = PlayerPosition {
            x: f64::NAN,
            y: 64.0,
            z: 0.0,
            on_ground: false,
        };
        assert!(!pos.is_valid());
    }

    #[test]
    fn test_player_position_invalid_infinity() {
        let pos = PlayerPosition {
            x: 0.0,
            y: f64::INFINITY,
            z: 0.0,
            on_ground: false,
        };
        assert!(!pos.is_valid());
    }

    #[test]
    fn test_player_position_invalid_overflow() {
        let pos = PlayerPosition {
            x: 30_000_001.0,
            y: 64.0,
            z: 0.0,
            on_ground: false,
        };
        assert!(!pos.is_valid());

        let pos_z = PlayerPosition {
            x: 0.0,
            y: 64.0,
            z: -30_000_001.0,
            on_ground: false,
        };
        assert!(!pos_z.is_valid());
    }

    #[test]
    fn test_player_position_invalid_y() {
        let below = PlayerPosition {
            x: 0.0,
            y: -65.0,
            z: 0.0,
            on_ground: false,
        };
        assert!(!below.is_valid());

        let above = PlayerPosition {
            x: 0.0,
            y: 321.0,
            z: 0.0,
            on_ground: false,
        };
        assert!(!above.is_valid());
    }

    #[test]
    fn test_player_position_and_rotation_valid() {
        let pr = PlayerPositionAndRotation {
            x: 0.0,
            y: 64.0,
            z: 0.0,
            yaw: 180.0,
            pitch: 45.0,
            on_ground: true,
        };
        assert!(pr.is_valid());
    }

    #[test]
    fn test_player_position_and_rotation_invalid_pitch() {
        let pr = PlayerPositionAndRotation {
            x: 0.0,
            y: 64.0,
            z: 0.0,
            yaw: 0.0,
            pitch: 91.0,
            on_ground: false,
        };
        assert!(!pr.is_valid());

        let pr_neg = PlayerPositionAndRotation {
            x: 0.0,
            y: 64.0,
            z: 0.0,
            yaw: 0.0,
            pitch: -91.0,
            on_ground: false,
        };
        assert!(!pr_neg.is_valid());
    }

    #[test]
    fn test_player_rotation_valid() {
        let rot = PlayerRotation {
            yaw: 359.0,
            pitch: -90.0,
            on_ground: true,
        };
        assert!(rot.is_valid());
    }

    #[test]
    fn test_player_rotation_invalid_pitch() {
        let rot = PlayerRotation {
            yaw: 0.0,
            pitch: 90.1,
            on_ground: false,
        };
        assert!(!rot.is_valid());

        let rot_neg = PlayerRotation {
            yaw: 0.0,
            pitch: -90.1,
            on_ground: false,
        };
        assert!(!rot_neg.is_valid());
    }

    #[test]
    fn test_player_rotation_invalid_nan() {
        let rot = PlayerRotation {
            yaw: f32::NAN,
            pitch: 0.0,
            on_ground: false,
        };
        assert!(!rot.is_valid());
    }

    #[test]
    fn test_player_command_valid() {
        for action in 0..=6 {
            let cmd = PlayerCommand {
                entity_id: 1,
                action_id: action,
                jump_boost: 50,
            };
            assert!(cmd.is_valid(), "action_id {} should be valid", action);
        }
        let cmd = PlayerCommand {
            entity_id: 1,
            action_id: 0,
            jump_boost: 100,
        };
        assert!(cmd.is_valid());
    }

    #[test]
    fn test_player_command_invalid_action() {
        let cmd = PlayerCommand {
            entity_id: 1,
            action_id: 7,
            jump_boost: 0,
        };
        assert!(!cmd.is_valid());

        let cmd_neg = PlayerCommand {
            entity_id: 1,
            action_id: -1,
            jump_boost: 0,
        };
        assert!(!cmd_neg.is_valid());
    }

    #[test]
    fn test_player_command_invalid_jump_boost() {
        let cmd = PlayerCommand {
            entity_id: 1,
            action_id: 0,
            jump_boost: 101,
        };
        assert!(!cmd.is_valid());

        let cmd_neg = PlayerCommand {
            entity_id: 1,
            action_id: 0,
            jump_boost: -1,
        };
        assert!(!cmd_neg.is_valid());
    }

    #[test]
    fn test_swing_valid() {
        assert!(Swing { hand: 0 }.is_valid());
        assert!(Swing { hand: 1 }.is_valid());
    }

    #[test]
    fn test_swing_invalid() {
        assert!(!Swing { hand: 2 }.is_valid());
        assert!(!Swing { hand: -1 }.is_valid());
        assert!(!Swing { hand: 100 }.is_valid());
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
                prop_assert_eq!(packet.id, ids::TRANSFER);

                let mut cursor = Cursor::new(&packet.data);
                let decoded_host = read_string(&mut cursor).unwrap();
                let decoded_port = VarInt::read(&mut cursor).unwrap().0;
                prop_assert_eq!(decoded_host, host);
                prop_assert_eq!(decoded_port, port);
            }
        }
    }
}
