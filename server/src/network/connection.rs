use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::protocol::handshake::{Handshake, NextState};
use crate::protocol::login::{LoginStart, LoginSuccess};
use crate::protocol::packet::{Packet, PacketWriter};
use crate::protocol::play;
use crate::protocol::status::{
    decode_ping_request, decode_status_request, encode_pong_response, StatusResponse,
};
use crate::protocol::types::VarInt;
use crate::world::World;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Handshake,
    Status,
    Login,
    Play,
}

pub struct Connection {
    addr: SocketAddr,
    state: ConnectionState,
    world: Arc<RwLock<World>>,
}

impl Connection {
    pub fn new(addr: SocketAddr, world: Arc<RwLock<World>>) -> Self {
        Self {
            addr,
            state: ConnectionState::Handshake,
            world,
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
        PacketWriter::new(&mut packet_data).write_packet(packet)?;
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
        if packet_id != 0x00 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Expected login start 0x00, got {packet_id:#04x}"),
            ));
        }

        let login = LoginStart::decode(data)?;
        info!("Player login: {} ({})", login.name, login.uuid);

        let success = LoginSuccess::new(login.uuid, login.name.clone());
        let packet = success.to_packet()?;
        self.write_packet(writer, &packet).await?;

        self.state = ConnectionState::Play;

        let entity_id = {
            let mut world = self.world.write().await;
            world.add_player(login.uuid, login.name.clone())
        };

        let login_play = play::encode_login_play(entity_id)?;
        self.write_packet(writer, &login_play).await?;

        let pos = play::encode_player_position_and_look(0.0, 64.0, 0.0, 0.0, 0.0, 0, 0);
        self.write_packet(writer, &pos).await?;

        Ok(true)
    }

    async fn handle_play(
        &mut self,
        packet_id: i32,
        data: &[u8],
        _writer: &mut BufWriter<tokio::net::tcp::OwnedWriteHalf>,
    ) -> std::io::Result<bool> {
        match packet_id {
            0x14 => {
                let pos = play::PlayerPosition::decode(data)?;
                debug!("Player position: ({}, {}, {})", pos.x, pos.y, pos.z);
            }
            0x05 => {
                let chat = play::ChatMessage::decode(data)?;
                info!("Chat from {}: {}", self.addr, chat.message);
            }
            0x15 => {
                // Keep alive response - acknowledged
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
