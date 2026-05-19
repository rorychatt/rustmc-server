use super::nbt_encoder::json_to_nbt;
use crate::protocol::configuration::RegistryEntry;
use serde_json::Value;
use std::collections::HashMap;
use std::io;
use std::sync::LazyLock;

const DIMENSION_TYPE_JSON: &str = include_str!("../../data/registries/dimension_type.json");
const WORLDGEN_BIOME_JSON: &str = include_str!("../../data/registries/worldgen_biome.json");
const DAMAGE_TYPE_JSON: &str = include_str!("../../data/registries/damage_type.json");
const PAINTING_VARIANT_JSON: &str = include_str!("../../data/registries/painting_variant.json");
const WOLF_VARIANT_JSON: &str = include_str!("../../data/registries/wolf_variant.json");
const CHAT_TYPE_JSON: &str = include_str!("../../data/registries/chat_type.json");
const TRIM_MATERIAL_JSON: &str = include_str!("../../data/registries/trim_material.json");
const TRIM_PATTERN_JSON: &str = include_str!("../../data/registries/trim_pattern.json");
const BANNER_PATTERN_JSON: &str = include_str!("../../data/registries/banner_pattern.json");
const ENCHANTMENT_JSON: &str = include_str!("../../data/registries/enchantment.json");
const JUKEBOX_SONG_JSON: &str = include_str!("../../data/registries/jukebox_song.json");
const INSTRUMENT_JSON: &str = include_str!("../../data/registries/instrument.json");

const ALL_REGISTRY_JSON: &[(&str, &str)] = &[
    ("minecraft:dimension_type", DIMENSION_TYPE_JSON),
    ("minecraft:worldgen/biome", WORLDGEN_BIOME_JSON),
    ("minecraft:damage_type", DAMAGE_TYPE_JSON),
    ("minecraft:painting_variant", PAINTING_VARIANT_JSON),
    ("minecraft:wolf_variant", WOLF_VARIANT_JSON),
    ("minecraft:chat_type", CHAT_TYPE_JSON),
    ("minecraft:trim_material", TRIM_MATERIAL_JSON),
    ("minecraft:trim_pattern", TRIM_PATTERN_JSON),
    ("minecraft:banner_pattern", BANNER_PATTERN_JSON),
    ("minecraft:enchantment", ENCHANTMENT_JSON),
    ("minecraft:jukebox_song", JUKEBOX_SONG_JSON),
    ("minecraft:instrument", INSTRUMENT_JSON),
];

static REGISTRY_CACHE: LazyLock<HashMap<&'static str, Vec<RegistryEntry>>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for (id, json) in ALL_REGISTRY_JSON {
        let entries = parse_registry_json(json)
            .unwrap_or_else(|e| panic!("failed to parse registry {id}: {e}"));
        map.insert(*id, entries);
    }
    map
});

pub fn load_registry(registry_id: &str) -> io::Result<&'static [RegistryEntry]> {
    REGISTRY_CACHE
        .get(registry_id)
        .map(|v| v.as_slice())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("unknown registry: {registry_id}"),
            )
        })
}

fn parse_registry_json(json_str: &str) -> io::Result<Vec<RegistryEntry>> {
    let entries: Vec<Value> = serde_json::from_str(json_str).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("JSON parse error: {e}"))
    })?;

    entries
        .iter()
        .map(|entry| {
            let id = entry["id"]
                .as_str()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing 'id' field"))?
                .to_string();

            let data = &entry["data"];
            let nbt_data = json_to_nbt(data)?;

            Ok(RegistryEntry { id, nbt_data })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_REGISTRIES: &[&str] = &[
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

    #[test]
    fn test_load_all_registries() {
        for registry_id in ALL_REGISTRIES {
            let entries = load_registry(registry_id).unwrap_or_else(|e| {
                panic!("Failed to load {registry_id}: {e}");
            });
            assert!(
                !entries.is_empty(),
                "{registry_id} should have at least one entry"
            );
        }
    }

    #[test]
    fn test_registry_entry_counts() {
        assert_eq!(load_registry("minecraft:dimension_type").unwrap().len(), 4);
        assert!(load_registry("minecraft:worldgen/biome").unwrap().len() >= 50);
        assert!(load_registry("minecraft:damage_type").unwrap().len() >= 40);
        assert!(load_registry("minecraft:painting_variant").unwrap().len() >= 26);
        assert_eq!(load_registry("minecraft:wolf_variant").unwrap().len(), 9);
        assert_eq!(load_registry("minecraft:chat_type").unwrap().len(), 7);
        assert_eq!(load_registry("minecraft:trim_material").unwrap().len(), 10);
        assert!(load_registry("minecraft:trim_pattern").unwrap().len() >= 16);
        assert!(load_registry("minecraft:banner_pattern").unwrap().len() >= 40);
        assert!(load_registry("minecraft:enchantment").unwrap().len() >= 40);
        assert!(load_registry("minecraft:jukebox_song").unwrap().len() >= 15);
        assert_eq!(load_registry("minecraft:instrument").unwrap().len(), 8);
    }

    #[test]
    fn test_unknown_registry_returns_error() {
        assert!(load_registry("minecraft:nonexistent").is_err());
    }

    #[test]
    fn test_entries_have_valid_nbt() {
        let entries = load_registry("minecraft:dimension_type").unwrap();
        for entry in entries {
            assert_eq!(entry.nbt_data[0], 0x0A, "NBT must start with TAG_Compound");
            assert!(entry.nbt_data.len() > 3);
        }
    }

    #[test]
    fn test_cache_returns_same_reference() {
        let first = load_registry("minecraft:dimension_type").unwrap();
        let second = load_registry("minecraft:dimension_type").unwrap();
        assert!(std::ptr::eq(first, second));
    }
}
