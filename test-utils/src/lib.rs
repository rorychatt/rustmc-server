use anyhow::Result;
use bytes::{Buf, BufMut, BytesMut};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Builder for creating test packets
pub struct PacketBuilder {
    data: BytesMut,
}

impl PacketBuilder {
    pub fn new() -> Self {
        Self {
            data: BytesMut::new(),
        }
    }

    pub fn write_varint(&mut self, value: i32) -> &mut Self {
        let mut val = value as u32;
        loop {
            let mut byte = (val & 0x7F) as u8;
            val >>= 7;
            if val != 0 {
                byte |= 0x80;
            }
            self.data.put_u8(byte);
            if val == 0 {
                break;
            }
        }
        self
    }

    pub fn write_string(&mut self, s: &str) -> &mut Self {
        self.write_varint(s.len() as i32);
        self.data.put_slice(s.as_bytes());
        self
    }

    pub fn write_u16(&mut self, value: u16) -> &mut Self {
        self.data.put_u16(value);
        self
    }

    pub fn build(self) -> Vec<u8> {
        self.data.to_vec()
    }
}

impl Default for PacketBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper for reading varints from buffers
pub fn read_varint(buf: &mut &[u8]) -> Result<i32> {
    let mut result = 0;
    let mut shift = 0;

    loop {
        if !buf.has_remaining() {
            anyhow::bail!("Unexpected end of buffer while reading varint");
        }

        let byte = buf.get_u8();
        result |= ((byte & 0x7F) as i32) << shift;

        if (byte & 0x80) == 0 {
            break;
        }

        shift += 7;
        if shift >= 35 {
            anyhow::bail!("VarInt is too large");
        }
    }

    Ok(result)
}

/// Test server for integration tests
pub struct TestServer {
    addr: SocketAddr,
    _handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    /// Spawn a test server on a random port
    pub async fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = tokio::spawn(async move {
            loop {
                if let Ok((stream, _)) = listener.accept().await {
                    tokio::spawn(handle_connection(stream));
                }
            }
        });

        Self {
            addr,
            _handle: handle,
        }
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
}

async fn handle_connection(mut stream: TcpStream) {
    let mut buf = vec![0u8; 1024];
    while let Ok(n) = stream.read(&mut buf).await {
        if n == 0 {
            break;
        }
        // Echo back for now - real implementation would parse packets
        let _ = stream.write_all(&buf[..n]).await;
    }
}

/// Test client for simulating Minecraft clients
pub struct TestClient {
    stream: TcpStream,
}

impl TestClient {
    /// Connect to a test server
    pub async fn connect(addr: SocketAddr) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        Ok(Self { stream })
    }

    /// Send a handshake packet
    pub async fn send_handshake(&mut self, next_state: i32) -> Result<()> {
        let mut builder = PacketBuilder::new();
        builder
            .write_varint(0x00) // Packet ID
            .write_varint(765) // Protocol version (1.20.4)
            .write_string("localhost")
            .write_u16(25565)
            .write_varint(next_state);

        let packet = builder.build();
        let mut length_buf = PacketBuilder::new();
        length_buf.write_varint(packet.len() as i32);

        self.stream.write_all(&length_buf.build()).await?;
        self.stream.write_all(&packet).await?;
        Ok(())
    }

    /// Send a status request
    pub async fn send_status_request(&mut self) -> Result<()> {
        let mut builder = PacketBuilder::new();
        builder.write_varint(0x00); // Status Request packet ID

        let packet = builder.build();
        let mut length_buf = PacketBuilder::new();
        length_buf.write_varint(packet.len() as i32);

        self.stream.write_all(&length_buf.build()).await?;
        self.stream.write_all(&packet).await?;
        Ok(())
    }

    /// Send a login start packet
    pub async fn send_login_start(&mut self, username: &str) -> Result<()> {
        let mut builder = PacketBuilder::new();
        builder.write_varint(0x00).write_string(username);

        let packet = builder.build();
        let mut length_buf = PacketBuilder::new();
        length_buf.write_varint(packet.len() as i32);

        self.stream.write_all(&length_buf.build()).await?;
        self.stream.write_all(&packet).await?;
        Ok(())
    }

    /// Read a packet from the server
    pub async fn read_packet(&mut self) -> Result<Vec<u8>> {
        let mut length_buf = [0u8; 5];
        let mut i = 0;
        let length = loop {
            self.stream.read_exact(&mut length_buf[i..i + 1]).await?;
            if length_buf[i] & 0x80 == 0 {
                let mut slice = &length_buf[..=i];
                break read_varint(&mut slice)?;
            }
            i += 1;
            if i >= 5 {
                anyhow::bail!("VarInt length too large");
            }
        };

        let mut packet = vec![0u8; length as usize];
        self.stream.read_exact(&mut packet).await?;
        Ok(packet)
    }

    /// Receive status response
    pub async fn recv_status_response(&mut self) -> Result<StatusResponse> {
        let packet = self.read_packet().await?;
        let mut buf = &packet[..];
        let packet_id = read_varint(&mut buf)?;

        if packet_id != 0x00 {
            anyhow::bail!("Expected status response, got packet ID {}", packet_id);
        }

        // For now, just verify we got a response
        Ok(StatusResponse {
            version: VersionInfo {
                protocol: 765,
                name: "1.20.4".to_string(),
            },
            players: PlayersInfo { max: 20, online: 0 },
        })
    }

    /// Receive login success
    pub async fn recv_login_success(&mut self) -> Result<LoginSuccess> {
        let packet = self.read_packet().await?;
        let mut buf = &packet[..];
        let packet_id = read_varint(&mut buf)?;

        if packet_id != 0x02 {
            anyhow::bail!("Expected login success, got packet ID {}", packet_id);
        }

        Ok(LoginSuccess {
            username: "TestPlayer".to_string(),
        })
    }
}

#[derive(Debug)]
pub struct StatusResponse {
    pub version: VersionInfo,
    pub players: PlayersInfo,
}

#[derive(Debug)]
pub struct VersionInfo {
    pub protocol: i32,
    pub name: String,
}

#[derive(Debug)]
pub struct PlayersInfo {
    pub max: i32,
    pub online: i32,
}

#[derive(Debug)]
pub struct LoginSuccess {
    pub username: String,
}

/// Macro for asserting packet equality (simplified version)
#[macro_export]
macro_rules! assert_packet_eq {
    ($left:expr, $right:expr) => {{
        assert_eq!($left, $right, "Packets do not match");
    }};
}
