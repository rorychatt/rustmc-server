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
    addr: String,
    config: Arc<RwLock<ServerConfig>>,
    world: Arc<RwLock<World>>,
    operators: Arc<RwLock<Operators>>,
    broadcast_tx: broadcast::Sender<BroadcastEvent>,
}

impl Server {
    pub fn new(addr: String, config: ServerConfig) -> Self {
        let (broadcast_tx, _) = broadcast::channel(256);
        
        let world_dir = if let Ok(env_dir) = std::env::var("RUSTMC_WORLD_DIR") {
            if env_dir == "memory" {
                None
            } else {
                Some(std::path::PathBuf::from(env_dir))
            }
        } else {
            Some(std::path::PathBuf::from(&config.gameplay.world_dir))
        };

        let world = Arc::new(RwLock::new(World::new_with_dir(
            config.gameplay.world_type.clone(),
            config.gameplay.seed,
            config.gameplay.sea_level,
            world_dir,
        )));
        Self {
            addr,
            config: Arc::new(RwLock::new(config)),
            world,
            operators: Arc::new(RwLock::new(Operators::load())),
            broadcast_tx,
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(&self.addr).await?;
        let bound_addr = listener.local_addr()?;
        info!("RustMC Server listening on {}", bound_addr);

        if let Some(port_file) = self.config.read().await.port_file() {
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

        let autosave_world = self.world.clone();
        let save_interval = {
            let cfg = self.config.read().await;
            cfg.gameplay.save_interval_secs
        };
        tokio::spawn(async move {
            Self::autosave_loop(autosave_world, save_interval).await;
        });

        let backup_world = self.world.clone();
        let (backup_interval, max_backups) = {
            let cfg = self.config.read().await;
            (cfg.gameplay.backup_interval_secs, cfg.gameplay.max_backups)
        };
        tokio::spawn(async move {
            Self::backup_loop(backup_world, backup_interval, max_backups).await;
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
                    };
                    tokio::spawn(async move {
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

    async fn autosave_loop(world: Arc<RwLock<World>>, save_interval_secs: u64) {
        if save_interval_secs == 0 {
            return;
        }
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(save_interval_secs));
        interval.tick().await; // Initial tick completes immediately
        loop {
            interval.tick().await;
            info!("Autosaving world chunks...");
            let world = world.read().await;
            match world.save_all() {
                Ok(_) => info!("Autosave complete."),
                Err(e) => error!("Autosave failed: {}", e),
            }
        }
    }

    async fn backup_loop(world: Arc<RwLock<World>>, backup_interval_secs: u64, max_backups: usize) {
        if backup_interval_secs == 0 || max_backups == 0 {
            return;
        }
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(backup_interval_secs));
        interval.tick().await; // Initial tick completes immediately
        loop {
            interval.tick().await;
            
            let world_dir = {
                let world = world.read().await;
                world.world_dir.clone()
            };
            
            if let Some(dir) = world_dir {
                info!("Creating automated backup of world directory...");
                {
                    let world = world.read().await;
                    if let Err(e) = world.save_all() {
                        error!("Failed to save world before backup: {}", e);
                    }
                }
                
                let dir_clone = dir.clone();
                let result = tokio::task::spawn_blocking(move || {
                    crate::world::persistence::create_backup(&dir_clone, std::path::Path::new("."), max_backups)
                }).await;
                
                match result {
                    Ok(Ok(_)) => info!("Automated backup complete and pruned."),
                    Ok(Err(e)) => error!("Automated backup failed: {}", e),
                    Err(e) => error!("Backup task joined with error: {}", e),
                }
            }
        }
    }
}
