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
                return Err(io::Error::new(io::ErrorKind::InvalidData, "VarInt too long"));
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
    let len = VarInt::read(reader)?.0 as usize;
    if len > 32767 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "String too long"));
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
}
