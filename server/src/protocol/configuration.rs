use super::packet::Packet;
use super::types::{write_string, VarInt};
use std::io;

pub fn encode_known_packs() -> io::Result<Packet> {
    let mut data = Vec::new();
    VarInt(1).write(&mut data)?;
    write_string(&mut data, "minecraft")?;
    write_string(&mut data, "core")?;
    write_string(&mut data, "1.21")?;
    Ok(Packet::new(0x0E, data))
}

pub fn encode_registry_data(registry_id: &str, entries: &[RegistryEntry]) -> io::Result<Packet> {
    let mut data = Vec::new();
    write_string(&mut data, registry_id)?;
    VarInt(entries.len() as i32).write(&mut data)?;
    for entry in entries {
        write_string(&mut data, &entry.id)?;
        data.push(1);
        data.extend_from_slice(&entry.nbt_data);
    }
    Ok(Packet::new(0x07, data))
}

pub fn encode_update_tags() -> io::Result<Packet> {
    let mut data = Vec::new();
    VarInt(0).write(&mut data)?;
    Ok(Packet::new(0x0D, data))
}

pub fn encode_finish_configuration() -> Packet {
    Packet::new(0x03, Vec::new())
}

pub struct RegistryEntry {
    pub id: String,
    pub nbt_data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry;

    #[test]
    fn test_encode_known_packs() {
        let packet = encode_known_packs().unwrap();
        assert_eq!(packet.id, 0x0E);
        assert!(!packet.data.is_empty());
    }

    #[test]
    fn test_encode_finish_configuration() {
        let packet = encode_finish_configuration();
        assert_eq!(packet.id, 0x03);
        assert!(packet.data.is_empty());
    }

    #[test]
    fn test_encode_registry_data() {
        let entries = registry::load("minecraft:dimension_type").unwrap();
        let packet = encode_registry_data("minecraft:dimension_type", &entries).unwrap();
        assert_eq!(packet.id, 0x07);
        assert!(!packet.data.is_empty());
    }

    #[test]
    fn test_encode_update_tags() {
        let packet = encode_update_tags().unwrap();
        assert_eq!(packet.id, 0x0D);
    }

    #[test]
    fn test_dimension_type_registry() {
        let entries = registry::load("minecraft:dimension_type").unwrap();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].id, "minecraft:overworld");
    }

    #[test]
    fn test_biome_registry() {
        let entries = registry::load("minecraft:worldgen/biome").unwrap();
        assert!(entries.len() >= 50);
        assert!(entries.iter().any(|e| e.id == "minecraft:plains"));
    }

    #[test]
    fn test_damage_type_registry() {
        let entries = registry::load("minecraft:damage_type").unwrap();
        assert!(entries.len() >= 40);
    }

    #[test]
    fn test_painting_variant_registry() {
        let entries = registry::load("minecraft:painting_variant").unwrap();
        assert!(entries.len() >= 26);
        assert!(entries.iter().any(|e| e.id == "minecraft:kebab"));
    }

    #[test]
    fn test_wolf_variant_registry() {
        let entries = registry::load("minecraft:wolf_variant").unwrap();
        assert_eq!(entries.len(), 9);
        assert!(entries.iter().any(|e| e.id == "minecraft:pale"));
    }
}
