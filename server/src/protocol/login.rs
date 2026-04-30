use std::io::{self, Cursor, Read};
use uuid::Uuid;
use super::types::{VarInt, read_string, write_string};
use super::packet::Packet;

#[derive(Debug, Clone)]
pub struct LoginStart {
    pub name: String,
    pub uuid: Uuid,
}

impl LoginStart {
    pub fn decode(data: &[u8]) -> io::Result<Self> {
        let mut cursor = Cursor::new(data);
        let name = read_string(&mut cursor)?;
        let mut uuid_bytes = [0u8; 16];
        cursor.read_exact(&mut uuid_bytes)?;
        let uuid = Uuid::from_bytes(uuid_bytes);
        Ok(Self { name, uuid })
    }
}

#[derive(Debug, Clone)]
pub struct LoginSuccess {
    pub uuid: Uuid,
    pub username: String,
}

impl LoginSuccess {
    pub fn new(uuid: Uuid, username: String) -> Self {
        Self { uuid, username }
    }

    pub fn to_packet(&self) -> io::Result<Packet> {
        let mut data = Vec::new();
        data.extend_from_slice(self.uuid.as_bytes());
        write_string(&mut data, &self.username)?;
        VarInt(0).write(&mut data)?; // Number of properties
        Ok(Packet::new(0x02, data))
    }
}

pub fn encode_set_compression(threshold: i32) -> Packet {
    let mut data = Vec::new();
    VarInt(threshold).write(&mut data).unwrap();
    Packet::new(0x03, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_start_decode() {
        let mut data = Vec::new();
        write_string(&mut data, "TestPlayer").unwrap();
        let uuid = Uuid::new_v4();
        data.extend_from_slice(uuid.as_bytes());

        let login = LoginStart::decode(&data).unwrap();
        assert_eq!(login.name, "TestPlayer");
        assert_eq!(login.uuid, uuid);
    }

    #[test]
    fn test_login_success_packet() {
        let uuid = Uuid::new_v4();
        let success = LoginSuccess::new(uuid, "TestPlayer".to_string());
        let packet = success.to_packet().unwrap();
        assert_eq!(packet.id, 0x02);
        assert!(packet.data.len() > 16); // UUID + string
    }
}
