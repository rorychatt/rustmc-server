use flate2::read::ZlibDecoder;
use std::io::{Cursor, Read, Write};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use uuid::Uuid;

pub struct TestClient {
    stream: TcpStream,
    compression_threshold: Option<i32>,
}

impl TestClient {
    pub async fn connect(port: u16) -> anyhow::Result<Self> {
        let stream = TcpStream::connect(format!("127.0.0.1:{port}")).await?;
        Ok(TestClient {
            stream,
            compression_threshold: None,
        })
    }

    pub fn enable_compression(&mut self, threshold: i32) {
        self.compression_threshold = Some(threshold);
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

    pub async fn send_known_packs_response(&mut self) -> anyhow::Result<()> {
        let mut data = Vec::new();
        write_varint(&mut data, 0)?; // Zero known packs
        self.send_packet(0x07, &data).await
    }

    pub async fn send_acknowledge_finish_configuration(&mut self) -> anyhow::Result<()> {
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
        self.send_packet(0x1E, &data).await
    }

    pub async fn send_chat_command(&mut self, command: &str) -> anyhow::Result<()> {
        let mut data = Vec::new();
        write_string(&mut data, command)?;
        self.send_packet(0x07, &data).await
    }

    #[allow(dead_code)]
    pub async fn send_chat_message(&mut self, message: &str) -> anyhow::Result<()> {
        let mut data = Vec::new();
        write_string(&mut data, message)?;
        self.send_packet(0x09, &data).await
    }

    #[allow(dead_code)]
    pub async fn send_confirm_teleportation(&mut self, teleport_id: i32) -> anyhow::Result<()> {
        let mut data = Vec::new();
        write_varint(&mut data, teleport_id)?;
        self.send_packet(0x00, &data).await
    }

    #[allow(dead_code)]
    pub async fn send_player_loaded(&mut self) -> anyhow::Result<()> {
        self.send_packet(0x2C, &[]).await
    }

    #[allow(dead_code)]
    pub async fn send_chunk_batch_received(&mut self, chunks_per_tick: f32) -> anyhow::Result<()> {
        let data = chunks_per_tick.to_be_bytes().to_vec();
        self.send_packet(0x0B, &data).await
    }

    #[allow(dead_code)]
    pub async fn send_client_tick_end(&mut self) -> anyhow::Result<()> {
        self.send_packet(0x0D, &[]).await
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
        let packet_length = self.read_varint().await?;
        if packet_length == 0 {
            return Err(anyhow::anyhow!("Zero-length packet"));
        }

        match self.compression_threshold {
            None => {
                // No compression
                let mut payload = vec![0u8; packet_length as usize];
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
            Some(_) => {
                // Compression enabled
                let data_length = self.read_varint().await?;
                let remaining_length = packet_length - varint_size(data_length);

                let mut compressed_or_uncompressed = vec![0u8; remaining_length as usize];
                self.stream
                    .read_exact(&mut compressed_or_uncompressed)
                    .await?;

                let payload = if data_length == 0 {
                    // Below threshold - uncompressed
                    compressed_or_uncompressed
                } else {
                    // Above threshold - decompress
                    let mut decoder = ZlibDecoder::new(&compressed_or_uncompressed[..]);
                    let mut decompressed = Vec::new();
                    decoder.read_to_end(&mut decompressed)?;
                    decompressed
                };

                let mut cursor = Cursor::new(&payload);
                let packet_id = read_varint(&mut cursor)?;
                let data_start = cursor.position() as usize;
                let data = payload[data_start..].to_vec();

                Ok(RawPacket {
                    id: packet_id,
                    data,
                })
            }
        }
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

impl RawPacket {
    pub fn read_transfer(&self) -> anyhow::Result<(String, i32)> {
        let mut cursor = Cursor::new(&self.data);
        let host = read_string(&mut cursor)?;
        let port = read_varint(&mut cursor)?;
        Ok((host, port))
    }
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

fn varint_size(value: i32) -> i32 {
    let mut val = value as u32;
    let mut size = 0;
    loop {
        val >>= 7;
        size += 1;
        if val == 0 {
            break;
        }
    }
    size
}
