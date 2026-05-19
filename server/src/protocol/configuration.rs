use super::packet::Packet;
use super::types::{read_string, write_string, VarInt};
use std::io::{self, Cursor, Read, Write};

pub fn encode_cookie_request(key: &str) -> io::Result<Packet> {
    let mut data = Vec::new();
    write_string(&mut data, key)?;
    Ok(Packet::new(0x01, data))
}

pub fn encode_store_cookie(key: &str, payload: &[u8]) -> io::Result<Packet> {
    let mut data = Vec::new();
    write_string(&mut data, key)?;
    data.write_all(payload)?;
    Ok(Packet::new(0x0B, data))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookieResponse {
    pub key: String,
    pub payload: Option<Vec<u8>>,
}

impl CookieResponse {
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

pub fn encode_known_packs() -> io::Result<Packet> {
    let mut data = Vec::new();
    VarInt(1).write(&mut data)?;
    write_string(&mut data, "minecraft")?;
    write_string(&mut data, "core")?;
    write_string(&mut data, "1.21")?;
    Ok(Packet::new(0x0E, data))
}

pub fn encode_registry_data(registry_id: &str, entries: &[RegistryEntry]) -> io::Result<Packet> {
    let mut data = Vec::new();
    write_string(&mut data, registry_id)?;
    VarInt(entries.len() as i32).write(&mut data)?;
    for entry in entries {
        write_string(&mut data, &entry.id)?;
        data.push(1);
        data.extend_from_slice(&entry.nbt_data);
    }
    Ok(Packet::new(0x07, data))
}

pub fn encode_update_tags() -> io::Result<Packet> {
    let mut data = Vec::new();
    VarInt(0).write(&mut data)?;
    Ok(Packet::new(0x0D, data))
}

pub fn encode_finish_configuration() -> Packet {
    Packet::new(0x03, Vec::new())
}

pub struct RegistryEntry {
    pub id: String,
    pub nbt_data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry;

    #[test]
    fn test_encode_known_packs() {
        let packet = encode_known_packs().unwrap();
        assert_eq!(packet.id, 0x0E);
        assert!(!packet.data.is_empty());
    }

    #[test]
    fn test_encode_finish_configuration() {
        let packet = encode_finish_configuration();
        assert_eq!(packet.id, 0x03);
        assert!(packet.data.is_empty());
    }

    #[test]
    fn test_encode_registry_data() {
        let entries = registry::load("minecraft:dimension_type").unwrap();
        let packet = encode_registry_data("minecraft:dimension_type", entries).unwrap();
        assert_eq!(packet.id, 0x07);
        assert!(!packet.data.is_empty());
    }

    #[test]
    fn test_encode_update_tags() {
        let packet = encode_update_tags().unwrap();
        assert_eq!(packet.id, 0x0D);
    }

    #[test]
    fn test_dimension_type_registry() {
        let entries = registry::load("minecraft:dimension_type").unwrap();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].id, "minecraft:overworld");
    }

    #[test]
    fn test_biome_registry() {
        let entries = registry::load("minecraft:worldgen/biome").unwrap();
        assert!(entries.len() >= 50);
        assert!(entries.iter().any(|e| e.id == "minecraft:plains"));
    }

    #[test]
    fn test_damage_type_registry() {
        let entries = registry::load("minecraft:damage_type").unwrap();
        assert!(entries.len() >= 40);
    }

    #[test]
    fn test_painting_variant_registry() {
        let entries = registry::load("minecraft:painting_variant").unwrap();
        assert!(entries.len() >= 26);
        assert!(entries.iter().any(|e| e.id == "minecraft:kebab"));
    }

    #[test]
    fn test_wolf_variant_registry() {
        let entries = registry::load("minecraft:wolf_variant").unwrap();
        assert_eq!(entries.len(), 9);
        assert!(entries.iter().any(|e| e.id == "minecraft:pale"));
    }

    #[test]
    fn test_encode_cookie_request() {
        let packet = encode_cookie_request("minecraft:test_cookie").unwrap();
        assert_eq!(packet.id, 0x01);
        assert!(!packet.data.is_empty());
    }

    #[test]
    fn test_encode_store_cookie() {
        let payload = b"hello world";
        let packet = encode_store_cookie("minecraft:session", payload).unwrap();
        assert_eq!(packet.id, 0x0B);
        assert!(!packet.data.is_empty());
    }

    #[test]
    fn test_cookie_response_decode_with_payload() {
        let mut data = Vec::new();
        write_string(&mut data, "minecraft:my_cookie").unwrap();
        data.push(1); // has_payload = true
        let payload = b"test_data";
        VarInt(payload.len() as i32).write(&mut data).unwrap();
        data.extend_from_slice(payload);

        let response = CookieResponse::decode(&data).unwrap();
        assert_eq!(response.key, "minecraft:my_cookie");
        assert_eq!(response.payload, Some(b"test_data".to_vec()));
    }

    #[test]
    fn test_cookie_response_decode_without_payload() {
        let mut data = Vec::new();
        write_string(&mut data, "minecraft:empty").unwrap();
        data.push(0); // has_payload = false

        let response = CookieResponse::decode(&data).unwrap();
        assert_eq!(response.key, "minecraft:empty");
        assert_eq!(response.payload, None);
    }

    #[test]
    fn test_cookie_response_max_payload() {
        let mut data = Vec::new();
        write_string(&mut data, "minecraft:big").unwrap();
        data.push(1);
        let payload = vec![0xAB; 5120];
        VarInt(5120).write(&mut data).unwrap();
        data.extend_from_slice(&payload);

        let response = CookieResponse::decode(&data).unwrap();
        assert_eq!(response.payload.unwrap().len(), 5120);
    }

    #[test]
    fn test_cookie_response_payload_too_large() {
        let mut data = Vec::new();
        write_string(&mut data, "minecraft:toobig").unwrap();
        data.push(1);
        VarInt(5121).write(&mut data).unwrap();
        data.extend(vec![0u8; 5121]);

        let result = CookieResponse::decode(&data);
        assert!(result.is_err());
    }

    mod proptest_tests {
        use super::*;
        use proptest::prelude::*;

        fn identifier_strategy() -> impl Strategy<Value = String> {
            "[a-z][a-z0-9_]{0,15}:[a-z][a-z0-9_/]{0,30}"
        }

        proptest! {
            #[test]
            fn test_cookie_response_roundtrip(
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

                let response = CookieResponse::decode(&data).unwrap();
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
