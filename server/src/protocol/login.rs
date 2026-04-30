use super::packet::Packet;
use super::types::{read_string, write_string, VarInt};
use std::io::{self, Cursor, Read};
use uuid::Uuid;

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

    mod proptest_tests {
        use super::*;
        use proptest::prelude::*;

        // Valid Minecraft username pattern: 3-16 chars, alphanumeric + underscore
        fn minecraft_username_strategy() -> impl Strategy<Value = String> {
            "[a-zA-Z0-9_]{3,16}"
        }

        proptest! {
            #[test]
            fn test_login_start_roundtrip(username in minecraft_username_strategy(), uuid_bytes in prop::array::uniform16(any::<u8>())) {
                let uuid = Uuid::from_bytes(uuid_bytes);
                let mut data = Vec::new();
                write_string(&mut data, &username).unwrap();
                data.extend_from_slice(uuid.as_bytes());

                let login = LoginStart::decode(&data).unwrap();
                prop_assert_eq!(login.name, username);
                prop_assert_eq!(login.uuid, uuid);
            }

            #[test]
            fn test_login_success_roundtrip(username in minecraft_username_strategy(), uuid_bytes in prop::array::uniform16(any::<u8>())) {
                let uuid = Uuid::from_bytes(uuid_bytes);
                let success = LoginSuccess::new(uuid, username.clone());
                let packet = success.to_packet().unwrap();

                prop_assert_eq!(packet.id, 0x02);

                // Verify packet contains UUID at the start
                let uuid_from_packet = Uuid::from_bytes(packet.data[0..16].try_into().unwrap());
                prop_assert_eq!(uuid_from_packet, uuid);
            }

            #[test]
            fn test_set_compression_encoding(threshold in any::<i32>()) {
                let packet = encode_set_compression(threshold);
                prop_assert_eq!(packet.id, 0x03);

                // Decode the threshold from packet data
                let decoded_threshold = VarInt::read(&mut Cursor::new(&packet.data)).unwrap().0;
                prop_assert_eq!(decoded_threshold, threshold);
            }
        }
    }
}
