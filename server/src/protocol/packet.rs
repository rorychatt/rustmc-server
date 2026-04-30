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
}
