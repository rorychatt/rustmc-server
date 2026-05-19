use super::nbt_encoder::json_to_nbt;
use crate::protocol::configuration::RegistryEntry;
use serde_json::Value;
use std::io;
use tracing::warn;

mod v775 {
    pub const DIMENSION_TYPE_JSON: &str =
        include_str!("../../data/registries/v775/dimension_type.json");
    pub const WORLDGEN_BIOME_JSON: &str =
        include_str!("../../data/registries/v775/worldgen_biome.json");
    pub const DAMAGE_TYPE_JSON: &str = include_str!("../../data/registries/v775/damage_type.json");
    pub const PAINTING_VARIANT_JSON: &str =
        include_str!("../../data/registries/v775/painting_variant.json");
    pub const WOLF_VARIANT_JSON: &str =
        include_str!("../../data/registries/v775/wolf_variant.json");
    pub const CHAT_TYPE_JSON: &str = include_str!("../../data/registries/v775/chat_type.json");
    pub const TRIM_MATERIAL_JSON: &str =
        include_str!("../../data/registries/v775/trim_material.json");
    pub const TRIM_PATTERN_JSON: &str =
        include_str!("../../data/registries/v775/trim_pattern.json");
    pub const BANNER_PATTERN_JSON: &str =
        include_str!("../../data/registries/v775/banner_pattern.json");
    pub const ENCHANTMENT_JSON: &str = include_str!("../../data/registries/v775/enchantment.json");
    pub const JUKEBOX_SONG_JSON: &str =
        include_str!("../../data/registries/v775/jukebox_song.json");
    pub const INSTRUMENT_JSON: &str = include_str!("../../data/registries/v775/instrument.json");

    pub const REGISTRY_IDS: &[&str] = &[
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
}

pub struct RegistrySet {
    pub registry_ids: &'static [&'static str],
    version: i32,
}

impl RegistrySet {
    pub fn load(&self, registry_id: &str) -> io::Result<Vec<RegistryEntry>> {
        let json_str = self.get_json(registry_id)?;
        parse_registry_json(json_str)
    }

    fn get_json(&self, registry_id: &str) -> io::Result<&'static str> {
        match self.version {
            775 => match registry_id {
                "minecraft:dimension_type" => Ok(v775::DIMENSION_TYPE_JSON),
                "minecraft:worldgen/biome" => Ok(v775::WORLDGEN_BIOME_JSON),
                "minecraft:damage_type" => Ok(v775::DAMAGE_TYPE_JSON),
                "minecraft:painting_variant" => Ok(v775::PAINTING_VARIANT_JSON),
                "minecraft:wolf_variant" => Ok(v775::WOLF_VARIANT_JSON),
                "minecraft:chat_type" => Ok(v775::CHAT_TYPE_JSON),
                "minecraft:trim_material" => Ok(v775::TRIM_MATERIAL_JSON),
                "minecraft:trim_pattern" => Ok(v775::TRIM_PATTERN_JSON),
                "minecraft:banner_pattern" => Ok(v775::BANNER_PATTERN_JSON),
                "minecraft:enchantment" => Ok(v775::ENCHANTMENT_JSON),
                "minecraft:jukebox_song" => Ok(v775::JUKEBOX_SONG_JSON),
                "minecraft:instrument" => Ok(v775::INSTRUMENT_JSON),
                _ => Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("unknown registry: {registry_id}"),
                )),
            },
            _ => Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("unsupported protocol version: {}", self.version),
            )),
        }
    }
}

pub fn registry_set_for(protocol_version: i32) -> &'static RegistrySet {
    static V775_SET: RegistrySet = RegistrySet {
        registry_ids: v775::REGISTRY_IDS,
        version: 775,
    };

    match protocol_version {
        775 => &V775_SET,
        _ => {
            warn!(
                "Unsupported protocol version {protocol_version}, falling back to registry set for version 775"
            );
            &V775_SET
        }
    }
}

pub fn load_registry(registry_id: &str, protocol_version: i32) -> io::Result<Vec<RegistryEntry>> {
    let set = registry_set_for(protocol_version);
    set.load(registry_id)
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
    use crate::protocol::version::PROTOCOL_VERSION;

    #[test]
    fn test_load_registry_with_protocol_version() {
        let entries = load_registry("minecraft:dimension_type", 775).unwrap();
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_registry_set_for_unknown_falls_back() {
        let set = registry_set_for(999);
        assert_eq!(set.registry_ids, registry_set_for(775).registry_ids);
    }

    #[test]
    fn test_registry_ids_for_775() {
        let set = registry_set_for(775);
        assert_eq!(set.registry_ids.len(), 12);
        assert!(set.registry_ids.contains(&"minecraft:dimension_type"));
        assert!(set.registry_ids.contains(&"minecraft:worldgen/biome"));
        assert!(set.registry_ids.contains(&"minecraft:instrument"));
    }

    #[test]
    fn test_load_all_registries() {
        let set = registry_set_for(PROTOCOL_VERSION);
        for registry_id in set.registry_ids {
            let entries = load_registry(registry_id, PROTOCOL_VERSION).unwrap_or_else(|e| {
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
        assert_eq!(
            load_registry("minecraft:dimension_type", 775)
                .unwrap()
                .len(),
            4
        );
        assert_eq!(
            load_registry("minecraft:worldgen/biome", 775)
                .unwrap()
                .len(),
            65
        );
        assert_eq!(
            load_registry("minecraft:damage_type", 775).unwrap().len(),
            49
        );
        assert_eq!(
            load_registry("minecraft:painting_variant", 775)
                .unwrap()
                .len(),
            50
        );
        assert_eq!(
            load_registry("minecraft:wolf_variant", 775).unwrap().len(),
            9
        );
        assert_eq!(load_registry("minecraft:chat_type", 775).unwrap().len(), 7);
        assert_eq!(
            load_registry("minecraft:trim_material", 775).unwrap().len(),
            11
        );
        assert_eq!(
            load_registry("minecraft:trim_pattern", 775).unwrap().len(),
            18
        );
        assert_eq!(
            load_registry("minecraft:banner_pattern", 775)
                .unwrap()
                .len(),
            43
        );
        assert_eq!(
            load_registry("minecraft:enchantment", 775).unwrap().len(),
            42
        );
        assert_eq!(
            load_registry("minecraft:jukebox_song", 775).unwrap().len(),
            19
        );
        assert_eq!(load_registry("minecraft:instrument", 775).unwrap().len(), 8);
    }

    #[test]
    fn test_unknown_registry_returns_error() {
        assert!(load_registry("minecraft:nonexistent", 775).is_err());
    }

    #[test]
    fn test_entries_have_valid_nbt() {
        let entries = load_registry("minecraft:dimension_type", 775).unwrap();
        for entry in &entries {
            assert_eq!(entry.nbt_data[0], 0x0A, "NBT must start with TAG_Compound");
            assert!(entry.nbt_data.len() > 3);
        }
    }
}
