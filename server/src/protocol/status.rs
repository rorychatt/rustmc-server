use super::packet::Packet;
use super::types::write_string;
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
        Self {
            version: StatusVersion {
                name: "1.20.4".to_string(),
                protocol: 765,
            },
            players: StatusPlayers {
                max: max_players,
                online: online_count,
                sample: Vec::new(),
            },
            description: StatusDescription {
                text: "RustMC Server - A Rust-powered Minecraft server".to_string(),
            },
            enforces_secure_chat: false,
        }
    }

    pub fn to_packet(&self) -> io::Result<Packet> {
        let json =
            serde_json::to_string(self).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let mut data = Vec::new();
        write_string(&mut data, &json)?;
        Ok(Packet::new(0x00, data))
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
    Packet::new(0x01, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_response_serialization() {
        let response = StatusResponse::default_response(0, 20);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("RustMC Server"));
        assert!(json.contains("\"protocol\":765"));
    }

    #[test]
    fn test_ping_pong_roundtrip() {
        let payload: i64 = 123456789;
        let data = payload.to_be_bytes().to_vec();
        let decoded = decode_ping_request(&data).unwrap();
        assert_eq!(decoded, payload);

        let pong = encode_pong_response(decoded);
        assert_eq!(pong.id, 0x01);
        assert_eq!(pong.data, data);
    }
}
