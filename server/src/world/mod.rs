pub mod block_registry;
pub mod chunk;

use chunk::{Chunk, ChunkPos};
use std::collections::{HashMap, HashSet};
use tracing::debug;
use uuid::Uuid;

pub struct Player {
    pub uuid: Uuid,
    pub name: String,
    pub entity_id: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub is_sneaking: bool,
    pub is_sprinting: bool,
    pub selected_slot: u8,
    pub loaded_chunks: HashSet<ChunkPos>,
    pub chunks_per_tick: f32,
    pub op_level: u8,
    pub view_distance: i32,
}

pub struct World {
    pub players: HashMap<Uuid, Player>,
    chunks: HashMap<ChunkPos, Chunk>,
    next_entity_id: i32,
    tick_count: u64,
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            players: HashMap::new(),
            chunks: HashMap::new(),
            next_entity_id: 1,
            tick_count: 0,
        };
        world.generate_spawn_chunks();
        world
    }

    pub fn add_player(&mut self, uuid: Uuid, name: String, view_distance: i32) -> i32 {
        self.add_player_with_op_level(uuid, name, 0, view_distance)
    }

    pub fn add_player_with_op_level(
        &mut self,
        uuid: Uuid,
        name: String,
        op_level: u8,
        view_distance: i32,
    ) -> i32 {
        let entity_id = self.next_entity_id;
        self.next_entity_id += 1;
        self.players.insert(
            uuid,
            Player {
                uuid,
                name,
                entity_id,
                x: 0.0,
                y: 64.0,
                z: 0.0,
                yaw: 0.0,
                pitch: 0.0,
                on_ground: false,
                is_sneaking: false,
                is_sprinting: false,
                selected_slot: 0,
                loaded_chunks: HashSet::new(),
                chunks_per_tick: 25.0,
                op_level,
                view_distance,
            },
        );
        entity_id
    }

    pub fn remove_player(&mut self, uuid: &Uuid) {
        self.players.remove(uuid);
    }

    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    pub fn update_player_position(&mut self, uuid: &Uuid, x: f64, y: f64, z: f64) {
        if let Some(player) = self.players.get_mut(uuid) {
            player.x = x;
            player.y = y;
            player.z = z;
        }
    }

    pub fn update_player_rotation(&mut self, uuid: &Uuid, yaw: f32, pitch: f32) {
        if let Some(player) = self.players.get_mut(uuid) {
            player.yaw = yaw;
            player.pitch = pitch;
        }
    }

    pub fn tick(&mut self) {
        self.tick_count += 1;
        if self.tick_count.is_multiple_of(600) {
            debug!(
                "World tick {}, {} players, {} chunks loaded",
                self.tick_count,
                self.players.len(),
                self.chunks.len()
            );
        }
    }

    fn generate_spawn_chunks(&mut self) {
        let spawn_radius = 4;
        for cx in -spawn_radius..=spawn_radius {
            for cz in -spawn_radius..=spawn_radius {
                let pos = ChunkPos::new(cx, cz);
                self.chunks.insert(pos, Chunk::new_flat(pos));
            }
        }
    }

    pub fn get_chunk(&self, pos: &ChunkPos) -> Option<&Chunk> {
        self.chunks.get(pos)
    }

    pub fn get_or_generate_chunk(&mut self, pos: ChunkPos) -> &Chunk {
        self.chunks
            .entry(pos)
            .or_insert_with(|| Chunk::new_flat(pos))
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ChunkUpdate {
    pub to_load: Vec<ChunkPos>,
    pub to_unload: Vec<ChunkPos>,
}

impl World {
    pub fn compute_chunk_updates(
        &mut self,
        uuid: &Uuid,
        view_distance: i32,
    ) -> Option<ChunkUpdate> {
        let player = self.players.get_mut(uuid)?;

        // Calculate current chunk position
        let player_chunk = ChunkPos::from_block(player.x as i32, player.z as i32);

        // Determine visible chunk set
        let mut visible_chunks = HashSet::new();
        for dx in -view_distance..=view_distance {
            for dz in -view_distance..=view_distance {
                visible_chunks.insert(ChunkPos::new(player_chunk.x + dx, player_chunk.z + dz));
            }
        }

        // Compute differences
        let to_load: Vec<ChunkPos> = visible_chunks
            .difference(&player.loaded_chunks)
            .copied()
            .collect();

        let to_unload: Vec<ChunkPos> = player
            .loaded_chunks
            .difference(&visible_chunks)
            .copied()
            .collect();

        // Update loaded set
        player.loaded_chunks = visible_chunks;

        if to_load.is_empty() && to_unload.is_empty() {
            return None;
        }

        Some(ChunkUpdate { to_load, to_unload })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_add_remove_player() {
        let mut world = World::new();
        let uuid = Uuid::new_v4();
        let eid = world.add_player(uuid, "Test".to_string(), 8);
        assert_eq!(world.player_count(), 1);
        assert!(eid > 0);

        world.remove_player(&uuid);
        assert_eq!(world.player_count(), 0);
    }

    #[test]
    fn test_world_spawn_chunks() {
        let world = World::new();
        let center = ChunkPos::new(0, 0);
        assert!(world.get_chunk(&center).is_some());
    }

    #[test]
    fn test_world_tick() {
        let mut world = World::new();
        for _ in 0..100 {
            world.tick();
        }
    }

    #[test]
    fn test_update_player_position() {
        let mut world = World::new();
        let uuid = Uuid::new_v4();
        world.add_player(uuid, "Test".to_string(), 8);

        world.update_player_position(&uuid, 100.0, 65.0, 200.0);

        let player = world.players.get(&uuid).unwrap();
        assert_eq!(player.x, 100.0);
        assert_eq!(player.y, 65.0);
        assert_eq!(player.z, 200.0);
    }

    #[test]
    fn test_compute_chunk_updates_initial() {
        let mut world = World::new();
        let uuid = Uuid::new_v4();
        world.add_player(uuid, "Test".to_string(), 8);

        let update = world.compute_chunk_updates(&uuid, 2).unwrap();
        // View distance 2 means 5x5 grid = 25 chunks
        assert_eq!(update.to_load.len(), 25);
        assert_eq!(update.to_unload.len(), 0);
    }

    #[test]
    fn test_compute_chunk_updates_move_one_chunk() {
        let mut world = World::new();
        let uuid = Uuid::new_v4();
        world.add_player(uuid, "Test".to_string(), 8);

        // Initial load at (0, 0)
        world.compute_chunk_updates(&uuid, 2);

        // Move to chunk (1, 0) - move 16 blocks in X
        world.update_player_position(&uuid, 16.0, 64.0, 0.0);

        let update = world.compute_chunk_updates(&uuid, 2).unwrap();
        // Moving one chunk should load 5 chunks on the right, unload 5 on the left
        assert_eq!(update.to_load.len(), 5);
        assert_eq!(update.to_unload.len(), 5);
    }

    #[test]
    fn test_compute_chunk_updates_no_move() {
        let mut world = World::new();
        let uuid = Uuid::new_v4();
        world.add_player(uuid, "Test".to_string(), 8);

        // Initial load
        world.compute_chunk_updates(&uuid, 2);

        // Stay in same chunk
        world.update_player_position(&uuid, 1.0, 64.0, 1.0);

        let update = world.compute_chunk_updates(&uuid, 2);
        assert!(update.is_none());
    }

    #[test]
    fn test_player_chunks_per_tick_default() {
        let mut world = World::new();
        let uuid = Uuid::new_v4();
        world.add_player(uuid, "Test".to_string(), 8);

        let player = world.players.get(&uuid).unwrap();
        assert_eq!(player.chunks_per_tick, 25.0);
    }

    #[test]
    fn test_player_chunks_per_tick_update() {
        let mut world = World::new();
        let uuid = Uuid::new_v4();
        world.add_player(uuid, "Test".to_string(), 8);

        let player = world.players.get_mut(&uuid).unwrap();
        player.chunks_per_tick = 10.0;
        assert_eq!(player.chunks_per_tick, 10.0);
    }

    #[test]
    fn test_player_chunks_per_tick_clamping() {
        let mut world = World::new();
        let uuid = Uuid::new_v4();
        world.add_player(uuid, "Test".to_string(), 8);

        let player = world.players.get_mut(&uuid).unwrap();

        // Clamp to minimum
        player.chunks_per_tick = 0.0_f32.clamp(1.0, 100.0);
        assert_eq!(player.chunks_per_tick, 1.0);

        // Clamp to maximum
        player.chunks_per_tick = 999.0_f32.clamp(1.0, 100.0);
        assert_eq!(player.chunks_per_tick, 100.0);

        // Negative value clamps to minimum
        player.chunks_per_tick = (-1.0_f32).clamp(1.0, 100.0);
        assert_eq!(player.chunks_per_tick, 1.0);

        // Valid value stays unchanged
        player.chunks_per_tick = 50.0_f32.clamp(1.0, 100.0);
        assert_eq!(player.chunks_per_tick, 50.0);
    }

    #[test]
    fn test_player_op_level_default() {
        let mut world = World::new();
        let uuid = Uuid::new_v4();
        world.add_player(uuid, "Test".to_string(), 8);

        let player = world.players.get(&uuid).unwrap();
        assert_eq!(player.op_level, 0);
    }

    #[test]
    fn test_player_op_level_with_value() {
        let mut world = World::new();
        let uuid = Uuid::new_v4();
        world.add_player_with_op_level(uuid, "OpPlayer".to_string(), 4, 8);

        let player = world.players.get(&uuid).unwrap();
        assert_eq!(player.op_level, 4);
    }

    #[test]
    fn test_update_player_rotation() {
        let mut world = World::new();
        let uuid = Uuid::new_v4();
        world.add_player(uuid, "Test".to_string(), 8);

        world.update_player_rotation(&uuid, 90.0, -45.0);

        let player = world.players.get(&uuid).unwrap();
        assert_eq!(player.yaw, 90.0);
        assert_eq!(player.pitch, -45.0);
    }
}
