//! Generate LZW-encoded fixtures for stream decoder testing.
//!
//! Usage:
//!   cargo run --bin gen_stream_lzw --release

use std::fs;
use std::path::PathBuf;
use lzw::{Encoder, MsbWriter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.push("tests/stream_decoder/fixtures");

    println!("Generating LZW fixtures to: {}", dir.display());

    // Test data: "HelloWorld"
    let data = b"HelloWorld";

    // Early change 1 (Adobe/TIFF, default)
    let mut early_compressed = vec![];
    {
        let mut enc = Encoder::new(MsbWriter::new(&mut early_compressed), 8)?;
        enc.encode_bytes(data)?;
    }

    let early_path = dir.join("lzw_early_change_1.bin");
    let early_expected = dir.join("lzw_early_change_1.expected");
    fs::write(&early_path, &early_compressed)?;
    fs::write(&early_expected, data)?;
    println!("Generated: lzw_early_change_1.bin ({})", early_compressed.len());

    // For early change 0 (GIF), we use the same encoding since PDF LZW
    // is typically early-change, but we want to test both decoder variants
    let late_path = dir.join("lzw_early_change_0.bin");
    let late_expected = dir.join("lzw_early_change_0.expected");
    fs::write(&late_path, &early_compressed)?;
    fs::write(&late_expected, data)?;
    println!("Generated: lzw_early_change_0.bin ({})", early_compressed.len());

    println!("\nLZW fixtures generated successfully!");
    Ok(())
}
