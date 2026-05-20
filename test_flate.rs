use flate2::write::ZlibEncoder;
use flate2::Compression;
use flate2::read::ZlibDecoder;
use std::io::{Write, Read};

fn main() {
    let header = b"1 0 2 3";
    let obj1 = b"42";
    let obj2 = b"true";
    let mut stream_data = Vec::new();
    stream_data.extend_from_slice(header);
    stream_data.extend_from_slice(obj1);
    stream_data.extend_from_slice(obj2);

    println!("Original data: {:?}", stream_data);
    println!("Original data as string: {:?}", String::from_utf8_lossy(&stream_data));

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&stream_data).unwrap();
    let compressed = encoder.finish().unwrap();

    println!("Compressed: {:?}", compressed);
    println!("Compressed len: {}", compressed.len());

    // Now try to decompress
    let mut decoder = ZlibDecoder::new(&compressed[..]);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).unwrap();

    println!("Decompressed: {:?}", decompressed);
    println!("Decompressed as string: {:?}", String::from_utf8_lossy(&decompressed));
}
