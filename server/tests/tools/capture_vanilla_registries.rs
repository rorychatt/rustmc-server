//! Standalone tool to capture registry entry ordering from a vanilla Minecraft 1.21.4 server.
//!
//! Connects to a vanilla server, completes handshake → login → configuration,
//! captures all Registry Data packets (0x07), and writes the ordered entry IDs
//! to `server/tests/data/vanilla_registry_order.json`.
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
use std::io::{self, Read, Write};
use std::net::TcpStream;

use rustmc_server::registry::ALL_REGISTRY_IDS;

const PROTOCOL_VERSION: i32 = 775;
const REGISTRY_DATA_PACKET_ID: i32 = 0x07;

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

fn read_varint(stream: &mut TcpStream) -> io::Result<i32> {
    let mut result: i32 = 0;
    let mut shift = 0;
    loop {
        let mut byte = [0u8; 1];
        stream.read_exact(&mut byte)?;
        result |= ((byte[0] & 0x7F) as i32) << shift;
        if byte[0] & 0x80 == 0 {
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

fn send_packet(stream: &mut TcpStream, packet_id: i32, data: &[u8]) -> io::Result<()> {
    let mut id_buf = Vec::new();
    write_varint(&mut id_buf, packet_id);

    let total_len = id_buf.len() + data.len();
    let mut len_buf = Vec::new();
    write_varint(&mut len_buf, total_len as i32);

    stream.write_all(&len_buf)?;
    stream.write_all(&id_buf)?;
    stream.write_all(data)?;
    stream.flush()
}

fn read_packet(stream: &mut TcpStream) -> io::Result<(i32, Vec<u8>)> {
    let length = read_varint(stream)?;
    let mut buf = vec![0u8; length as usize];
    stream.read_exact(&mut buf)?;
    let mut offset = 0;
    let packet_id = read_varint_from_buf(&buf, &mut offset)?;
    Ok((packet_id, buf[offset..].to_vec()))
}

fn skip_nbt(buf: &[u8], offset: &mut usize) -> io::Result<()> {
    if *offset >= buf.len() {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "NBT: no data"));
    }
    let tag_type = buf[*offset];
    *offset += 1;

    if tag_type == 0x00 {
        return Ok(());
    }

    skip_nbt_payload(buf, offset, tag_type)
}

fn skip_nbt_payload(buf: &[u8], offset: &mut usize, tag_type: u8) -> io::Result<()> {
    match tag_type {
        0x00 => Ok(()),
        0x01 => {
            *offset += 1;
            Ok(())
        }
        0x02 => {
            *offset += 2;
            Ok(())
        }
        0x03 => {
            *offset += 4;
            Ok(())
        }
        0x04 => {
            *offset += 8;
            Ok(())
        }
        0x05 => {
            *offset += 4;
            Ok(())
        }
        0x06 => {
            *offset += 8;
            Ok(())
        }
        0x07 => {
            if *offset + 4 > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "NBT byte array",
                ));
            }
            let len = i32::from_be_bytes([
                buf[*offset],
                buf[*offset + 1],
                buf[*offset + 2],
                buf[*offset + 3],
            ]) as usize;
            *offset += 4 + len;
            Ok(())
        }
        0x08 => {
            if *offset + 2 > buf.len() {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "NBT string"));
            }
            let len = u16::from_be_bytes([buf[*offset], buf[*offset + 1]]) as usize;
            *offset += 2 + len;
            Ok(())
        }
        0x09 => {
            if *offset + 5 > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "NBT list header",
                ));
            }
            let elem_type = buf[*offset];
            *offset += 1;
            let count = i32::from_be_bytes([
                buf[*offset],
                buf[*offset + 1],
                buf[*offset + 2],
                buf[*offset + 3],
            ]);
            *offset += 4;
            for _ in 0..count {
                skip_nbt_payload(buf, offset, elem_type)?;
            }
            Ok(())
        }
        0x0A => {
            loop {
                if *offset >= buf.len() {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "NBT compound"));
                }
                let child_type = buf[*offset];
                *offset += 1;
                if child_type == 0x00 {
                    break;
                }
                if *offset + 2 > buf.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "NBT compound name",
                    ));
                }
                let name_len = u16::from_be_bytes([buf[*offset], buf[*offset + 1]]) as usize;
                *offset += 2 + name_len;
                skip_nbt_payload(buf, offset, child_type)?;
            }
            Ok(())
        }
        0x0B => {
            if *offset + 4 > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "NBT int array",
                ));
            }
            let count = i32::from_be_bytes([
                buf[*offset],
                buf[*offset + 1],
                buf[*offset + 2],
                buf[*offset + 3],
            ]) as usize;
            *offset += 4 + count * 4;
            Ok(())
        }
        0x0C => {
            if *offset + 4 > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "NBT long array",
                ));
            }
            let count = i32::from_be_bytes([
                buf[*offset],
                buf[*offset + 1],
                buf[*offset + 2],
                buf[*offset + 3],
            ]) as usize;
            *offset += 4 + count * 8;
            Ok(())
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Unknown NBT tag type: {tag_type}"),
        )),
    }
}

fn parse_registry_data_packet(data: &[u8]) -> io::Result<(String, Vec<String>)> {
    let mut offset = 0;
    let registry_id = read_string_from_buf(data, &mut offset)?;
    let entry_count = read_varint_from_buf(data, &mut offset)? as usize;
    let mut entries = Vec::with_capacity(entry_count);

    for _ in 0..entry_count {
        let entry_id = read_string_from_buf(data, &mut offset)?;
        entries.push(entry_id);

        if offset >= data.len() {
            break;
        }
        let has_data = data[offset] != 0;
        offset += 1;

        if has_data {
            skip_nbt(data, &mut offset)?;
        }
    }

    Ok((registry_id, entries))
}

#[test]
#[ignore]
fn capture_vanilla_registry_ordering() {
    let host = std::env::var("VANILLA_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("VANILLA_PORT")
        .unwrap_or_else(|_| "25565".to_string())
        .parse()
        .expect("VANILLA_PORT must be a valid port number");

    let mut stream =
        TcpStream::connect(format!("{host}:{port}")).expect("Failed to connect to vanilla server");

    // Handshake (state=2 for Login)
    let mut handshake_data = Vec::new();
    write_varint(&mut handshake_data, PROTOCOL_VERSION);
    write_string(&mut handshake_data, &host);
    handshake_data.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut handshake_data, 2); // Next state: Login
    send_packet(&mut stream, 0x00, &handshake_data).unwrap();

    // Login Start
    let mut login_data = Vec::new();
    write_string(&mut login_data, "RegistryCapture");
    login_data.extend_from_slice(&[0u8; 16]); // UUID (all zeros for offline)
    send_packet(&mut stream, 0x00, &login_data).unwrap();

    let mut registries: HashMap<String, Vec<String>> = HashMap::new();

    // Read packets until we get all registry data
    loop {
        let (packet_id, data) = read_packet(&mut stream).unwrap();

        match packet_id {
            0x02 => {
                // Login Success — send Login Acknowledged
                send_packet(&mut stream, 0x03, &[]).unwrap();
            }
            REGISTRY_DATA_PACKET_ID => {
                let (reg_id, entries) = parse_registry_data_packet(&data).unwrap();
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
                send_packet(&mut stream, 0x07, &response).unwrap();
            }
            _ => {
                // Skip other packets
            }
        }
    }

    // Filter to only registries our server implements
    registries.retain(|id, _| ALL_REGISTRY_IDS.contains(&id.as_str()));
    println!(
        "Retained {} registries (filtered from vanilla set)",
        registries.len()
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
    ]
    .into_iter()
    .collect();

    for (registry_id, entries) in registries.iter_mut() {
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
    let json = serde_json::to_string_pretty(&registries).unwrap();
    let output_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/vanilla_registry_order.json");
    std::fs::write(&output_path, json).unwrap();
    println!("Written registry ordering to {}", output_path.display());
}

#[test]
#[ignore]
fn reorder_registry_files_to_match_vanilla() {
    let snapshot: HashMap<String, Vec<String>> = serde_json::from_str(include_str!(
        "../../tests/data/vanilla_registry_order.json"
    ))
    .unwrap();

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
                sorted.push(entry.clone());
            }
        }
        // Append any entries from our file that aren't in the vanilla snapshot
        for entry in &entries {
            let id = entry["id"].as_str().unwrap();
            if !expected_order.contains(&id.to_string()) {
                sorted.push(entry.clone());
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
