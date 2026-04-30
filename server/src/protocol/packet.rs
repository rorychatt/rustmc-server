use super::types::VarInt;
use std::io::{self, Cursor, Read, Write};

#[derive(Debug, Clone)]
pub struct Packet {
    pub id: i32,
    pub data: Vec<u8>,
}

impl Packet {
    pub fn new(id: i32, data: Vec<u8>) -> Self {
        Self { id, data }
    }
}

pub struct PacketReader<R: Read> {
    reader: R,
}

impl<R: Read> PacketReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub fn read_packet(&mut self) -> io::Result<Packet> {
        let length = VarInt::read(&mut self.reader)?.0 as usize;
        if length == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Zero-length packet",
            ));
        }
        if length > 2_097_152 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Packet too large",
            ));
        }

        let mut payload = vec![0u8; length];
        self.reader.read_exact(&mut payload)?;

        let mut cursor = Cursor::new(&payload);
        let packet_id = VarInt::read(&mut cursor)?.0;
        let data_start = cursor.position() as usize;
        let data = payload[data_start..].to_vec();

        Ok(Packet {
            id: packet_id,
            data,
        })
    }
}

pub struct PacketWriter<W: Write> {
    writer: W,
}

impl<W: Write> PacketWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn write_packet(&mut self, packet: &Packet) -> io::Result<()> {
        let id_varint = VarInt(packet.id);
        let total_len = id_varint.size() + packet.data.len();

        VarInt(total_len as i32).write(&mut self.writer)?;
        id_varint.write(&mut self.writer)?;
        self.writer.write_all(&packet.data)?;
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_roundtrip() {
        let packet = Packet::new(0x00, vec![1, 2, 3, 4]);
        let mut buf = Vec::new();
        PacketWriter::new(&mut buf).write_packet(&packet).unwrap();

        let read_back = PacketReader::new(Cursor::new(&buf)).read_packet().unwrap();
        assert_eq!(packet.id, read_back.id);
        assert_eq!(packet.data, read_back.data);
    }

    mod proptest_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn test_packet_roundtrip(id in any::<i32>(), data in prop::collection::vec(any::<u8>(), 0..32768)) {
                let packet = Packet::new(id, data.clone());
                let mut buf = Vec::new();
                PacketWriter::new(&mut buf).write_packet(&packet).unwrap();

                let read_back = PacketReader::new(Cursor::new(&buf)).read_packet().unwrap();
                prop_assert_eq!(packet.id, read_back.id);
                prop_assert_eq!(packet.data, read_back.data);
            }

            #[test]
            fn test_packet_valid_sizes(id in any::<i32>(), size in 0..32768usize) {
                let data = vec![0u8; size];
                let packet = Packet::new(id, data);
                let mut buf = Vec::new();
                let result = PacketWriter::new(&mut buf).write_packet(&packet);
                prop_assert!(result.is_ok());
            }

            #[test]
            fn test_packet_reader_rejects_oversized(size in 2_097_153i32..3_000_000i32) {
                let mut buf = Vec::new();
                VarInt(size).write(&mut buf).unwrap();
                let result = PacketReader::new(Cursor::new(&buf)).read_packet();
                prop_assert!(result.is_err());
                prop_assert!(result.unwrap_err().to_string().contains("too large"));
            }

            #[test]
            fn test_packet_reader_rejects_truncated(claimed_size in 10..100usize, actual_size in 1..9usize) {
                let mut buf = Vec::new();
                VarInt(claimed_size as i32).write(&mut buf).unwrap();
                buf.extend_from_slice(&vec![0u8; actual_size]);
                let result = PacketReader::new(Cursor::new(&buf)).read_packet();
                prop_assert!(result.is_err());
            }
        }

        #[test]
        fn test_packet_reader_rejects_zero_length() {
            let mut buf = Vec::new();
            VarInt(0).write(&mut buf).unwrap();
            let result = PacketReader::new(Cursor::new(&buf)).read_packet();
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Zero-length"));
        }
    }
}
