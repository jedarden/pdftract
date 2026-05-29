//! Generate proper LZW fixtures for stream decoder tests.
//!
//! This script generates LZW-encoded test fixtures.
//! Run with: cargo run --bin gen_lzw_fixtures
//!
//! Output: tests/stream_decoder/fixtures/lzw_early_change_0.bin and lzw_early_change_1.bin

use lzw::{MsbWriter, Encoder, DecoderEarlyChange};
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.push("tests/stream_decoder/fixtures");

    println!("Generating LZW fixtures to: {}", dir.display());

    // Test data: "HelloWorld"
    let data = b"HelloWorld";

    // Early change 1 (Adobe/TIFF, PDF default)
    let mut early_change_1_data = Vec::new();
    // LZW minimum code size (always 8 for PDF)
    early_change_1_data.push(8u8);
    {
        let mut enc = EncoderEarlyChange::new(MsbitWriter::new(&mut early_change_1_data), 8)?;
        enc.encode_bytes(data)?;
        enc.finish()?;
    }

    let early_change_1_path = dir.join("lzw_early_change_1.bin");
    let early_change_1_expected = dir.join("lzw_early_change_1.expected");
    fs::write(&early_change_1_path, &early_change_1_data)?;
    fs::write(&early_change_1_expected, data)?;
    fs::write(
        &early_change_1_path.with_extension("meta"),
        "LZWDecode with /EarlyChange 1 (default, Adobe/TIFF variant)",
    )?;
    println!(
        "Generated: lzw_early_change_1.bin ({} bytes)",
        early_change_1_data.len()
    );

    // Early change 0 (GIF variant)
    let mut early_change_0_data = Vec::new();
    early_change_0_data.push(8u8);
    {
        let mut enc = Encoder::new(MsbitWriter::new(&mut early_change_0_data), 8)?;
        enc.encode_bytes(data)?;
        enc.finish()?;
    }

    let early_change_0_path = dir.join("lzw_early_change_0.bin");
    let early_change_0_expected = dir.join("lzw_early_change_0.expected");
    fs::write(&early_change_0_path, &early_change_0_data)?;
    fs::write(&early_change_0_expected, data)?;
    fs::write(
        &early_change_0_path.with_extension("meta"),
        "LZWDecode with /EarlyChange 0 (GIF variant)",
    )?;
    println!(
        "Generated: lzw_early_change_0.bin ({} bytes)",
        early_change_0_data.len()
    );

    // Verify the two encodings are different
    if early_change_0_data == early_change_1_data {
        println!("WARNING: Both encodings are identical! This shouldn't happen.");
    } else {
        println!("OK: The two encodings are different as expected.");
    }

    println!("\nLZW fixtures generated successfully!");
    Ok(())
}
