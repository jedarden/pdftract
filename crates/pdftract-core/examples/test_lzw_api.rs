use lzw::{Decoder, DecoderEarlyChange, MsbReader};

fn main() {
    // Test basic encoding/decoding
    let data = b"hello world!";

    // Encode with early change
    let mut encoder = lzw::EncoderEarlyChange::new(lzw::MsbWriter::new(), 8);
    let encoded_early: Vec<u8> = encoder.encode_bytes(data).0;
    println!("Encoded (early change): {:02x?}", encoded_early);

    // Decode with early change
    let mut decoder = DecoderEarlyChange::new(MsbReader::new(), 8);
    let (consumed, decoded) = decoder.decode_bytes(&encoded_early).unwrap();
    println!(
        "Decoded (early change): {:?}",
        std::str::from_utf8(decoded).unwrap()
    );

    // Encode with late change
    let mut encoder2 = lzw::Encoder::new(lzw::MsbWriter::new(), 8);
    let encoded_late: Vec<u8> = encoder2.encode_bytes(data).0;
    println!("Encoded (late change): {:02x?}", encoded_late);

    // Decode with late change
    let mut decoder2 = Decoder::new(MsbReader::new(), 8);
    let (consumed2, decoded2) = decoder2.decode_bytes(&encoded_late).unwrap();
    println!(
        "Decoded (late change): {:?}",
        std::str::from_utf8(decoded2).unwrap()
    );
}
