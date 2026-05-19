use serde_json::Value;
use std::io::{self, Write};

const FLOAT_FIELDS: &[&str] = &[
    "temperature",
    "downfall",
    "music_volume",
    "offset",
    "creature_spawn_probability",
];

pub fn json_to_nbt(value: &Value) -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    match value {
        Value::Object(_) => {
            data.push(0x0A); // TAG_Compound
            data.extend_from_slice(&0u16.to_be_bytes()); // empty root name
            write_compound_payload(&mut data, value)?;
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "root value must be a compound (JSON object)",
            ));
        }
    }
    Ok(data)
}

fn write_compound_payload(writer: &mut Vec<u8>, value: &Value) -> io::Result<()> {
    if let Value::Object(map) = value {
        for (key, val) in map {
            write_named_tag(writer, key, val)?;
        }
        writer.push(0x00); // TAG_End
    }
    Ok(())
}

fn write_named_tag(writer: &mut Vec<u8>, name: &str, value: &Value) -> io::Result<()> {
    let tag_type = get_tag_type_for_field(name, value);
    writer.push(tag_type);
    writer.write_all(&(name.len() as u16).to_be_bytes())?;
    writer.write_all(name.as_bytes())?;
    write_tag_payload_for_field(writer, name, value)?;
    Ok(())
}

fn get_tag_type_for_field(name: &str, value: &Value) -> u8 {
    if let Value::Number(n) = value {
        if n.is_i64() && FLOAT_FIELDS.contains(&name) {
            return 0x05; // TAG_Float
        }
    }
    get_tag_type(value)
}

fn write_tag_payload_for_field(writer: &mut Vec<u8>, name: &str, value: &Value) -> io::Result<()> {
    if let Value::Number(n) = value {
        if n.is_i64() && FLOAT_FIELDS.contains(&name) {
            let f = n.as_i64().unwrap() as f32;
            writer.write_all(&f.to_be_bytes())?;
            return Ok(());
        }
    }
    write_tag_payload(writer, value)
}

fn write_tag_payload(writer: &mut Vec<u8>, value: &Value) -> io::Result<()> {
    match value {
        Value::Bool(b) => {
            writer.push(if *b { 1 } else { 0 });
        }
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                    writer.write_all(&(i as i32).to_be_bytes())?;
                } else {
                    writer.write_all(&i.to_be_bytes())?;
                }
            } else if let Some(f) = n.as_f64() {
                if is_float_representable(f) {
                    writer.write_all(&(f as f32).to_be_bytes())?;
                } else {
                    writer.write_all(&f.to_be_bytes())?;
                }
            }
        }
        Value::String(s) => {
            writer.write_all(&(s.len() as u16).to_be_bytes())?;
            writer.write_all(s.as_bytes())?;
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                writer.push(0x00); // TAG_End type for empty list
                writer.write_all(&0i32.to_be_bytes())?;
            } else {
                let elem_type = get_tag_type(&arr[0]);
                writer.push(elem_type);
                writer.write_all(&(arr.len() as i32).to_be_bytes())?;
                for item in arr {
                    write_tag_payload(writer, item)?;
                }
            }
        }
        Value::Object(_) => {
            write_compound_payload(writer, value)?;
        }
        Value::Null => {
            writer.push(0x00); // TAG_End for null
        }
    }
    Ok(())
}

fn get_tag_type(value: &Value) -> u8 {
    match value {
        Value::Bool(_) => 0x01, // TAG_Byte
        Value::Number(n) => {
            if n.is_i64() {
                let i = n.as_i64().unwrap();
                if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                    0x03 // TAG_Int
                } else {
                    0x04 // TAG_Long
                }
            } else {
                let f = n.as_f64().unwrap_or(0.0);
                if is_float_representable(f) {
                    0x05 // TAG_Float
                } else {
                    0x06 // TAG_Double
                }
            }
        }
        Value::String(_) => 0x08, // TAG_String
        Value::Array(_) => 0x09,  // TAG_List
        Value::Object(_) => 0x0A, // TAG_Compound
        Value::Null => 0x00,      // TAG_End
    }
}

fn is_float_representable(f: f64) -> bool {
    (f as f32) as f64 == f
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_nbt_encoder_compound() {
        let value = json!({
            "name": "test",
            "value": 42
        });
        let nbt = json_to_nbt(&value).unwrap();
        assert_eq!(nbt[0], 0x0A); // TAG_Compound root
        assert!(!nbt.is_empty());
    }

    #[test]
    fn test_nbt_encoder_nested() {
        let value = json!({
            "outer": {
                "inner": "hello"
            },
            "list": [1, 2, 3]
        });
        let nbt = json_to_nbt(&value).unwrap();
        assert_eq!(nbt[0], 0x0A);
        assert!(nbt.len() > 10);
    }

    #[test]
    fn test_nbt_encoder_bool_as_byte() {
        let value = json!({"flag": true});
        let nbt = json_to_nbt(&value).unwrap();
        assert!(nbt.contains(&0x01)); // TAG_Byte present
    }

    #[test]
    fn test_nbt_encoder_rejects_non_object_root() {
        let value = json!("not an object");
        assert!(json_to_nbt(&value).is_err());
    }

    #[test]
    fn test_nbt_encoder_empty_list() {
        let value = json!({"items": []});
        let nbt = json_to_nbt(&value).unwrap();
        assert!(!nbt.is_empty());
    }

    #[test]
    fn test_nbt_encoder_string_values() {
        let value = json!({"message_id": "generic", "scaling": "never"});
        let nbt = json_to_nbt(&value).unwrap();
        assert!(nbt.len() > 20);
    }

    #[test]
    fn test_float_field_integer_value_encoded_as_float() {
        let value = json!({"temperature": 0, "downfall": 1});
        let nbt = json_to_nbt(&value).unwrap();
        let temp_name = b"temperature";
        let temp_pos = nbt
            .windows(temp_name.len())
            .position(|w| w == temp_name)
            .unwrap();
        // tag type byte is before the 2-byte name length prefix
        assert_eq!(nbt[temp_pos - 2 - 1], 0x05); // TAG_Float

        let downfall_name = b"downfall";
        let downfall_pos = nbt
            .windows(downfall_name.len())
            .position(|w| w == downfall_name)
            .unwrap();
        assert_eq!(nbt[downfall_pos - 2 - 1], 0x05); // TAG_Float
    }

    #[test]
    fn test_non_float_field_integer_stays_int() {
        let value = json!({"min_delay": 12000});
        let nbt = json_to_nbt(&value).unwrap();
        let name = b"min_delay";
        let pos = nbt.windows(name.len()).position(|w| w == name).unwrap();
        assert_eq!(nbt[pos - 2 - 1], 0x03); // TAG_Int
    }

    #[test]
    fn test_float_field_with_float_value_unchanged() {
        let value = json!({"temperature": 0.5});
        let nbt = json_to_nbt(&value).unwrap();
        let name = b"temperature";
        let pos = nbt.windows(name.len()).position(|w| w == name).unwrap();
        assert_eq!(nbt[pos - 2 - 1], 0x05); // TAG_Float (already float from value)
    }
}
