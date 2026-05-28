//! Generate LZW-encoded fixtures with proper early_change 0 and 1.

use std::env;
use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <output.bin> <early_change: 0|1>", args[0]);
        std::process::exit(1);
    }

    let output_path = &args[1];
    let early_change: i32 = args[2].parse()?;

    // Test data: "HelloWorld"
    let data = b"HelloWorld";

    // LZW encode using the lzw crate
    let mut encoded = Vec::new();

    // Write LZW minimum code size (always 8 for PDF)
    encoded.push(8u8);

    // LZW encode
    use lzw::{MsbReader, DecoderEarlyChange};

    let lzw_data = if early_change == 1 {
        // Early change 1 (Adobe/TIFF, default)
        let mut encoder = lzw::EncoderEarlyChange::new(MsbReader::new(), 8);
        encoder.encode_bytes(data).to_vec()
    } else {
        // Early change 0 (GIF variant)
        let mut encoder = lzw::Encoder::new(MsbReader::new(), 8);
        encoder.encode_bytes(data).to_vec()
    };

    encoded.extend_from_slice(&lzw_data);

    // Write output
    let mut file = File::create(output_path)?;
    file.write_all(&encoded)?;

    // Also write expected output
    let expected_path = format!("{}.expected", output_path);
    let mut file = File::create(expected_path)?;
    file.write_all(data)?;

    Ok(())
}
