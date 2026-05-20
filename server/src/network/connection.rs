use std::collections::HashMap;
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};

use super::broadcast::{is_within_render_distance, BroadcastEvent};
use crate::config::Operators;
use crate::protocol::chunk_data;
use crate::protocol::configuration;
use crate::protocol::handshake::{Handshake, NextState};
use crate::protocol::login::{LoginCookieResponse, LoginStart, LoginSuccess};
use crate::protocol::packet::{Packet, PacketWriter};
use crate::protocol::packet_ids;
use crate::protocol::play;
use crate::protocol::status::{
    decode_ping_request, decode_status_request, encode_pong_response, StatusResponse,
};
use crate::protocol::types::VarInt;
use crate::network::transfer_token;
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
    operators: Arc<RwLock<Operators>>,
    player_uuid: Option<Uuid>,
    player_name: Option<String>,
    compression_enabled: bool,
    protocol_version: i32,

    configuration_finish_sent: bool,
    last_keep_alive_sent: Option<Instant>,
    last_keep_alive_id: i64,
    last_keep_alive_response: Option<Instant>,
    pending_chunks: Vec<ChunkPos>,
    cookies: HashMap<String, Vec<u8>>,
    pub transferred_from: Option<String>,

    broadcast_tx: broadcast::Sender<BroadcastEvent>,
}

impl Connection {
    pub fn new(
        addr: SocketAddr,
        world: Arc<RwLock<World>>,
        operators: Arc<RwLock<Operators>>,
        broadcast_tx: broadcast::Sender<BroadcastEvent>,
    ) -> Self {
        Self {
            addr,
            state: ConnectionState::Handshake,
            world,
            operators,
            player_uuid: None,
            player_name: None,
            compression_enabled: false,
            protocol_version: 0,

            configuration_finish_sent: false,
            last_keep_alive_sent: None,
            last_keep_alive_id: 0,
            last_keep_alive_response: None,
            pending_chunks: Vec::new(),
            cookies: HashMap::new(),
            transferred_from: None,

            broadcast_tx,
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

    async fn is_source_in_range(&self, source_chunk_x: i32, source_chunk_z: i32) -> bool {
        if let Some(ref my_uuid) = self.player_uuid {
            let world = self.world.read().await;
            if let Some(me) = world.players.get(my_uuid) {
                let my_chunk_x = me.x as i32 >> 4;
                let my_chunk_z = me.z as i32 >> 4;
                return is_within_render_distance(
                    source_chunk_x,
                    source_chunk_z,
                    my_chunk_x,
                    my_chunk_z,
                );
            }
        }
        false
    }

    pub async fn handle(
        mut self,
        stream: TcpStream,
        mut broadcast_rx: broadcast::Receiver<BroadcastEvent>,
    ) {
        info!("New connection from {}", self.addr);

        let (reader, writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut writer = BufWriter::new(writer);

        let keep_alive_interval = Duration::from_secs(15);
        let keep_alive_timeout = Duration::from_secs(30);
        let non_play_timeout = Duration::from_secs(
            std::env::var("RUSTMC_NON_PLAY_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
        );

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
                    event = broadcast_rx.recv() => {
                        match event {
                            Ok(BroadcastEvent::EntityAnimation { exclude_uuid, entity_id, animation, source_chunk_x, source_chunk_z }) => {
                                if self.player_uuid != Some(exclude_uuid) && self.is_source_in_range(source_chunk_x, source_chunk_z).await {
                                    let packet = play::encode_entity_animation(entity_id, animation);
                                    if let Err(e) = self.write_packet(&mut writer, &packet).await {
                                        error!("Failed to send entity animation to {}: {}", self.addr, e);
                                        break;
                                    }
                                }
                            }
                            Ok(BroadcastEvent::EntityMetadata { exclude_uuid, entity_id, metadata_bytes, source_chunk_x, source_chunk_z }) => {
                                if self.player_uuid != Some(exclude_uuid) && self.is_source_in_range(source_chunk_x, source_chunk_z).await {
                                    let packet = play::encode_set_entity_metadata(entity_id, &metadata_bytes);
                                    if let Err(e) = self.write_packet(&mut writer, &packet).await {
                                        error!("Failed to send entity metadata to {}: {}", self.addr, e);
                                        break;
                                    }
                                }
                            }
                            Err(_) => {}
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
                    _ = tokio::time::sleep(non_play_timeout) => {
                        warn!("Connection from {} timed out in {:?} state", self.addr, self.state);
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
        use packet_ids::handshake::serverbound::*;

        if packet_id != HANDSHAKE {
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

        self.protocol_version = handshake.protocol_version;
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
        use packet_ids::status::serverbound::*;

        match packet_id {
            STATUS_REQUEST => {
                decode_status_request(data)?;
                let world = self.world.read().await;
                let response = StatusResponse::default_response(world.player_count() as i32, 20);
                let packet = response.to_packet()?;
                self.write_packet(writer, &packet).await?;
                Ok(true)
            }
            PING_REQUEST => {
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
        use packet_ids::login::serverbound::*;

        match packet_id {
            LOGIN_START => {
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
            COOKIE_RESPONSE => {
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
        use packet_ids::configuration::serverbound::*;

        match packet_id {
            COOKIE_RESPONSE => {
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

            ACKNOWLEDGE_FINISH => {
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
            KNOWN_PACKS => {
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
        let data_pack_version =
            crate::protocol::version::data_pack_version_for(self.protocol_version);
        let known_packs = configuration::encode_known_packs(data_pack_version)?;
        self.write_packet(writer, &known_packs).await?;
        Ok(())
    }

    #[allow(dead_code)]
    async fn send_login_cookie_request(
        &mut self,
        writer: &mut BufWriter<tokio::net::tcp::OwnedWriteHalf>,
        key: &str,
    ) -> std::io::Result<()> {
        let packet = crate::protocol::login::encode_login_cookie_request(key)?;
        self.write_packet(writer, &packet).await?;
        Ok(())
    }

    #[allow(dead_code)]
    async fn send_cookie_request(
        &mut self,
        writer: &mut BufWriter<tokio::net::tcp::OwnedWriteHalf>,
        key: &str,
    ) -> std::io::Result<()> {
        let packet = crate::protocol::configuration::encode_cookie_request(key)?;
        self.write_packet(writer, &packet).await?;
        Ok(())
    }

    #[allow(dead_code)]
    async fn send_play_cookie_request(
        &mut self,
        writer: &mut BufWriter<tokio::net::tcp::OwnedWriteHalf>,
        key: &str,
    ) -> std::io::Result<()> {
        let packet = crate::protocol::play::encode_play_cookie_request(key)?;
        self.write_packet(writer, &packet).await?;
        Ok(())
    }

    async fn send_registry_data(
        &mut self,
        writer: &mut BufWriter<tokio::net::tcp::OwnedWriteHalf>,
    ) -> std::io::Result<()> {
        let packets = registry::cached_registry_packets(self.protocol_version)?;
        for packet in packets {
            self.write_packet(writer, packet).await?;
        }

        let tags = configuration::encode_update_tags()?;
        self.write_packet(writer, &tags).await?;

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
        let op_level = {
            let ops = self.operators.read().await;
            ops.get_op_level(&uuid)
        };

        let entity_id = {
            let mut world = self.world.write().await;
            world.add_player_with_op_level(uuid, name.clone(), op_level)
        };

        // 1. Login (Play) packet
        let login_play = play::encode_login_play(entity_id)?;
        self.write_packet(writer, &login_play).await?;

        // Request transfer token cookie if secret is configured
        if std::env::var("RUSTMC_TRANSFER_SECRET").is_ok() {
            let cookie_request =
                play::encode_play_cookie_request("rustmc:transfer_token")?;
            self.write_packet(writer, &cookie_request).await?;
        }

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
        self.last_keep_alive_sent = Some(Instant::now());
        self.last_keep_alive_response = Some(Instant::now());
        self.last_keep_alive_sent = Some(Instant::now());

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

    async fn process_chunk_updates(
        &mut self,
        writer: &mut BufWriter<tokio::net::tcp::OwnedWriteHalf>,
        uuid: &Uuid,
    ) -> std::io::Result<()> {
        let mut world = self.world.write().await;
        let view_distance = 8;
        if let Some(update) = world.compute_chunk_updates(uuid, view_distance) {
            if !update.to_load.is_empty() || !update.to_unload.is_empty() {
                if let Some(player) = world.players.get(uuid) {
                    let chunk_x = (player.x as i32) >> 4;
                    let chunk_z = (player.z as i32) >> 4;
                    let center_chunk = play::encode_set_center_chunk(chunk_x, chunk_z);
                    self.write_packet(writer, &center_chunk).await?;
                }
            }

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
            .get(uuid)
            .map(|p| p.chunks_per_tick)
            .unwrap_or(25.0);
        drop(world);

        self.drain_pending_chunks(writer, limit).await?;
        Ok(())
    }

    async fn handle_play(
        &mut self,
        packet_id: i32,
        data: &[u8],
        writer: &mut BufWriter<tokio::net::tcp::OwnedWriteHalf>,
    ) -> std::io::Result<bool> {
        use packet_ids::play::serverbound::*;

        match packet_id {
            CONFIRM_TELEPORTATION => {
                debug!("Received teleport confirmation");
            }
            CHAT_COMMAND => {
                let command = play::ChatCommand::decode(data)?;
                let has_permission = if let Some(uuid) = self.player_uuid {
                    let world = self.world.read().await;
                    world.players.get(&uuid).is_some_and(|p| p.op_level >= 3)
                } else {
                    false
                };

                if command.command.starts_with("transfer ") {
                    if !has_permission {
                        let msg = play::encode_system_chat_message(
                            "You don't have permission to use this command.",
                        )?;
                        self.write_packet(writer, &msg).await?;
                    } else {
                        let parts: Vec<&str> = command.command.splitn(3, ' ').collect();
                        if parts.len() == 3 {
                            if let Ok(port) = parts[2].parse::<i32>() {
                                if let Ok(secret) = std::env::var("RUSTMC_TRANSFER_SECRET") {
                                    if let (Some(uuid), Some(name)) =
                                        (self.player_uuid, self.player_name.as_ref())
                                    {
                                        let token = transfer_token::TransferToken {
                                            origin: self.addr.to_string(),
                                            player_uuid: uuid,
                                            player_name: name.clone(),
                                            timestamp: transfer_token::current_timestamp(),
                                        };
                                        let payload =
                                            transfer_token::generate_token(secret.as_bytes(), &token);
                                        let cookie_packet = play::encode_play_store_cookie(
                                            "rustmc:transfer_token",
                                            &payload,
                                        )?;
                                        self.write_packet(writer, &cookie_packet).await?;
                                    }
                                }
                                let packet = play::encode_transfer(parts[1], port)?;
                                self.write_packet(writer, &packet).await?;
                                return Ok(false);
                            }
                        }
                    }
                } else if command.command.starts_with("op ") {
                    if !has_permission {
                        let msg = play::encode_system_chat_message(
                            "You don't have permission to use this command.",
                        )?;
                        self.write_packet(writer, &msg).await?;
                    } else {
                        let parts: Vec<&str> = command.command.splitn(3, ' ').collect();
                        let target_name = parts[1];
                        let level: u8 = parts
                            .get(2)
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(3);

                        let target_uuid = {
                            let world = self.world.read().await;
                            world
                                .players
                                .iter()
                                .find(|(_, p)| p.name == target_name)
                                .map(|(uuid, _)| *uuid)
                        };

                        if let Some(target_uuid) = target_uuid {
                            {
                                let mut ops = self.operators.write().await;
                                ops.set_op_level(
                                    target_uuid,
                                    target_name.to_string(),
                                    level,
                                );
                                ops.save();
                            }
                            {
                                let mut world = self.world.write().await;
                                if let Some(player) = world.players.get_mut(&target_uuid) {
                                    player.op_level = level;
                                }
                            }
                            let msg = play::encode_system_chat_message(&format!(
                                "Set {} as operator level {}",
                                target_name, level
                            ))?;
                            self.write_packet(writer, &msg).await?;
                        } else {
                            let msg = play::encode_system_chat_message(&format!(
                                "Player '{}' is not online",
                                target_name
                            ))?;
                            self.write_packet(writer, &msg).await?;
                        }
                    }
                } else if command.command.starts_with("deop ") {
                    if !has_permission {
                        let msg = play::encode_system_chat_message(
                            "You don't have permission to use this command.",
                        )?;
                        self.write_packet(writer, &msg).await?;
                    } else {
                        let target_name = &command.command[5..];

                        let target_uuid = {
                            let world = self.world.read().await;
                            world
                                .players
                                .iter()
                                .find(|(_, p)| p.name == target_name)
                                .map(|(uuid, _)| *uuid)
                        };

                        if let Some(target_uuid) = target_uuid {
                            {
                                let mut ops = self.operators.write().await;
                                ops.remove_op(&target_uuid);
                                ops.save();
                            }
                            {
                                let mut world = self.world.write().await;
                                if let Some(player) = world.players.get_mut(&target_uuid) {
                                    player.op_level = 0;
                                }
                            }
                            let msg = play::encode_system_chat_message(&format!(
                                "Removed {} as operator",
                                target_name
                            ))?;
                            self.write_packet(writer, &msg).await?;
                        } else {
                            let msg = play::encode_system_chat_message(&format!(
                                "Player '{}' is not online",
                                target_name
                            ))?;
                            self.write_packet(writer, &msg).await?;
                        }
                    }
                }
                debug!("Received chat command: {}", command.command);
            }
            CHAT_MESSAGE => {
                let chat = play::ChatMessage::decode(data)?;
                info!("Chat from {}: {}", self.addr, chat.message);
            }
            CHUNK_BATCH_RECEIVED => {
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
            COOKIE_RESPONSE => {
                let response = play::PlayCookieResponse::decode(data)?;
                debug!("Received play cookie response: key={}", response.key);
                if response.key == "rustmc:transfer_token" {
                    if let Some(ref payload) = response.payload {
                        if let Ok(secret) = std::env::var("RUSTMC_TRANSFER_SECRET") {
                            if let Some(token) =
                                transfer_token::validate_token(secret.as_bytes(), payload)
                            {
                                info!(
                                    "Valid transfer token from origin={} for player={}",
                                    token.origin, token.player_name
                                );
                                self.transferred_from = Some(token.origin);
                            } else {
                                debug!("Invalid or expired transfer token received");
                            }
                        }
                    }
                }
                if let Some(payload) = response.payload {
                    self.cookies.insert(response.key, payload);
                } else {
                    self.cookies.remove(&response.key);
                }
            }
            CLIENT_TICK_END => {
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
            KEEP_ALIVE => {
                debug!("Received keep-alive response from {}", self.addr);
                self.last_keep_alive_response = Some(Instant::now());
            }
            SET_PLAYER_POSITION => {
                let pos = play::PlayerPosition::decode(data)?;
                if !pos.is_valid() {
                    warn!(
                        "Invalid player position from client: ({}, {}, {})",
                        pos.x, pos.y, pos.z
                    );
                    return Ok(true);
                }
                debug!("Player position: ({}, {}, {})", pos.x, pos.y, pos.z);

                if let Some(uuid) = self.player_uuid {
                    {
                        let mut world = self.world.write().await;
                        world.update_player_position(&uuid, pos.x, pos.y, pos.z);
                    }
                    self.process_chunk_updates(writer, &uuid).await?;
                }
            }
            SET_PLAYER_POSITION_AND_ROTATION => {
                let pos_rot = play::PlayerPositionAndRotation::decode(data)?;
                if !pos_rot.is_valid() {
                    warn!(
                        "Invalid player position+rotation from client: ({}, {}, {}) pitch={}",
                        pos_rot.x, pos_rot.y, pos_rot.z, pos_rot.pitch
                    );
                    return Ok(true);
                }
                debug!(
                    "Player pos+rot: ({}, {}, {}) yaw={} pitch={}",
                    pos_rot.x, pos_rot.y, pos_rot.z, pos_rot.yaw, pos_rot.pitch
                );
                if let Some(uuid) = self.player_uuid {
                    {
                        let mut world = self.world.write().await;
                        world.update_player_position(&uuid, pos_rot.x, pos_rot.y, pos_rot.z);
                        world.update_player_rotation(&uuid, pos_rot.yaw, pos_rot.pitch);
                        if let Some(player) = world.players.get_mut(&uuid) {
                            player.on_ground = pos_rot.on_ground;
                        }
                    }
                    self.process_chunk_updates(writer, &uuid).await?;
                }
            }
            SET_PLAYER_ROTATION => {
                let rot = play::PlayerRotation::decode(data)?;
                if !rot.is_valid() {
                    warn!(
                        "Invalid player rotation from client: yaw={} pitch={}",
                        rot.yaw, rot.pitch
                    );
                    return Ok(true);
                }
                debug!("Player rotation: yaw={} pitch={}", rot.yaw, rot.pitch);
                if let Some(uuid) = self.player_uuid {
                    let mut world = self.world.write().await;
                    world.update_player_rotation(&uuid, rot.yaw, rot.pitch);
                    if let Some(player) = world.players.get_mut(&uuid) {
                        player.on_ground = rot.on_ground;
                    }
                }
            }
            SET_PLAYER_STATUS_ONLY => {
                let status = play::PlayerStatusOnly::decode(data)?;
                if let Some(uuid) = self.player_uuid {
                    let mut world = self.world.write().await;
                    if let Some(player) = world.players.get_mut(&uuid) {
                        player.on_ground = status.on_ground;
                    }
                }
            }
            PLAYER_COMMAND => {
                let cmd = play::PlayerCommand::decode(data)?;
                if !cmd.is_valid() {
                    warn!(
                        "Invalid player command from client: action={} jump_boost={}",
                        cmd.action_id, cmd.jump_boost
                    );
                    return Ok(true);
                }
                debug!(
                    "Player command: action={} jump_boost={}",
                    cmd.action_id, cmd.jump_boost
                );
                if let Some(uuid) = self.player_uuid {
                    let mut world = self.world.write().await;
                    if let Some(player) = world.players.get_mut(&uuid) {
                        match cmd.action_id {
                            0 => player.is_sneaking = true,
                            1 => player.is_sneaking = false,
                            3 => player.is_sprinting = true,
                            4 => player.is_sprinting = false,
                            _ => {}
                        }
                        let flags: u8 = (if player.is_sneaking { 0x02 } else { 0 })
                            | (if player.is_sprinting { 0x08 } else { 0 });
                        let metadata_bytes = play::encode_entity_base_flags_metadata(flags);
                        let source_chunk_x = player.x as i32 >> 4;
                        let source_chunk_z = player.z as i32 >> 4;
                        let _ = self.broadcast_tx.send(BroadcastEvent::EntityMetadata {
                            exclude_uuid: uuid,
                            entity_id: player.entity_id,
                            metadata_bytes,
                            source_chunk_x,
                            source_chunk_z,
                        });
                    }
                }
            }
            SET_CARRIED_ITEM => {
                let item = play::SetCarriedItem::decode(data)?;
                if !item.is_valid_slot() {
                    warn!("Invalid carried item slot {} from client", item.slot);
                    return Ok(true);
                }
                debug!("Set carried item: slot={}", item.slot);
                if let Some(uuid) = self.player_uuid {
                    let mut world = self.world.write().await;
                    if let Some(player) = world.players.get_mut(&uuid) {
                        player.selected_slot = item.slot as u8;
                    }
                }
            }
            SWING => {
                let swing = play::Swing::decode(data)?;
                if !swing.is_valid() {
                    warn!("Invalid swing hand from client: {}", swing.hand);
                    return Ok(true);
                }
                let animation = if swing.hand == 0 { 0u8 } else { 3u8 };
                debug!(
                    "Player swing: hand={}",
                    if swing.hand == 0 { "main" } else { "off" }
                );
                if let Some(uuid) = self.player_uuid {
                    let player_info = {
                        let world = self.world.read().await;
                        world
                            .players
                            .get(&uuid)
                            .map(|p| (p.entity_id, p.x as i32 >> 4, p.z as i32 >> 4))
                    };
                    if let Some((eid, source_chunk_x, source_chunk_z)) = player_info {
                        let _ = self.broadcast_tx.send(BroadcastEvent::EntityAnimation {
                            exclude_uuid: uuid,
                            entity_id: eid,
                            animation,
                            source_chunk_x,
                            source_chunk_z,
                        });
                    }
                }
            }
            PLAYER_LOADED => {
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
        let operators = Arc::new(RwLock::new(Operators::empty()));
        let addr: SocketAddr = "127.0.0.1:25565".parse().unwrap();
        let (broadcast_tx, _broadcast_rx) = broadcast::channel(16);
        Connection::new(addr, world, operators, broadcast_tx)
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
