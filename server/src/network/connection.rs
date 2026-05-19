use std::collections::HashMap;
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::protocol::chunk_data;
use crate::protocol::configuration;
use crate::protocol::handshake::{Handshake, NextState};
use crate::protocol::login::{LoginCookieResponse, LoginStart, LoginSuccess};
use crate::protocol::packet::{Packet, PacketWriter};
use crate::protocol::play;
use crate::protocol::status::{
    decode_ping_request, decode_status_request, encode_pong_response, StatusResponse,
};
use crate::protocol::types::VarInt;
use crate::registry;
use crate::world::chunk::ChunkPos;
use crate::world::World;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Handshake,
    Status,
    Login,
    Configuration,
    Play,
}

pub struct Connection {
    addr: SocketAddr,
    state: ConnectionState,
    world: Arc<RwLock<World>>,
    player_uuid: Option<Uuid>,
    player_name: Option<String>,
    compression_enabled: bool,

    configuration_finish_sent: bool,
    last_keep_alive_sent: Option<Instant>,
    last_keep_alive_id: i64,
    last_keep_alive_response: Option<Instant>,
    pending_chunks: Vec<ChunkPos>,
    cookies: HashMap<String, Vec<u8>>,

}

impl Connection {
    pub fn new(addr: SocketAddr, world: Arc<RwLock<World>>) -> Self {
        Self {
            addr,
            state: ConnectionState::Handshake,
            world,
            player_uuid: None,
            player_name: None,
            compression_enabled: false,

            configuration_finish_sent: false,
            last_keep_alive_sent: None,
            last_keep_alive_id: 0,
            last_keep_alive_response: None,
            pending_chunks: Vec::new(),
            cookies: HashMap::new(),

        }
    }

    pub fn get_cookie(&self, key: &str) -> Option<&Vec<u8>> {
        self.cookies.get(key)
    }

    pub fn set_cookie(&mut self, key: String, payload: Vec<u8>) {
        self.cookies.insert(key, payload);
    }

    pub fn remove_cookie(&mut self, key: &str) -> Option<Vec<u8>> {
        self.cookies.remove(key)
    }

    pub async fn handle(mut self, stream: TcpStream) {
        info!("New connection from {}", self.addr);

        let (reader, writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut writer = BufWriter::new(writer);

        let keep_alive_interval = Duration::from_secs(15);
        let keep_alive_timeout = Duration::from_secs(30);

        loop {
            if self.state == ConnectionState::Play {
                let timeout = if let Some(last_sent) = self.last_keep_alive_sent {
                    let elapsed = last_sent.elapsed();
                    if elapsed >= keep_alive_interval {
                        Duration::ZERO
                    } else {
                        keep_alive_interval - elapsed
                    }
                } else {
                    Duration::ZERO
                };

                tokio::select! {
                    result = self.read_and_handle_packet(&mut reader, &mut writer) => {
                        match result {
                            Ok(true) => continue,
                            Ok(false) => {
                                debug!("Connection from {} closed normally", self.addr);
                                break;
                            }
                            Err(e) => {
                                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                                    debug!("Connection from {} disconnected", self.addr);
                                } else {
                                    error!("Error handling connection from {}: {}", self.addr, e);
                                }
                                break;
                            }
                        }
                    }
                    _ = tokio::time::sleep(timeout) => {
                        // Check for keep-alive timeout
                        if let Some(last_sent) = self.last_keep_alive_sent {
                            if let Some(last_response) = self.last_keep_alive_response {
                                if last_sent.duration_since(last_response) > keep_alive_timeout {
                                    info!("Client {} timed out (no keep-alive response)", self.addr);
                                    break;
                                }
                            } else if last_sent.elapsed() > keep_alive_timeout {
                                info!("Client {} timed out (no keep-alive response)", self.addr);
                                break;
                            }
                        }

                        // Send keep-alive
                        let id = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as i64;
                        let packet = play::encode_keep_alive(id);
                        if let Err(e) = self.write_packet(&mut writer, &packet).await {
                            error!("Failed to send keep-alive to {}: {}", self.addr, e);
                            break;
                        }
                        self.last_keep_alive_id = id;
                        self.last_keep_alive_sent = Some(Instant::now());
                    }
                }
            } else {
                match self.read_and_handle_packet(&mut reader, &mut writer).await {
                    Ok(true) => continue,
                    Ok(false) => {
                        debug!("Connection from {} closed normally", self.addr);
                        break;
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::UnexpectedEof {
                            debug!("Connection from {} disconnected", self.addr);
                        } else {
                            error!("Error handling connection from {}: {}", self.addr, e);
                        }
                        break;
                    }
                }
            }
        }
    }

    async fn read_and_handle_packet(
        &mut self,
        reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
        writer: &mut BufWriter<tokio::net::tcp::OwnedWriteHalf>,
    ) -> std::io::Result<bool> {
        let length = self.read_varint_async(reader).await?;
        if length == 0 {
            return Ok(false);
        }
        if length > 2_097_152 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Packet too large",
            ));
        }

        let mut payload = vec![0u8; length as usize];
        reader.read_exact(&mut payload).await?;

        let mut cursor = Cursor::new(&payload);
        let packet_id = VarInt::read(&mut cursor)?.0;
        let data_start = cursor.position() as usize;
        let data = payload[data_start..].to_vec();

        match self.state {
            ConnectionState::Handshake => self.handle_handshake(packet_id, &data, writer).await,
            ConnectionState::Status => self.handle_status(packet_id, &data, writer).await,
            ConnectionState::Login => self.handle_login(packet_id, &data, writer).await,
            ConnectionState::Configuration => {
                self.handle_configuration(packet_id, &data, writer).await
            }
            ConnectionState::Play => self.handle_play(packet_id, &data, writer).await,
        }
    }

    async fn read_varint_async(
        &self,
        reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
    ) -> std::io::Result<i32> {
        let mut result: i32 = 0;
        let mut shift: u32 = 0;
        loop {
            let byte = reader.read_u8().await?;
            result |= ((byte & 0x7F) as i32) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
            if shift >= 32 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "VarInt too long",
                ));
            }
        }
        Ok(result)
    }

    async fn write_packet(
        &self,
        writer: &mut BufWriter<tokio::net::tcp::OwnedWriteHalf>,
        packet: &Packet,
    ) -> std::io::Result<()> {
        let mut packet_data = Vec::new();
        let mut packet_writer = PacketWriter::new(&mut packet_data);

        if self.compression_enabled {
            packet_writer.set_compression_threshold(256);
        }

        packet_writer.write_packet(packet)?;
        writer.write_all(&packet_data).await?;
        writer.flush().await
    }

    async fn handle_handshake(
        &mut self,
        packet_id: i32,
        data: &[u8],
        _writer: &mut BufWriter<tokio::net::tcp::OwnedWriteHalf>,
    ) -> std::io::Result<bool> {
        if packet_id != 0x00 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Expected handshake packet 0x00, got {packet_id:#04x}"),
            ));
        }

        let handshake = Handshake::decode(data)?;
        debug!(
            "Handshake from {}: protocol={}, address={}:{}, next_state={:?}",
            self.addr,
            handshake.protocol_version,
            handshake.server_address,
            handshake.server_port,
            handshake.next_state
        );

        self.state = match handshake.next_state {
            NextState::Status => ConnectionState::Status,
            NextState::Login => ConnectionState::Login,
        };

        Ok(true)
    }

    async fn handle_status(
        &mut self,
        packet_id: i32,
        data: &[u8],
        writer: &mut BufWriter<tokio::net::tcp::OwnedWriteHalf>,
    ) -> std::io::Result<bool> {
        match packet_id {
            0x00 => {
                decode_status_request(data)?;
                let world = self.world.read().await;
                let response = StatusResponse::default_response(world.player_count() as i32, 20);
                let packet = response.to_packet()?;
                self.write_packet(writer, &packet).await?;
                Ok(true)
            }
            0x01 => {
                let payload = decode_ping_request(data)?;
                let pong = encode_pong_response(payload);
                self.write_packet(writer, &pong).await?;
                Ok(false) // Close after pong
            }
            _ => {
                warn!("Unknown status packet: {packet_id:#04x}");
                Ok(true)
            }
        }
    }

    async fn handle_login(
        &mut self,
        packet_id: i32,
        data: &[u8],
        writer: &mut BufWriter<tokio::net::tcp::OwnedWriteHalf>,
    ) -> std::io::Result<bool> {
        match packet_id {
            0x00 => {
                let login = LoginStart::decode(data)?;
                info!("Player login: {} ({})", login.name, login.uuid);

                // Enable compression with 256 byte threshold
                let compression_packet = crate::protocol::login::encode_set_compression(256);
                self.write_packet(writer, &compression_packet).await?;
                self.compression_enabled = true;

                let success = LoginSuccess::new(login.uuid, login.name.clone());
                let packet = success.to_packet()?;
                self.write_packet(writer, &packet).await?;

                // Store player info for later use
                self.player_uuid = Some(login.uuid);
                self.player_name = Some(login.name);

                // Transition to Configuration state (wait for Login Acknowledged from client)
                self.state = ConnectionState::Configuration;

                Ok(true)
            }
            0x04 => {
                let response = LoginCookieResponse::decode(data)?;
                debug!("Received login cookie response: key={}", response.key);
                if let Some(payload) = response.payload {
                    self.cookies.insert(response.key, payload);
                } else {
                    self.cookies.remove(&response.key);
                }
                Ok(true)
            }
            _ => {
                warn!("Unknown login packet: {packet_id:#04x}");
                Ok(true)
            }
        }
    }

    async fn handle_configuration(
        &mut self,
        packet_id: i32,
        _data: &[u8],
        writer: &mut BufWriter<tokio::net::tcp::OwnedWriteHalf>,
    ) -> std::io::Result<bool> {
        match packet_id {
            // Cookie Response (0x02)
            0x02 => {
                let response = configuration::CookieResponse::decode(_data)?;
                debug!(
                    "Received configuration cookie response: key={}",
                    response.key
                );
                if let Some(payload) = response.payload {
                    self.cookies.insert(response.key, payload);
                } else {
                    self.cookies.remove(&response.key);
                }
                Ok(true)
            }

            0x03 => {
                if !self.configuration_finish_sent {
                    // First 0x03: Login Acknowledged — send configuration data
                    debug!("Client acknowledged login, sending configuration data");
                    self.send_configuration_data(writer).await?;
                } else {
                    // Second 0x03: Acknowledge Finish Configuration — transition to Play
                    debug!("Client acknowledged finish configuration, transitioning to Play");
                    self.state = ConnectionState::Play;
                    self.send_play_login_sequence(writer).await?;
                }
                Ok(true)
            }
            0x07 => {
                debug!("Received Known Packs response from client");
                self.send_registry_data(writer).await?;
                Ok(true)
            }
            _ => {
                debug!(
                    "Unhandled configuration packet: {packet_id:#04x} ({} bytes)",
                    _data.len()
                );
                Ok(true)
            }
        }
    }

    async fn send_configuration_data(
        &mut self,
        writer: &mut BufWriter<tokio::net::tcp::OwnedWriteHalf>,
    ) -> std::io::Result<()> {
        // Send Known Packs
        let known_packs = configuration::encode_known_packs()?;
        self.write_packet(writer, &known_packs).await?;
        Ok(())
    }

    async fn send_registry_data(
        &mut self,
        writer: &mut BufWriter<tokio::net::tcp::OwnedWriteHalf>,
    ) -> std::io::Result<()> {
        for reg_id in registry::ALL_REGISTRY_IDS {
            let entries = registry::load(reg_id)?;
            let packet = configuration::encode_registry_data(reg_id, &entries)?;
            self.write_packet(writer, &packet).await?;
        }

        let tags = configuration::encode_update_tags()?;
        self.write_packet(writer, &tags).await?;


        // Send Finish Configuration — remain in Configuration state
        // and wait for client to send Acknowledge Finish Configuration (0x03)
        let finish = configuration::encode_finish_configuration();
        self.write_packet(writer, &finish).await?;
        self.configuration_finish_sent = true;


        Ok(())
    }

    async fn send_play_login_sequence(
        &mut self,
        writer: &mut BufWriter<tokio::net::tcp::OwnedWriteHalf>,
    ) -> std::io::Result<()> {
        let uuid = self.player_uuid.unwrap();
        let name = self.player_name.clone().unwrap();

        let entity_id = {
            let mut world = self.world.write().await;
            world.add_player(uuid, name.clone())
        };

        // 1. Login (Play) packet
        let login_play = play::encode_login_play(entity_id)?;
        self.write_packet(writer, &login_play).await?;

        // 2. Player Info Update (required for client to finalize join)
        let player_info = play::encode_player_info_update(uuid, &name, 1); // game_mode=1 (creative)
        self.write_packet(writer, &player_info).await?;

        // 3. Synchronize Player Position
        let pos = play::encode_player_position_and_look(0.0, 64.0, 0.0, 0.0, 0.0, 0, 0);
        self.write_packet(writer, &pos).await?;

        // 4. Game Event (Start waiting for level chunks, event=13, value=0)
        let game_event = play::encode_game_event(13, 0.0);
        self.write_packet(writer, &game_event).await?;

        // 5. Set Center Chunk (required before sending chunk data)
        let player_chunk_x = 0;
        let player_chunk_z = 0;
        let center_chunk = play::encode_set_center_chunk(player_chunk_x, player_chunk_z);
        self.write_packet(writer, &center_chunk).await?;

        // 6. Chunk Batch Start
        let batch_start = play::encode_chunk_batch_start();
        self.write_packet(writer, &batch_start).await?;

        // 7. Send initial chunks around spawn
        let view_distance = 8;

        let mut initial_chunks = std::collections::HashSet::new();
        let mut chunk_count = 0;
        {
            let mut world = self.world.write().await;
            for cx in (player_chunk_x - view_distance)..=(player_chunk_x + view_distance) {
                for cz in (player_chunk_z - view_distance)..=(player_chunk_z + view_distance) {
                    let pos = ChunkPos::new(cx, cz);
                    initial_chunks.insert(pos);
                    let chunk = world.get_or_generate_chunk(pos);
                    let packet = chunk_data::encode_chunk_data(chunk)?;
                    self.write_packet(writer, &packet).await?;
                    chunk_count += 1;
                }
            }

            // Mark chunks as loaded for the player
            if let Some(player) = world.players.get_mut(&uuid) {
                player.loaded_chunks = initial_chunks;
            }
        }

        // 8. Chunk Batch Finished
        let batch_finished = play::encode_chunk_batch_finished(chunk_count)?;
        self.write_packet(writer, &batch_finished).await?;

        // Initialize keep-alive tracking
        self.last_keep_alive_response = Some(Instant::now());

        info!("Sent play login sequence to player {}", name);

        Ok(())
    }

    async fn drain_pending_chunks(
        &mut self,
        writer: &mut BufWriter<tokio::net::tcp::OwnedWriteHalf>,
        limit: f32,
    ) -> std::io::Result<()> {
        if self.pending_chunks.is_empty() {
            return Ok(());
        }

        let send_count = (limit.ceil() as usize).min(self.pending_chunks.len());
        let to_send: Vec<ChunkPos> = self.pending_chunks.drain(..send_count).collect();

        let batch_start = play::encode_chunk_batch_start();
        self.write_packet(writer, &batch_start).await?;

        let mut count = 0;
        {
            let mut world = self.world.write().await;
            for chunk_pos in &to_send {
                let chunk = world.get_or_generate_chunk(*chunk_pos);
                let chunk_packet = chunk_data::encode_chunk_data(chunk)?;
                self.write_packet(writer, &chunk_packet).await?;
                count += 1;
            }
        }

        let batch_finished = play::encode_chunk_batch_finished(count)?;
        self.write_packet(writer, &batch_finished).await?;

        Ok(())
    }

    async fn handle_play(
        &mut self,
        packet_id: i32,
        data: &[u8],
        writer: &mut BufWriter<tokio::net::tcp::OwnedWriteHalf>,
    ) -> std::io::Result<bool> {
        match packet_id {
            // Confirm Teleportation
            0x00 => {
                debug!("Received teleport confirmation");
            }
            // Chat Command
            0x07 => {
                let command = play::ChatCommand::decode(data)?;
                if command.command.starts_with("transfer ") {
                    let parts: Vec<&str> = command.command.splitn(3, ' ').collect();
                    if parts.len() == 3 {
                        if let Ok(port) = parts[2].parse::<i32>() {
                            let packet = play::encode_transfer(parts[1], port)?;
                            self.write_packet(writer, &packet).await?;
                            return Ok(false);
                        }
                    }
                }
                debug!("Received chat command: {}", command.command);
            }
            // Chat Message
            0x09 => {
                let chat = play::ChatMessage::decode(data)?;
                info!("Chat from {}: {}", self.addr, chat.message);
            }
            // Chunk Batch Received
            0x0B => {
                if data.len() >= 4 {
                    let chunks_per_tick = f32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                    let clamped = chunks_per_tick.clamp(1.0, 100.0);
                    if let Some(uuid) = self.player_uuid {
                        let mut world = self.world.write().await;
                        if let Some(player) = world.players.get_mut(&uuid) {
                            player.chunks_per_tick = clamped;
                        }
                    }
                    debug!("Chunk batch received: chunks_per_tick={:.1}", clamped);
                    self.drain_pending_chunks(writer, clamped).await?;
                }
            }
            // Play Cookie Response
            0x12 => {
                let response = play::PlayCookieResponse::decode(data)?;
                debug!("Received play cookie response: key={}", response.key);
                if let Some(payload) = response.payload {
                    self.cookies.insert(response.key, payload);
                } else {
                    self.cookies.remove(&response.key);
                }
            }
            // Client Tick End
            0x0D => {
                if let Some(uuid) = self.player_uuid {
                    let world = self.world.read().await;
                    let limit = world
                        .players
                        .get(&uuid)
                        .map(|p| p.chunks_per_tick)
                        .unwrap_or(25.0);
                    drop(world);
                    self.drain_pending_chunks(writer, limit).await?;
                }
            }
            // Keep Alive (serverbound)
            0x1C => {
                debug!("Received keep-alive response from {}", self.addr);
                self.last_keep_alive_response = Some(Instant::now());

            }
            // Set Player Position
            0x1E => {
                let pos = play::PlayerPosition::decode(data)?;
                debug!("Player position: ({}, {}, {})", pos.x, pos.y, pos.z);

                if let Some(uuid) = self.player_uuid {
                    let mut world = self.world.write().await;
                    world.update_player_position(&uuid, pos.x, pos.y, pos.z);

                    let view_distance = 8;
                    if let Some(update) = world.compute_chunk_updates(&uuid, view_distance) {
                        for chunk_pos in &update.to_unload {
                            let unload_packet = play::encode_unload_chunk(chunk_pos.x, chunk_pos.z);
                            self.write_packet(writer, &unload_packet).await?;
                        }

                        if !update.to_load.is_empty() {
                            self.pending_chunks.extend(update.to_load.iter());
                        }

                        debug!(
                            "Chunk update: queued {}, unloaded {}",
                            update.to_load.len(),
                            update.to_unload.len()
                        );
                    }

                    let limit = world
                        .players
                        .get(&uuid)
                        .map(|p| p.chunks_per_tick)
                        .unwrap_or(25.0);
                    drop(world);

                    self.drain_pending_chunks(writer, limit).await?;
                }
            }
            // Set Player Position and Rotation
            0x1F => {
                debug!("Player position and rotation ({} bytes)", data.len());
            }
            // Player Loaded
            0x2C => {
                debug!("Player loaded signal received");
            }
            _ => {
                debug!(
                    "Unhandled play packet: {packet_id:#04x} ({} bytes)",
                    data.len()
                );
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_connection() -> Connection {
        let world = Arc::new(RwLock::new(World::new()));
        let addr: SocketAddr = "127.0.0.1:25565".parse().unwrap();
        Connection::new(addr, world)
    }

    #[test]
    fn test_set_and_get_cookie() {
        let mut conn = make_connection();
        conn.set_cookie("minecraft:test".to_string(), vec![1, 2, 3]);
        assert_eq!(conn.get_cookie("minecraft:test"), Some(&vec![1, 2, 3]));
    }

    #[test]
    fn test_get_cookie_missing() {
        let conn = make_connection();
        assert_eq!(conn.get_cookie("minecraft:nonexistent"), None);
    }

    #[test]
    fn test_remove_cookie() {
        let mut conn = make_connection();
        conn.set_cookie("minecraft:session".to_string(), vec![10, 20]);
        let removed = conn.remove_cookie("minecraft:session");
        assert_eq!(removed, Some(vec![10, 20]));
        assert_eq!(conn.get_cookie("minecraft:session"), None);
    }

    #[test]
    fn test_remove_cookie_missing() {
        let mut conn = make_connection();
        let removed = conn.remove_cookie("minecraft:missing");
        assert_eq!(removed, None);
    }

    #[test]
    fn test_cookie_overwrite() {
        let mut conn = make_connection();
        conn.set_cookie("minecraft:key".to_string(), vec![1]);
        conn.set_cookie("minecraft:key".to_string(), vec![2, 3]);
        assert_eq!(conn.get_cookie("minecraft:key"), Some(&vec![2, 3]));
    }

    #[test]
    fn test_none_payload_removes_cookie() {
        let mut conn = make_connection();
        conn.set_cookie("minecraft:transfer".to_string(), vec![5, 6, 7]);

        // Simulate what the handler does when payload is None
        let payload: Option<Vec<u8>> = None;
        let key = "minecraft:transfer".to_string();
        if let Some(p) = payload {
            conn.cookies.insert(key, p);
        } else {
            conn.cookies.remove(&key);
        }

        assert_eq!(conn.get_cookie("minecraft:transfer"), None);
    }
}
