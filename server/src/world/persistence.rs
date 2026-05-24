use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use crate::world::chunk::{Chunk, ChunkPos};
use redb::TableDefinition;

pub const CHUNKS_TABLE: TableDefinition<[u8; 8], &[u8]> = TableDefinition::new("chunks");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LevelInfo {
    pub seed: u64,
    pub world_type: String,
    pub sea_level: i32,
}

pub fn chunk_key(pos: ChunkPos) -> [u8; 8] {
    let mut key = [0u8; 8];
    key[0..4].copy_from_slice(&pos.x.to_be_bytes());
    key[4..8].copy_from_slice(&pos.z.to_be_bytes());
    key
}

pub fn open_database(world_dir: &Path) -> anyhow::Result<redb::Database> {
    fs::create_dir_all(world_dir)?;
    let db_path = world_dir.join("chunks.redb");
    let db = redb::Database::create(&db_path)?;
    
    // Ensure chunks table is created
    let write_txn = db.begin_write()?;
    {
        let _ = write_txn.open_table(CHUNKS_TABLE)?;
    }
    write_txn.commit()?;
    
    Ok(db)
}

pub fn save_chunk(db: &redb::Database, chunk: &Chunk) -> anyhow::Result<()> {
    let key = chunk_key(chunk.pos);
    let serialized = bincode::serialize(chunk)?;
    
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(CHUNKS_TABLE)?;
        table.insert(&key, serialized.as_slice())?;
    }
    write_txn.commit()?;
    
    Ok(())
}

pub fn load_chunk(db: &redb::Database, pos: ChunkPos) -> anyhow::Result<Option<Chunk>> {
    let key = chunk_key(pos);
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(CHUNKS_TABLE)?;
    let value = table.get(&key)?;
    
    match value {
        Some(guard) => {
            let chunk: Chunk = bincode::deserialize(guard.value())?;
            Ok(Some(chunk))
        }
        None => Ok(None),
    }
}

pub fn save_level_info(world_dir: &Path, level_info: &LevelInfo) -> anyhow::Result<()> {
    fs::create_dir_all(world_dir)?;
    let level_file = world_dir.join("level.json");
    let file = fs::File::create(level_file)?;
    serde_json::to_writer_pretty(file, level_info)?;
    Ok(())
}

pub fn load_level_info(world_dir: &Path) -> anyhow::Result<Option<LevelInfo>> {
    let level_file = world_dir.join("level.json");
    if !level_file.exists() {
        return Ok(None);
    }
    let file = fs::File::open(level_file)?;
    let level_info: LevelInfo = serde_json::from_reader(file)?;
    Ok(Some(level_info))
}

pub fn create_backup(world_dir: &Path, backups_parent: &Path, max_backups: usize) -> anyhow::Result<()> {
    if !world_dir.exists() {
        return Ok(());
    }
    
    let backups_dir = backups_parent.join("backups");
    fs::create_dir_all(&backups_dir)?;
    
    let now = std::time::SystemTime::now();
    let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let backup_path = backups_dir.join(format!("backup_{}", duration.as_millis()));
    
    copy_dir_all(world_dir, &backup_path)?;
    prune_old_backups(&backups_dir, max_backups)?;
    
    Ok(())
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn prune_old_backups(backups_dir: &Path, max_backups: usize) -> std::io::Result<()> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(backups_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("backup_") {
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    entries.push((entry.path(), modified));
                }
            }
        }
    }
    
    entries.sort_by_key(|&(_, time)| time);
    
    if entries.len() > max_backups {
        let to_remove = entries.len() - max_backups;
        for (path, _) in &entries[..to_remove] {
            if path.is_dir() {
                let _ = fs::remove_dir_all(path);
            } else {
                let _ = fs::remove_file(path);
            }
        }
    }
    
    Ok(())
}

use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct InventorySlot {
    pub slot: i32,
    pub item_id: String,
    pub count: u8,
    pub nbt: Option<Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PlayerData {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub is_sneaking: bool,
    pub is_sprinting: bool,
    pub selected_slot: u8,
    pub op_level: u8,
    pub view_distance: i32,
    pub gamemode: Option<i32>,
    pub inventory: Vec<InventorySlot>,
}

pub fn save_player_data(world_dir: &Path, uuid: Uuid, player_data: &PlayerData) -> anyhow::Result<()> {
    let player_dir = world_dir.join("playerdata");
    fs::create_dir_all(&player_dir)?;
    let file_path = player_dir.join(format!("{}.json", uuid));
    let file = fs::File::create(file_path)?;
    serde_json::to_writer_pretty(file, player_data)?;
    Ok(())
}

pub fn load_player_data(world_dir: &Path, uuid: Uuid) -> anyhow::Result<Option<PlayerData>> {
    let file_path = world_dir.join("playerdata").join(format!("{}.json", uuid));
    if !file_path.exists() {
        return Ok(None);
    }
    let file = fs::File::open(file_path)?;
    let player_data: PlayerData = serde_json::from_reader(file)?;
    Ok(Some(player_data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_level_info_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path();
        
        let level_info = LevelInfo {
            seed: 123456789,
            world_type: "flat".to_string(),
            sea_level: 63,
        };
        
        save_level_info(path, &level_info).unwrap();
        
        let loaded = load_level_info(path).unwrap().unwrap();
        assert_eq!(loaded, level_info);
    }

    #[test]
    fn test_missing_level_info() {
        let dir = tempdir().unwrap();
        let loaded = load_level_info(dir.path()).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_chunk_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path();
        let db = open_database(path).unwrap();
        
        let pos = ChunkPos::new(3, -2);
        let mut chunk = Chunk::new(pos);
        
        // Put some custom blocks
        chunk.set_block(2, 4, 5, crate::world::chunk::BlockState::STONE);
        chunk.set_block(7, 12, 11, crate::world::chunk::BlockState::GRASS_BLOCK);
        
        save_chunk(&db, &chunk).unwrap();
        
        let loaded_chunk = load_chunk(&db, pos).unwrap().unwrap();
        assert_eq!(loaded_chunk.pos, pos);
        assert_eq!(loaded_chunk.get_block(2, 4, 5), crate::world::chunk::BlockState::STONE);
        assert_eq!(loaded_chunk.get_block(7, 12, 11), crate::world::chunk::BlockState::GRASS_BLOCK);
        assert_eq!(loaded_chunk.get_block(0, 0, 0), crate::world::chunk::BlockState::AIR);
    }

    #[test]
    fn test_missing_chunk() {
        let dir = tempdir().unwrap();
        let path = dir.path();
        let db = open_database(path).unwrap();
        let loaded = load_chunk(&db, ChunkPos::new(0, 0)).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_backup_and_prune() {
        let temp = tempdir().unwrap();
        let path = temp.path();
        
        let world_dir = path.join("world");
        fs::create_dir_all(&world_dir).unwrap();
        fs::write(world_dir.join("level.json"), "{}").unwrap();
        
        // Create 3 backups (with max_backups = 2)
        // Sleep slightly to guarantee distinct modification timestamps
        create_backup(&world_dir, path, 2).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        create_backup(&world_dir, path, 2).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        create_backup(&world_dir, path, 2).unwrap();
        
        // Verify only 2 backups remain under path/backups/
        let backups_dir = path.join("backups");
        let count = fs::read_dir(backups_dir).unwrap().count();
        assert_eq!(count, 2);
    }
}
