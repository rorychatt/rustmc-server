use serde_json::Value;
use std::io::{self, Write};

const FLOAT_FIELDS: &[(&str, &str)] = &[
    ("", "temperature"),
    ("", "downfall"),
    ("", "creature_spawn_probability"),
    ("effects", "music_volume"),
    ("mood_sound", "offset"),
    ("", "ambient_light"),
    ("effect", "factor"),
    ("effect", "volume"),
    ("effect", "speed"),
    ("horizontal_velocity", "movement_scale"),
    ("vertical_position", "offset"),
    ("pitch", "min_inclusive"),
    ("pitch", "max_inclusive"),
    ("pitch", "min_exclusive"),
    ("pitch", "max_exclusive"),
];

const DOUBLE_FIELDS: &[(&str, &str)] = &[
    ("", "coordinate_scale"),
];

fn is_float_field(parent: &str, name: &str) -> bool {
    if name == "base" || name == "per_level_above_first" {
        return parent != "min_cost" && parent != "max_cost";
    }
    if name == "volume"
        || name == "pitch"
        || name == "speed"
        || name == "added"
        || name == "amount"
        || name == "duration"
        || name == "radius"
        || name == "values"
    {
        return true;
    }
    if name == "offset" {
        return parent == "mood_sound" || parent == "vertical_position" || parent == "effects";
    }
    FLOAT_FIELDS.contains(&(parent, name))
}

fn is_double_field(parent: &str, name: &str) -> bool {
    DOUBLE_FIELDS.contains(&(parent, name))
}

fn is_long_field(_parent: &str, name: &str) -> bool {
    name == "fixed_time"
}

pub fn json_to_nbt(value: &Value) -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    match value {
        Value::Object(_) => {
            data.push(0x0A); // TAG_Compound root (unnamed)
            write_compound_payload(&mut data, "", value)?;
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

fn write_compound_payload(writer: &mut Vec<u8>, parent: &str, value: &Value) -> io::Result<()> {
    if let Value::Object(map) = value {
        for (key, val) in map {
            write_named_tag(writer, parent, key, val)?;
        }
        writer.push(0x00); // TAG_End
    }
    Ok(())
}

fn write_named_tag(
    writer: &mut Vec<u8>,
    parent: &str,
    name: &str,
    value: &Value,
) -> io::Result<()> {
    let tag_type = get_tag_type_for_field(parent, name, value);
    writer.push(tag_type);
    writer.write_all(&(name.len() as u16).to_be_bytes())?;
    writer.write_all(name.as_bytes())?;
    write_tag_payload_for_field(writer, parent, name, value)?;
    Ok(())
}

fn get_tag_type_for_field(parent: &str, name: &str, value: &Value) -> u8 {
    if let Value::Number(_) = value {
        if is_float_field(parent, name) {
            return 0x05; // TAG_Float
        }
        if is_double_field(parent, name) {
            return 0x06; // TAG_Double
        }
        if is_long_field(parent, name) {
            return 0x04; // TAG_Long
        }
    }
    get_tag_type(value)
}

fn write_tag_payload_for_field(
    writer: &mut Vec<u8>,
    parent: &str,
    name: &str,
    value: &Value,
) -> io::Result<()> {
    if let Value::Number(n) = value {
        if is_float_field(parent, name) {
            let f = n.as_f64().unwrap_or(0.0) as f32;
            writer.write_all(&f.to_be_bytes())?;
            return Ok(());
        }
        if is_double_field(parent, name) {
            let d = n.as_f64().unwrap_or(0.0);
            writer.write_all(&d.to_be_bytes())?;
            return Ok(());
        }
        if is_long_field(parent, name) {
            let l = n.as_i64().unwrap_or(0);
            writer.write_all(&l.to_be_bytes())?;
            return Ok(());
        }
    }
    write_tag_payload(writer, parent, name, value)
}

fn get_unified_array_type(parent: &str, name: &str, arr: &[Value]) -> u8 {
    if arr.is_empty() {
        return 0x00;
    }
    if is_float_field(parent, name) {
        return 0x05; // TAG_Float
    }
    if is_double_field(parent, name) {
        return 0x06; // TAG_Double
    }
    if is_long_field(parent, name) {
        return 0x04; // TAG_Long
    }
    let all_numbers = arr.iter().all(|v| v.is_number());
    if all_numbers {
        let mut has_double = false;
        let mut has_float = false;
        let mut has_long = false;
        for val in arr {
            let t = get_tag_type(val);
            match t {
                0x06 => has_double = true,
                0x05 => has_float = true,
                0x04 => has_long = true,
                _ => {}
            }
        }
        if has_double {
            0x06 // TAG_Double
        } else if has_float {
            0x05 // TAG_Float
        } else if has_long {
            0x04 // TAG_Long
        } else {
            0x03 // TAG_Int
        }
    } else {
        let all_bools = arr.iter().all(|v| v.is_boolean());
        if all_bools {
            0x01 // TAG_Byte
        } else {
            get_tag_type(&arr[0])
        }
    }
}

fn write_tag_payload_with_type(
    writer: &mut Vec<u8>,
    parent: &str,
    name: &str,
    value: &Value,
    expected_type: u8,
) -> io::Result<()> {
    match expected_type {
        0x01 => {
            // TAG_Byte
            let val = match value {
                Value::Bool(b) => if *b { 1 } else { 0 },
                Value::Number(n) => n.as_i64().unwrap_or(0) as i8,
                _ => 0,
            };
            writer.push(val as u8);
        }
        0x03 => {
            // TAG_Int
            let val = match value {
                Value::Number(n) => n.as_i64().unwrap_or(0) as i32,
                Value::Bool(b) => if *b { 1 } else { 0 },
                _ => 0,
            };
            writer.write_all(&val.to_be_bytes())?;
        }
        0x04 => {
            // TAG_Long
            let val = match value {
                Value::Number(n) => n.as_i64().unwrap_or(0),
                Value::Bool(b) => if *b { 1 } else { 0 },
                _ => 0,
            };
            writer.write_all(&val.to_be_bytes())?;
        }
        0x05 => {
            // TAG_Float
            let val = match value {
                Value::Number(n) => n.as_f64().unwrap_or(0.0) as f32,
                Value::Bool(b) => if *b { 1.0 } else { 0.0 },
                _ => 0.0,
            };
            writer.write_all(&val.to_be_bytes())?;
        }
        0x06 => {
            // TAG_Double
            let val = match value {
                Value::Number(n) => n.as_f64().unwrap_or(0.0),
                Value::Bool(b) => if *b { 1.0 } else { 0.0 },
                _ => 0.0,
            };
            writer.write_all(&val.to_be_bytes())?;
        }
        0x08 => {
            // TAG_String
            let s = match value {
                Value::String(s) => s.as_str(),
                _ => "",
            };
            writer.write_all(&(s.len() as u16).to_be_bytes())?;
            writer.write_all(s.as_bytes())?;
        }
        0x09 => {
            if let Value::Array(sub_arr) = value {
                if sub_arr.is_empty() {
                    writer.push(0x00);
                    writer.write_all(&0i32.to_be_bytes())?;
                } else {
                    let sub_elem_type = get_unified_array_type(parent, name, sub_arr);
                    writer.push(sub_elem_type);
                    writer.write_all(&(sub_arr.len() as i32).to_be_bytes())?;
                    for item in sub_arr {
                        write_tag_payload_with_type(writer, parent, name, item, sub_elem_type)?;
                    }
                }
            } else {
                writer.push(0x00);
                writer.write_all(&0i32.to_be_bytes())?;
            }
        }
        0x0A => {
            write_compound_payload(writer, name, value)?;
        }
        _ => {
            write_tag_payload(writer, parent, name, value)?;
        }
    }
    Ok(())
}

fn write_tag_payload(
    writer: &mut Vec<u8>,
    parent: &str,
    name: &str,
    value: &Value,
) -> io::Result<()> {
    match value {
        Value::Bool(b) => {
            writer.push(if *b { 1 } else { 0 });
        }
        Value::Number(n) => {
            if is_float_field(parent, name) {
                let f = n.as_f64().unwrap_or(0.0) as f32;
                writer.write_all(&f.to_be_bytes())?;
            } else if is_double_field(parent, name) {
                let d = n.as_f64().unwrap_or(0.0);
                writer.write_all(&d.to_be_bytes())?;
            } else if is_long_field(parent, name) {
                let l = n.as_i64().unwrap_or(0);
                writer.write_all(&l.to_be_bytes())?;
            } else if let Some(i) = n.as_i64() {
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
                let elem_type = get_unified_array_type(parent, name, arr);
                writer.push(elem_type);
                writer.write_all(&(arr.len() as i32).to_be_bytes())?;
                for item in arr {
                    write_tag_payload_with_type(writer, parent, name, item, elem_type)?;
                }
            }
        }
        Value::Object(_) => {
            write_compound_payload(writer, name, value)?;
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
        assert!(nbt[0] >= 0x01 && nbt[0] <= 0x0C);
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
        assert!(nbt[0] >= 0x01 && nbt[0] <= 0x0C);
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
        assert_eq!(nbt[temp_pos - 2 - 1], 0x05); // TAG_Float

        let downfall_name = b"downfall";
        let downfall_pos = nbt
            .windows(downfall_name.len())
            .position(|w| w == downfall_name)
            .unwrap();
        assert_eq!(nbt[downfall_pos - 2 - 1], 0x05); // TAG_Float
    }

    #[test]
    fn test_nested_float_field_encoded_as_float() {
        let value = json!({"effects": {"music_volume": 1}});
        let nbt = json_to_nbt(&value).unwrap();
        let name = b"music_volume";
        let pos = nbt.windows(name.len()).position(|w| w == name).unwrap();
        assert_eq!(nbt[pos - 2 - 1], 0x05); // TAG_Float
    }

    #[test]
    fn test_offset_in_mood_sound_is_float() {
        let value = json!({"mood_sound": {"offset": 2}});
        let nbt = json_to_nbt(&value).unwrap();
        let name = b"offset";
        let pos = nbt.windows(name.len()).position(|w| w == name).unwrap();
        assert_eq!(nbt[pos - 2 - 1], 0x05); // TAG_Float
    }

    #[test]
    fn test_offset_at_root_stays_int() {
        let value = json!({"offset": 1});
        let nbt = json_to_nbt(&value).unwrap();
        let name = b"offset";
        let pos = nbt.windows(name.len()).position(|w| w == name).unwrap();
        assert_eq!(nbt[pos - 2 - 1], 0x03); // TAG_Int — not coerced at root
    }

    #[test]
    fn test_offset_in_wrong_parent_stays_int() {
        let value = json!({"particles": {"offset": 5}});
        let nbt = json_to_nbt(&value).unwrap();
        let name = b"offset";
        let pos = nbt.windows(name.len()).position(|w| w == name).unwrap();
        assert_eq!(nbt[pos - 2 - 1], 0x03); // TAG_Int — wrong parent
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

    #[test]
    fn test_network_nbt_has_root_header() {
        let value = json!({"key": "value"});
        let nbt = json_to_nbt(&value).unwrap();
        assert_eq!(nbt[0], 0x0A); // TAG_Compound root
    }

    #[test]
    fn test_network_nbt_ends_with_tag_end() {
        let value = json!({"key": "value"});
        let nbt = json_to_nbt(&value).unwrap();
        assert_eq!(*nbt.last().unwrap(), 0x00, "Network NBT must end with TAG_End");
    }

    #[test]
    fn test_network_nbt_empty_object() {
        let value = json!({});
        let nbt = json_to_nbt(&value).unwrap();
        assert_eq!(nbt, vec![0x0A, 0x00], "Empty compound should be root compound + TAG_End");
    }

    #[test]
    fn test_ambient_light_is_coerced_to_float() {
        let value = json!({"ambient_light": 0});
        let nbt = json_to_nbt(&value).unwrap();
        let name = b"ambient_light";
        let pos = nbt.windows(name.len()).position(|w| w == name).unwrap();
        assert_eq!(nbt[pos - 2 - 1], 0x05); // TAG_Float
    }

    #[test]
    fn test_array_mixed_floats_coerced_to_double() {
        let value = json!({"mixed_doubles": [1.2, 1.75, 2.2]});
        let nbt = json_to_nbt(&value).unwrap();
        let name = b"mixed_doubles";
        let pos = nbt.windows(name.len()).position(|w| w == name).unwrap();
        assert_eq!(nbt[pos - 2 - 1], 0x09); // TAG_List
        let list_payload_start = pos + name.len();
        assert_eq!(nbt[list_payload_start], 0x06); // elem_type: TAG_Double
        assert_eq!(&nbt[list_payload_start + 1..list_payload_start + 5], &3i32.to_be_bytes());
        let values_start = list_payload_start + 5;
        assert_eq!(nbt.len() - 1 - values_start, 24); // 24 bytes of double payload + 1 byte TAG_End
    }

    #[test]
    fn test_values_array_coerced_to_float() {
        let value = json!({"values": [1.2, 1.75, 2.2]});
        let nbt = json_to_nbt(&value).unwrap();
        let name = b"values";
        let pos = nbt.windows(name.len()).position(|w| w == name).unwrap();
        assert_eq!(nbt[pos - 2 - 1], 0x09); // TAG_List
        let list_payload_start = pos + name.len();
        assert_eq!(nbt[list_payload_start], 0x05); // elem_type: TAG_Float
        assert_eq!(&nbt[list_payload_start + 1..list_payload_start + 5], &3i32.to_be_bytes());
        let values_start = list_payload_start + 5;
        assert_eq!(nbt.len() - 1 - values_start, 12); // 12 bytes of float payload + 1 byte TAG_End
    }

    #[test]
    fn test_fixed_time_coerced_to_long() {
        let value = json!({"fixed_time": 6000});
        let nbt = json_to_nbt(&value).unwrap();
        let name = b"fixed_time";
        let pos = nbt.windows(name.len()).position(|w| w == name).unwrap();
        assert_eq!(nbt[pos - 2 - 1], 0x04); // TAG_Long
    }
}
