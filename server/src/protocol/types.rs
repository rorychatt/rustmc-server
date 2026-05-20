use std::io::{self, Read, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VarInt(pub i32);

impl VarInt {
    pub fn read(reader: &mut impl Read) -> io::Result<Self> {
        let mut result: i32 = 0;
        let mut shift: u32 = 0;
        loop {
            let mut buf = [0u8; 1];
            reader.read_exact(&mut buf)?;
            let byte = buf[0];
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
        Ok(VarInt(result))
    }

    pub fn write(&self, writer: &mut impl Write) -> io::Result<()> {
        let mut value = self.0 as u32;
        loop {
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            writer.write_all(&[byte])?;
            if value == 0 {
                break;
            }
        }
        Ok(())
    }

    pub fn size(&self) -> usize {
        let mut value = self.0 as u32;
        let mut size = 0;
        loop {
            size += 1;
            value >>= 7;
            if value == 0 {
                break;
            }
        }
        size
    }
}

pub fn read_string(reader: &mut impl Read) -> io::Result<String> {
    read_string_with_max(reader, 32767)
}

pub fn read_string_with_max(reader: &mut impl Read, max_len: usize) -> io::Result<String> {
    let len = VarInt::read(reader)?.0 as usize;
    if len > 32767 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "String too long",
        ));
    }
    if len > max_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("String length {len} exceeds maximum {max_len}"),
        ));
    }
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn write_string(writer: &mut impl Write, s: &str) -> io::Result<()> {
    VarInt(s.len() as i32).write(writer)?;
    writer.write_all(s.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_varint_roundtrip() {
        let values = [0, 1, 127, 128, 255, 25565, 2097151, -1, i32::MIN, i32::MAX];
        for &val in &values {
            let vi = VarInt(val);
            let mut buf = Vec::new();
            vi.write(&mut buf).unwrap();
            let read_back = VarInt::read(&mut Cursor::new(&buf)).unwrap();
            assert_eq!(vi, read_back, "VarInt roundtrip failed for {val}");
        }
    }

    #[test]
    fn test_varint_size() {
        assert_eq!(VarInt(0).size(), 1);
        assert_eq!(VarInt(127).size(), 1);
        assert_eq!(VarInt(128).size(), 2);
        assert_eq!(VarInt(25565).size(), 3);
    }

    #[test]
    fn test_string_roundtrip() {
        let s = "Hello, Minecraft!";
        let mut buf = Vec::new();
        write_string(&mut buf, s).unwrap();
        let read_back = read_string(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(s, read_back);
    }

    #[test]
    fn test_string_empty() {
        let s = "";
        let mut buf = Vec::new();
        write_string(&mut buf, s).unwrap();
        let read_back = read_string(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(s, read_back);
    }

    #[test]
    fn test_string_utf8() {
        let s = "Hello, 世界! 🎮";
        let mut buf = Vec::new();
        write_string(&mut buf, s).unwrap();
        let read_back = read_string(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(s, read_back);
    }

    #[test]
    fn test_varint_too_long() {
        let bad_data = [0x80, 0x80, 0x80, 0x80, 0x80, 0x01];
        let result = VarInt::read(&mut &bad_data[..]);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_string_with_max_accepts_at_limit() {
        let s = "a".repeat(256);
        let mut buf = Vec::new();
        write_string(&mut buf, &s).unwrap();
        let result = read_string_with_max(&mut Cursor::new(&buf), 256).unwrap();
        assert_eq!(s, result);
    }

    #[test]
    fn test_read_string_with_max_rejects_over_limit() {
        let s = "a".repeat(257);
        let mut buf = Vec::new();
        write_string(&mut buf, &s).unwrap();
        let result = read_string_with_max(&mut Cursor::new(&buf), 256);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("exceeds maximum"));
    }

    #[test]
    fn test_read_string_default_limit() {
        let s = "a".repeat(1000);
        let mut buf = Vec::new();
        write_string(&mut buf, &s).unwrap();
        let result = read_string(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(s, result);
    }

    #[cfg(test)]
    mod proptest_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn test_varint_roundtrip_proptest(n in any::<i32>()) {
                let vi = VarInt(n);
                let mut buf = Vec::new();
                vi.write(&mut buf).unwrap();
                let decoded = VarInt::read(&mut Cursor::new(&buf)).unwrap();
                prop_assert_eq!(vi, decoded);
            }

            #[test]
            fn test_string_roundtrip_proptest(s in "\\PC{0,100}") {
                let mut buf = Vec::new();
                write_string(&mut buf, &s).unwrap();
                let decoded = read_string(&mut Cursor::new(&buf)).unwrap();
                prop_assert_eq!(s, decoded);
            }
        }
    }
}
