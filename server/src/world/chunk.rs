pub const CHUNK_WIDTH: usize = 16;
pub const CHUNK_HEIGHT: usize = 384; // -64 to 320
pub const SECTION_HEIGHT: usize = 16;
pub const SECTIONS_PER_CHUNK: usize = CHUNK_HEIGHT / SECTION_HEIGHT;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl ChunkPos {
    pub fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    pub fn from_block(block_x: i32, block_z: i32) -> Self {
        Self {
            x: block_x >> 4,
            z: block_z >> 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockState(pub u16);

// IDs from protocol 775 — validated by test_block_state_ids_match_generated_data
impl BlockState {
    pub const AIR: Self = Self(0);
    pub const STONE: Self = Self(1);
    pub const DIRT: Self = Self(10);
    pub const GRASS_BLOCK: Self = Self(9);
    pub const BEDROCK: Self = Self(33);
}

#[derive(Clone)]
pub struct ChunkSection {
    blocks: Vec<BlockState>,
    non_air_count: u16,
}

impl Default for ChunkSection {
    fn default() -> Self {
        Self {
            blocks: vec![BlockState::AIR; CHUNK_WIDTH * CHUNK_WIDTH * SECTION_HEIGHT],
            non_air_count: 0,
        }
    }
}

impl ChunkSection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_block(&self, x: usize, y: usize, z: usize) -> BlockState {
        self.blocks[y * CHUNK_WIDTH * CHUNK_WIDTH + z * CHUNK_WIDTH + x]
    }

    pub fn set_block(&mut self, x: usize, y: usize, z: usize, state: BlockState) {
        let idx = y * CHUNK_WIDTH * CHUNK_WIDTH + z * CHUNK_WIDTH + x;
        let old = self.blocks[idx];
        if old == BlockState::AIR && state != BlockState::AIR {
            self.non_air_count += 1;
        } else if old != BlockState::AIR && state == BlockState::AIR {
            self.non_air_count -= 1;
        }
        self.blocks[idx] = state;
    }

    pub fn is_empty(&self) -> bool {
        self.non_air_count == 0
    }

    pub fn non_air_count(&self) -> u16 {
        self.non_air_count
    }

    pub fn blocks(&self) -> &[BlockState] {
        &self.blocks
    }
}

pub struct Chunk {
    pub pos: ChunkPos,
    sections: Vec<ChunkSection>,
}

impl Chunk {
    pub fn new(pos: ChunkPos) -> Self {
        Self {
            pos,
            sections: (0..SECTIONS_PER_CHUNK)
                .map(|_| ChunkSection::new())
                .collect(),
        }
    }

    pub fn new_flat(pos: ChunkPos) -> Self {
        let mut chunk = Self::new(pos);
        // Flat world: bedrock at y=-64 (section 0, local y 0), dirt up to y=62, grass at y=63
        // Section index 0 = y -64 to -49
        // Section index 4 = y 0 to 15
        // y=63 is section index 7, local y 15

        for x in 0..CHUNK_WIDTH {
            for z in 0..CHUNK_WIDTH {
                // y=-64 (section 0, local y 0): bedrock
                chunk.set_block(x, 0, z, BlockState::BEDROCK);
                // y=-63 to y=62 (1..127 in absolute-from-bottom)
                for abs_y in 1..127 {
                    chunk.set_block(x, abs_y, z, BlockState::DIRT);
                }
                // y=63 (abs 127): grass
                chunk.set_block(x, 127, z, BlockState::GRASS_BLOCK);
            }
        }
        chunk
    }

    pub fn get_block(&self, x: usize, y: usize, z: usize) -> BlockState {
        let section_idx = y / SECTION_HEIGHT;
        let local_y = y % SECTION_HEIGHT;
        if section_idx >= SECTIONS_PER_CHUNK {
            return BlockState::AIR;
        }
        self.sections[section_idx].get_block(x, local_y, z)
    }

    pub fn set_block(&mut self, x: usize, y: usize, z: usize, state: BlockState) {
        let section_idx = y / SECTION_HEIGHT;
        let local_y = y % SECTION_HEIGHT;
        if section_idx >= SECTIONS_PER_CHUNK {
            return;
        }
        self.sections[section_idx].set_block(x, local_y, z, state);
    }

    pub fn get_section(&self, index: usize) -> Option<&ChunkSection> {
        self.sections.get(index)
    }
}

#[cfg(test)]
mod block_state_tests {
    use super::BlockState;
    use serde_json::Value;

    const BLOCK_STATES_JSON: &str = include_str!("../../data/block_states.json");

    #[test]
    fn test_block_state_ids_match_generated_data() {
        let data: Value = serde_json::from_str(BLOCK_STATES_JSON).unwrap();
        let blocks = data["blocks"].as_object().unwrap();

        assert_eq!(
            BlockState::AIR.0 as i64,
            blocks["minecraft:air"]["default_state_id"].as_i64().unwrap()
        );
        assert_eq!(
            BlockState::STONE.0 as i64,
            blocks["minecraft:stone"]["default_state_id"].as_i64().unwrap()
        );
        assert_eq!(
            BlockState::GRASS_BLOCK.0 as i64,
            blocks["minecraft:grass_block"]["default_state_id"]
                .as_i64()
                .unwrap()
        );
        assert_eq!(
            BlockState::DIRT.0 as i64,
            blocks["minecraft:dirt"]["default_state_id"].as_i64().unwrap()
        );
        assert_eq!(
            BlockState::BEDROCK.0 as i64,
            blocks["minecraft:bedrock"]["default_state_id"]
                .as_i64()
                .unwrap()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_pos_from_block() {
        assert_eq!(ChunkPos::from_block(0, 0), ChunkPos::new(0, 0));
        assert_eq!(ChunkPos::from_block(15, 15), ChunkPos::new(0, 0));
        assert_eq!(ChunkPos::from_block(16, 0), ChunkPos::new(1, 0));
        assert_eq!(ChunkPos::from_block(-1, -1), ChunkPos::new(-1, -1));
    }

    #[test]
    fn test_flat_chunk_generation() {
        let chunk = Chunk::new_flat(ChunkPos::new(0, 0));
        // Bedrock at y=0 (bottom)
        assert_eq!(chunk.get_block(0, 0, 0), BlockState::BEDROCK);
        // Dirt in the middle
        assert_eq!(chunk.get_block(0, 64, 0), BlockState::DIRT);
        // Grass at top of terrain
        assert_eq!(chunk.get_block(0, 127, 0), BlockState::GRASS_BLOCK);
        // Air above terrain
        assert_eq!(chunk.get_block(0, 128, 0), BlockState::AIR);
    }

    #[test]
    fn test_chunk_section_block_count() {
        let mut section = ChunkSection::new();
        assert!(section.is_empty());
        section.set_block(0, 0, 0, BlockState::STONE);
        assert!(!section.is_empty());
        section.set_block(0, 0, 0, BlockState::AIR);
        assert!(section.is_empty());
    }
}
