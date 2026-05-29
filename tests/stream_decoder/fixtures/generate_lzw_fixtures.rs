//! Generate LZW fixtures for testing.
//! Usage: cargo run --bin generate_lzw_fixtures <early_change: 0|1>

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <output_name> <early_change: 0|1>", args[0]);
        eprintln!("Example: {} lzw_early_change_0 0", args[0]);
        std::process::exit(1);
    }

    let output_name = &args[1];
    let early_change: i32 = args[2].parse()?;

    // Test data: "HelloWorld"
    let data = b"HelloWorld";

    // LZW encode using the lzw crate
    let mut encoded = Vec::new();

    // Write LZW minimum code size (always 8 for PDF)
    encoded.push(8u8);

    // LZW encode
    use lzw::{MsbReader, EncoderEarlyChange, Encoder};

    let lzw_data = if early_change == 1 {
        // Early change 1 (Adobe/TIFF, default)
        let mut encoder = EncoderEarlyChange::new(MsbReader::new(), 8);
        encoder.encode_bytes(data).to_vec()
    } else {
        // Early change 0 (GIF variant)
        let mut encoder = Encoder::new(MsbReader::new(), 8);
        encoder.encode_bytes(data).to_vec()
    };

    encoded.extend_from_slice(&lzw_data);

    // Get fixtures directory
    let mut fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixtures_dir.push("../../tests/stream_decoder/fixtures");
    let fixtures_dir = fixtures_dir.canonicalize()?;

    let fixture_path = fixtures_dir.join(format!("{}.bin", output_name));
    let expected_path = fixtures_dir.join(format!("{}.expected", output_name));

    // Write fixture
    let mut file = File::create(&fixture_path)?;
    file.write_all(&encoded)?;

    // Write expected
    let mut file = File::create(&expected_path)?;
    file.write_all(data)?;

    println!("Generated: {}.bin ({} bytes -> {} bytes)", output_name, encoded.len(), data.len());

    Ok(())
}
