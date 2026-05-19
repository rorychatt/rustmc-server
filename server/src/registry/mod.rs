mod loader;
pub mod nbt_encoder;

use crate::protocol::configuration::RegistryEntry;
use std::io;

pub const ALL_REGISTRY_IDS: &[&str] = &[
    "minecraft:dimension_type",
    "minecraft:worldgen/biome",
    "minecraft:damage_type",
    "minecraft:painting_variant",
    "minecraft:wolf_variant",
    "minecraft:chat_type",
    "minecraft:trim_material",
    "minecraft:trim_pattern",
    "minecraft:banner_pattern",
    "minecraft:enchantment",
    "minecraft:jukebox_song",
    "minecraft:instrument",
];

pub fn load(registry_id: &str) -> io::Result<Vec<RegistryEntry>> {
    loader::load_registry(registry_id)
}
