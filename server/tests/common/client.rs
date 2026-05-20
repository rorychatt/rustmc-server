use flate2::read::ZlibDecoder;
use rustmc_server::protocol::packet_ids;
use std::io::{Cursor, Read, Write};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use uuid::Uuid;

use packet_ids::configuration::serverbound as config_sb;
use packet_ids::handshake::serverbound as handshake_sb;
use packet_ids::login::serverbound as login_sb;
use packet_ids::play::serverbound as play_sb;
use packet_ids::status::serverbound as status_sb;

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

        self.send_packet(handshake_sb::HANDSHAKE, &data).await
    }

    #[allow(dead_code)]
    pub async fn send_status_request(&mut self) -> anyhow::Result<()> {
        self.send_packet(status_sb::STATUS_REQUEST, &[]).await
    }

    #[allow(dead_code)]
    pub async fn send_ping(&mut self, payload: i64) -> anyhow::Result<()> {
        let data = payload.to_be_bytes().to_vec();
        self.send_packet(status_sb::PING_REQUEST, &data).await
    }

    pub async fn send_login_start(&mut self, username: &str, uuid: Uuid) -> anyhow::Result<()> {
        let mut data = Vec::new();
        write_string(&mut data, username)?;
        data.extend_from_slice(uuid.as_bytes());
        self.send_packet(login_sb::LOGIN_START, &data).await
    }

    pub async fn send_login_acknowledged(&mut self) -> anyhow::Result<()> {
        self.send_packet(login_sb::LOGIN_ACKNOWLEDGED, &[]).await
    }

    pub async fn send_known_packs_response(&mut self) -> anyhow::Result<()> {
        let mut data = Vec::new();
        write_varint(&mut data, 0)?; // Zero known packs
        self.send_packet(config_sb::KNOWN_PACKS, &data).await
    }

    #[allow(dead_code)]
    pub async fn send_acknowledge_finish_configuration(&mut self) -> anyhow::Result<()> {
        self.send_packet(config_sb::ACKNOWLEDGE_FINISH, &[]).await
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
        self.send_packet(play_sb::SET_PLAYER_POSITION, &data).await
    }

    #[allow(dead_code)]
    pub async fn send_chat_command(&mut self, command: &str) -> anyhow::Result<()> {
        let mut data = Vec::new();
        write_string(&mut data, command)?;
        self.send_packet(play_sb::CHAT_COMMAND, &data).await
    }

    #[allow(dead_code)]
    pub async fn send_chat_message(&mut self, message: &str) -> anyhow::Result<()> {
        let mut data = Vec::new();
        write_string(&mut data, message)?;
        self.send_packet(play_sb::CHAT_MESSAGE, &data).await
    }

    #[allow(dead_code)]
    pub async fn send_confirm_teleportation(&mut self, teleport_id: i32) -> anyhow::Result<()> {
        let mut data = Vec::new();
        write_varint(&mut data, teleport_id)?;
        self.send_packet(play_sb::CONFIRM_TELEPORTATION, &data).await
    }

    #[allow(dead_code)]
    pub async fn send_player_loaded(&mut self) -> anyhow::Result<()> {
        self.send_packet(play_sb::PLAYER_LOADED, &[]).await
    }

    #[allow(dead_code)]
    pub async fn send_chunk_batch_received(&mut self, chunks_per_tick: f32) -> anyhow::Result<()> {
        let data = chunks_per_tick.to_be_bytes().to_vec();
        self.send_packet(play_sb::CHUNK_BATCH_RECEIVED, &data).await
    }

    #[allow(dead_code)]
    pub async fn send_client_tick_end(&mut self) -> anyhow::Result<()> {
        self.send_packet(play_sb::CLIENT_TICK_END, &[]).await
    }

    async fn send_packet(&mut self, packet_id: i32, data: &[u8]) -> anyhow::Result<()> {
        let mut packet_buf = Vec::new();
        write_varint(&mut packet_buf, packet_id)?;
        packet_buf.extend_from_slice(data);

        let mut full_packet = Vec::new();

        match self.compression_threshold {
            None => {
                write_varint(&mut full_packet, packet_buf.len() as i32)?;
                full_packet.extend_from_slice(&packet_buf);
            }
            Some(threshold) if packet_buf.len() < threshold as usize => {
                let data_len_varint_size = varint_size(0);
                let packet_length = data_len_varint_size + packet_buf.len() as i32;
                write_varint(&mut full_packet, packet_length)?;
                write_varint(&mut full_packet, 0)?;
                full_packet.extend_from_slice(&packet_buf);
            }
            Some(_) => {
                use flate2::write::ZlibEncoder;
                use flate2::Compression;
                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&packet_buf)?;
                let compressed = encoder.finish()?;

                let uncompressed_len = packet_buf.len() as i32;
                let packet_length = varint_size(uncompressed_len) + compressed.len() as i32;
                write_varint(&mut full_packet, packet_length)?;
                write_varint(&mut full_packet, uncompressed_len)?;
                full_packet.extend_from_slice(&compressed);
            }
        }

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
    #[allow(dead_code)]
    pub fn read_transfer(&self) -> anyhow::Result<(String, i32)> {
        let mut cursor = Cursor::new(&self.data);
        let host = read_string(&mut cursor)?;
        let port = read_varint(&mut cursor)?;
        Ok((host, port))
    }

    #[allow(dead_code)]
    pub fn read_system_chat(&self) -> anyhow::Result<String> {
        let mut cursor = Cursor::new(&self.data);
        let json = read_string(&mut cursor)?;
        Ok(json)
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
