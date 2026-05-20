use super::nbt_encoder::json_to_nbt;
use crate::protocol::configuration::{encode_registry_data, RegistryEntry};
use crate::protocol::packet::Packet;
use crate::protocol::version::{ProtocolVersionError, SUPPORTED_VERSIONS};
use serde_json::Value;
use std::collections::HashMap;
use std::io;
use std::sync::LazyLock;

pub(crate) mod v775 {
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

#[derive(Debug)]
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
            _ => Err(ProtocolVersionError::UnsupportedVersion {
                requested: self.version,
                supported: SUPPORTED_VERSIONS,
            }
            .into()),
        }
    }
}

pub fn registry_set_for(protocol_version: i32) -> io::Result<&'static RegistrySet> {
    static V775_SET: RegistrySet = RegistrySet {
        registry_ids: v775::REGISTRY_IDS,
        version: 775,
    };

    match protocol_version {
        775 => Ok(&V775_SET),
        _ => Err(ProtocolVersionError::UnsupportedVersion {
            requested: protocol_version,
            supported: SUPPORTED_VERSIONS,
        }
        .into()),
    }
}

static ENTRY_CACHE: LazyLock<HashMap<(i32, &'static str), Vec<RegistryEntry>>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();
        for &version in SUPPORTED_VERSIONS {
            let set = registry_set_for(version).unwrap();
            for &reg_id in set.registry_ids {
                let entries = set.load(reg_id).unwrap();
                map.insert((version, reg_id), entries);
            }
        }
        map
    });

static PACKET_CACHE: LazyLock<HashMap<i32, Vec<Packet>>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for &version in SUPPORTED_VERSIONS {
        let set = registry_set_for(version).unwrap();
        let mut packets = Vec::new();
        for &reg_id in set.registry_ids {
            let entries = load_registry(reg_id, version).unwrap();
            let packet = encode_registry_data(reg_id, &entries).unwrap();
            packets.push(packet);
        }
        map.insert(version, packets);
    }
    map
});

pub fn load_registry(registry_id: &str, protocol_version: i32) -> io::Result<Vec<RegistryEntry>> {
    let set = registry_set_for(protocol_version)?;
    for &known_id in set.registry_ids {
        if known_id == registry_id {
            if let Some(entries) = ENTRY_CACHE.get(&(protocol_version, known_id)) {
                return Ok(entries.clone());
            }
        }
    }
    set.load(registry_id)
}

pub fn cached_registry_packets(protocol_version: i32) -> io::Result<&'static [Packet]> {
    registry_set_for(protocol_version)?;
    PACKET_CACHE
        .get(&protocol_version)
        .map(|v| v.as_slice())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("no cached packets for protocol version: {protocol_version}"),
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
    use crate::protocol::version::{ProtocolVersionError, PROTOCOL_VERSION, SUPPORTED_VERSIONS};

    #[test]
    fn test_load_registry_with_protocol_version() {
        let entries = load_registry("minecraft:dimension_type", 775).unwrap();
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_registry_set_for_unknown_returns_error() {
        let result = registry_set_for(999);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
    }

    #[test]
    fn test_registry_ids_for_775() {
        let set = registry_set_for(775).unwrap();
        assert_eq!(set.registry_ids.len(), 12);
        assert!(set.registry_ids.contains(&"minecraft:dimension_type"));
        assert!(set.registry_ids.contains(&"minecraft:worldgen/biome"));
        assert!(set.registry_ids.contains(&"minecraft:instrument"));
    }

    #[test]
    fn test_load_all_registries() {
        let set = registry_set_for(PROTOCOL_VERSION).unwrap();
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

    #[test]
    fn test_cached_registry_packets_returns_correct_count() {
        let packets = cached_registry_packets(775).unwrap();
        let set = registry_set_for(775).unwrap();
        assert_eq!(packets.len(), set.registry_ids.len());
    }

    #[test]
    fn test_cached_registry_packets_same_reference() {
        let first = cached_registry_packets(775).unwrap();
        let second = cached_registry_packets(775).unwrap();
        assert!(std::ptr::eq(first, second));
    }

    #[test]
    fn test_cached_registry_packets_unknown_version() {
        let result = cached_registry_packets(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_all_supported_versions_have_cached_packets() {
        for &version in SUPPORTED_VERSIONS {
            let packets = cached_registry_packets(version).unwrap_or_else(|e| {
                panic!("cached_registry_packets({version}) failed: {e}");
            });
            assert!(
                !packets.is_empty(),
                "version {version} should have cached packets"
            );
        }
    }

    #[test]
    fn test_all_supported_versions_have_entry_cache() {
        for &version in SUPPORTED_VERSIONS {
            let set = registry_set_for(version).unwrap();
            for &reg_id in set.registry_ids {
                let entries = ENTRY_CACHE.get(&(version, reg_id)).unwrap_or_else(|| {
                    panic!("ENTRY_CACHE missing ({version}, {reg_id})");
                });
                assert!(
                    !entries.is_empty(),
                    "({version}, {reg_id}) should have cached entries"
                );
            }
        }
    }

    #[test]
    fn test_registry_set_for_error_downcast() {
        let result = registry_set_for(999);
        let err = result.unwrap_err();
        let source = err.get_ref().unwrap();
        let pve = source.downcast_ref::<ProtocolVersionError>().unwrap();
        assert_eq!(
            *pve,
            ProtocolVersionError::UnsupportedVersion {
                requested: 999,
                supported: SUPPORTED_VERSIONS,
            }
        );
    }
}
