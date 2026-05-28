//! Generate LZW-encoded test fixtures.
//!
//! Run with: cargo run --bin gen_lzw_fixtures

use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fixtures_dir = Path::new("tests/stream_decoder/fixtures");

    // Test data: "HelloWorld"
    let data = b"HelloWorld";

    // Generate LZW with early_change 0 (GIF variant)
    let lzw_0 = encode_lzw(data, 0)?;
    fs::write(fixtures_dir.join("lzw_early_change_0.bin"), lzw_0)?;
    fs::write(fixtures_dir.join("lzw_early_change_0.expected"), data)?;

    // Generate LZW with early_change 1 (Adobe/TIFF variant, default)
    let lzw_1 = encode_lzw(data, 1)?;
    fs::write(fixtures_dir.join("lzw_early_change_1.bin"), lzw_1)?;
    fs::write(fixtures_dir.join("lzw_early_change_1.expected"), data)?;

    println!("Generated LZW fixtures!");

    Ok(())
}

/// Encode data using LZW with the specified early_change setting.
fn encode_lzw(data: &[u8], early_change: i32) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use lzw::{Encoder, EncoderEarlyChange, MsbReader};

    // LZW minimum code size is always 8 in PDF
    const MIN_CODE_SIZE: u8 = 8;

    // Create encoder based on early_change setting
    let encoded_bytes = if early_change == 1 {
        let mut encoder = EncoderEarlyChange::new(MsbReader::new(), MIN_CODE_SIZE);
        encoder.encode_bytes(data).to_vec()
    } else {
        let mut encoder = Encoder::new(MsbReader::new(), MIN_CODE_SIZE);
        encoder.encode_bytes(data).to_vec()
    };

    // Add minimum code size byte at the start (LZW format)
    let mut result = Vec::with_capacity(1 + encoded_bytes.len());
    result.push(MIN_CODE_SIZE);
    result.extend_from_slice(&encoded_bytes);

    Ok(result)
}
