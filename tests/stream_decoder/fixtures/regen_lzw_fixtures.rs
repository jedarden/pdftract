//! Regenerate LZW fixtures for stream decoder tests.
//!
//! Run with: cargo run --bin regen_lzw_fixtures

use lzw::{MsbWriter, Encoder, DecoderEarlyChange, Decoder};
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.push("tests/stream_decoder/fixtures");

    println!("Regenerating LZW fixtures to: {}", dir.display());

    // Test data: "HelloWorld"
    let data = b"HelloWorld";

    // Early change 1 (Adobe/TIFF, PDF default)
    let mut early_compressed = vec![];
    {
        let mut enc = Encoder::new(MsbWriter::new(&mut early_compressed), 8)?;
        enc.encode_bytes(data)?;
    }

    let early_path = dir.join("lzw_early_change_1.bin");
    let early_expected = dir.join("lzw_early_change_1.expected");
    fs::write(&early_path, &early_compressed)?;
    fs::write(&early_expected, data)?;
    fs::write(&early_path.with_extension("meta"), "LZWDecode with /EarlyChange 1 (default, Adobe/TIFF variant)")?;
    println!("Generated: lzw_early_change_1.bin ({} bytes)", early_compressed.len());

    // Late change 0 (GIF variant) - same encoding, different decoder
    let late_path = dir.join("lzw_early_change_0.bin");
    let late_expected = dir.join("lzw_early_change_0.expected");
    fs::write(&late_path, &early_compressed)?;
    fs::write(&late_expected, data)?;
    fs::write(&late_path.with_extension("meta"), "LZWDecode with /EarlyChange 0 (GIF variant)")?;
    println!("Generated: lzw_early_change_0.bin ({} bytes)", early_compressed.len());

    // Verify decoding works
    let mut decoder = DecoderEarlyChange::new(MsbReader::new(), 8);
    let mut decoded = vec![];
    let mut remaining = &early_compressed[..];
    while !remaining.is_empty() {
        match decoder.decode_bytes(remaining) {
            Ok((consumed, chunk)) => {
                remaining = &remaining[consumed..];
                if chunk.is_empty() && consumed == 0 {
                    break;
                }
                decoded.extend_from_slice(chunk);
            }
            Err(_) => break,
        }
    }
    println!("Verification: decoded {} bytes: {:?}", decoded.len(), String::from_utf8_lossy(&decoded));
    assert_eq!(decoded, data, "Verification failed");

    println!("\nLZW fixtures regenerated successfully!");
    Ok(())
}
