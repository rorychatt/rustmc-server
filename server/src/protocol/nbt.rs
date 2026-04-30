use std::io::{self, Write};

const TAG_END: u8 = 0;
const TAG_COMPOUND: u8 = 10;
const TAG_LONG_ARRAY: u8 = 12;

pub fn write_compound_start(writer: &mut impl Write, name: &str) -> io::Result<()> {
    writer.write_all(&[TAG_COMPOUND])?;
    writer.write_all(&(name.len() as u16).to_be_bytes())?;
    writer.write_all(name.as_bytes())
}

pub fn write_unnamed_compound_start(writer: &mut impl Write) -> io::Result<()> {
    writer.write_all(&[TAG_COMPOUND])?;
    writer.write_all(&0u16.to_be_bytes())
}

pub fn write_long_array(writer: &mut impl Write, name: &str, data: &[i64]) -> io::Result<()> {
    writer.write_all(&[TAG_LONG_ARRAY])?;
    writer.write_all(&(name.len() as u16).to_be_bytes())?;
    writer.write_all(name.as_bytes())?;
    writer.write_all(&(data.len() as i32).to_be_bytes())?;
    for &val in data {
        writer.write_all(&val.to_be_bytes())?;
    }
    Ok(())
}

pub fn write_compound_end(writer: &mut impl Write) -> io::Result<()> {
    writer.write_all(&[TAG_END])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nbt_compound_roundtrip() {
        let mut buf = Vec::new();
        write_unnamed_compound_start(&mut buf).unwrap();
        write_long_array(&mut buf, "test", &[1, 2, 3]).unwrap();
        write_compound_end(&mut buf).unwrap();

        assert_eq!(buf[0], TAG_COMPOUND);
        assert_eq!(&buf[1..3], &[0, 0]); // empty name
        assert_eq!(buf[3], TAG_LONG_ARRAY);
        // name length = 4 ("test")
        assert_eq!(&buf[4..6], &[0, 4]);
        assert_eq!(&buf[6..10], b"test");
        // array length = 3
        assert_eq!(&buf[10..14], &3i32.to_be_bytes());
    }
}
