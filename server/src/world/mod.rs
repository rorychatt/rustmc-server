pub mod chunk;

use chunk::{Chunk, ChunkPos};
use std::collections::HashMap;
use tracing::debug;
use uuid::Uuid;

pub struct Player {
    pub uuid: Uuid,
    pub name: String,
    pub entity_id: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

pub struct World {
    players: HashMap<Uuid, Player>,
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

    pub fn add_player(&mut self, uuid: Uuid, name: String) -> i32 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_add_remove_player() {
        let mut world = World::new();
        let uuid = Uuid::new_v4();
        let eid = world.add_player(uuid, "Test".to_string());
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
}
