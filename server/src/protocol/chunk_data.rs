use std::collections::HashMap;
use std::io::{self, Write};

use crate::protocol::nbt;
use crate::protocol::packet::Packet;
use crate::protocol::types::VarInt;
use crate::world::chunk::{BlockState, Chunk, CHUNK_WIDTH, SECTIONS_PER_CHUNK, SECTION_HEIGHT};

pub struct PalettedContainer {
    bits_per_entry: u8,
    palette: Vec<i32>,
    data: Vec<i64>,
}

impl PalettedContainer {
    pub fn from_blocks(blocks: &[BlockState]) -> Self {
        let mut palette_map: HashMap<u16, usize> = HashMap::new();
        let mut palette: Vec<i32> = Vec::new();

        for block in blocks {
            if !palette_map.contains_key(&block.0) {
                palette_map.insert(block.0, palette.len());
                palette.push(block.0 as i32);
            }
        }

        if palette.len() == 1 {
            return Self {
                bits_per_entry: 0,
                palette,
                data: Vec::new(),
            };
        }

        let bits_per_entry = if palette.len() <= 16 {
            4u8
        } else if palette.len() <= 256 {
            let min_bits = (usize::BITS - (palette.len() - 1).leading_zeros()) as u8;
            min_bits.max(4)
        } else {
            15
        };

        let use_direct = palette.len() > 256;
        let values_per_long = 64 / bits_per_entry as usize;
        let total_blocks = blocks.len();
        let num_longs = (total_blocks + values_per_long - 1) / values_per_long;
        let mut data = vec![0i64; num_longs];

        for (i, block) in blocks.iter().enumerate() {
            let index = if use_direct {
                block.0 as u64
            } else {
                palette_map[&block.0] as u64
            };

            let long_index = i / values_per_long;
            let bit_offset = (i % values_per_long) * bits_per_entry as usize;
            data[long_index] |= (index as i64) << bit_offset;
        }

        let final_palette = if use_direct { Vec::new() } else { palette };

        Self {
            bits_per_entry,
            palette: final_palette,
            data,
        }
    }

    pub fn write(&self, writer: &mut impl Write) -> io::Result<()> {
        writer.write_all(&[self.bits_per_entry])?;

        if self.bits_per_entry == 0 {
            VarInt(self.palette[0]).write(writer)?;
            VarInt(0).write(writer)?;
        } else if !self.palette.is_empty() {
            VarInt(self.palette.len() as i32).write(writer)?;
            for &entry in &self.palette {
                VarInt(entry).write(writer)?;
            }
            VarInt(self.data.len() as i32).write(writer)?;
            for &val in &self.data {
                writer.write_all(&val.to_be_bytes())?;
            }
        } else {
            VarInt(self.data.len() as i32).write(writer)?;
            for &val in &self.data {
                writer.write_all(&val.to_be_bytes())?;
            }
        }

        Ok(())
    }
}

fn pack_heightmap(heights: &[u16; 256]) -> Vec<i64> {
    let bits_per_value = 9;
    let values_per_long = 64 / bits_per_value;
    let num_longs = (256 + values_per_long - 1) / values_per_long;
    let mut data = vec![0i64; num_longs];

    for (i, &height) in heights.iter().enumerate() {
        let long_index = i / values_per_long;
        let bit_offset = (i % values_per_long) * bits_per_value;
        data[long_index] |= (height as i64) << bit_offset;
    }

    data
}

pub fn encode_chunk_data(chunk: &Chunk) -> io::Result<Packet> {
    let mut data = Vec::new();

    data.extend_from_slice(&chunk.pos.x.to_be_bytes());
    data.extend_from_slice(&chunk.pos.z.to_be_bytes());

    let mut motion_blocking = [0u16; 256];
    let mut world_surface = [0u16; 256];

    for x in 0..CHUNK_WIDTH {
        for z in 0..CHUNK_WIDTH {
            let idx = z * CHUNK_WIDTH + x;
            for abs_y in (0..SECTIONS_PER_CHUNK * SECTION_HEIGHT).rev() {
                let block = chunk.get_block(x, abs_y, z);
                if block != BlockState::AIR {
                    let height = (abs_y + 1) as u16;
                    if motion_blocking[idx] == 0 {
                        motion_blocking[idx] = height;
                    }
                    if world_surface[idx] == 0 {
                        world_surface[idx] = height;
                    }
                    break;
                }
            }
        }
    }

    let motion_blocking_packed = pack_heightmap(&motion_blocking);
    let world_surface_packed = pack_heightmap(&world_surface);

    nbt::write_unnamed_compound_start(&mut data)?;
    nbt::write_long_array(&mut data, "MOTION_BLOCKING", &motion_blocking_packed)?;
    nbt::write_long_array(&mut data, "WORLD_SURFACE", &world_surface_packed)?;
    nbt::write_compound_end(&mut data)?;

    let mut chunk_section_data = Vec::new();

    for section_idx in 0..SECTIONS_PER_CHUNK {
        let section = chunk.get_section(section_idx).unwrap();
        let block_count = section.non_air_count();
        chunk_section_data.extend_from_slice(&block_count.to_be_bytes());

        let block_palette = PalettedContainer::from_blocks(section.blocks());
        block_palette.write(&mut chunk_section_data)?;

        // Biomes: single-value palette (Plains = 1)
        chunk_section_data.push(0); // bits_per_entry = 0
        VarInt(1).write(&mut chunk_section_data)?; // Plains biome ID
        VarInt(0).write(&mut chunk_section_data)?; // 0 longs
    }

    VarInt(chunk_section_data.len() as i32).write(&mut data)?;
    data.extend_from_slice(&chunk_section_data);

    // Block entities (none)
    VarInt(0).write(&mut data)?;

    // Light data
    write_light_data(&mut data)?;

    Ok(Packet::new(0x27, data))
}

fn write_light_data(writer: &mut impl Write) -> io::Result<()> {
    let section_count = SECTIONS_PER_CHUNK + 2; // includes above and below

    // Trust edges
    writer.write_all(&[1])?;

    // Sky light mask: all sections present
    let mask_longs = (section_count + 63) / 64;
    VarInt(mask_longs as i32).write(writer)?;
    let mut mask = 0i64;
    for i in 0..section_count {
        mask |= 1i64 << i;
    }
    writer.write_all(&mask.to_be_bytes())?;

    // Block light mask: none
    VarInt(mask_longs as i32).write(writer)?;
    writer.write_all(&0i64.to_be_bytes())?;

    // Empty sky light mask: none
    VarInt(mask_longs as i32).write(writer)?;
    writer.write_all(&0i64.to_be_bytes())?;

    // Empty block light mask: all sections
    VarInt(mask_longs as i32).write(writer)?;
    writer.write_all(&mask.to_be_bytes())?;

    // Sky light arrays: full brightness (0xFF) for every section
    VarInt(section_count as i32).write(writer)?;
    let full_light = [0xFFu8; 2048];
    for _ in 0..section_count {
        VarInt(2048).write(writer)?;
        writer.write_all(&full_light)?;
    }

    // Block light arrays: none (matched by empty block light mask = all)
    VarInt(0).write(writer)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::chunk::{Chunk, ChunkPos};

    #[test]
    fn test_paletted_container_single_value() {
        let blocks = vec![BlockState::AIR; 4096];
        let container = PalettedContainer::from_blocks(&blocks);
        assert_eq!(container.bits_per_entry, 0);
        assert_eq!(container.palette.len(), 1);
        assert_eq!(container.palette[0], 0); // AIR = 0
        assert!(container.data.is_empty());
    }

    #[test]
    fn test_paletted_container_small_palette() {
        let mut blocks = vec![BlockState::AIR; 4096];
        blocks[0] = BlockState::STONE;
        blocks[1] = BlockState::DIRT;
        let container = PalettedContainer::from_blocks(&blocks);
        assert_eq!(container.bits_per_entry, 4);
        assert_eq!(container.palette.len(), 3); // AIR, STONE, DIRT
    }

    #[test]
    fn test_paletted_container_write_single_value() {
        let blocks = vec![BlockState::STONE; 4096];
        let container = PalettedContainer::from_blocks(&blocks);
        let mut buf = Vec::new();
        container.write(&mut buf).unwrap();

        assert_eq!(buf[0], 0); // bits_per_entry = 0
                               // Next bytes: VarInt for stone ID (1), then VarInt(0) for data length
    }

    #[test]
    fn test_heightmap_flat_world() {
        let chunk = Chunk::new_flat(ChunkPos::new(0, 0));
        let mut motion_blocking = [0u16; 256];
        for x in 0..CHUNK_WIDTH {
            for z in 0..CHUNK_WIDTH {
                let idx = z * CHUNK_WIDTH + x;
                for abs_y in (0..SECTIONS_PER_CHUNK * SECTION_HEIGHT).rev() {
                    let block = chunk.get_block(x, abs_y, z);
                    if block != BlockState::AIR {
                        motion_blocking[idx] = (abs_y + 1) as u16;
                        break;
                    }
                }
            }
        }

        // Flat world: grass at y=127 (abs), so height = 128
        for &h in &motion_blocking {
            assert_eq!(h, 128);
        }
    }

    #[test]
    fn test_chunk_data_packet_structure() {
        let chunk = Chunk::new_flat(ChunkPos::new(3, -2));
        let packet = encode_chunk_data(&chunk).unwrap();

        assert_eq!(packet.id, 0x27);
        assert!(!packet.data.is_empty());

        // First 8 bytes: chunk X and Z as i32 BE
        let x = i32::from_be_bytes(packet.data[0..4].try_into().unwrap());
        let z = i32::from_be_bytes(packet.data[4..8].try_into().unwrap());
        assert_eq!(x, 3);
        assert_eq!(z, -2);
    }

    #[test]
    fn test_pack_heightmap() {
        let mut heights = [0u16; 256];
        heights[0] = 128;
        heights[1] = 64;
        let packed = pack_heightmap(&heights);

        // 9 bits per value, 7 values per long, ceil(256/7) = 37 longs
        assert_eq!(packed.len(), 37);

        // Verify first value is packed correctly
        let val0 = packed[0] & 0x1FF;
        assert_eq!(val0, 128);
        let val1 = (packed[0] >> 9) & 0x1FF;
        assert_eq!(val1, 64);
    }
}
