use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::world::World;
use super::connection::Connection;

pub struct Server {
    addr: String,
    world: Arc<RwLock<World>>,
}

impl Server {
    pub fn new(addr: String) -> Self {
        Self {
            addr,
            world: Arc::new(RwLock::new(World::new())),
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(&self.addr).await?;
        info!("RustMC Server listening on {}", self.addr);

        let tick_world = self.world.clone();
        tokio::spawn(async move {
            Self::world_tick_loop(tick_world).await;
        });

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let world = self.world.clone();
                    tokio::spawn(async move {
                        let connection = Connection::new(addr, world);
                        connection.handle(stream).await;
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    async fn world_tick_loop(world: Arc<RwLock<World>>) {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(50)); // 20 TPS
        loop {
            interval.tick().await;
            let mut world = world.write().await;
            world.tick();
        }
    }
}
