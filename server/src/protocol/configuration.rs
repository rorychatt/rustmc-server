use super::packet::Packet;
use super::types::{write_string, VarInt};
use std::io::{self, Write};

pub fn encode_known_packs() -> io::Result<Packet> {
    let mut data = Vec::new();
    VarInt(1).write(&mut data)?; // Known pack count
    write_string(&mut data, "minecraft")?; // Namespace
    write_string(&mut data, "core")?; // ID
    write_string(&mut data, "1.21")?; // Version
    Ok(Packet::new(0x0E, data))
}

pub fn encode_registry_data(registry_id: &str, entries: &[RegistryEntry]) -> io::Result<Packet> {
    let mut data = Vec::new();
    write_string(&mut data, registry_id)?;
    VarInt(entries.len() as i32).write(&mut data)?;
    for entry in entries {
        write_string(&mut data, &entry.id)?;
        data.push(1); // Has data
        data.extend_from_slice(&entry.nbt_data);
    }
    Ok(Packet::new(0x07, data))
}

pub fn encode_update_tags() -> io::Result<Packet> {
    let mut data = Vec::new();
    VarInt(0).write(&mut data)?; // Zero tag registries (minimal)
    Ok(Packet::new(0x0D, data))
}

pub fn encode_finish_configuration() -> Packet {
    Packet::new(0x03, Vec::new())
}

pub struct RegistryEntry {
    pub id: String,
    pub nbt_data: Vec<u8>,
}

pub fn dimension_type_registry() -> io::Result<Vec<RegistryEntry>> {
    let nbt_data = encode_dimension_type_nbt()?;
    Ok(vec![RegistryEntry {
        id: "minecraft:overworld".to_string(),
        nbt_data,
    }])
}

pub fn biome_registry() -> io::Result<Vec<RegistryEntry>> {
    let nbt_data = encode_biome_nbt()?;
    Ok(vec![RegistryEntry {
        id: "minecraft:plains".to_string(),
        nbt_data,
    }])
}

pub fn damage_type_registry() -> io::Result<Vec<RegistryEntry>> {
    let entries = vec![
        ("minecraft:generic", encode_damage_type_nbt("generic")?),
        (
            "minecraft:generic_kill",
            encode_damage_type_nbt("generic_kill")?,
        ),
        (
            "minecraft:player_attack",
            encode_damage_type_nbt("player_attack")?,
        ),
    ];
    Ok(entries
        .into_iter()
        .map(|(id, nbt_data)| RegistryEntry {
            id: id.to_string(),
            nbt_data,
        })
        .collect())
}

pub fn painting_variant_registry() -> io::Result<Vec<RegistryEntry>> {
    let nbt_data = encode_painting_variant_nbt()?;
    Ok(vec![RegistryEntry {
        id: "minecraft:kebab".to_string(),
        nbt_data,
    }])
}

pub fn wolf_variant_registry() -> io::Result<Vec<RegistryEntry>> {
    let nbt_data = encode_wolf_variant_nbt()?;
    Ok(vec![RegistryEntry {
        id: "minecraft:pale".to_string(),
        nbt_data,
    }])
}

fn encode_dimension_type_nbt() -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    // TAG_Compound (root, no name for registry entry inline NBT)
    data.push(0x0A);
    data.extend_from_slice(&0u16.to_be_bytes()); // empty name

    write_nbt_byte(&mut data, "has_skylight", 1)?;
    write_nbt_byte(&mut data, "has_ceiling", 0)?;
    write_nbt_byte(&mut data, "ultrawarm", 0)?;
    write_nbt_byte(&mut data, "natural", 1)?;
    write_nbt_double(&mut data, "coordinate_scale", 1.0)?;
    write_nbt_byte(&mut data, "bed_works", 1)?;
    write_nbt_byte(&mut data, "respawn_anchor_works", 0)?;
    write_nbt_int(&mut data, "min_y", -64)?;
    write_nbt_int(&mut data, "height", 384)?;
    write_nbt_int(&mut data, "logical_height", 384)?;
    write_nbt_string(&mut data, "infiniburn", "#minecraft:infiniburn_overworld")?;
    write_nbt_string(&mut data, "effects", "minecraft:overworld")?;
    write_nbt_float(&mut data, "ambient_light", 0.0)?;
    write_nbt_byte(&mut data, "piglin_safe", 0)?;
    write_nbt_byte(&mut data, "has_raids", 1)?;
    write_nbt_int(&mut data, "monster_spawn_light_level", 0)?;
    write_nbt_int(&mut data, "monster_spawn_block_light_limit", 0)?;

    data.push(0x00); // TAG_End
    Ok(data)
}

fn encode_biome_nbt() -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    // TAG_Compound root
    data.push(0x0A);
    data.extend_from_slice(&0u16.to_be_bytes());

    write_nbt_byte(&mut data, "has_precipitation", 1)?;
    write_nbt_float(&mut data, "temperature", 0.8)?;
    write_nbt_float(&mut data, "downfall", 0.4)?;

    // effects compound
    write_nbt_compound_start(&mut data, "effects")?;
    write_nbt_int(&mut data, "sky_color", 7907327)?;
    write_nbt_int(&mut data, "water_fog_color", 329011)?;
    write_nbt_int(&mut data, "fog_color", 12638463)?;
    write_nbt_int(&mut data, "water_color", 4159204)?;
    data.push(0x00); // TAG_End (effects)

    data.push(0x00); // TAG_End (root)
    Ok(data)
}

fn encode_damage_type_nbt(name: &str) -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    data.push(0x0A);
    data.extend_from_slice(&0u16.to_be_bytes());

    write_nbt_string(&mut data, "message_id", name)?;
    write_nbt_string(&mut data, "scaling", "never")?;
    write_nbt_float(&mut data, "exhaustion", 0.0)?;

    data.push(0x00); // TAG_End
    Ok(data)
}

fn encode_painting_variant_nbt() -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    data.push(0x0A);
    data.extend_from_slice(&0u16.to_be_bytes());

    write_nbt_string(&mut data, "asset_id", "minecraft:kebab")?;
    write_nbt_int(&mut data, "width", 1)?;
    write_nbt_int(&mut data, "height", 1)?;

    data.push(0x00);
    Ok(data)
}

fn encode_wolf_variant_nbt() -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    data.push(0x0A);
    data.extend_from_slice(&0u16.to_be_bytes());

    write_nbt_string(&mut data, "wild_texture", "minecraft:entity/wolf/wolf")?;
    write_nbt_string(&mut data, "tame_texture", "minecraft:entity/wolf/wolf_tame")?;
    write_nbt_string(
        &mut data,
        "angry_texture",
        "minecraft:entity/wolf/wolf_angry",
    )?;
    write_nbt_string(&mut data, "biomes", "minecraft:plains")?;

    data.push(0x00);
    Ok(data)
}

fn write_nbt_byte(writer: &mut impl Write, name: &str, value: i8) -> io::Result<()> {
    writer.write_all(&[0x01])?; // TAG_Byte
    writer.write_all(&(name.len() as u16).to_be_bytes())?;
    writer.write_all(name.as_bytes())?;
    writer.write_all(&[value as u8])?;
    Ok(())
}

fn write_nbt_int(writer: &mut impl Write, name: &str, value: i32) -> io::Result<()> {
    writer.write_all(&[0x03])?; // TAG_Int
    writer.write_all(&(name.len() as u16).to_be_bytes())?;
    writer.write_all(name.as_bytes())?;
    writer.write_all(&value.to_be_bytes())?;
    Ok(())
}

fn write_nbt_float(writer: &mut impl Write, name: &str, value: f32) -> io::Result<()> {
    writer.write_all(&[0x05])?; // TAG_Float
    writer.write_all(&(name.len() as u16).to_be_bytes())?;
    writer.write_all(name.as_bytes())?;
    writer.write_all(&value.to_be_bytes())?;
    Ok(())
}

fn write_nbt_double(writer: &mut impl Write, name: &str, value: f64) -> io::Result<()> {
    writer.write_all(&[0x06])?; // TAG_Double
    writer.write_all(&(name.len() as u16).to_be_bytes())?;
    writer.write_all(name.as_bytes())?;
    writer.write_all(&value.to_be_bytes())?;
    Ok(())
}

fn write_nbt_string(writer: &mut impl Write, name: &str, value: &str) -> io::Result<()> {
    writer.write_all(&[0x08])?; // TAG_String
    writer.write_all(&(name.len() as u16).to_be_bytes())?;
    writer.write_all(name.as_bytes())?;
    writer.write_all(&(value.len() as u16).to_be_bytes())?;
    writer.write_all(value.as_bytes())?;
    Ok(())
}

fn write_nbt_compound_start(writer: &mut impl Write, name: &str) -> io::Result<()> {
    writer.write_all(&[0x0A])?; // TAG_Compound
    writer.write_all(&(name.len() as u16).to_be_bytes())?;
    writer.write_all(name.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let entries = dimension_type_registry().unwrap();
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
    fn test_biome_registry() {
        let entries = biome_registry().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "minecraft:plains");
    }

    #[test]
    fn test_damage_type_registry() {
        let entries = damage_type_registry().unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn test_painting_variant_registry() {
        let entries = painting_variant_registry().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "minecraft:kebab");
    }

    #[test]
    fn test_wolf_variant_registry() {
        let entries = wolf_variant_registry().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "minecraft:pale");
    }
}
