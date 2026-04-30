use std::io::{Cursor, Read, Write};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use uuid::Uuid;

pub struct TestClient {
    stream: TcpStream,
}

impl TestClient {
    pub async fn connect(port: u16) -> anyhow::Result<Self> {
        let stream = TcpStream::connect(format!("127.0.0.1:{port}")).await?;
        Ok(TestClient { stream })
    }

    pub async fn send_handshake(
        &mut self,
        protocol_version: i32,
        next_state: u8,
    ) -> anyhow::Result<()> {
        let mut data = Vec::new();
        write_varint(&mut data, protocol_version)?;
        write_string(&mut data, "localhost")?;
        data.extend_from_slice(&25565u16.to_be_bytes());
        write_varint(&mut data, next_state as i32)?;

        self.send_packet(0x00, &data).await
    }

    pub async fn send_status_request(&mut self) -> anyhow::Result<()> {
        self.send_packet(0x00, &[]).await
    }

    pub async fn send_ping(&mut self, payload: i64) -> anyhow::Result<()> {
        let data = payload.to_be_bytes().to_vec();
        self.send_packet(0x01, &data).await
    }

    pub async fn send_login_start(&mut self, username: &str, uuid: Uuid) -> anyhow::Result<()> {
        let mut data = Vec::new();
        write_string(&mut data, username)?;
        data.extend_from_slice(uuid.as_bytes());
        self.send_packet(0x00, &data).await
    }

    pub async fn send_login_acknowledged(&mut self) -> anyhow::Result<()> {
        self.send_packet(0x03, &[]).await
    }

    pub async fn send_player_position(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        on_ground: bool,
    ) -> anyhow::Result<()> {
        let mut data = Vec::new();
        data.extend_from_slice(&x.to_be_bytes());
        data.extend_from_slice(&y.to_be_bytes());
        data.extend_from_slice(&z.to_be_bytes());
        data.push(if on_ground { 1 } else { 0 });
        self.send_packet(0x14, &data).await
    }

    pub async fn send_chat_message(&mut self, message: &str) -> anyhow::Result<()> {
        let mut data = Vec::new();
        write_string(&mut data, message)?;
        self.send_packet(0x05, &data).await
    }

    async fn send_packet(&mut self, packet_id: i32, data: &[u8]) -> anyhow::Result<()> {
        let mut packet_buf = Vec::new();
        write_varint(&mut packet_buf, packet_id)?;
        packet_buf.extend_from_slice(data);

        let mut full_packet = Vec::new();
        write_varint(&mut full_packet, packet_buf.len() as i32)?;
        full_packet.extend_from_slice(&packet_buf);

        self.stream.write_all(&full_packet).await?;
        self.stream.flush().await?;
        Ok(())
    }

    pub async fn read_packet(&mut self) -> anyhow::Result<RawPacket> {
        let length = self.read_varint().await?;
        if length == 0 {
            return Err(anyhow::anyhow!("Zero-length packet"));
        }

        let mut payload = vec![0u8; length as usize];
        self.stream.read_exact(&mut payload).await?;

        let mut cursor = Cursor::new(&payload);
        let packet_id = read_varint(&mut cursor)?;
        let data_start = cursor.position() as usize;
        let data = payload[data_start..].to_vec();

        Ok(RawPacket {
            id: packet_id,
            data,
        })
    }

    async fn read_varint(&mut self) -> anyhow::Result<i32> {
        let mut result: i32 = 0;
        let mut shift: u32 = 0;
        loop {
            let byte = self.stream.read_u8().await?;
            result |= ((byte & 0x7F) as i32) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
            if shift >= 32 {
                return Err(anyhow::anyhow!("VarInt too long"));
            }
        }
        Ok(result)
    }
}

#[derive(Debug)]
pub struct RawPacket {
    pub id: i32,
    pub data: Vec<u8>,
}

fn write_varint(writer: &mut impl Write, value: i32) -> anyhow::Result<()> {
    let mut val = value as u32;
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if val != 0 {
            byte |= 0x80;
        }
        writer.write_all(&[byte])?;
        if val == 0 {
            break;
        }
    }
    Ok(())
}

fn read_varint(reader: &mut impl Read) -> anyhow::Result<i32> {
    let mut result: i32 = 0;
    let mut shift: u32 = 0;
    loop {
        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf)?;
        let byte = buf[0];
        result |= ((byte & 0x7F) as i32) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 32 {
            return Err(anyhow::anyhow!("VarInt too long"));
        }
    }
    Ok(result)
}

fn write_string(writer: &mut impl Write, s: &str) -> anyhow::Result<()> {
    write_varint(writer, s.len() as i32)?;
    writer.write_all(s.as_bytes())?;
    Ok(())
}

pub fn read_string(reader: &mut impl Read) -> anyhow::Result<String> {
    let len = read_varint(reader)? as usize;
    if len > 32767 {
        return Err(anyhow::anyhow!("String too long"));
    }
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    Ok(String::from_utf8(buf)?)
}
