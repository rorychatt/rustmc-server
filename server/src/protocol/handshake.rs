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
}
