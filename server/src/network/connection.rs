use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
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
    pending_chunks: Vec<ChunkPos>,
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
            pending_chunks: Vec::new(),
        }
    }

    pub async fn handle(mut self, stream: TcpStream) {
        info!("New connection from {}", self.addr);

        let (reader, writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut writer = BufWriter::new(writer);

        loop {
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
                debug!("Received cookie response: key={}", response.key);
                Ok(true)
            }
            // Login Acknowledged (0x03) - client confirms transition from Login to Configuration
            0x03 => {
                debug!("Client acknowledged login, sending configuration data");
                self.send_configuration_data(writer).await?;
                Ok(true)
            }
            // Known Packs response (0x07) from client
            0x07 => {
                debug!("Received Known Packs response from client");
                // Client confirmed known packs; send registry data
                self.send_registry_data(writer).await?;
                Ok(true)
            }
            // Acknowledge Finish Configuration (0x03 again, but after we send Finish)
            // In practice the client sends 0x03 for both Login Acknowledged and
            // Acknowledge Finish Configuration. We handle this by tracking flow state
            // via whether we've already sent Finish Configuration.
            // However, the way the flow works:
            // 1. Client sends Login Acknowledged (0x03) -> we send Known Packs
            // 2. Client sends Known Packs (0x07) -> we send registries + finish
            // 3. Client sends Acknowledge Finish (0x03) -> transition to Play
            // Since 0x03 appears twice, we need the second occurrence.
            // The simplest approach: After sending finish, the next 0x03 transitions to Play.
            // We'll handle this by checking if it's the "second" 0x03.
            // Actually, let me re-read: After Login Success, client sends Login Acknowledged
            // which is serverbound Login packet 0x03. Then we enter Configuration.
            // In Configuration state, client's Acknowledge Finish is also 0x03.
            // Since handle_login already waits for 0x03 as "Login Acknowledged",
            // we need to restructure. Let me handle it differently:
            // The first time we get 0x03 in Configuration, it's actually the
            // "Login Acknowledged" if we enter Configuration immediately after Login Success.
            // But actually looking at the current code, handle_login sets state to Configuration
            // BEFORE reading the Login Acknowledged. So the first 0x03 here IS Login Acknowledged.
            // Let me fix this: we handle the flow linearly.
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
        // Send registry data for each required registry
        let dim_entries = configuration::dimension_type_registry()?;
        let dim_packet =
            configuration::encode_registry_data("minecraft:dimension_type", &dim_entries)?;
        self.write_packet(writer, &dim_packet).await?;

        let biome_entries = configuration::biome_registry()?;
        let biome_packet =
            configuration::encode_registry_data("minecraft:worldgen/biome", &biome_entries)?;
        self.write_packet(writer, &biome_packet).await?;

        let damage_entries = configuration::damage_type_registry()?;
        let damage_packet =
            configuration::encode_registry_data("minecraft:damage_type", &damage_entries)?;
        self.write_packet(writer, &damage_packet).await?;

        let painting_entries = configuration::painting_variant_registry()?;
        let painting_packet =
            configuration::encode_registry_data("minecraft:painting_variant", &painting_entries)?;
        self.write_packet(writer, &painting_packet).await?;

        let wolf_entries = configuration::wolf_variant_registry()?;
        let wolf_packet =
            configuration::encode_registry_data("minecraft:wolf_variant", &wolf_entries)?;
        self.write_packet(writer, &wolf_packet).await?;

        // Send Update Tags
        let tags = configuration::encode_update_tags()?;
        self.write_packet(writer, &tags).await?;

        // Send Finish Configuration
        let finish = configuration::encode_finish_configuration();
        self.write_packet(writer, &finish).await?;

        // Transition to Play state - next packet from client will be
        // Acknowledge Finish Configuration (0x03), which we handle below
        self.state = ConnectionState::Play;

        // Now enter Play: send login play sequence
        self.send_play_login_sequence(writer).await?;

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

        // 2. Synchronize Player Position
        let pos = play::encode_player_position_and_look(0.0, 64.0, 0.0, 0.0, 0.0, 0, 0);
        self.write_packet(writer, &pos).await?;

        // 3. Game Event (Start waiting for level chunks, event=13, value=0)
        let game_event = play::encode_game_event(13, 0.0);
        self.write_packet(writer, &game_event).await?;

        // 4. Chunk Batch Start
        let batch_start = play::encode_chunk_batch_start();
        self.write_packet(writer, &batch_start).await?;

        // 5. Send initial chunks around spawn
        let view_distance = 8;
        let player_chunk_x = 0;
        let player_chunk_z = 0;

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

        // 6. Chunk Batch Finished
        let batch_finished = play::encode_chunk_batch_finished(chunk_count)?;
        self.write_packet(writer, &batch_finished).await?;

        info!("Sent chunk data to player {}", name);

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
            }
            // Client Tick End
            0x0D => {
                // Empty packet, just acknowledge
            }
            // Keep Alive (serverbound)
            0x1C => {
                // Keep alive response - acknowledged
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
