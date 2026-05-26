use lzw::{Decoder, MsbReader};

fn main() {
    // Test basic encoding/decoding
    let data = b"hello world!";

    // Encode with LzwWriter (LSB first)
    let mut encoded = Vec::new();
    {
        let mut encoder = lzw::LsbWriter::new(&mut encoded);
        std::io::Write::write_all(&mut encoder, data).expect("Failed to write data");
    }
    println!("Encoded: {:02x?}", encoded);

    // Decode
    let mut decoder = Decoder::<MsbReader>::new(MsbReader::new(), 8);
    let (consumed, decoded) = decoder.decode_bytes(&encoded).unwrap();
    println!("Decoded: {:?}", std::str::from_utf8(decoded).unwrap());
}
