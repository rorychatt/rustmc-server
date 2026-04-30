use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rustmc_server::protocol::types::{VarInt, write_string, read_string};
use rustmc_server::protocol::handshake::Handshake;
use rustmc_server::protocol::packet::{Packet, PacketWriter, PacketReader};
use std::io::Cursor;

fn bench_varint_encode(c: &mut Criterion) {
    c.bench_function("varint encode small", |b| {
        b.iter(|| {
            let mut buf = Vec::new();
            VarInt(black_box(127)).write(&mut buf).unwrap();
            buf
        })
    });

    c.bench_function("varint encode medium", |b| {
        b.iter(|| {
            let mut buf = Vec::new();
            VarInt(black_box(25565)).write(&mut buf).unwrap();
            buf
        })
    });

    c.bench_function("varint encode large", |b| {
        b.iter(|| {
            let mut buf = Vec::new();
            VarInt(black_box(2097151)).write(&mut buf).unwrap();
            buf
        })
    });
}

fn bench_varint_decode(c: &mut Criterion) {
    let mut buf = Vec::new();
    VarInt(25565).write(&mut buf).unwrap();

    c.bench_function("varint decode", |b| {
        b.iter(|| {
            let mut cursor = Cursor::new(&buf);
            VarInt::read(black_box(&mut cursor)).unwrap()
        })
    });
}

fn bench_string_encode(c: &mut Criterion) {
    c.bench_function("string encode short", |b| {
        let s = black_box("localhost");
        b.iter(|| {
            let mut buf = Vec::new();
            write_string(&mut buf, s).unwrap();
            buf
        })
    });

    c.bench_function("string encode long", |b| {
        let s = black_box("a".repeat(256));
        b.iter(|| {
            let mut buf = Vec::new();
            write_string(&mut buf, &s).unwrap();
            buf
        })
    });
}

fn bench_string_decode(c: &mut Criterion) {
    let mut buf = Vec::new();
    write_string(&mut buf, "localhost").unwrap();

    c.bench_function("string decode", |b| {
        b.iter(|| {
            let mut cursor = Cursor::new(&buf);
            read_string(black_box(&mut cursor)).unwrap()
        })
    });
}

fn bench_handshake_decode(c: &mut Criterion) {
    let mut data = Vec::new();
    VarInt(765).write(&mut data).unwrap();
    write_string(&mut data, "localhost").unwrap();
    data.extend_from_slice(&25565u16.to_be_bytes());
    VarInt(1).write(&mut data).unwrap();

    c.bench_function("handshake decode", |b| {
        b.iter(|| Handshake::decode(black_box(&data)).unwrap())
    });
}

fn bench_packet_write(c: &mut Criterion) {
    let packet = Packet::new(0x00, vec![1, 2, 3, 4, 5, 6, 7, 8]);

    c.bench_function("packet write", |b| {
        b.iter(|| {
            let mut buf = Vec::new();
            PacketWriter::new(&mut buf)
                .write_packet(black_box(&packet))
                .unwrap();
            buf
        })
    });
}

fn bench_packet_read(c: &mut Criterion) {
    let packet = Packet::new(0x00, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    let mut buf = Vec::new();
    PacketWriter::new(&mut buf).write_packet(&packet).unwrap();

    c.bench_function("packet read", |b| {
        b.iter(|| {
            let cursor = Cursor::new(&buf);
            PacketReader::new(cursor).read_packet().unwrap()
        })
    });
}

criterion_group!(
    benches,
    bench_varint_encode,
    bench_varint_decode,
    bench_string_encode,
    bench_string_decode,
    bench_handshake_decode,
    bench_packet_write,
    bench_packet_read
);
criterion_main!(benches);
