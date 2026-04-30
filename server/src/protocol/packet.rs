use super::types::VarInt;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
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
    compression_threshold: Option<i32>,
}

impl<R: Read> PacketReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            compression_threshold: None,
        }
    }

    pub fn set_compression_threshold(&mut self, threshold: i32) {
        self.compression_threshold = Some(threshold);
    }

    pub fn read_packet(&mut self) -> io::Result<Packet> {
        let packet_length = VarInt::read(&mut self.reader)?.0 as usize;

        if packet_length == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Zero-length packet",
            ));
        }
        if packet_length > 2_097_152 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Packet too large",
            ));
        }

        match self.compression_threshold {
            None => {
                // No compression - original behavior
                let mut payload = vec![0u8; packet_length];
                self.reader.read_exact(&mut payload)?;

                let mut cursor = Cursor::new(&payload);
                let packet_id = VarInt::read(&mut cursor)?.0;
                let data_start = cursor.position() as usize;
                let data = payload[data_start..].to_vec();

                Ok(Packet { id: packet_id, data })
            }
            Some(_) => {
                // Read data length
                let data_length = VarInt::read(&mut self.reader)?.0;
                let remaining_length = packet_length - VarInt(data_length).size();

                let mut compressed_or_uncompressed = vec![0u8; remaining_length];
                self.reader.read_exact(&mut compressed_or_uncompressed)?;

                let payload = if data_length == 0 {
                    // Below threshold - uncompressed
                    compressed_or_uncompressed
                } else {
                    // Above threshold - decompress
                    let mut decoder = ZlibDecoder::new(&compressed_or_uncompressed[..]);
                    let mut decompressed = Vec::new();
                    decoder.read_to_end(&mut decompressed)?;
                    decompressed
                };

                let mut cursor = Cursor::new(&payload);
                let packet_id = VarInt::read(&mut cursor)?.0;
                let data_start = cursor.position() as usize;
                let data = payload[data_start..].to_vec();

                Ok(Packet { id: packet_id, data })
            }
        }
    }
}

pub struct PacketWriter<W: Write> {
    writer: W,
    compression_threshold: Option<i32>,
}

impl<W: Write> PacketWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            compression_threshold: None,
        }
    }

    pub fn set_compression_threshold(&mut self, threshold: i32) {
        self.compression_threshold = Some(threshold);
    }

    pub fn write_packet(&mut self, packet: &Packet) -> io::Result<()> {
        let id_varint = VarInt(packet.id);
        let uncompressed_len = id_varint.size() + packet.data.len();

        match self.compression_threshold {
            None => {
                // No compression - original behavior
                VarInt(uncompressed_len as i32).write(&mut self.writer)?;
                id_varint.write(&mut self.writer)?;
                self.writer.write_all(&packet.data)?;
            }
            Some(threshold) if uncompressed_len < threshold as usize => {
                // Below threshold - send uncompressed with Data Length = 0
                let packet_length = 1 + uncompressed_len; // 1 byte for VarInt(0)
                VarInt(packet_length as i32).write(&mut self.writer)?;
                VarInt(0).write(&mut self.writer)?; // Data Length = 0
                id_varint.write(&mut self.writer)?;
                self.writer.write_all(&packet.data)?;
            }
            Some(_) => {
                // Above threshold - compress
                let mut uncompressed_data = Vec::new();
                id_varint.write(&mut uncompressed_data)?;
                uncompressed_data.extend_from_slice(&packet.data);

                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&uncompressed_data)?;
                let compressed_data = encoder.finish()?;

                let data_length_varint = VarInt(uncompressed_data.len() as i32);
                let packet_length = data_length_varint.size() + compressed_data.len();

                VarInt(packet_length as i32).write(&mut self.writer)?;
                data_length_varint.write(&mut self.writer)?;
                self.writer.write_all(&compressed_data)?;
            }
        }
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

    #[test]
    fn test_packet_compression_above_threshold() {
        // Create a packet with 300 bytes of data (above 256 threshold)
        let data = vec![0x42; 300];
        let packet = Packet::new(0x27, data.clone());

        let mut buf = Vec::new();
        let mut writer = PacketWriter::new(&mut buf);
        writer.set_compression_threshold(256);
        writer.write_packet(&packet).unwrap();

        // Verify format: [Packet Length] [Data Length > 0] [Compressed Data]
        let mut cursor = Cursor::new(&buf);
        let packet_length = VarInt::read(&mut cursor).unwrap().0;
        let data_length = VarInt::read(&mut cursor).unwrap().0;

        assert!(packet_length > 0, "Packet length must be positive");
        assert!(data_length > 0, "Data length must be positive for compressed packet");
        assert!(
            data_length > 256,
            "Data length should be above threshold (256)"
        );

        // Verify roundtrip
        let mut reader = PacketReader::new(Cursor::new(&buf));
        reader.set_compression_threshold(256);
        let read_back = reader.read_packet().unwrap();
        assert_eq!(packet.id, read_back.id);
        assert_eq!(packet.data, read_back.data);
    }

    #[test]
    fn test_packet_compression_below_threshold() {
        // Create a packet with 100 bytes of data (below 256 threshold)
        let data = vec![0x42; 100];
        let packet = Packet::new(0x01, data.clone());

        let mut buf = Vec::new();
        let mut writer = PacketWriter::new(&mut buf);
        writer.set_compression_threshold(256);
        writer.write_packet(&packet).unwrap();

        // Verify format: [Packet Length] [Data Length = 0] [Uncompressed Data]
        let mut cursor = Cursor::new(&buf);
        let _packet_length = VarInt::read(&mut cursor).unwrap().0;
        let data_length = VarInt::read(&mut cursor).unwrap().0;

        assert_eq!(
            data_length, 0,
            "Data length must be 0 for uncompressed packet"
        );

        // Verify roundtrip
        let mut reader = PacketReader::new(Cursor::new(&buf));
        reader.set_compression_threshold(256);
        let read_back = reader.read_packet().unwrap();
        assert_eq!(packet.id, read_back.id);
        assert_eq!(packet.data, read_back.data);
    }

    #[test]
    fn test_packet_compression_roundtrip() {
        // Test various packet sizes
        for size in [50, 200, 256, 300, 1000] {
            let data = vec![0x42; size];
            let packet = Packet::new(0x10, data.clone());

            let mut buf = Vec::new();
            let mut writer = PacketWriter::new(&mut buf);
            writer.set_compression_threshold(256);
            writer.write_packet(&packet).unwrap();

            let mut reader = PacketReader::new(Cursor::new(&buf));
            reader.set_compression_threshold(256);
            let read_back = reader.read_packet().unwrap();

            assert_eq!(packet.id, read_back.id, "Packet ID mismatch for size {}", size);
            assert_eq!(
                packet.data, read_back.data,
                "Packet data mismatch for size {}",
                size
            );
        }
    }

    #[test]
    fn test_compression_threshold_boundary() {
        // Test exactly at threshold boundary (256 bytes)
        // VarInt for packet ID (0x10) takes 1 byte, so we need 255 bytes of data
        let data = vec![0x42; 255];
        let packet = Packet::new(0x10, data.clone());

        let mut buf = Vec::new();
        let mut writer = PacketWriter::new(&mut buf);
        writer.set_compression_threshold(256);
        writer.write_packet(&packet).unwrap();

        // At exactly 256 bytes (1 byte ID + 255 bytes data), should be compressed (>= threshold)
        let mut cursor = Cursor::new(&buf);
        let _packet_length = VarInt::read(&mut cursor).unwrap().0;
        let data_length = VarInt::read(&mut cursor).unwrap().0;

        assert!(
            data_length > 0,
            "Data at threshold boundary (256 bytes) should be compressed"
        );

        // Verify roundtrip
        let mut reader = PacketReader::new(Cursor::new(&buf));
        reader.set_compression_threshold(256);
        let read_back = reader.read_packet().unwrap();
        assert_eq!(packet.id, read_back.id);
        assert_eq!(packet.data, read_back.data);
    }
}
