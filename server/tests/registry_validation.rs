use rustmc_server::protocol::version::PROTOCOL_VERSION;
use rustmc_server::registry;

const EXPECTED_COUNTS: &[(&str, usize)] = &[
    ("minecraft:dimension_type", 4),
    ("minecraft:worldgen/biome", 65),
    ("minecraft:damage_type", 49),
    ("minecraft:painting_variant", 50),
    ("minecraft:wolf_variant", 9),
    ("minecraft:chat_type", 7),
    ("minecraft:trim_material", 11),
    ("minecraft:trim_pattern", 18),
    ("minecraft:banner_pattern", 43),
    ("minecraft:enchantment", 42),
    ("minecraft:jukebox_song", 19),
    ("minecraft:instrument", 8),
];

#[test]
fn test_registry_entry_counts_match_generated() {
    for (registry_id, expected) in EXPECTED_COUNTS {
        let entries = registry::load(registry_id, PROTOCOL_VERSION).unwrap_or_else(|e| {
            panic!("Failed to load {registry_id}: {e}");
        });
        assert_eq!(
            entries.len(),
            *expected,
            "{registry_id}: expected {expected} entries but got {}",
            entries.len()
        );
    }
}

#[test]
fn test_registry_field_completeness() {
    let reg_set = registry::registry_set_for(PROTOCOL_VERSION);
    for registry_id in reg_set.registry_ids {
        let entries = registry::load(registry_id, PROTOCOL_VERSION).unwrap();
        for entry in &entries {
            assert!(
                !entry.id.is_empty(),
                "{registry_id}: entry has empty id"
            );
            assert!(
                entry.id.starts_with("minecraft:"),
                "{registry_id}: entry id '{}' missing minecraft: namespace",
                entry.id
            );
            assert_eq!(
                entry.nbt_data[0], 0x0A,
                "{registry_id}/{}: NBT must start with TAG_Compound (0x0A), got 0x{:02X}",
                entry.id, entry.nbt_data[0]
            );
            assert!(
                entry.nbt_data.len() > 3,
                "{registry_id}/{}: NBT data too short ({} bytes)",
                entry.id,
                entry.nbt_data.len()
            );
            let last_byte = entry.nbt_data[entry.nbt_data.len() - 1];
            assert_eq!(
                last_byte, 0x00,
                "{registry_id}/{}: NBT compound must end with TAG_End (0x00), got 0x{:02X}",
                entry.id, last_byte
            );
        }
    }
}

#[test]
fn test_registry_field_types() {
    let dimension_entries = registry::load("minecraft:dimension_type", PROTOCOL_VERSION).unwrap();
    for entry in &dimension_entries {
        assert!(
            entry.nbt_data.len() > 20,
            "dimension_type/{}: NBT data suspiciously short ({} bytes)",
            entry.id,
            entry.nbt_data.len()
        );
    }

    let biome_entries = registry::load("minecraft:worldgen/biome", PROTOCOL_VERSION).unwrap();
    for entry in &biome_entries {
        assert!(
            entry.nbt_data.len() > 10,
            "worldgen/biome/{}: NBT data suspiciously short ({} bytes)",
            entry.id,
            entry.nbt_data.len()
        );
    }

    let damage_entries = registry::load("minecraft:damage_type", PROTOCOL_VERSION).unwrap();
    for entry in &damage_entries {
        assert!(
            entry.nbt_data.len() > 10,
            "damage_type/{}: NBT data suspiciously short ({} bytes)",
            entry.id,
            entry.nbt_data.len()
        );
    }

    let enchantment_entries = registry::load("minecraft:enchantment", PROTOCOL_VERSION).unwrap();
    for entry in &enchantment_entries {
        assert!(
            entry.nbt_data.len() > 20,
            "enchantment/{}: NBT data suspiciously short ({} bytes)",
            entry.id,
            entry.nbt_data.len()
        );
    }
}
