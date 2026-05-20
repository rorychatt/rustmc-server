use super::packet::Packet;
use super::packet_ids::status::clientbound as ids;
use super::types::write_string;
use super::version::{PROTOCOL_VERSION, VERSION_NAME};
use serde::{Deserialize, Serialize};
use std::io::{self, Cursor, Read};

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub version: StatusVersion,
    pub players: StatusPlayers,
    pub description: StatusDescription,
    #[serde(rename = "enforcesSecureChat")]
    pub enforces_secure_chat: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusVersion {
    pub name: String,
    pub protocol: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusPlayers {
    pub max: i32,
    pub online: i32,
    #[serde(default)]
    pub sample: Vec<StatusPlayer>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusPlayer {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusDescription {
    pub text: String,
}

impl StatusResponse {
    pub fn default_response(online_count: i32, max_players: i32) -> Self {
        Self::new(
            online_count,
            max_players,
            "RustMC Server - A Rust-powered Minecraft server",
        )
    }

    pub fn new(online_count: i32, max_players: i32, motd: &str) -> Self {
        Self {
            version: StatusVersion {
                name: VERSION_NAME.to_string(),
                protocol: PROTOCOL_VERSION,
            },
            players: StatusPlayers {
                max: max_players,
                online: online_count,
                sample: Vec::new(),
            },
            description: StatusDescription {
                text: motd.to_string(),
            },
            enforces_secure_chat: false,
        }
    }

    pub fn to_packet(&self) -> io::Result<Packet> {
        let json = serde_json::to_string(self).map_err(io::Error::other)?;
        let mut data = Vec::new();
        write_string(&mut data, &json)?;
        Ok(Packet::new(ids::STATUS_RESPONSE, data))
    }
}

pub fn decode_status_request(data: &[u8]) -> io::Result<()> {
    if data.is_empty() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Status request should have no payload",
        ))
    }
}

pub fn decode_ping_request(data: &[u8]) -> io::Result<i64> {
    if data.len() != 8 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Ping payload must be 8 bytes",
        ));
    }
    let mut cursor = Cursor::new(data);
    let mut buf = [0u8; 8];
    cursor.read_exact(&mut buf)?;
    Ok(i64::from_be_bytes(buf))
}

pub fn encode_pong_response(payload: i64) -> Packet {
    let data = payload.to_be_bytes().to_vec();
    Packet::new(ids::PONG_RESPONSE, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_response_serialization() {
        let response = StatusResponse::default_response(0, 20);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("RustMC Server"));
        assert!(json.contains("\"protocol\":775"));
    }

    #[test]
    fn test_ping_pong_roundtrip() {
        let payload: i64 = 123456789;
        let data = payload.to_be_bytes().to_vec();
        let decoded = decode_ping_request(&data).unwrap();
        assert_eq!(decoded, payload);

        let pong = encode_pong_response(decoded);
        assert_eq!(pong.id, ids::PONG_RESPONSE);
        assert_eq!(pong.data, data);
    }

    mod proptest_tests {
        use super::*;
        use crate::protocol::types::read_string;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn test_ping_pong_roundtrip(payload in any::<i64>()) {
                let data = payload.to_be_bytes().to_vec();
                let decoded = decode_ping_request(&data).unwrap();
                prop_assert_eq!(decoded, payload);

                let pong = encode_pong_response(decoded);
                prop_assert_eq!(pong.id, ids::PONG_RESPONSE);
                prop_assert_eq!(pong.data, data);
            }

            #[test]
            fn test_ping_rejects_invalid_size(size in prop::sample::select(vec![0usize, 1, 2, 4, 7, 9, 16, 32])) {
                prop_assume!(size != 8);
                let data = vec![0u8; size];
                let result = decode_ping_request(&data);
                prop_assert!(result.is_err());
                prop_assert!(result.unwrap_err().to_string().contains("8 bytes"));
            }

            #[test]
            fn test_status_request_rejects_non_empty(data in prop::collection::vec(any::<u8>(), 1..100)) {
                let result = decode_status_request(&data);
                prop_assert!(result.is_err());
                prop_assert!(result.unwrap_err().to_string().contains("no payload"));
            }

            #[test]
            fn test_status_response_json_roundtrip(
                online in 0..10000i32,
                max_players in 1..10000i32,
                version_name in "\\PC{1,50}",
                protocol in 0..10000i32
            ) {
                let response = StatusResponse {
                    version: StatusVersion {
                        name: version_name.clone(),
                        protocol,
                    },
                    players: StatusPlayers {
                        max: max_players,
                        online,
                        sample: Vec::new(),
                    },
                    description: StatusDescription {
                        text: "Test server".to_string(),
                    },
                    enforces_secure_chat: false,
                };

                let packet = response.to_packet().unwrap();
                prop_assert_eq!(packet.id, ids::STATUS_RESPONSE);

                // Verify we can parse the JSON back
                let mut data_cursor = Cursor::new(&packet.data);
                let json_str = read_string(&mut data_cursor).unwrap();
                let parsed: StatusResponse = serde_json::from_str(&json_str).unwrap();

                prop_assert_eq!(parsed.version.name, version_name);
                prop_assert_eq!(parsed.version.protocol, protocol);
                prop_assert_eq!(parsed.players.online, online);
                prop_assert_eq!(parsed.players.max, max_players);
            }
        }

        #[test]
        fn test_status_request_accepts_empty() {
            let result = decode_status_request(&[]);
            assert!(result.is_ok());
        }
    }
}
