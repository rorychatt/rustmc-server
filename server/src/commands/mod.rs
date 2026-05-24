use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use crate::world::World;
use crate::network::broadcast::BroadcastEvent;

pub struct CommandContext {
    pub uuid: Option<Uuid>, // None if executed from console
    pub name: String,
    pub op_level: u8,
    pub world: Arc<RwLock<World>>,
    pub broadcast_tx: tokio::sync::broadcast::Sender<BroadcastEvent>,
    pub stop_signal_tx: Option<tokio::sync::mpsc::Sender<()>>,
}

impl CommandContext {
    pub fn send_message(&self, message: &str) {
        if let Some(uuid) = self.uuid {
            let _ = self.broadcast_tx.send(BroadcastEvent::SystemMessage {
                target_uuid: Some(uuid),
                message: message.to_string(),
            });
        } else {
            tracing::info!("[Console] {}", message);
        }
    }

    pub fn broadcast_message(&self, message: &str) {
        let _ = self.broadcast_tx.send(BroadcastEvent::SystemMessage {
            target_uuid: None,
            message: message.to_string(),
        });
        tracing::info!("[Broadcast] {}", message);
    }
}

pub async fn handle_command(ctx: &CommandContext, command_str: &str) {
    let raw = command_str.trim().strip_prefix('/').unwrap_or(command_str);
    let parts: Vec<&str> = raw.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }

    let cmd_name = parts[0].to_lowercase();
    let args = &parts[1..];

    match cmd_name.as_str() {
        "help" => handle_help(ctx, args).await,
        "tp" => handle_tp(ctx, args).await,
        "gamemode" => handle_gamemode(ctx, args).await,
        "kill" => handle_kill(ctx, args).await,
        "kick" => handle_kick(ctx, args).await,
        "op" => handle_op(ctx, args).await,
        "deop" => handle_deop(ctx, args).await,
        "save-all" => handle_save_all(ctx, args).await,
        "stop" => handle_stop(ctx, args).await,
        "worldborder" => handle_worldborder(ctx, args).await,
        _ => ctx.send_message(&format!("Unknown command: /{}. Type /help for help.", cmd_name)),
    }
}

async fn handle_help(ctx: &CommandContext, _args: &[&str]) {
    ctx.send_message("=== Available Commands ===");
    ctx.send_message("/help - Show this help menu");
    ctx.send_message("/tp <player> [x y z] or /tp <x> <y> <z> - Teleport player");
    ctx.send_message("/gamemode <survival|creative|adventure|spectator> [player] - Set gamemode");
    ctx.send_message("/kill [player] - Kill player");
    ctx.send_message("/kick <player> [reason] - Kick player");
    ctx.send_message("/op <player> - Promote to operator");
    ctx.send_message("/deop <player> - Demote operator");
    ctx.send_message("/save-all - Save chunks and player data");
    ctx.send_message("/stop - Stop the server");
    ctx.send_message("/worldborder <set|center|warning> - Manage world borders");
}

async fn handle_tp(ctx: &CommandContext, args: &[&str]) {
    if ctx.op_level < 2 {
        ctx.send_message("You do not have permission to use this command.");
        return;
    }

    if args.is_empty() {
        ctx.send_message("Usage: /tp <player> [x y z] or /tp <x> <y> <z>");
        return;
    }

    let mut world = ctx.world.write().await;

    // Check if first arg is a player name or coordinate
    if let Ok(x) = args[0].parse::<f64>() {
        // Teleporting self to coords: /tp <x> <y> <z>
        if args.len() < 3 {
            ctx.send_message("Usage: /tp <x> <y> <z>");
            return;
        }
        let y = match args[1].parse::<f64>() {
            Ok(v) => v,
            Err(_) => {
                ctx.send_message("Invalid Y coordinate");
                return;
            }
        };
        let z = match args[2].parse::<f64>() {
            Ok(v) => v,
            Err(_) => {
                ctx.send_message("Invalid Z coordinate");
                return;
            }
        };

        if let Some(ref executor_uuid) = ctx.uuid {
            if let Some(player) = world.players.get_mut(executor_uuid) {
                player.x = x;
                player.y = y;
                player.z = z;
                let _ = ctx.broadcast_tx.send(BroadcastEvent::TeleportPlayer {
                    target_uuid: *executor_uuid,
                    x,
                    y,
                    z,
                });
                ctx.send_message(&format!("Teleported to {:.2}, {:.2}, {:.2}", x, y, z));
            }
        } else {
            ctx.send_message("Console cannot teleport without specifying a player.");
        }
        return;
    }

    // First arg is player name: /tp <player> ...
    let target_name = args[0];
    let target_uuid = world
        .players
        .iter()
        .find(|(_, p)| p.name.to_lowercase() == target_name.to_lowercase())
        .map(|(&uuid, _)| uuid);

    let target_uuid = match target_uuid {
        Some(uuid) => uuid,
        None => {
            ctx.send_message(&format!("Player '{}' not found", target_name));
            return;
        }
    };

    if args.len() == 1 {
        // Teleporting self to target player: /tp <target_player>
        if let Some(ref executor_uuid) = ctx.uuid {
            let (tx, ty, tz) = {
                let target = world.players.get(&target_uuid).unwrap();
                (target.x, target.y, target.z)
            };
            if let Some(player) = world.players.get_mut(executor_uuid) {
                player.x = tx;
                player.y = ty;
                player.z = tz;
                let _ = ctx.broadcast_tx.send(BroadcastEvent::TeleportPlayer {
                    target_uuid: *executor_uuid,
                    x: tx,
                    y: ty,
                    z: tz,
                });
                ctx.send_message(&format!("Teleported to {}", target_name));
            }
        } else {
            ctx.send_message("Console cannot teleport to players.");
        }
    } else if args.len() >= 4 {
        // Teleporting target player to coords: /tp <player> <x> <y> <z>
        let x = match args[1].parse::<f64>() {
            Ok(v) => v,
            Err(_) => {
                ctx.send_message("Invalid X coordinate");
                return;
            }
        };
        let y = match args[2].parse::<f64>() {
            Ok(v) => v,
            Err(_) => {
                ctx.send_message("Invalid Y coordinate");
                return;
            }
        };
        let z = match args[3].parse::<f64>() {
            Ok(v) => v,
            Err(_) => {
                ctx.send_message("Invalid Z coordinate");
                return;
            }
        };

        if let Some(player) = world.players.get_mut(&target_uuid) {
            player.x = x;
            player.y = y;
            player.z = z;
            let _ = ctx.broadcast_tx.send(BroadcastEvent::TeleportPlayer {
                target_uuid,
                x,
                y,
                z,
            });
            ctx.send_message(&format!(
                "Teleported {} to {:.2}, {:.2}, {:.2}",
                target_name, x, y, z
            ));
        }
    } else {
        ctx.send_message("Usage: /tp <player> [x y z]");
    }
}

async fn handle_gamemode(ctx: &CommandContext, args: &[&str]) {
    if ctx.op_level < 2 {
        ctx.send_message("You do not have permission to use this command.");
        return;
    }

    if args.is_empty() {
        ctx.send_message("Usage: /gamemode <survival|creative|adventure|spectator> [player]");
        return;
    }

    let gm_str = args[0].to_lowercase();
    let gamemode_id = match gm_str.as_str() {
        "survival" | "s" | "0" => 0,
        "creative" | "c" | "1" => 1,
        "adventure" | "a" | "2" => 2,
        "spectator" | "sp" | "3" => 3,
        _ => {
            ctx.send_message("Invalid gamemode. Use survival, creative, adventure, or spectator.");
            return;
        }
    };

    let mut world = ctx.world.write().await;
    let target_uuid = if args.len() >= 2 {
        let target_name = args[1];
        match world.players.iter().find(|(_, p)| p.name.to_lowercase() == target_name.to_lowercase()) {
            Some((&uuid, _)) => Some(uuid),
            None => {
                ctx.send_message(&format!("Player '{}' not found", target_name));
                return;
            }
        }
    } else {
        ctx.uuid
    };

    if let Some(uuid) = target_uuid {
        if let Some(player) = world.players.get_mut(&uuid) {
            player.gamemode = Some(gamemode_id);
            ctx.send_message(&format!("Set {}'s gamemode to {}", player.name, gm_str));
            // In a complete implementation, this would send an Game Event packet to change Client Gamemode
        }
    } else {
        ctx.send_message("No player specified and console cannot have a gamemode.");
    }
}

async fn handle_kill(ctx: &CommandContext, args: &[&str]) {
    if ctx.op_level < 2 {
        ctx.send_message("You do not have permission to use this command.");
        return;
    }

    let mut world = ctx.world.write().await;
    let target_uuid = if args.is_empty() {
        ctx.uuid
    } else {
        let target_name = args[0];
        match world.players.iter().find(|(_, p)| p.name.to_lowercase() == target_name.to_lowercase()) {
            Some((&uuid, _)) => Some(uuid),
            None => {
                ctx.send_message(&format!("Player '{}' not found", target_name));
                return;
            }
        }
    };

    if let Some(uuid) = target_uuid {
        if let Some(player) = world.players.get_mut(&uuid) {
            // Respawn or teleport back to spawn on kill
            player.x = 0.0;
            player.y = 64.0;
            player.z = 0.0;
            ctx.broadcast_message(&format!("{} was killed", player.name));
        }
    } else {
        ctx.send_message("No player specified and console cannot be killed.");
    }
}

async fn handle_kick(ctx: &CommandContext, args: &[&str]) {
    if ctx.op_level < 3 {
        ctx.send_message("You do not have permission to use this command.");
        return;
    }

    if args.is_empty() {
        ctx.send_message("Usage: /kick <player> [reason]");
        return;
    }

    let target_name = args[0];
    let reason = if args.len() >= 2 { args[1..].join(" ") } else { "Kicked by an operator.".to_string() };

    let world = ctx.world.read().await;
    let target_uuid = world.players.iter()
        .find(|(_, p)| p.name.to_lowercase() == target_name.to_lowercase())
        .map(|(&uuid, _)| uuid);

    if let Some(uuid) = target_uuid {
        // Kick via token/cookie or direct network disconnect event.
        // For simplicity, we broadcast the kick signal. The Connection loop checks for it.
        let _ = ctx.broadcast_tx.send(BroadcastEvent::SystemMessage {
            target_uuid: Some(uuid),
            message: format!("KICKED: {}", reason),
        });
        ctx.send_message(&format!("Kicked player {} for: {}", target_name, reason));
    } else {
        ctx.send_message(&format!("Player '{}' not found", target_name));
    }
}

async fn handle_op(ctx: &CommandContext, args: &[&str]) {
    if ctx.op_level < 3 {
        ctx.send_message("You do not have permission to use this command.");
        return;
    }

    if args.is_empty() {
        ctx.send_message("Usage: /op <player>");
        return;
    }

    let target_name = args[0];
    // Find player uuid
    let mut world = ctx.world.write().await;
    let target_uuid = world.players.iter()
        .find(|(_, p)| p.name.to_lowercase() == target_name.to_lowercase())
        .map(|(&uuid, _)| uuid);

    if let Some(uuid) = target_uuid {
        if let Some(player) = world.players.get_mut(&uuid) {
            player.op_level = 3;
            ctx.send_message(&format!("Made {} a server operator", target_name));
        }
    } else {
        ctx.send_message(&format!("Player '{}' not found online", target_name));
    }
}

async fn handle_deop(ctx: &CommandContext, args: &[&str]) {
    if ctx.op_level < 3 {
        ctx.send_message("You do not have permission to use this command.");
        return;
    }

    if args.is_empty() {
        ctx.send_message("Usage: /deop <player>");
        return;
    }

    let target_name = args[0];
    let mut world = ctx.world.write().await;
    let target_uuid = world.players.iter()
        .find(|(_, p)| p.name.to_lowercase() == target_name.to_lowercase())
        .map(|(&uuid, _)| uuid);

    if let Some(uuid) = target_uuid {
        if let Some(player) = world.players.get_mut(&uuid) {
            player.op_level = 0;
            ctx.send_message(&format!("Demoted {} from server operator", target_name));
        }
    } else {
        ctx.send_message(&format!("Player '{}' not found online", target_name));
    }
}

async fn handle_save_all(ctx: &CommandContext, _args: &[&str]) {
    if ctx.op_level < 4 {
        ctx.send_message("You do not have permission to use this command.");
        return;
    }

    ctx.send_message("Saving the game...");
    let world = ctx.world.read().await;
    match world.save_all() {
        Ok(_) => ctx.send_message("Save complete."),
        Err(e) => ctx.send_message(&format!("Save failed: {}", e)),
    }
}

async fn handle_stop(ctx: &CommandContext, _args: &[&str]) {
    if ctx.op_level < 4 {
        ctx.send_message("You do not have permission to use this command.");
        return;
    }

    ctx.broadcast_message("Stopping the server...");
    // Trigger save
    let world = ctx.world.read().await;
    let _ = world.save_all();

    if let Some(ref tx) = ctx.stop_signal_tx {
        let _ = tx.send(()).await;
    } else {
        std::process::exit(0);
    }
}

async fn handle_worldborder(ctx: &CommandContext, args: &[&str]) {
    if ctx.op_level < 2 {
        ctx.send_message("You do not have permission to use this command.");
        return;
    }

    if args.is_empty() {
        ctx.send_message("Usage: /worldborder <set|center|warning>");
        return;
    }

    let action = args[0].to_lowercase();
    match action.as_str() {
        "set" => {
            if args.len() < 2 {
                ctx.send_message("Usage: /worldborder set <size> [time]");
                return;
            }
            if let Ok(size) = args[1].parse::<f64>() {
                let mut world = ctx.world.write().await;
                let old_size = world.border_size;
                world.border_size = size;
                world.border_target_size = size;
                let lerp_time = if args.len() >= 3 {
                    args[2].parse::<i64>().unwrap_or(0) * 1000
                } else {
                    0
                };
                world.border_speed = lerp_time;

                let _ = ctx.broadcast_tx.send(BroadcastEvent::WorldBorderLerpSize {
                    old_size,
                    new_size: size,
                    lerp_time,
                });

                if lerp_time > 0 {
                    ctx.send_message(&format!(
                        "Set world border to {} blocks over {} seconds",
                        size,
                        lerp_time / 1000
                    ));
                } else {
                    ctx.send_message(&format!("Set world border to {} blocks", size));
                }
            } else {
                ctx.send_message("Invalid border size.");
            }
        }
        "center" => {
            if args.len() < 3 {
                ctx.send_message("Usage: /worldborder center <x> <z>");
                return;
            }
            if let (Ok(x), Ok(z)) = (args[1].parse::<f64>(), args[2].parse::<f64>()) {
                let mut world = ctx.world.write().await;
                world.border_x = x;
                world.border_z = z;
                let _ = ctx.broadcast_tx.send(BroadcastEvent::WorldBorderCenter { x, z });
                ctx.send_message(&format!("Set world border center to {}, {}", x, z));
            } else {
                ctx.send_message("Invalid center coordinates.");
            }
        }
        "warning" => {
            if args.len() < 3 {
                ctx.send_message("Usage: /worldborder warning <time|distance> <value>");
                return;
            }
            let sub = args[1].to_lowercase();
            if sub == "time" {
                if let Ok(time) = args[2].parse::<i32>() {
                    let mut world = ctx.world.write().await;
                    world.border_warning_time = time;
                    let _ = ctx.broadcast_tx.send(BroadcastEvent::WorldBorderWarningTime {
                        warning_time: time,
                    });
                    ctx.send_message(&format!("Set world border warning time to {} seconds", time));
                } else {
                    ctx.send_message("Invalid warning time.");
                }
            } else if sub == "distance" {
                if let Ok(dist) = args[2].parse::<i32>() {
                    let mut world = ctx.world.write().await;
                    world.border_warning_blocks = dist;
                    let _ = ctx.broadcast_tx.send(BroadcastEvent::WorldBorderWarningBlocks {
                        warning_blocks: dist,
                    });
                    ctx.send_message(&format!(
                        "Set world border warning distance to {} blocks",
                        dist
                    ));
                } else {
                    ctx.send_message("Invalid warning distance.");
                }
            } else {
                ctx.send_message("Usage: /worldborder warning <time|distance> <value>");
            }
        }
        _ => {
            ctx.send_message("Usage: /worldborder <set|center|warning>");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::World;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_help_command() {
        let world = Arc::new(RwLock::new(World::new()));
        let (broadcast_tx, mut broadcast_rx) = broadcast::channel(16);
        let ctx = CommandContext {
            uuid: Some(Uuid::new_v4()),
            name: "TestPlayer".to_string(),
            op_level: 0,
            world,
            broadcast_tx,
            stop_signal_tx: None,
        };

        handle_command(&ctx, "/help").await;

        let event = broadcast_rx.recv().await.unwrap();
        if let BroadcastEvent::SystemMessage { target_uuid, message } = event {
            assert_eq!(target_uuid, ctx.uuid);
            assert!(message.contains("Available Commands"));
        } else {
            panic!("Expected SystemMessage");
        }
    }

    #[tokio::test]
    async fn test_tp_command_permission_denied() {
        let world = Arc::new(RwLock::new(World::new()));
        let (broadcast_tx, mut broadcast_rx) = broadcast::channel(16);
        let ctx = CommandContext {
            uuid: Some(Uuid::new_v4()),
            name: "TestPlayer".to_string(),
            op_level: 0,
            world,
            broadcast_tx,
            stop_signal_tx: None,
        };

        handle_command(&ctx, "/tp TestPlayer 10 20 30").await;

        let event = broadcast_rx.recv().await.unwrap();
        if let BroadcastEvent::SystemMessage { target_uuid, message } = event {
            assert_eq!(target_uuid, ctx.uuid);
            assert!(message.contains("You do not have permission"));
        } else {
            panic!("Expected SystemMessage");
        }
    }

    #[tokio::test]
    async fn test_op_command_sets_op_level() {
        let world = Arc::new(RwLock::new(World::new()));
        let player_uuid = Uuid::new_v4();
        {
            let mut w = world.write().await;
            w.add_player_with_op_level(player_uuid, "OtherPlayer".to_string(), 0, 8);
        }

        let (broadcast_tx, mut broadcast_rx) = broadcast::channel(16);
        let ctx = CommandContext {
            uuid: Some(Uuid::new_v4()),
            name: "AdminPlayer".to_string(),
            op_level: 3,
            world: world.clone(),
            broadcast_tx,
            stop_signal_tx: None,
        };

        handle_command(&ctx, "/op OtherPlayer").await;

        let w = world.read().await;
        let player = w.players.get(&player_uuid).unwrap();
        assert_eq!(player.op_level, 3);

        let event = broadcast_rx.recv().await.unwrap();
        if let BroadcastEvent::SystemMessage { target_uuid, message } = event {
            assert!(message.contains("Made OtherPlayer a server operator"));
        } else {
            panic!("Expected SystemMessage");
        }
    }
}
