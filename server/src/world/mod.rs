pub mod block_registry;
pub mod chunk;
pub mod persistence;

use chunk::{Chunk, ChunkPos, BlockState};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::num::NonZeroUsize;
use lru::LruCache;
use tokio::sync::RwLock;
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
    chunks: Mutex<LruCache<ChunkPos, Arc<Chunk>>>,
    dirty_chunks: Mutex<HashSet<ChunkPos>>,
    next_entity_id: i32,
    tick_count: u64,
    pub world_type: String,
    pub seed: u64,
    pub sea_level: i32,
    pub world_dir: Option<std::path::PathBuf>,
    pub db: Option<Arc<redb::Database>>,
}

impl World {
    pub fn new() -> Self {
        Self::new_with_config("flat".to_string(), 0, 63)
    }

    pub fn new_with_config(world_type: String, seed: u64, sea_level: i32) -> Self {
        Self::new_with_dir(world_type, seed, sea_level, None)
    }

    pub fn new_with_dir(
        world_type: String,
        seed: u64,
        sea_level: i32,
        world_dir: Option<std::path::PathBuf>,
    ) -> Self {
        let db = if let Some(ref dir) = world_dir {
            match persistence::open_database(dir) {
                Ok(db) => Some(Arc::new(db)),
                Err(e) => {
                    tracing::error!("Failed to open world database: {:?}", e);
                    None
                }
            }
        } else {
            None
        };

        let mut world = Self {
            players: HashMap::new(),
            chunks: Mutex::new(LruCache::new(NonZeroUsize::new(2048).unwrap())),
            dirty_chunks: Mutex::new(HashSet::new()),
            next_entity_id: 1,
            tick_count: 0,
            world_type,
            seed,
            sea_level,
            world_dir,
            db,
        };
        // Load level metadata if it exists
        if let Some(ref dir) = world.world_dir {
            if let Ok(Some(level_info)) = persistence::load_level_info(dir) {
                world.world_type = level_info.world_type;
                world.seed = level_info.seed;
                world.sea_level = level_info.sea_level;
            } else {
                // Save current configuration to level.json
                let level_info = persistence::LevelInfo {
                    seed: world.seed,
                    world_type: world.world_type.clone(),
                    sea_level: world.sea_level,
                };
                let _ = persistence::save_level_info(dir, &level_info);
            }
        }
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

        // Dynamic spawn height based on terrain structure
        let mut spawn_y = 64.0;
        let chunk_pos = ChunkPos::from_block(0, 0);
        if let Some(chunk) = self.get_chunk(&chunk_pos) {
            for abs_y in (0..384).rev() {
                let block = chunk.get_block(0, abs_y, 0);
                if block != BlockState::AIR && block != BlockState::WATER {
                    spawn_y = (abs_y as f64 - 64.0) + 1.0;
                    break;
                }
            }
        }

        self.players.insert(
            uuid,
            Player {
                uuid,
                name,
                entity_id,
                x: 0.0,
                y: spawn_y,
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
                self.chunks.lock().unwrap().len()
            );
        }
    }

    fn generate_spawn_chunks(&mut self) {
        let spawn_radius = 2;
        let world_type = self.world_type.clone();
        let seed = self.seed;
        let sea_level = self.sea_level;
        let db = self.db.clone();

        let mut chunks_lock = self.chunks.lock().unwrap();
        for cx in -spawn_radius..=spawn_radius {
            for cz in -spawn_radius..=spawn_radius {
                let pos = ChunkPos::new(cx, cz);
                let chunk = if let Some(ref database) = db {
                    if let Ok(Some(loaded)) = persistence::load_chunk(database, pos) {
                        loaded
                    } else {
                        let new_chunk = if world_type == "flat" {
                            Chunk::new_flat(pos)
                        } else {
                            Chunk::new_normal(pos, seed, sea_level)
                        };
                        let _ = persistence::save_chunk(database, &new_chunk);
                        new_chunk
                    }
                } else {
                    if world_type == "flat" {
                        Chunk::new_flat(pos)
                    } else {
                        Chunk::new_normal(pos, seed, sea_level)
                    }
                };
                chunks_lock.put(pos, Arc::new(chunk));
            }
        }
    }

    pub fn get_chunk(&self, pos: &ChunkPos) -> Option<Arc<Chunk>> {
        self.chunks.lock().unwrap().get(pos).cloned()
    }

    pub fn insert_chunk(&self, pos: ChunkPos, chunk: Arc<Chunk>) {
        let evicted = {
            let mut chunks = self.chunks.lock().unwrap();
            chunks.push(pos, chunk)
        };

        if let Some((evicted_pos, evicted_chunk)) = evicted {
            // Only handle eviction if it's a different chunk, not replacing/updating the same chunk
            if evicted_pos != pos {
                // Check if evicted chunk is dirty
                let was_dirty = {
                    let mut dirty = self.dirty_chunks.lock().unwrap();
                    dirty.remove(&evicted_pos)
                };

                if was_dirty {
                    if let Some(ref db) = self.db {
                        let db = db.clone();
                        tokio::spawn(async move {
                            let _ = tokio::task::spawn_blocking(move || {
                                if let Err(e) = persistence::save_chunk(&db, &evicted_chunk) {
                                    tracing::error!("Failed to save evicted chunk at {:?}: {:?}", evicted_pos, e);
                                }
                            }).await;
                        });
                    }
                }
            }
        }
    }

    pub fn get_or_generate_chunk(&mut self, pos: ChunkPos) -> Arc<Chunk> {
        if let Some(chunk) = self.get_chunk(&pos) {
            return chunk;
        }

        let world_type = self.world_type.clone();
        let seed = self.seed;
        let sea_level = self.sea_level;
        let db = self.db.clone();

        let chunk = if let Some(ref database) = db {
            if let Ok(Some(loaded)) = persistence::load_chunk(database, pos) {
                loaded
            } else {
                let new_chunk = if world_type == "flat" {
                    Chunk::new_flat(pos)
                } else {
                    Chunk::new_normal(pos, seed, sea_level)
                };
                let _ = persistence::save_chunk(database, &new_chunk);
                new_chunk
            }
        } else {
            if world_type == "flat" {
                Chunk::new_flat(pos)
            } else {
                Chunk::new_normal(pos, seed, sea_level)
            }
        };

        let chunk = Arc::new(chunk);
        self.insert_chunk(pos, chunk.clone());
        chunk
    }

    pub fn set_block(&self, pos: ChunkPos, x: usize, y: usize, z: usize, state: BlockState) {
        let chunk = self.get_chunk(&pos);
        if let Some(chunk) = chunk {
            let mut new_chunk = (*chunk).clone();
            new_chunk.set_block(x, y, z, state);
            {
                let mut dirty = self.dirty_chunks.lock().unwrap();
                dirty.insert(pos);
            }
            self.insert_chunk(pos, Arc::new(new_chunk));
        }
    }

    pub fn save_all(&self) -> anyhow::Result<()> {
        if let Some(ref dir) = self.world_dir {
            let level_info = persistence::LevelInfo {
                seed: self.seed,
                world_type: self.world_type.clone(),
                sea_level: self.sea_level,
            };
            persistence::save_level_info(dir, &level_info)?;
        }
        if let Some(ref db) = self.db {
            let chunks_to_save: Vec<Arc<Chunk>> = {
                let mut dirty = self.dirty_chunks.lock().unwrap();
                let mut chunks = self.chunks.lock().unwrap();
                let mut list = Vec::new();
                for pos in dirty.drain() {
                    if let Some(chunk) = chunks.get(&pos) {
                        list.push(chunk.clone());
                    }
                }
                list
            };
            for chunk in chunks_to_save {
                persistence::save_chunk(db, &chunk)?;
            }
        }
        Ok(())
    }

    pub async fn save_all_async(world: Arc<RwLock<Self>>) -> anyhow::Result<()> {
        let (db, level_info, world_dir, chunks_to_save) = {
            let w = world.read().await;
            let db = w.db.clone();
            let level_info = persistence::LevelInfo {
                seed: w.seed,
                world_type: w.world_type.clone(),
                sea_level: w.sea_level,
            };
            let world_dir = w.world_dir.clone();
            let chunks_to_save = {
                let mut dirty = w.dirty_chunks.lock().unwrap();
                let mut chunks = w.chunks.lock().unwrap();
                let mut list = Vec::new();
                for pos in dirty.drain() {
                    if let Some(chunk) = chunks.get(&pos) {
                        list.push(chunk.clone());
                    }
                }
                list
            };
            (db, level_info, world_dir, chunks_to_save)
        };

        tokio::task::spawn_blocking(move || {
            if let Some(ref dir) = world_dir {
                persistence::save_level_info(dir, &level_info)?;
            }
            if let Some(ref db) = db {
                for chunk in chunks_to_save {
                    persistence::save_chunk(db, &chunk)?;
                }
            }
            Ok::<(), anyhow::Error>(())
        }).await.unwrap()
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

    #[tokio::test]
    async fn test_world_dirty_save_all() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let path = dir.path();

        let world = World::new_with_dir("flat".to_string(), 0, 63, Some(path.to_path_buf()));
        let pos = ChunkPos::new(0, 0);

        // Verify the chunk exists and is clean initially
        let chunk = world.get_chunk(&pos).unwrap();
        assert_eq!(chunk.get_block(0, 4, 0), BlockState::AIR);

        // Modify block - triggers marking it as dirty
        world.set_block(pos, 0, 4, 0, BlockState::STONE);

        // Verify block modification is reflected in-memory
        let chunk_after = world.get_chunk(&pos).unwrap();
        assert_eq!(chunk_after.get_block(0, 4, 0), BlockState::STONE);

        {
            let dirty = world.dirty_chunks.lock().unwrap();
            assert!(dirty.contains(&pos));
        }

        // Save all chunks to disk
        world.save_all().unwrap();

        // Verify dirty set is cleared
        {
            let dirty = world.dirty_chunks.lock().unwrap();
            assert!(!dirty.contains(&pos));
        }

        // Drop world to release redb file lock
        drop(world);

        // Load new world from the same directory to verify serialization
        let world2 = World::new_with_dir("flat".to_string(), 0, 63, Some(path.to_path_buf()));
        let loaded = world2.get_chunk(&pos).unwrap();
        assert_eq!(loaded.get_block(0, 4, 0), BlockState::STONE);
    }

    #[tokio::test]
    async fn test_world_eviction_auto_save() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let path = dir.path();

        // Construct a world with the dir
        let mut world = World::new_with_dir("flat".to_string(), 0, 63, Some(path.to_path_buf()));

        // Insert a dirty chunk at pos (100, 100)
        let pos = ChunkPos::new(100, 100);
        let chunk = world.get_or_generate_chunk(pos);
        assert_eq!(chunk.get_block(0, 4, 0), BlockState::AIR);

        // Modify to make it dirty
        world.set_block(pos, 0, 4, 0, BlockState::STONE);

        {
            let dirty = world.dirty_chunks.lock().unwrap();
            assert!(dirty.contains(&pos));
        }

        // Now, generate/insert 2048 new chunks at coordinates that are NOT (100, 100)
        // to force the LRU cache to evict the chunk at (100, 100).
        for i in 0..2048 {
            let dummy_pos = ChunkPos::new(200 + i, 200);
            world.get_or_generate_chunk(dummy_pos);
        }

        // The chunk at (100, 100) should be evicted from the in-memory cache now.
        assert!(world.get_chunk(&pos).is_none());

        // Eviction spawns a background task. Yield execution to allow the task to finish writing.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Since it was dirty and evicted, it should have been saved to the database and removed from the dirty set.
        {
            let dirty = world.dirty_chunks.lock().unwrap();
            assert!(!dirty.contains(&pos));
        }

        // Drop the world to release file handle/lock
        drop(world);

        // Verify the saved data can be loaded in a new world
        let mut world2 = World::new_with_dir("flat".to_string(), 0, 63, Some(path.to_path_buf()));
        let loaded = world2.get_or_generate_chunk(pos);
        assert_eq!(loaded.get_block(0, 4, 0), BlockState::STONE);
    }
}

