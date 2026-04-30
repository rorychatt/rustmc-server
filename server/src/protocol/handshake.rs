use super::types::{read_string, VarInt};
use std::io::{self, Cursor, Read};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NextState {
    Status = 1,
    Login = 2,
}

#[derive(Debug, Clone)]
pub struct Handshake {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: NextState,
}

impl Handshake {
    pub fn decode(data: &[u8]) -> io::Result<Self> {
        let mut cursor = Cursor::new(data);
        let protocol_version = VarInt::read(&mut cursor)?.0;
        let server_address = read_string(&mut cursor)?;
        let mut port_buf = [0u8; 2];
        cursor.read_exact(&mut port_buf)?;
        let server_port = u16::from_be_bytes(port_buf);
        let next_state_raw = VarInt::read(&mut cursor)?.0;
        let next_state = match next_state_raw {
            1 => NextState::Status,
            2 => NextState::Login,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid next_state: {next_state_raw}"),
                ))
            }
        };

        Ok(Handshake {
            protocol_version,
            server_address,
            server_port,
            next_state,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::types::{write_string, VarInt};

    #[test]
    fn test_handshake_decode() {
        let mut data = Vec::new();
        VarInt(765).write(&mut data).unwrap(); // Protocol version 1.20.4
        write_string(&mut data, "localhost").unwrap();
        data.extend_from_slice(&25565u16.to_be_bytes());
        VarInt(1).write(&mut data).unwrap(); // Status

        let handshake = Handshake::decode(&data).unwrap();
        assert_eq!(handshake.protocol_version, 765);
        assert_eq!(handshake.server_address, "localhost");
        assert_eq!(handshake.server_port, 25565);
        assert_eq!(handshake.next_state, NextState::Status);
    }

    mod proptest_tests {
        use super::*;
        use crate::protocol::types::{write_string, VarInt};
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn test_handshake_roundtrip_valid(
                protocol_version in 0..10000i32,
                server_address in "\\PC{0,255}",
                server_port in any::<u16>(),
                next_state_val in prop::sample::select(vec![1i32, 2i32])
            ) {
                let mut data = Vec::new();
                VarInt(protocol_version).write(&mut data).unwrap();
                write_string(&mut data, &server_address).unwrap();
                data.extend_from_slice(&server_port.to_be_bytes());
                VarInt(next_state_val).write(&mut data).unwrap();

                let handshake = Handshake::decode(&data).unwrap();
                prop_assert_eq!(handshake.protocol_version, protocol_version);
                prop_assert_eq!(handshake.server_address, server_address);
                prop_assert_eq!(handshake.server_port, server_port);

                let expected_state = if next_state_val == 1 {
                    NextState::Status
                } else {
                    NextState::Login
                };
                prop_assert_eq!(handshake.next_state, expected_state);
            }

            #[test]
            fn test_handshake_rejects_invalid_next_state(
                protocol_version in 0..10000i32,
                server_address in "\\PC{0,100}",
                server_port in any::<u16>(),
                invalid_next_state in prop::sample::select(vec![0i32, 3i32, -1i32, 100i32])
            ) {
                let mut data = Vec::new();
                VarInt(protocol_version).write(&mut data).unwrap();
                write_string(&mut data, &server_address).unwrap();
                data.extend_from_slice(&server_port.to_be_bytes());
                VarInt(invalid_next_state).write(&mut data).unwrap();

                let result = Handshake::decode(&data);
                prop_assert!(result.is_err());
                prop_assert!(result.unwrap_err().to_string().contains("Invalid next_state"));
            }

            #[test]
            fn test_handshake_rejects_truncated(protocol_version in 0..10000i32) {
                let mut data = Vec::new();
                VarInt(protocol_version).write(&mut data).unwrap();

                let result = Handshake::decode(&data);
                prop_assert!(result.is_err());
            }
        }
    }
}
