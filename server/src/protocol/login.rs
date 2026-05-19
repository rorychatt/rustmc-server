use super::packet::Packet;
use super::types::{read_string, write_string, VarInt};
use std::io::{self, Cursor, Read, Write};
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

pub fn encode_login_cookie_request(key: &str) -> io::Result<Packet> {
    let mut data = Vec::new();
    write_string(&mut data, key)?;
    Ok(Packet::new(0x05, data))
}

pub fn encode_login_store_cookie(key: &str, payload: &[u8]) -> io::Result<Packet> {
    let mut data = Vec::new();
    write_string(&mut data, key)?;
    data.write_all(payload)?;
    Ok(Packet::new(0x06, data))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginCookieResponse {
    pub key: String,
    pub payload: Option<Vec<u8>>,
}

impl LoginCookieResponse {
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

    #[test]
    fn test_encode_login_cookie_request() {
        let packet = encode_login_cookie_request("minecraft:auth_token").unwrap();
        assert_eq!(packet.id, 0x05);
        assert!(!packet.data.is_empty());
    }

    #[test]
    fn test_encode_login_store_cookie() {
        let payload = b"session_data_here";
        let packet = encode_login_store_cookie("minecraft:session", payload).unwrap();
        assert_eq!(packet.id, 0x06);
        assert!(!packet.data.is_empty());
    }

    #[test]
    fn test_login_cookie_response_decode_with_payload() {
        let mut data = Vec::new();
        write_string(&mut data, "minecraft:auth").unwrap();
        data.push(1); // has_payload
        let payload = b"auth_data";
        VarInt(payload.len() as i32).write(&mut data).unwrap();
        data.extend_from_slice(payload);

        let response = LoginCookieResponse::decode(&data).unwrap();
        assert_eq!(response.key, "minecraft:auth");
        assert_eq!(response.payload, Some(b"auth_data".to_vec()));
    }

    #[test]
    fn test_login_cookie_response_decode_without_payload() {
        let mut data = Vec::new();
        write_string(&mut data, "minecraft:empty").unwrap();
        data.push(0); // no payload

        let response = LoginCookieResponse::decode(&data).unwrap();
        assert_eq!(response.key, "minecraft:empty");
        assert_eq!(response.payload, None);
    }

    mod proptest_tests {
        use super::*;
        use proptest::prelude::*;

        fn minecraft_username_strategy() -> impl Strategy<Value = String> {
            "[a-zA-Z0-9_]{3,16}"
        }

        fn identifier_strategy() -> impl Strategy<Value = String> {
            "[a-z][a-z0-9_]{0,15}:[a-z][a-z0-9_/]{0,30}"
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

                let uuid_from_packet = Uuid::from_bytes(packet.data[0..16].try_into().unwrap());
                prop_assert_eq!(uuid_from_packet, uuid);
            }

            #[test]
            fn test_set_compression_encoding(threshold in any::<i32>()) {
                let packet = encode_set_compression(threshold);
                prop_assert_eq!(packet.id, 0x03);

                let decoded_threshold = VarInt::read(&mut Cursor::new(&packet.data)).unwrap().0;
                prop_assert_eq!(decoded_threshold, threshold);
            }

            #[test]
            fn test_login_cookie_response_roundtrip(
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

                let response = LoginCookieResponse::decode(&data).unwrap();
                prop_assert_eq!(&response.key, &key);
                if has_payload {
                    prop_assert_eq!(response.payload, Some(payload_data));
                } else {
                    prop_assert_eq!(response.payload, None);
                }
            }
        }
    }
}
