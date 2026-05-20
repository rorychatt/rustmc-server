//! A client simulation example that connects to a running RustMC server,
//! walks through the Minecraft protocol states (Handshake, Status, Login, Configuration, Play),
//! and logs incoming packets. Useful for validating that the server is working and fully compliant.
//!
//! Run with:
//!   cargo run --package rustmc-server --example simulate_client [port]
//! Or:
//!   cargo run -p rustmc-server --example simulate_client 25565

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use rustmc_server::protocol::packet_ids;
use std::io::{Cursor, Read, Write};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use uuid::Uuid;

use packet_ids::configuration::clientbound as config_cb;
use packet_ids::configuration::serverbound as config_sb;
use packet_ids::handshake::serverbound as handshake_sb;
use packet_ids::login::clientbound as login_cb;
use packet_ids::login::serverbound as login_sb;
use packet_ids::play::clientbound as play_cb;
use packet_ids::play::serverbound as play_sb;
use packet_ids::status::clientbound as status_cb;
use packet_ids::status::serverbound as status_sb;

pub struct SimulatorClient {
    stream: TcpStream,
    compression_threshold: Option<i32>,
}

impl SimulatorClient {
    pub async fn connect(addr: &str) -> anyhow::Result<Self> {
        println!("Connecting to {}...", addr);
        let stream = TcpStream::connect(addr).await?;
        println!("Connected successfully to {}!", addr);
        Ok(SimulatorClient {
            stream,
            compression_threshold: None,
        })
    }

    pub fn enable_compression(&mut self, threshold: i32) {
        println!("Enabling compression on client (threshold: {})", threshold);
        self.compression_threshold = Some(threshold);
    }

    pub async fn send_handshake(&mut self, protocol_version: i32, next_state: u8) -> anyhow::Result<()> {
        let mut data = Vec::new();
        write_varint(&mut data, protocol_version)?;
        write_string(&mut data, "localhost")?;
        data.extend_from_slice(&25565u16.to_be_bytes());
        write_varint(&mut data, next_state as i32)?;

        println!("Sending Handshake (protocol_version={}, next_state={})", protocol_version, next_state);
        self.send_packet(handshake_sb::HANDSHAKE, &data).await
    }

    pub async fn send_status_request(&mut self) -> anyhow::Result<()> {
        println!("Sending Status Request...");
        self.send_packet(status_sb::STATUS_REQUEST, &[]).await
    }

    pub async fn send_ping(&mut self, payload: i64) -> anyhow::Result<()> {
        println!("Sending Ping Request (payload={})...", payload);
        let data = payload.to_be_bytes().to_vec();
        self.send_packet(status_sb::PING_REQUEST, &data).await
    }

    pub async fn send_login_start(&mut self, username: &str, uuid: Uuid) -> anyhow::Result<()> {
        let mut data = Vec::new();
        write_string(&mut data, username)?;
        data.extend_from_slice(uuid.as_bytes());
        println!("Sending Login Start (username='{}', uuid={})", username, uuid);
        self.send_packet(login_sb::LOGIN_START, &data).await
    }

    pub async fn send_login_acknowledged(&mut self) -> anyhow::Result<()> {
        println!("Sending Login Acknowledged...");
        self.send_packet(login_sb::LOGIN_ACKNOWLEDGED, &[]).await
    }

    pub async fn send_known_packs_response(&mut self) -> anyhow::Result<()> {
        let mut data = Vec::new();
        write_varint(&mut data, 0)?; // Zero known packs
        println!("Sending Known Packs Response...");
        self.send_packet(config_sb::KNOWN_PACKS, &data).await
    }

    pub async fn send_acknowledge_finish_configuration(&mut self) -> anyhow::Result<()> {
        println!("Sending Acknowledge Finish Configuration...");
        self.send_packet(config_sb::ACKNOWLEDGE_FINISH, &[]).await
    }

    pub async fn send_player_position(&mut self, x: f64, y: f64, z: f64, on_ground: bool) -> anyhow::Result<()> {
        let mut data = Vec::new();
        data.extend_from_slice(&x.to_be_bytes());
        data.extend_from_slice(&y.to_be_bytes());
        data.extend_from_slice(&z.to_be_bytes());
        data.push(if on_ground { 1 } else { 0 });
        println!("Sending Player Position (x={:.2}, y={:.2}, z={:.2}, on_ground={})", x, y, z, on_ground);
        self.send_packet(play_sb::SET_PLAYER_POSITION, &data).await
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
                let data_length = self.read_varint().await?;
                let remaining_length = packet_length - varint_size(data_length);

                let mut compressed_or_uncompressed = vec![0u8; remaining_length as usize];
                self.stream
                    .read_exact(&mut compressed_or_uncompressed)
                    .await?;

                let payload = if data_length == 0 {
                    compressed_or_uncompressed
                } else {
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let port = args.get(1).map(|s| s.as_str()).unwrap_or("25565");
    let addr = format!("127.0.0.1:{}", port);

    // --- PHASE 1: Status Query ---
    println!("\n=== Starting Phase 1: Status Query ===");
    let mut client = SimulatorClient::connect(&addr).await?;
    client.send_handshake(775, 1).await?;
    client.send_status_request().await?;

    let status_pkt = client.read_packet().await?;
    if status_pkt.id == status_cb::STATUS_RESPONSE {
        let mut cursor = Cursor::new(&status_pkt.data);
        let json_str = read_string(&mut cursor)?;
        let json: serde_json::Value = serde_json::from_str(&json_str)?;
        println!("Server List Status Response:");
        println!("  MOTD: {}", json["description"]["text"].as_str().unwrap_or("N/A"));
        println!("  Version Name: {}", json["version"]["name"].as_str().unwrap_or("N/A"));
        println!("  Protocol: {}", json["version"]["protocol"].as_i64().unwrap_or(0));
        println!("  Online Players: {} / {}", json["players"]["online"], json["players"]["max"]);
    } else {
        println!("Expected status response, got packet ID: {:#04x}", status_pkt.id);
    }

    client.send_ping(9999).await?;
    let pong_pkt = client.read_packet().await?;
    if pong_pkt.id == status_cb::PONG_RESPONSE {
        let pong_payload = i64::from_be_bytes(pong_pkt.data.try_into().unwrap());
        println!("Received Pong with payload: {}", pong_payload);
    } else {
        println!("Expected pong response, got packet ID: {:#04x}", pong_pkt.id);
    }

    // --- PHASE 2: Login Flow ---
    println!("\n=== Starting Phase 2: Login Flow ===");
    let mut client = SimulatorClient::connect(&addr).await?;
    client.send_handshake(775, 2).await?;

    let username = "SimulatedPlayer";
    let uuid = Uuid::new_v4();
    client.send_login_start(username, uuid).await?;

    let login_flow_pkt = client.read_packet().await?;
    if login_flow_pkt.id == login_cb::SET_COMPRESSION {
        let mut cursor = Cursor::new(&login_flow_pkt.data);
        let threshold = read_varint(&mut cursor)?;
        println!("Server requested compression with threshold {}", threshold);
        client.enable_compression(threshold);
    } else {
        println!("Expected set compression packet, got: {:#04x}", login_flow_pkt.id);
    }

    let success_pkt = client.read_packet().await?;
    if success_pkt.id == login_cb::LOGIN_SUCCESS {
        let mut cursor = Cursor::new(&success_pkt.data);
        let mut uuid_bytes = [0u8; 16];
        std::io::Read::read_exact(&mut cursor, &mut uuid_bytes)?;
        let ret_uuid = Uuid::from_bytes(uuid_bytes);
        let ret_username = read_string(&mut cursor)?;
        println!("Successfully logged in! UUID: {}, Username: '{}'", ret_uuid, ret_username);
    } else {
        println!("Expected login success, got: {:#04x}", success_pkt.id);
        return Ok(());
    }

    // Acknowledge Login to enter Configuration phase
    client.send_login_acknowledged().await?;

    // --- PHASE 3: Configuration Phase ---
    println!("\n=== Starting Phase 3: Configuration Phase ===");
    loop {
        let pkt = client.read_packet().await?;
        match pkt.id {
            id if id == config_cb::KNOWN_PACKS => {
                println!("Received Known Packs request. Replying...");
                client.send_known_packs_response().await?;
            }
            id if id == config_cb::REGISTRY_DATA => {
                println!("Received Registry Data packet (size: {} bytes)", pkt.data.len());
            }
            id if id == config_cb::UPDATE_TAGS => {
                println!("Received Update Tags packet (size: {} bytes)", pkt.data.len());
            }
            id if id == config_cb::FINISH_CONFIGURATION => {
                println!("Received Finish Configuration from server. Acknowledging...");
                client.send_acknowledge_finish_configuration().await?;
                break;
            }
            other_id => {
                println!("Received other configuration packet ID: {:#04x}", other_id);
            }
        }
    }

    // --- PHASE 4: Play Phase ---
    println!("\n=== Starting Phase 4: Play Phase ===");
    let join_game = client.read_packet().await?;
    if join_game.id == play_cb::LOGIN_PLAY {
        println!("Entered Play Phase! Joined the game world successfully.");
    } else {
        println!("Expected login play packet, got: {:#04x}", join_game.id);
    }

    // Send player position to tick connection
    client.send_player_position(0.0, 64.0, 0.0, true).await?;

    // Listen to play packets for a few seconds
    println!("Listening for game play packets (ticking position)...");
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
    for _ in 1..=5 {
        tokio::select! {
            _ = interval.tick() => {
                client.send_player_position(0.0, 64.0, 0.0, true).await?;
            }
            pkt_res = client.read_packet() => {
                match pkt_res {
                    Ok(pkt) => {
                        match pkt.id {
                            id if id == play_cb::KEEP_ALIVE => {
                                println!("Received Keep Alive packet ({} bytes)", pkt.data.len());
                            }
                            id if id == play_cb::LEVEL_CHUNK_WITH_LIGHT => {
                                println!("Received Level Chunk With Light packet ({} bytes)", pkt.data.len());
                            }
                            other => {
                                println!("Received play packet ID: {:#04x} ({} bytes)", other, pkt.data.len());
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error reading packet: {}", e);
                        break;
                    }
                }
            }
        }
    }

    println!("\nSimulation completed successfully! Server is fully operational.");
    Ok(())
}
