use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info};

use super::broadcast::BroadcastEvent;
use super::connection::Connection;
use crate::config::Operators;
use crate::server_config::ServerConfig;
use crate::world::World;

pub struct Server {
    view_distance: i32,
    addr: String,
    config: Arc<RwLock<ServerConfig>>,
    world: Arc<RwLock<World>>,
    operators: Arc<RwLock<Operators>>,
    broadcast_tx: broadcast::Sender<BroadcastEvent>,
}

impl Server {
    pub fn new(addr: String, config: ServerConfig) -> Self {
        let view_distance = config.server.view_distance;
        let (broadcast_tx, _) = broadcast::channel(256);
        Self {
            view_distance,
            addr,
            config: Arc::new(RwLock::new(config)),
            world: Arc::new(RwLock::new(World::new())),
            operators: Arc::new(RwLock::new(Operators::load())),
            broadcast_tx,
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(&self.addr).await?;
        let bound_addr = listener.local_addr()?;
        info!("RustMC Server listening on {}", bound_addr);

        if let Ok(port_file) = std::env::var("RUSTMC_PORT_FILE") {
            std::fs::write(&port_file, bound_addr.port().to_string())?;
        }

        let tick_world = self.world.clone();
        tokio::spawn(async move {
            Self::world_tick_loop(tick_world).await;
        });

        let ops_watch = self.operators.clone();
        let ops_world = self.world.clone();
        tokio::spawn(async move {
            Self::ops_reload_loop(ops_watch, ops_world).await;
        });

        let config_watch = self.config.clone();
        tokio::spawn(async move {
            Self::config_reload_loop(config_watch).await;
        });

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let world = self.world.clone();
                    let operators = self.operators.clone();
                    let broadcast_tx = self.broadcast_tx.clone();
                    let broadcast_rx = self.broadcast_tx.subscribe();
                    let config = {
                        let cfg = self.config.read().await;
                        cfg.clone()
                    };                    tokio::spawn(async move {
                        let connection =
                            Connection::new(addr, world, operators, broadcast_tx, config);
                        connection.handle(stream, broadcast_rx).await;
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    async fn world_tick_loop(world: Arc<RwLock<World>>) {
        let mut interval = tokio::time::interval(Duration::from_millis(50)); // 20 TPS
        loop {
            interval.tick().await;
            let mut world = world.write().await;
            world.tick();
        }
    }

    async fn ops_reload_loop(operators: Arc<RwLock<Operators>>, world: Arc<RwLock<World>>) {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            let changed = {
                let ops = operators.read().await;
                ops.has_file_changed()
            };
            if changed {
                let mut ops = operators.write().await;
                ops.reload();
                let mut world = world.write().await;
                for (uuid, player) in world.players.iter_mut() {
                    player.op_level = ops.get_op_level(uuid);
                }
            }
        }
    }

    async fn config_reload_loop(config: Arc<RwLock<ServerConfig>>) {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            let changed = {
                let cfg = config.read().await;
                cfg.has_file_changed()
            };
            if changed {
                let mut cfg = config.write().await;
                cfg.reload();
            }
        }
    }
}
