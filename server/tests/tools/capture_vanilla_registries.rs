//! Standalone tool to capture registry entry ordering and data from a vanilla Minecraft 1.21.4 server.
//!
//! Connects to a vanilla server, completes handshake -> login -> configuration,
//! captures all Registry Data packets (0x07), and writes the ordered entry IDs
//! to `server/tests/data/vanilla_registry_order.json`.
//!
//! Also supports capturing full NBT payloads decoded to JSON via `capture_vanilla_registry_data`.
//!
//! Supports servers with compression enabled (default `network-compression-threshold=256`).
//!
//! # Usage
//!
//! Start a vanilla 1.21.4 server with `online-mode=false`, then run:
//!
//! ```sh
//! VANILLA_HOST=127.0.0.1 VANILLA_PORT=25565 cargo test --test capture_vanilla_registries -- --ignored
//! ```
//!
//! Environment variables:
//! - `VANILLA_HOST` — server hostname (default: `127.0.0.1`)
//! - `VANILLA_PORT` — server port (default: `25565`)

use std::collections::HashMap;
use std::io;
use std::net::TcpStream;

use rustmc_server::protocol::packet::{Packet, PacketReader, PacketWriter};
use rustmc_server::registry::ALL_REGISTRY_IDS;

type RegistryEntry = (String, Option<serde_json::Value>);

const PROTOCOL_VERSION: i32 = 775;
const REGISTRY_DATA_PACKET_ID: i32 = 0x07;

const BOOLEAN_FIELDS: &[&str] = &[
    "active",
    "bed_works",
    "can_see_sky",
    "decal",
    "expected",
    "has_ceiling",
    "has_precipitation",
    "has_raids",
    "has_skylight",
    "is_direct",
    "is_flying",
    "is_on_ground",
    "italic",
    "natural",
    "piglin_safe",
    "replace_current_music",
    "respawn_anchor_works",
    "thundering",
    "ultrawarm",
];

const FLOAT_FIELDS: &[(&str, &str)] = &[
    ("", "temperature"),
    ("", "downfall"),
    ("", "creature_spawn_probability"),
    ("effects", "music_volume"),
    ("mood_sound", "offset"),
];

fn is_float_field(parent: &str, name: &str) -> bool {
    FLOAT_FIELDS.contains(&(parent, name))
}

fn write_varint(buf: &mut Vec<u8>, mut value: i32) {
    loop {
        let byte = (value & 0x7F) as u8;
        value = ((value as u32) >> 7) as i32;
        if value == 0 {
            buf.push(byte);
            break;
        }
        buf.push(byte | 0x80);
    }
}

fn read_varint_from_buf(buf: &[u8], offset: &mut usize) -> io::Result<i32> {
    let mut result: i32 = 0;
    let mut shift = 0;
    loop {
        if *offset >= buf.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "buffer too short",
            ));
        }
        let byte = buf[*offset];
        *offset += 1;
        result |= ((byte & 0x7F) as i32) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "VarInt too long",
            ));
        }
    }
    Ok(result)
}

fn read_string_from_buf(buf: &[u8], offset: &mut usize) -> io::Result<String> {
    let len = read_varint_from_buf(buf, offset)? as usize;
    if *offset + len > buf.len() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "string too long",
        ));
    }
    let s = String::from_utf8(buf[*offset..*offset + len].to_vec())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    *offset += len;
    Ok(s)
}

fn write_string(buf: &mut Vec<u8>, s: &str) {
    write_varint(buf, s.len() as i32);
    buf.extend_from_slice(s.as_bytes());
}

fn nbt_to_json(buf: &[u8], offset: &mut usize) -> io::Result<serde_json::Value> {
    if *offset >= buf.len() {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "NBT: no data"));
    }
    let tag_type = buf[*offset];
    *offset += 1;

    if tag_type == 0x00 {
        return Ok(serde_json::Value::Null);
    }

    // For network NBT (root compound without name), the root tag is 0x0A with an empty name
    // but in registry data packets, the root compound has tag type + empty name (2 bytes of 0)
    if tag_type == 0x0A {
        // Skip root name (2-byte length + name bytes)
        if *offset + 2 > buf.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "NBT root name",
            ));
        }
        let name_len = u16::from_be_bytes([buf[*offset], buf[*offset + 1]]) as usize;
        *offset += 2 + name_len;
        return decode_compound_payload(buf, offset, "");
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("Expected compound root tag, got: {tag_type:#x}"),
    ))
}

fn decode_nbt_payload(
    buf: &[u8],
    offset: &mut usize,
    tag_type: u8,
    parent: &str,
    field_name: &str,
) -> io::Result<serde_json::Value> {
    match tag_type {
        0x01 => {
            if *offset >= buf.len() {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "TAG_Byte"));
            }
            let val = buf[*offset] as i8;
            *offset += 1;
            if (val == 0 || val == 1) && BOOLEAN_FIELDS.contains(&field_name) {
                Ok(serde_json::Value::Bool(val != 0))
            } else {
                Ok(serde_json::Value::Number(serde_json::Number::from(
                    val as i64,
                )))
            }
        }
        0x02 => {
            if *offset + 2 > buf.len() {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "TAG_Short"));
            }
            let val = i16::from_be_bytes([buf[*offset], buf[*offset + 1]]);
            *offset += 2;
            Ok(serde_json::Value::Number(serde_json::Number::from(
                val as i64,
            )))
        }
        0x03 => {
            if *offset + 4 > buf.len() {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "TAG_Int"));
            }
            let val = i32::from_be_bytes([
                buf[*offset],
                buf[*offset + 1],
                buf[*offset + 2],
                buf[*offset + 3],
            ]);
            *offset += 4;
            Ok(serde_json::Value::Number(serde_json::Number::from(
                val as i64,
            )))
        }
        0x04 => {
            if *offset + 8 > buf.len() {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "TAG_Long"));
            }
            let val = i64::from_be_bytes([
                buf[*offset],
                buf[*offset + 1],
                buf[*offset + 2],
                buf[*offset + 3],
                buf[*offset + 4],
                buf[*offset + 5],
                buf[*offset + 6],
                buf[*offset + 7],
            ]);
            *offset += 8;
            Ok(serde_json::Value::Number(serde_json::Number::from(val)))
        }
        0x05 => {
            if *offset + 4 > buf.len() {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "TAG_Float"));
            }
            let val = f32::from_be_bytes([
                buf[*offset],
                buf[*offset + 1],
                buf[*offset + 2],
                buf[*offset + 3],
            ]);
            *offset += 4;
            let f64_val = val as f64;
            if f64_val.fract() == 0.0 && is_float_field(parent, field_name) {
                Ok(serde_json::Value::Number(serde_json::Number::from(
                    f64_val as i64,
                )))
            } else if let Some(n) = serde_json::Number::from_f64(f64_val) {
                Ok(serde_json::Value::Number(n))
            } else {
                Ok(serde_json::Value::Number(serde_json::Number::from(0)))
            }
        }
        0x06 => {
            if *offset + 8 > buf.len() {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "TAG_Double"));
            }
            let val = f64::from_be_bytes([
                buf[*offset],
                buf[*offset + 1],
                buf[*offset + 2],
                buf[*offset + 3],
                buf[*offset + 4],
                buf[*offset + 5],
                buf[*offset + 6],
                buf[*offset + 7],
            ]);
            *offset += 8;
            if let Some(n) = serde_json::Number::from_f64(val) {
                Ok(serde_json::Value::Number(n))
            } else {
                Ok(serde_json::Value::Number(serde_json::Number::from(0)))
            }
        }
        0x07 => {
            if *offset + 4 > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "TAG_ByteArray length",
                ));
            }
            let count = i32::from_be_bytes([
                buf[*offset],
                buf[*offset + 1],
                buf[*offset + 2],
                buf[*offset + 3],
            ]) as usize;
            *offset += 4;
            if *offset + count > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "TAG_ByteArray data",
                ));
            }
            let arr: Vec<serde_json::Value> = buf[*offset..*offset + count]
                .iter()
                .map(|&b| serde_json::Value::Number(serde_json::Number::from(b as i8 as i64)))
                .collect();
            *offset += count;
            Ok(serde_json::Value::Array(arr))
        }
        0x08 => {
            if *offset + 2 > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "TAG_String length",
                ));
            }
            let len = u16::from_be_bytes([buf[*offset], buf[*offset + 1]]) as usize;
            *offset += 2;
            if *offset + len > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "TAG_String data",
                ));
            }
            let s = String::from_utf8(buf[*offset..*offset + len].to_vec())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            *offset += len;
            Ok(serde_json::Value::String(s))
        }
        0x09 => {
            if *offset + 5 > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "TAG_List header",
                ));
            }
            let elem_type = buf[*offset];
            *offset += 1;
            let count = i32::from_be_bytes([
                buf[*offset],
                buf[*offset + 1],
                buf[*offset + 2],
                buf[*offset + 3],
            ]) as usize;
            *offset += 4;
            let mut arr = Vec::with_capacity(count);
            for _ in 0..count {
                arr.push(decode_nbt_payload(buf, offset, elem_type, field_name, "")?);
            }
            Ok(serde_json::Value::Array(arr))
        }
        0x0A => decode_compound_payload(buf, offset, field_name),
        0x0B => {
            if *offset + 4 > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "TAG_IntArray length",
                ));
            }
            let count = i32::from_be_bytes([
                buf[*offset],
                buf[*offset + 1],
                buf[*offset + 2],
                buf[*offset + 3],
            ]) as usize;
            *offset += 4;
            if *offset + count * 4 > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "TAG_IntArray data",
                ));
            }
            let mut arr = Vec::with_capacity(count);
            for i in 0..count {
                let base = *offset + i * 4;
                let val =
                    i32::from_be_bytes([buf[base], buf[base + 1], buf[base + 2], buf[base + 3]]);
                arr.push(serde_json::Value::Number(serde_json::Number::from(
                    val as i64,
                )));
            }
            *offset += count * 4;
            Ok(serde_json::Value::Array(arr))
        }
        0x0C => {
            if *offset + 4 > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "TAG_LongArray length",
                ));
            }
            let count = i32::from_be_bytes([
                buf[*offset],
                buf[*offset + 1],
                buf[*offset + 2],
                buf[*offset + 3],
            ]) as usize;
            *offset += 4;
            if *offset + count * 8 > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "TAG_LongArray data",
                ));
            }
            let mut arr = Vec::with_capacity(count);
            for i in 0..count {
                let base = *offset + i * 8;
                let val = i64::from_be_bytes([
                    buf[base],
                    buf[base + 1],
                    buf[base + 2],
                    buf[base + 3],
                    buf[base + 4],
                    buf[base + 5],
                    buf[base + 6],
                    buf[base + 7],
                ]);
                arr.push(serde_json::Value::Number(serde_json::Number::from(val)));
            }
            *offset += count * 8;
            Ok(serde_json::Value::Array(arr))
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Unknown NBT tag type: {tag_type:#x}"),
        )),
    }
}

fn decode_compound_payload(
    buf: &[u8],
    offset: &mut usize,
    parent: &str,
) -> io::Result<serde_json::Value> {
    let mut map = serde_json::Map::new();
    loop {
        if *offset >= buf.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "compound: expected tag type",
            ));
        }
        let child_type = buf[*offset];
        *offset += 1;
        if child_type == 0x00 {
            break;
        }
        if *offset + 2 > buf.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "compound: field name length",
            ));
        }
        let name_len = u16::from_be_bytes([buf[*offset], buf[*offset + 1]]) as usize;
        *offset += 2;
        if *offset + name_len > buf.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "compound: field name",
            ));
        }
        let name = String::from_utf8(buf[*offset..*offset + name_len].to_vec())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        *offset += name_len;
        let value = decode_nbt_payload(buf, offset, child_type, parent, &name)?;
        map.insert(name, value);
    }
    Ok(serde_json::Value::Object(map))
}


fn parse_registry_data_packet(data: &[u8]) -> io::Result<(String, Vec<RegistryEntry>)> {
    let mut offset = 0;
    let registry_id = read_string_from_buf(data, &mut offset)?;
    let entry_count = read_varint_from_buf(data, &mut offset)? as usize;
    let mut entries = Vec::with_capacity(entry_count);

    for _ in 0..entry_count {
        let entry_id = read_string_from_buf(data, &mut offset)?;

        if offset >= data.len() {
            entries.push((entry_id, None));
            break;
        }
        let has_data = data[offset] != 0;
        offset += 1;

        let nbt_value = if has_data {
            Some(nbt_to_json(data, &mut offset)?)
        } else {
            None
        };
        entries.push((entry_id, nbt_value));
    }

    Ok((registry_id, entries))
}

fn connect_and_capture_registries() -> HashMap<String, Vec<RegistryEntry>> {
    let host = std::env::var("VANILLA_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("VANILLA_PORT")
        .unwrap_or_else(|_| "25565".to_string())
        .parse()
        .expect("VANILLA_PORT must be a valid port number");

    let stream =
        TcpStream::connect(format!("{host}:{port}")).expect("Failed to connect to vanilla server");
    let mut reader = PacketReader::new(stream.try_clone().unwrap());
    let mut writer = PacketWriter::new(stream);

    // Handshake (state=2 for Login)
    let mut handshake_data = Vec::new();
    write_varint(&mut handshake_data, PROTOCOL_VERSION);
    write_string(&mut handshake_data, &host);
    handshake_data.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut handshake_data, 2); // Next state: Login
    writer
        .write_packet(&Packet::new(0x00, handshake_data))
        .unwrap();

    // Login Start
    let mut login_data = Vec::new();
    write_string(&mut login_data, "RegistryCapture");
    login_data.extend_from_slice(&[0u8; 16]); // UUID (all zeros for offline)
    writer.write_packet(&Packet::new(0x00, login_data)).unwrap();

    let mut registries: HashMap<String, Vec<RegistryEntry>> = HashMap::new();

    // Read packets until we get all registry data
    loop {
        let packet = reader.read_packet().unwrap();

        match packet.id {
            0x03 if registries.is_empty() => {
                // Set Compression (login phase, before any registries captured)
                let mut offset = 0;
                let threshold = read_varint_from_buf(&packet.data, &mut offset).unwrap();
                reader.set_compression_threshold(threshold);
                writer.set_compression_threshold(threshold);
                println!("Compression enabled with threshold {threshold}");
            }
            0x02 => {
                // Login Success — send Login Acknowledged
                writer.write_packet(&Packet::new(0x03, vec![])).unwrap();
            }
            REGISTRY_DATA_PACKET_ID => {
                let (reg_id, entries) = parse_registry_data_packet(&packet.data).unwrap();
                println!("Captured {}: {} entries", reg_id, entries.len());
                registries.insert(reg_id, entries);
            }
            0x03 => {
                // Finish Configuration — we have all registry data
                break;
            }
            0x0E => {
                // Known Packs — respond with empty known packs
                let mut response = Vec::new();
                write_varint(&mut response, 0); // 0 known packs
                writer.write_packet(&Packet::new(0x07, response)).unwrap();
            }
            _ => {}
        }
    }

    registries
}

#[test]
#[ignore]
fn capture_vanilla_registry_ordering() {
    let registries = connect_and_capture_registries();

    let mut ordering: HashMap<String, Vec<String>> = registries
        .iter()
        .filter(|(id, _)| ALL_REGISTRY_IDS.contains(&id.as_str()))
        .map(|(id, entries)| {
            (
                id.clone(),
                entries.iter().map(|(eid, _)| eid.clone()).collect(),
            )
        })
        .collect();

    println!(
        "Retained {} registries (filtered from vanilla set)",
        ordering.len()
    );

    // Filter entries within each registry to only those in our data files
    let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/registries/v775");
    let registry_to_file: HashMap<&str, &str> = [
        ("minecraft:banner_pattern", "banner_pattern.json"),
        ("minecraft:chat_type", "chat_type.json"),
        ("minecraft:damage_type", "damage_type.json"),
        ("minecraft:dimension_type", "dimension_type.json"),
        ("minecraft:enchantment", "enchantment.json"),
        ("minecraft:instrument", "instrument.json"),
        ("minecraft:jukebox_song", "jukebox_song.json"),
        ("minecraft:painting_variant", "painting_variant.json"),
        ("minecraft:trim_material", "trim_material.json"),
        ("minecraft:trim_pattern", "trim_pattern.json"),
        ("minecraft:wolf_variant", "wolf_variant.json"),
        ("minecraft:worldgen/biome", "worldgen_biome.json"),
        ("minecraft:cat_variant", "cat_variant.json"),
        ("minecraft:pig_sound_variant", "pig_sound_variant.json"),
        ("minecraft:wolf_sound_variant", "wolf_sound_variant.json"),
        ("minecraft:frog_variant", "frog_variant.json"),
        ("minecraft:pig_variant", "pig_variant.json"),
        ("minecraft:cat_sound_variant", "cat_sound_variant.json"),
        ("minecraft:cow_sound_variant", "cow_sound_variant.json"),
        (
            "minecraft:zombie_nautilus_variant",
            "zombie_nautilus_variant.json",
        ),
        ("minecraft:chicken_variant", "chicken_variant.json"),
        (
            "minecraft:chicken_sound_variant",
            "chicken_sound_variant.json",
        ),
        ("minecraft:cow_variant", "cow_variant.json"),
        ("minecraft:dialog", "dialog.json"),
        ("minecraft:world_clock", "world_clock.json"),
        ("minecraft:timeline", "timeline.json"),
        ("minecraft:test_environment", "test_environment.json"),
        ("minecraft:test_instance", "test_instance.json"),
    ]
    .into_iter()
    .collect();

    for (registry_id, entries) in ordering.iter_mut() {
        if let Some(&filename) = registry_to_file.get(registry_id.as_str()) {
            let file_path = data_dir.join(filename);
            let content = std::fs::read_to_string(&file_path).unwrap();
            let file_entries: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap();
            let our_ids: Vec<String> = file_entries
                .iter()
                .map(|e| e["id"].as_str().unwrap().to_string())
                .collect();
            entries.retain(|id| our_ids.contains(id));
        }
    }

    // Write to snapshot file
    let json = serde_json::to_string_pretty(&ordering).unwrap();
    let output_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/vanilla_registry_order.json");
    std::fs::write(&output_path, json).unwrap();
    println!("Written registry ordering to {}", output_path.display());
}

#[test]
#[ignore]
fn capture_vanilla_registry_data() {
    let registries = connect_and_capture_registries();

    let output_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/vanilla_registry_data");
    std::fs::create_dir_all(&output_dir).unwrap();

    for (registry_id, entries) in &registries {
        let filename = registry_id
            .strip_prefix("minecraft:")
            .unwrap_or(registry_id)
            .replace('/', "_")
            + ".json";

        let json_entries: Vec<serde_json::Value> = entries
            .iter()
            .map(|(id, data)| {
                let mut obj = serde_json::Map::new();
                obj.insert("id".to_string(), serde_json::Value::String(id.clone()));
                if let Some(d) = data {
                    obj.insert("data".to_string(), d.clone());
                }
                serde_json::Value::Object(obj)
            })
            .collect();

        // Write in compact format: one entry per line
        let mut output = String::from("[\n");
        for (i, entry) in json_entries.iter().enumerate() {
            output.push_str("  ");
            output.push_str(&serde_json::to_string(entry).unwrap());
            if i < json_entries.len() - 1 {
                output.push(',');
            }
            output.push('\n');
        }
        output.push_str("]\n");

        let file_path = output_dir.join(&filename);
        std::fs::write(&file_path, output).unwrap();
        println!("Written {filename} ({} entries)", entries.len());
    }

    println!(
        "Captured {} registries to {}",
        registries.len(),
        output_dir.display()
    );
}

fn rekey_entry(entry: &serde_json::Value) -> serde_json::Value {
    let obj = entry.as_object().unwrap();
    let mut new_obj = serde_json::Map::with_capacity(obj.len());
    if let Some(id) = obj.get("id") {
        new_obj.insert("id".to_string(), id.clone());
    }
    for (k, v) in obj {
        if k != "id" {
            new_obj.insert(k.clone(), v.clone());
        }
    }
    serde_json::Value::Object(new_obj)
}

#[test]
#[ignore]
fn reorder_registry_files_to_match_vanilla() {
    let snapshot: HashMap<String, Vec<String>> =
        serde_json::from_str(include_str!("../../tests/data/vanilla_registry_order.json")).unwrap();

    let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/registries/v775");

    let registry_to_file: HashMap<&str, &str> = [
        ("minecraft:banner_pattern", "banner_pattern.json"),
        ("minecraft:chat_type", "chat_type.json"),
        ("minecraft:damage_type", "damage_type.json"),
        ("minecraft:dimension_type", "dimension_type.json"),
        ("minecraft:enchantment", "enchantment.json"),
        ("minecraft:instrument", "instrument.json"),
        ("minecraft:jukebox_song", "jukebox_song.json"),
        ("minecraft:painting_variant", "painting_variant.json"),
        ("minecraft:trim_material", "trim_material.json"),
        ("minecraft:trim_pattern", "trim_pattern.json"),
        ("minecraft:wolf_variant", "wolf_variant.json"),
        ("minecraft:worldgen/biome", "worldgen_biome.json"),
        ("minecraft:cat_variant", "cat_variant.json"),
        ("minecraft:pig_sound_variant", "pig_sound_variant.json"),
        ("minecraft:wolf_sound_variant", "wolf_sound_variant.json"),
        ("minecraft:frog_variant", "frog_variant.json"),
        ("minecraft:pig_variant", "pig_variant.json"),
        ("minecraft:cat_sound_variant", "cat_sound_variant.json"),
        ("minecraft:cow_sound_variant", "cow_sound_variant.json"),
        (
            "minecraft:zombie_nautilus_variant",
            "zombie_nautilus_variant.json",
        ),
        ("minecraft:chicken_variant", "chicken_variant.json"),
        (
            "minecraft:chicken_sound_variant",
            "chicken_sound_variant.json",
        ),
        ("minecraft:cow_variant", "cow_variant.json"),
        ("minecraft:dialog", "dialog.json"),
        ("minecraft:world_clock", "world_clock.json"),
        ("minecraft:timeline", "timeline.json"),
        ("minecraft:test_environment", "test_environment.json"),
        ("minecraft:test_instance", "test_instance.json"),
    ]
    .into_iter()
    .collect();

    for (registry_id, expected_order) in &snapshot {
        let Some(&filename) = registry_to_file.get(registry_id.as_str()) else {
            println!("Skipping {registry_id}: no file mapping");
            continue;
        };

        let file_path = data_dir.join(filename);
        let content = std::fs::read_to_string(&file_path).unwrap();
        let entries: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap();

        let mut sorted = Vec::with_capacity(expected_order.len());
        for expected_id in expected_order {
            if let Some(entry) = entries
                .iter()
                .find(|e| e["id"].as_str() == Some(expected_id))
            {
                sorted.push(rekey_entry(entry));
            }
        }
        // Append any entries from our file that aren't in the vanilla snapshot
        for entry in &entries {
            let id = entry["id"].as_str().unwrap();
            if !expected_order.contains(&id.to_string()) {
                sorted.push(rekey_entry(entry));
            }
        }

        // Write in compact format: one JSON object per line
        let mut output = String::from("[\n");
        for (i, entry) in sorted.iter().enumerate() {
            output.push_str("  ");
            output.push_str(&serde_json::to_string(entry).unwrap());
            if i < sorted.len() - 1 {
                output.push(',');
            }
            output.push('\n');
        }
        output.push_str("]\n");
        std::fs::write(&file_path, output).unwrap();
        println!("Reordered {filename} ({} entries)", sorted.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustmc_server::registry::nbt_encoder::json_to_nbt;
    use serde_json::json;

    #[test]
    fn test_nbt_round_trip_simple_compound() {
        let original = json!({
            "name": "test",
            "value": 42
        });
        let nbt = json_to_nbt(&original).unwrap();
        let mut offset = 0;
        let decoded = nbt_to_json(&nbt, &mut offset).unwrap();
        assert_eq!(decoded["name"], "test");
        assert_eq!(decoded["value"], 42);
    }

    #[test]
    fn test_nbt_round_trip_nested() {
        let original = json!({
            "outer": {
                "inner": "hello"
            },
            "list": [1, 2, 3]
        });
        let nbt = json_to_nbt(&original).unwrap();
        let mut offset = 0;
        let decoded = nbt_to_json(&nbt, &mut offset).unwrap();
        assert_eq!(decoded["outer"]["inner"], "hello");
        assert_eq!(decoded["list"], json!([1, 2, 3]));
    }

    #[test]
    fn test_nbt_round_trip_booleans() {
        let original = json!({
            "has_precipitation": true,
            "ultrawarm": false
        });
        let nbt = json_to_nbt(&original).unwrap();
        let mut offset = 0;
        let decoded = nbt_to_json(&nbt, &mut offset).unwrap();
        assert_eq!(decoded["has_precipitation"], true);
        assert_eq!(decoded["ultrawarm"], false);
    }

    #[test]
    fn test_nbt_round_trip_float_fields() {
        let original = json!({
            "temperature": 0,
            "downfall": 1
        });
        let nbt = json_to_nbt(&original).unwrap();
        let mut offset = 0;
        let decoded = nbt_to_json(&nbt, &mut offset).unwrap();
        assert_eq!(decoded["temperature"], 0);
        assert_eq!(decoded["downfall"], 1);
    }

    #[test]
    fn test_nbt_round_trip_float_with_fraction() {
        let original = json!({
            "temperature": 0.5
        });
        let nbt = json_to_nbt(&original).unwrap();
        let mut offset = 0;
        let decoded = nbt_to_json(&nbt, &mut offset).unwrap();
        let temp = decoded["temperature"].as_f64().unwrap();
        assert!((temp - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_nbt_round_trip_empty_compound() {
        let original = json!({});
        let nbt = json_to_nbt(&original).unwrap();
        let mut offset = 0;
        let decoded = nbt_to_json(&nbt, &mut offset).unwrap();
        assert_eq!(decoded, json!({}));
    }

    #[test]
    fn test_nbt_round_trip_empty_list() {
        let original = json!({"items": []});
        let nbt = json_to_nbt(&original).unwrap();
        let mut offset = 0;
        let decoded = nbt_to_json(&nbt, &mut offset).unwrap();
        assert_eq!(decoded["items"], json!([]));
    }

    #[test]
    fn test_nbt_round_trip_string_values() {
        let original = json!({"message_id": "generic", "scaling": "never"});
        let nbt = json_to_nbt(&original).unwrap();
        let mut offset = 0;
        let decoded = nbt_to_json(&nbt, &mut offset).unwrap();
        assert_eq!(decoded["message_id"], "generic");
        assert_eq!(decoded["scaling"], "never");
    }

    #[test]
    fn test_nbt_round_trip_dimension_type() {
        let original = json!({
            "ambient_light": 0,
            "bed_works": true,
            "coordinate_scale": 1,
            "effects": "minecraft:overworld",
            "has_ceiling": false,
            "has_raids": true,
            "has_skylight": true,
            "height": 384,
            "infiniburn": "#minecraft:infiniburn_overworld",
            "logical_height": 384,
            "min_y": -64,
            "monster_spawn_block_light_limit": 0,
            "monster_spawn_light_level": {
                "max_inclusive": 7,
                "min_inclusive": 0,
                "type": "minecraft:uniform"
            },
            "natural": true,
            "piglin_safe": false,
            "respawn_anchor_works": false,
            "ultrawarm": false
        });
        let nbt = json_to_nbt(&original).unwrap();
        let mut offset = 0;
        let decoded = nbt_to_json(&nbt, &mut offset).unwrap();
        assert_eq!(decoded["bed_works"], true);
        assert_eq!(decoded["has_ceiling"], false);
        assert_eq!(decoded["height"], 384);
        assert_eq!(decoded["effects"], "minecraft:overworld");
        assert_eq!(decoded["monster_spawn_light_level"]["type"], "minecraft:uniform");
    }

    #[test]
    fn test_nbt_round_trip_nested_float() {
        let original = json!({
            "mood_sound": {
                "offset": 2
            }
        });
        let nbt = json_to_nbt(&original).unwrap();
        let mut offset = 0;
        let decoded = nbt_to_json(&nbt, &mut offset).unwrap();
        assert_eq!(decoded["mood_sound"]["offset"], 2);
    }

    #[test]
    fn test_parse_registry_data_packet_basic() {
        // Build a minimal registry data packet: registry_id + 1 entry with no data
        let mut data = Vec::new();
        write_string(&mut data, "minecraft:test");
        write_varint(&mut data, 1); // 1 entry
        write_string(&mut data, "minecraft:entry1");
        data.push(0); // has_data = false

        let (reg_id, entries) = parse_registry_data_packet(&data).unwrap();
        assert_eq!(reg_id, "minecraft:test");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "minecraft:entry1");
        assert!(entries[0].1.is_none());
    }

    #[test]
    fn test_parse_registry_data_packet_with_nbt() {
        let mut data = Vec::new();
        write_string(&mut data, "minecraft:dimension_type");
        write_varint(&mut data, 1); // 1 entry
        write_string(&mut data, "minecraft:overworld");
        data.push(1); // has_data = true

        let nbt_data = json_to_nbt(&json!({"height": 384, "natural": true})).unwrap();
        data.extend_from_slice(&nbt_data);

        let (reg_id, entries) = parse_registry_data_packet(&data).unwrap();
        assert_eq!(reg_id, "minecraft:dimension_type");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "minecraft:overworld");
        let entry_data = entries[0].1.as_ref().unwrap();
        assert_eq!(entry_data["height"], 384);
        assert_eq!(entry_data["natural"], true);
    }
}
