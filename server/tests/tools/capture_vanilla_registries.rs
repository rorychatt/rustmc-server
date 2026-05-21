//! Standalone tool to capture registry entry ordering and data from a vanilla Minecraft 1.21.4 server.
//!
//! Connects to a vanilla server, completes handshake → login → configuration,
//! captures all Registry Data packets (0x07), and writes the ordered entry IDs
//! to `server/tests/data/vanilla_registry_order.json`.
//!
//! The `capture_vanilla_registry_data` test additionally captures full NBT data
//! and writes complete registry files to `server/data/registries/v775/`.
//!
//! Supports servers with compression enabled (default `network-compression-threshold=256`).
//!
//! # Usage
//!
//! Start a vanilla 1.21.4 server with `online-mode=false`, then run:
//!
//! ```sh
//! # Capture ordering only:
//! VANILLA_HOST=127.0.0.1 VANILLA_PORT=25565 cargo test --test capture_vanilla_registries -- --ignored capture_vanilla_registry_ordering
//!
//! # Capture full registry data (IDs + NBT):
//! VANILLA_HOST=127.0.0.1 VANILLA_PORT=25565 cargo test --test capture_vanilla_registries -- --ignored capture_vanilla_registry_data
//! ```
//!
//! Environment variables:
//! - `VANILLA_HOST` — server hostname (default: `127.0.0.1`)
//! - `VANILLA_PORT` — server port (default: `25565`)

use std::collections::HashMap;
use std::io;
use std::net::TcpStream;

use rustmc_server::protocol::packet::{Packet, PacketReader, PacketWriter};
use rustmc_server::registry::nbt_encoder::json_to_nbt;
use rustmc_server::registry::ALL_REGISTRY_IDS;

const PROTOCOL_VERSION: i32 = 775;
const REGISTRY_DATA_PACKET_ID: i32 = 0x07;

const BOOL_BYTE_FIELDS: &[&str] = &[
    "bed_works",
    "decal",
    "has_ceiling",
    "has_precipitation",
    "has_raids",
    "has_skylight",
    "natural",
    "piglin_safe",
    "replace_current_music",
    "respawn_anchor_works",
    "ultrawarm",
];

type RegistryEntry = (String, Option<Vec<u8>>);

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

fn skip_nbt(buf: &[u8], offset: &mut usize) -> io::Result<()> {
    if *offset >= buf.len() {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "NBT: no data"));
    }
    let tag_type = buf[*offset];
    *offset += 1;

    if tag_type == 0x00 {
        return Ok(());
    }

    if tag_type != 0x0A {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("NBT root must be TAG_Compound (0x0A), got 0x{tag_type:02X}"),
        ));
    }

    // Network NBT: root compound has an empty name (2-byte length)
    if *offset + 2 > buf.len() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "NBT root name",
        ));
    }
    let name_len = u16::from_be_bytes([buf[*offset], buf[*offset + 1]]) as usize;
    *offset += 2 + name_len;

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

fn parse_nbt_to_json(buf: &[u8], offset: &mut usize) -> io::Result<serde_json::Value> {
    if *offset >= buf.len() {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "NBT: no data"));
    }
    let tag_type = buf[*offset];
    *offset += 1;

    if tag_type == 0x00 {
        return Ok(serde_json::Value::Null);
    }

    if tag_type != 0x0A {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("NBT root must be TAG_Compound (0x0A), got 0x{tag_type:02X}"),
        ));
    }

    // Network NBT: root compound has an empty name (2-byte length)
    if *offset + 2 > buf.len() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "NBT root name",
        ));
    }
    let name_len = u16::from_be_bytes([buf[*offset], buf[*offset + 1]]) as usize;
    *offset += 2 + name_len;

    parse_nbt_compound_to_json(buf, offset)
}

fn parse_nbt_compound_to_json(buf: &[u8], offset: &mut usize) -> io::Result<serde_json::Value> {
    let mut map = serde_json::Map::new();

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
        *offset += 2;
        if *offset + name_len > buf.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "NBT compound name data",
            ));
        }
        let name = String::from_utf8(buf[*offset..*offset + name_len].to_vec())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        *offset += name_len;

        let value = parse_nbt_payload_to_json(buf, offset, child_type, &name)?;
        map.insert(name, value);
    }

    Ok(serde_json::Value::Object(map))
}

fn parse_nbt_payload_to_json(
    buf: &[u8],
    offset: &mut usize,
    tag_type: u8,
    field_name: &str,
) -> io::Result<serde_json::Value> {
    match tag_type {
        0x01 => {
            // TAG_Byte → bool (0/1) or integer
            if *offset >= buf.len() {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "NBT byte"));
            }
            let b = buf[*offset];
            *offset += 1;
            if (b == 0 || b == 1) && BOOL_BYTE_FIELDS.contains(&field_name) {
                Ok(serde_json::Value::Bool(b == 1))
            } else {
                Ok(serde_json::json!(b as i8))
            }
        }
        0x02 => {
            // TAG_Short
            if *offset + 2 > buf.len() {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "NBT short"));
            }
            let v = i16::from_be_bytes([buf[*offset], buf[*offset + 1]]);
            *offset += 2;
            Ok(serde_json::json!(v))
        }
        0x03 => {
            // TAG_Int
            if *offset + 4 > buf.len() {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "NBT int"));
            }
            let v = i32::from_be_bytes([
                buf[*offset],
                buf[*offset + 1],
                buf[*offset + 2],
                buf[*offset + 3],
            ]);
            *offset += 4;
            Ok(serde_json::json!(v))
        }
        0x04 => {
            // TAG_Long
            if *offset + 8 > buf.len() {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "NBT long"));
            }
            let v = i64::from_be_bytes([
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
            Ok(serde_json::json!(v))
        }
        0x05 => {
            // TAG_Float → f64 (via f32 for vanilla precision)
            if *offset + 4 > buf.len() {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "NBT float"));
            }
            let v = f32::from_be_bytes([
                buf[*offset],
                buf[*offset + 1],
                buf[*offset + 2],
                buf[*offset + 3],
            ]);
            *offset += 4;
            Ok(serde_json::json!(v as f64))
        }
        0x06 => {
            // TAG_Double
            if *offset + 8 > buf.len() {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "NBT double"));
            }
            let v = f64::from_be_bytes([
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
            Ok(serde_json::json!(v))
        }
        0x07 => {
            // TAG_Byte_Array
            if *offset + 4 > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "NBT byte array len",
                ));
            }
            let len = i32::from_be_bytes([
                buf[*offset],
                buf[*offset + 1],
                buf[*offset + 2],
                buf[*offset + 3],
            ]) as usize;
            *offset += 4;
            if *offset + len > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "NBT byte array data",
                ));
            }
            let arr: Vec<serde_json::Value> = buf[*offset..*offset + len]
                .iter()
                .map(|&b| serde_json::json!(b as i8))
                .collect();
            *offset += len;
            Ok(serde_json::Value::Array(arr))
        }
        0x08 => {
            // TAG_String
            if *offset + 2 > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "NBT string len",
                ));
            }
            let len = u16::from_be_bytes([buf[*offset], buf[*offset + 1]]) as usize;
            *offset += 2;
            if *offset + len > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "NBT string data",
                ));
            }
            let s = String::from_utf8(buf[*offset..*offset + len].to_vec())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            *offset += len;
            Ok(serde_json::Value::String(s))
        }
        0x09 => {
            // TAG_List
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
            let mut arr = Vec::with_capacity(count as usize);
            for _ in 0..count {
                arr.push(parse_nbt_payload_to_json(buf, offset, elem_type, "")?);
            }
            Ok(serde_json::Value::Array(arr))
        }
        0x0A => {
            // TAG_Compound
            parse_nbt_compound_to_json(buf, offset)
        }
        0x0B => {
            // TAG_Int_Array
            if *offset + 4 > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "NBT int array len",
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
                    "NBT int array data",
                ));
            }
            let arr: Vec<serde_json::Value> = (0..count)
                .map(|i| {
                    let start = *offset + i * 4;
                    let v = i32::from_be_bytes([
                        buf[start],
                        buf[start + 1],
                        buf[start + 2],
                        buf[start + 3],
                    ]);
                    serde_json::json!(v)
                })
                .collect();
            *offset += count * 4;
            Ok(serde_json::Value::Array(arr))
        }
        0x0C => {
            // TAG_Long_Array
            if *offset + 4 > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "NBT long array len",
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
                    "NBT long array data",
                ));
            }
            let arr: Vec<serde_json::Value> = (0..count)
                .map(|i| {
                    let start = *offset + i * 8;
                    let v = i64::from_be_bytes([
                        buf[start],
                        buf[start + 1],
                        buf[start + 2],
                        buf[start + 3],
                        buf[start + 4],
                        buf[start + 5],
                        buf[start + 6],
                        buf[start + 7],
                    ]);
                    serde_json::json!(v)
                })
                .collect();
            *offset += count * 8;
            Ok(serde_json::Value::Array(arr))
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Unknown NBT tag type: 0x{tag_type:02X}"),
        )),
    }
}

fn parse_registry_data_packet_full(data: &[u8]) -> io::Result<(String, Vec<serde_json::Value>)> {
    let mut offset = 0;
    let registry_id = read_string_from_buf(data, &mut offset)?;
    let entry_count = read_varint_from_buf(data, &mut offset)? as usize;
    let mut entries = Vec::with_capacity(entry_count);

    for _ in 0..entry_count {
        let entry_id = read_string_from_buf(data, &mut offset)?;

        if offset >= data.len() {
            entries.push(serde_json::json!({"id": entry_id}));
            break;
        }
        let has_data = data[offset] != 0;
        offset += 1;

        if has_data {
            let nbt_value = parse_nbt_to_json(data, &mut offset)?;
            entries.push(serde_json::json!({"id": entry_id, "data": nbt_value}));
        } else {
            entries.push(serde_json::json!({"id": entry_id}));
        }
    }

    Ok((registry_id, entries))
}

fn parse_registry_data_packet(
    data: &[u8],
) -> io::Result<(String, Vec<RegistryEntry>)> {
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

        let nbt_bytes = if has_data {
            let start = offset;
            skip_nbt(data, &mut offset)?;
            Some(data[start..offset].to_vec())
        } else {
            None
        };

        entries.push((entry_id, nbt_bytes));
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

    // Filter to only registries our server implements
    registries.retain(|id, _| ALL_REGISTRY_IDS.contains(&id.as_str()));
    println!(
        "Retained {} registries (filtered from vanilla set)",
        registries.len()
    );

    // Filter entries within each registry to only those in our data files
    let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/registries/v775");
    let registry_to_file = registry_to_file_mapping();

    for (registry_id, entries) in registries.iter_mut() {
        if let Some(&filename) = registry_to_file.get(registry_id.as_str()) {
            let file_path = data_dir.join(filename);
            let content = std::fs::read_to_string(&file_path).unwrap();
            let file_entries: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap();
            let our_ids: Vec<String> = file_entries
                .iter()
                .map(|e| e["id"].as_str().unwrap().to_string())
                .collect();
            entries.retain(|(id, _)| our_ids.contains(id));
        }
    }

    // Write ordering snapshot (IDs only, for backward compatibility)
    let order_map: HashMap<&str, Vec<&str>> = registries
        .iter()
        .map(|(k, v)| (k.as_str(), v.iter().map(|(id, _)| id.as_str()).collect()))
        .collect();
    let order_json = serde_json::to_string_pretty(&order_map).unwrap();
    let order_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/vanilla_registry_order.json");
    std::fs::write(&order_path, order_json).unwrap();
    println!("Written registry ordering to {}", order_path.display());

    // Write full NBT data snapshot
    let mut data_map: HashMap<&str, Vec<serde_json::Value>> = HashMap::new();
    for (reg_id, entries) in &registries {
        let entry_list: Vec<serde_json::Value> = entries
            .iter()
            .map(|(id, nbt)| {
                serde_json::json!({
                    "id": id,
                    "nbt_hex": nbt.as_ref().map(hex::encode).unwrap_or_default()
                })
            })
            .collect();
        data_map.insert(reg_id.as_str(), entry_list);
    }
    let data_json = serde_json::to_string_pretty(&data_map).unwrap();
    let data_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/vanilla_registry_data.json");
    std::fs::write(&data_path, data_json).unwrap();
    println!("Written registry NBT data to {}", data_path.display());
}

fn registry_to_file_mapping() -> HashMap<&'static str, &'static str> {
    [
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
    .collect()
}

#[test]
#[ignore]
fn capture_vanilla_registry_data() {
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
    write_varint(&mut handshake_data, 2);
    writer
        .write_packet(&Packet::new(0x00, handshake_data))
        .unwrap();

    // Login Start
    let mut login_data = Vec::new();
    write_string(&mut login_data, "RegistryCapture");
    login_data.extend_from_slice(&[0u8; 16]);
    writer.write_packet(&Packet::new(0x00, login_data)).unwrap();

    let mut registries: HashMap<String, Vec<serde_json::Value>> = HashMap::new();

    loop {
        let packet = reader.read_packet().unwrap();

        match packet.id {
            0x03 if registries.is_empty() => {
                let mut offset = 0;
                let threshold = read_varint_from_buf(&packet.data, &mut offset).unwrap();
                reader.set_compression_threshold(threshold);
                writer.set_compression_threshold(threshold);
                println!("Compression enabled with threshold {threshold}");
            }
            0x02 => {
                writer.write_packet(&Packet::new(0x03, vec![])).unwrap();
            }
            REGISTRY_DATA_PACKET_ID => {
                let (reg_id, entries) = parse_registry_data_packet_full(&packet.data).unwrap();
                println!("Captured {}: {} entries", reg_id, entries.len());
                registries.insert(reg_id, entries);
            }
            0x03 => {
                break;
            }
            0x0E => {
                let mut response = Vec::new();
                write_varint(&mut response, 0);
                writer.write_packet(&Packet::new(0x07, response)).unwrap();
            }
            _ => {}
        }
    }

    let registry_to_file = registry_to_file_mapping();
    let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/registries/v775");

    for (registry_id, entries) in &registries {
        let Some(&filename) = registry_to_file.get(registry_id.as_str()) else {
            println!("Skipping {registry_id}: no file mapping");
            continue;
        };

        // Write in compact format: one JSON object per line
        let mut output = String::from("[\n");
        for (i, entry) in entries.iter().enumerate() {
            output.push_str("  ");
            output.push_str(&serde_json::to_string(entry).unwrap());
            if i < entries.len() - 1 {
                output.push(',');
            }
            output.push('\n');
        }
        output.push_str("]\n");

        let file_path = data_dir.join(filename);
        std::fs::write(&file_path, &output).unwrap();
        println!("Written {filename}: {} entries", entries.len());
    }

    println!(
        "\nCapture complete: {} registries written to {}",
        registries.len(),
        data_dir.display()
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
    let registry_to_file = registry_to_file_mapping();

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

#[test]
#[ignore]
fn validate_registry_nbt_data() {
    let snapshot_str = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data/vanilla_registry_data.json"),
    )
    .expect("vanilla_registry_data.json not found — run capture_vanilla_registry_ordering first");

    let snapshot: HashMap<String, Vec<serde_json::Value>> =
        serde_json::from_str(&snapshot_str).unwrap();

    let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/registries/v775");
    let registry_to_file = registry_to_file_mapping();

    let mut mismatches = Vec::new();
    let mut missing_in_ours = Vec::new();
    let mut extra_in_ours = Vec::new();

    for (registry_id, vanilla_entries) in &snapshot {
        let Some(&filename) = registry_to_file.get(registry_id.as_str()) else {
            println!("Skipping {registry_id}: no file mapping");
            continue;
        };

        let file_path = data_dir.join(filename);
        let content = std::fs::read_to_string(&file_path).unwrap();
        let our_entries: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap();

        let vanilla_ids: Vec<&str> = vanilla_entries
            .iter()
            .map(|e| e["id"].as_str().unwrap())
            .collect();
        let our_ids: Vec<&str> = our_entries
            .iter()
            .map(|e| e["id"].as_str().unwrap())
            .collect();

        // Check for entries vanilla has that we don't
        for &vid in &vanilla_ids {
            if !our_ids.contains(&vid) {
                missing_in_ours.push(format!("{registry_id}/{vid}"));
            }
        }

        // Check for entries we have that vanilla doesn't
        for &oid in &our_ids {
            if !vanilla_ids.contains(&oid) {
                extra_in_ours.push(format!("{registry_id}/{oid}"));
            }
        }

        // Compare NBT data for entries present in both
        for vanilla_entry in vanilla_entries {
            let entry_id = vanilla_entry["id"].as_str().unwrap();
            let nbt_hex = vanilla_entry["nbt_hex"].as_str().unwrap_or("");

            if nbt_hex.is_empty() {
                continue;
            }

            let vanilla_nbt = hex::decode(nbt_hex).unwrap();

            let Some(our_entry) = our_entries.iter().find(|e| e["id"].as_str() == Some(entry_id))
            else {
                continue;
            };

            if let Some(data) = our_entry.get("data") {
                match json_to_nbt(data) {
                    Ok(our_nbt) => {
                        if our_nbt != vanilla_nbt {
                            mismatches.push(format!(
                                "{registry_id}/{entry_id}:\n  vanilla: {}\n  ours:    {}",
                                hex::encode(&vanilla_nbt),
                                hex::encode(&our_nbt),
                            ));
                        }
                    }
                    Err(e) => {
                        mismatches.push(format!(
                            "{registry_id}/{entry_id}: NBT encoding error: {e}"
                        ));
                    }
                }
            }
        }
    }

    if !missing_in_ours.is_empty() {
        println!(
            "\nWARNING: Entries in vanilla but not in ours ({}):",
            missing_in_ours.len()
        );
        for id in &missing_in_ours {
            println!("  - {id}");
        }
    }

    if !extra_in_ours.is_empty() {
        println!(
            "\nERROR: Entries in ours but not in vanilla ({}):",
            extra_in_ours.len()
        );
        for id in &extra_in_ours {
            println!("  - {id}");
        }
    }

    if !mismatches.is_empty() {
        println!("\nNBT mismatches ({}):", mismatches.len());
        for m in &mismatches {
            println!("  {m}");
        }
    }

    assert!(
        extra_in_ours.is_empty(),
        "We have {} entries that vanilla doesn't have",
        extra_in_ours.len()
    );
    assert!(
        mismatches.is_empty(),
        "{} NBT data mismatches detected",
        mismatches.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn build_nbt_compound(fields: &[(&str, u8, &[u8])]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(0x0A); // TAG_Compound root
        buf.extend_from_slice(&0u16.to_be_bytes()); // empty root name
        for &(name, tag_type, payload) in fields {
            buf.push(tag_type);
            buf.extend_from_slice(&(name.len() as u16).to_be_bytes());
            buf.extend_from_slice(name.as_bytes());
            buf.extend_from_slice(payload);
        }
        buf.push(0x00); // TAG_End
        buf
    }

    #[test]
    fn test_parse_nbt_simple_compound() {
        let mut nbt = Vec::new();
        nbt.push(0x0A); // TAG_Compound root
        nbt.extend_from_slice(&0u16.to_be_bytes()); // empty root name
        // String field: "name" = "test"
        nbt.push(0x08); // TAG_String
        nbt.extend_from_slice(&4u16.to_be_bytes()); // name len
        nbt.extend_from_slice(b"name");
        nbt.extend_from_slice(&4u16.to_be_bytes()); // value len
        nbt.extend_from_slice(b"test");
        // Int field: "value" = 42
        nbt.push(0x03); // TAG_Int
        nbt.extend_from_slice(&5u16.to_be_bytes());
        nbt.extend_from_slice(b"value");
        nbt.extend_from_slice(&42i32.to_be_bytes());
        nbt.push(0x00); // TAG_End

        let mut offset = 0;
        let result = parse_nbt_to_json(&nbt, &mut offset).unwrap();
        assert_eq!(result["name"], "test");
        assert_eq!(result["value"], 42);
    }

    #[test]
    fn test_parse_nbt_nested_compound() {
        let mut nbt = Vec::new();
        nbt.push(0x0A); // root compound
        nbt.extend_from_slice(&0u16.to_be_bytes());
        // Nested compound: "inner" = { "msg": "hi" }
        nbt.push(0x0A); // TAG_Compound
        nbt.extend_from_slice(&5u16.to_be_bytes());
        nbt.extend_from_slice(b"inner");
        nbt.push(0x08); // TAG_String
        nbt.extend_from_slice(&3u16.to_be_bytes());
        nbt.extend_from_slice(b"msg");
        nbt.extend_from_slice(&2u16.to_be_bytes());
        nbt.extend_from_slice(b"hi");
        nbt.push(0x00); // end inner
        nbt.push(0x00); // end root

        let mut offset = 0;
        let result = parse_nbt_to_json(&nbt, &mut offset).unwrap();
        assert_eq!(result["inner"]["msg"], "hi");
    }

    #[test]
    fn test_parse_nbt_list() {
        let mut nbt = Vec::new();
        nbt.push(0x0A); // root compound
        nbt.extend_from_slice(&0u16.to_be_bytes());
        // List field: "items" = [1, 2, 3]
        nbt.push(0x09); // TAG_List
        nbt.extend_from_slice(&5u16.to_be_bytes());
        nbt.extend_from_slice(b"items");
        nbt.push(0x03); // element type: TAG_Int
        nbt.extend_from_slice(&3i32.to_be_bytes()); // count
        nbt.extend_from_slice(&1i32.to_be_bytes());
        nbt.extend_from_slice(&2i32.to_be_bytes());
        nbt.extend_from_slice(&3i32.to_be_bytes());
        nbt.push(0x00); // end root

        let mut offset = 0;
        let result = parse_nbt_to_json(&nbt, &mut offset).unwrap();
        let items = result["items"].as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], 1);
        assert_eq!(items[1], 2);
        assert_eq!(items[2], 3);
    }

    #[test]
    fn test_parse_nbt_all_primitive_types() {
        let mut nbt = Vec::new();
        nbt.push(0x0A);
        nbt.extend_from_slice(&0u16.to_be_bytes());

        // Byte (not in BOOL_BYTE_FIELDS, stays as integer)
        nbt.push(0x01);
        nbt.extend_from_slice(&4u16.to_be_bytes());
        nbt.extend_from_slice(b"flag");
        nbt.push(1);

        // Short
        nbt.push(0x02);
        nbt.extend_from_slice(&5u16.to_be_bytes());
        nbt.extend_from_slice(b"short");
        nbt.extend_from_slice(&256i16.to_be_bytes());

        // Long
        nbt.push(0x04);
        nbt.extend_from_slice(&4u16.to_be_bytes());
        nbt.extend_from_slice(b"long");
        nbt.extend_from_slice(&9999999999i64.to_be_bytes());

        // Float
        nbt.push(0x05);
        nbt.extend_from_slice(&5u16.to_be_bytes());
        nbt.extend_from_slice(b"float");
        nbt.extend_from_slice(&1.5f32.to_be_bytes());

        // Double
        nbt.push(0x06);
        nbt.extend_from_slice(&6u16.to_be_bytes());
        nbt.extend_from_slice(b"double");
        nbt.extend_from_slice(&1.23456f64.to_be_bytes());

        nbt.push(0x00);

        let mut offset = 0;
        let result = parse_nbt_to_json(&nbt, &mut offset).unwrap();
        assert_eq!(result["flag"], 1);
        assert_eq!(result["short"], 256);
        assert_eq!(result["long"], 9999999999i64);
        assert_eq!(result["float"], 1.5);
        assert_eq!(result["double"], 1.23456f64);
    }

    #[test]
    fn test_parse_nbt_empty_compound() {
        let nbt = build_nbt_compound(&[]);
        let mut offset = 0;
        let result = parse_nbt_to_json(&nbt, &mut offset).unwrap();
        assert_eq!(result, json!({}));
    }

    #[test]
    fn test_parse_nbt_roundtrip_with_encoder() {
        let original = json!({
            "asset_id": "minecraft:all_black",
            "spawn_conditions": [
                {
                    "context": {
                        "min_light": 0,
                        "max_light": 15
                    },
                    "chance": 0.0625
                }
            ]
        });

        let encoded = json_to_nbt(&original).unwrap();

        let mut offset = 0;
        let decoded = parse_nbt_to_json(&encoded, &mut offset).unwrap();

        assert_eq!(decoded["asset_id"], "minecraft:all_black");
        let conditions = decoded["spawn_conditions"].as_array().unwrap();
        assert_eq!(conditions.len(), 1);
        assert_eq!(conditions[0]["context"]["min_light"], 0);
        assert_eq!(conditions[0]["context"]["max_light"], 15);
        let chance = conditions[0]["chance"].as_f64().unwrap();
        assert!((chance - 0.0625).abs() < 0.001);
    }

    #[test]
    fn test_parse_nbt_roundtrip_biome() {
        let original = json!({
            "has_precipitation": true,
            "temperature": 0.8,
            "downfall": 0.4,
            "effects": {
                "fog_color": 12638463,
                "water_color": 4159204,
                "water_fog_color": 329011,
                "sky_color": 7972607
            }
        });

        let encoded = json_to_nbt(&original).unwrap();

        let mut offset = 0;
        let decoded = parse_nbt_to_json(&encoded, &mut offset).unwrap();

        assert_eq!(decoded["has_precipitation"], true);
        let temp = decoded["temperature"].as_f64().unwrap();
        assert!((temp - 0.8).abs() < 0.001);
        let downfall = decoded["downfall"].as_f64().unwrap();
        assert!((downfall - 0.4).abs() < 0.001);
        assert_eq!(decoded["effects"]["fog_color"], 12638463);
    }

    #[test]
    fn test_parse_nbt_byte_field_not_in_bool_list_stays_integer() {
        let nbt = build_nbt_compound(&[("variant_id", 0x01, &[1])]);
        let mut offset = 0;
        let result = parse_nbt_to_json(&nbt, &mut offset).unwrap();
        assert_eq!(result["variant_id"], 1);
    }

    #[test]
    fn test_parse_nbt_byte_field_in_bool_list_stays_boolean() {
        let nbt = build_nbt_compound(&[("has_precipitation", 0x01, &[1])]);
        let mut offset = 0;
        let result = parse_nbt_to_json(&nbt, &mut offset).unwrap();
        assert_eq!(result["has_precipitation"], true);

        let nbt_false = build_nbt_compound(&[("ultrawarm", 0x01, &[0])]);
        let mut offset = 0;
        let result = parse_nbt_to_json(&nbt_false, &mut offset).unwrap();
        assert_eq!(result["ultrawarm"], false);
    }
}
