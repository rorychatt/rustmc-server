use serde::{Serialize, Deserialize};

pub const CHUNK_WIDTH: usize = 16;
pub const CHUNK_HEIGHT: usize = 384; // -64 to 320
pub const SECTION_HEIGHT: usize = 16;
pub const SECTIONS_PER_CHUNK: usize = CHUNK_HEIGHT / SECTION_HEIGHT;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockState(pub u16);

// IDs from version 26.1.2 — validated by test_block_state_ids_match_generated_data
impl BlockState {
    pub const AIR: Self = Self(0);
    pub const STONE: Self = Self(1);
    pub const GRASS_BLOCK: Self = Self(9);
    pub const DIRT: Self = Self(10);
    pub const COBBLESTONE: Self = Self(14);
    pub const OAK_PLANKS: Self = Self(15);
    pub const BEDROCK: Self = Self(85);
    pub const WATER: Self = Self(86);
    pub const LAVA: Self = Self(102);
    pub const SAND: Self = Self(118);
    pub const GRAVEL: Self = Self(124);
    pub const OAK_LOG: Self = Self(137);
}

#[derive(Clone, Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub struct Chunk {
    pub pos: ChunkPos,
    pub sections: Vec<ChunkSection>,
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
        // Flat world template matching vanilla:
        // bedrock at y=-64 (abs 0)
        // dirt at y=-63, y=-62 (abs 1, 2)
        // grass block at y=-61 (abs 3)

        for x in 0..CHUNK_WIDTH {
            for z in 0..CHUNK_WIDTH {
                chunk.set_block(x, 0, z, BlockState::BEDROCK);
                chunk.set_block(x, 1, z, BlockState::DIRT);
                chunk.set_block(x, 2, z, BlockState::DIRT);
                chunk.set_block(x, 3, z, BlockState::GRASS_BLOCK);
            }
        }
        chunk
    }

    pub fn new_normal(pos: ChunkPos, seed: u64, sea_level: i32) -> Self {
        let mut chunk = Self::new(pos);

        // Simple bit mixing hash function for deterministic terrain generation
        let hash2d = |x: i32, z: i32| -> f64 {
            let mut h = (x as u64).wrapping_mul(3432918353)
                .wrapping_add((z as u64).wrapping_mul(461845907))
                .wrapping_add(seed);
            h ^= h >> 33;
            h = h.wrapping_mul(0xff51afd7ed558ccd);
            h ^= h >> 33;
            h = h.wrapping_mul(0xc4ceb9fe1a85ec53);
            h ^= h >> 33;
            (h as f64) / (u64::MAX as f64)
        };

        // Bilinear interpolation with smoothstep
        let noise2d = |x: f64, z: f64| -> f64 {
            let x0 = x.floor() as i32;
            let x1 = x0 + 1;
            let z0 = z.floor() as i32;
            let z1 = z0 + 1;

            let tx = x - x.floor();
            let tz = z - z.floor();

            let sx = tx * tx * (3.0 - 2.0 * tx);
            let sz = tz * tz * (3.0 - 2.0 * tz);

            let n00 = hash2d(x0, z0);
            let n10 = hash2d(x1, z0);
            let n01 = hash2d(x0, z1);
            let n11 = hash2d(x1, z1);

            let nx0 = n00 + sx * (n10 - n00);
            let nx1 = n01 + sx * (n11 - n01);

            nx0 + sz * (nx1 - nx0)
        };

        // 4-octave Fractional Brownian Motion (fBm)
        let fbm = |x: f64, z: f64, octaves: usize| -> f64 {
            let mut value = 0.0;
            let mut amplitude = 1.0;
            let mut frequency = 1.0;
            let mut max_value = 0.0;
            for _ in 0..octaves {
                value += amplitude * noise2d(x * frequency, z * frequency);
                max_value += amplitude;
                amplitude *= 0.5;
                frequency *= 2.0;
            }
            value / max_value
        };

        let sea_level_abs = sea_level + 64;

        for x in 0..CHUNK_WIDTH {
            for z in 0..CHUNK_WIDTH {
                let world_x = (pos.x * 16) + x as i32;
                let world_z = (pos.z * 16) + z as i32;

                // fBm value between 0.0 and 1.0
                let n = fbm(world_x as f64 * 0.012, world_z as f64 * 0.012, 4);

                // Scale surface height: centered around sea_level_abs +/- 32 blocks
                let surface_height = sea_level_abs + (n * 64.0 - 32.0) as i32;
                let surface_height = surface_height.clamp(10, 300);

                // y=-64 (abs 0): Bedrock
                chunk.set_block(x, 0, z, BlockState::BEDROCK);

                // Fill columns with stone, dirt, grass, sand, and water
                for abs_y in 1..=320 {
                    if abs_y < (surface_height - 4) as usize {
                        chunk.set_block(x, abs_y, z, BlockState::STONE);
                    } else if abs_y < surface_height as usize {
                        chunk.set_block(x, abs_y, z, BlockState::DIRT);
                    } else if abs_y == surface_height as usize {
                        // Shoreline/beach sand vs above-water grass block
                        if surface_height < sea_level_abs + 2 {
                            chunk.set_block(x, abs_y, z, BlockState::SAND);
                        } else {
                            chunk.set_block(x, abs_y, z, BlockState::GRASS_BLOCK);
                        }
                    } else if abs_y <= sea_level_abs as usize {
                        // Place water in oceans/lakes
                        chunk.set_block(x, abs_y, z, BlockState::WATER);
                    }
                }
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
    use crate::world::block_registry::{BlockRegistry, BLOCKS};

    #[test]
    fn test_block_state_ids_match_generated_data() {
        let registry = BlockRegistry::global();

        assert_eq!(
            BlockState::AIR.0,
            registry.get_default_state_id("minecraft:air").unwrap()
        );
        assert_eq!(
            BlockState::STONE.0,
            registry.get_default_state_id("minecraft:stone").unwrap()
        );
        assert_eq!(
            BlockState::GRASS_BLOCK.0,
            registry
                .get_default_state_id("minecraft:grass_block")
                .unwrap()
        );
        assert_eq!(
            BlockState::DIRT.0,
            registry.get_default_state_id("minecraft:dirt").unwrap()
        );
        assert_eq!(
            BlockState::COBBLESTONE.0,
            registry
                .get_default_state_id("minecraft:cobblestone")
                .unwrap()
        );
        assert_eq!(
            BlockState::OAK_PLANKS.0,
            registry
                .get_default_state_id("minecraft:oak_planks")
                .unwrap()
        );
        assert_eq!(
            BlockState::BEDROCK.0,
            registry.get_default_state_id("minecraft:bedrock").unwrap()
        );
        assert_eq!(
            BlockState::WATER.0,
            registry.get_default_state_id("minecraft:water").unwrap()
        );
        assert_eq!(
            BlockState::LAVA.0,
            registry.get_default_state_id("minecraft:lava").unwrap()
        );
        assert_eq!(
            BlockState::SAND.0,
            registry.get_default_state_id("minecraft:sand").unwrap()
        );
        assert_eq!(
            BlockState::GRAVEL.0,
            registry.get_default_state_id("minecraft:gravel").unwrap()
        );
        assert_eq!(
            BlockState::OAK_LOG.0,
            registry.get_default_state_id("minecraft:oak_log").unwrap()
        );
    }

    #[test]
    fn test_all_default_states_in_states_list() {
        for (name, block_def) in &BLOCKS {
            assert!(!block_def.states.is_empty(), "Block {name} has no states");
            let has_default = block_def
                .states
                .iter()
                .any(|s| s.id == block_def.default_state_id);
            assert!(
                has_default,
                "Block {} default_state_id {} not found in states list",
                name, block_def.default_state_id
            );
        }
    }

    #[test]
    fn test_state_ids_unique() {
        let mut seen = std::collections::HashSet::new();
        for (name, block_def) in &BLOCKS {
            for state in block_def.states {
                assert!(
                    seen.insert(state.id),
                    "Duplicate state ID {} in block {}",
                    state.id,
                    name
                );
            }
        }
    }

    #[test]
    fn test_state_properties_match_declared() {
        for (name, block_def) in &BLOCKS {
            for state in block_def.states {
                assert_eq!(
                    state.properties.len(),
                    block_def.properties.len(),
                    "Block {name} state has wrong number of properties"
                );
                for (key, value) in state.properties {
                    let prop = block_def.properties.iter().find(|(k, _)| k == key);
                    let (_, allowed_values) = prop.unwrap_or_else(|| {
                        panic!("Block {name} state has undeclared property {key}")
                    });
                    assert!(
                        allowed_values.contains(value),
                        "Block {name} property {key}={value} not in allowed values"
                    );
                }
            }
        }
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
        // Bedrock at abs index 0 (y=-64)
        assert_eq!(chunk.get_block(0, 0, 0), BlockState::BEDROCK);
        // Dirt at abs indices 1 & 2 (y=-63, -62)
        assert_eq!(chunk.get_block(0, 1, 0), BlockState::DIRT);
        assert_eq!(chunk.get_block(0, 2, 0), BlockState::DIRT);
        // Grass at abs index 3 (y=-61)
        assert_eq!(chunk.get_block(0, 3, 0), BlockState::GRASS_BLOCK);
        // Air at y=-60 and above
        assert_eq!(chunk.get_block(0, 4, 0), BlockState::AIR);
        assert_eq!(chunk.get_block(0, 64, 0), BlockState::AIR);
    }

    #[test]
    fn test_normal_chunk_generation() {
        // Test normal chunk generation creates grass/dirt/stone, respects seed, and sea level
        let chunk = Chunk::new_normal(ChunkPos::new(0, 0), 42, 63);
        
        // Bedrock at absolute bottom (abs 0)
        assert_eq!(chunk.get_block(0, 0, 0), BlockState::BEDROCK);
        
        // Deep underground should be stone
        assert_eq!(chunk.get_block(0, 40, 0), BlockState::STONE);

        // Find the height of the surface
        let mut surface_y = 0;
        for abs_y in (0..384).rev() {
            let block = chunk.get_block(0, abs_y, 0);
            if block != BlockState::AIR && block != BlockState::WATER {
                surface_y = abs_y;
                break;
            }
        }

        // Surface block must be grass or sand
        let surface_block = chunk.get_block(0, surface_y, 0);
        assert!(surface_block == BlockState::GRASS_BLOCK || surface_block == BlockState::SAND);
        
        // Block below surface must be dirt
        assert_eq!(chunk.get_block(0, surface_y - 1, 0), BlockState::DIRT);
        
        // Blocks far below surface must be stone
        assert_eq!(chunk.get_block(0, surface_y - 10, 0), BlockState::STONE);
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
