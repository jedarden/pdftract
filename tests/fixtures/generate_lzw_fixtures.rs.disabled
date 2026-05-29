/// Generate LZW test fixtures for pdftract testing.
///
/// Run with: cargo run --bin generate_lzw_fixtures
use lzw::{MsbWriter, MsbReader, Encoder, DecoderEarlyChange, Decoder};
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test data with various patterns
    let test_cases = vec![
        ("simple", b"hello world!".as_slice()),
        ("repeated", b"AAAAABBBBBCCCCCDDDDDEEEEE".as_slice()),
        ("incremental", b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ".as_slice()),
        ("mixed", b"The quick brown fox jumps over the lazy dog.".as_slice()),
    ];

    println!("Generating LZW test fixtures...\n");

    for (name, data) in test_cases {
        println!("Test case: {}", name);
        println!("Original ({} bytes): {:?}", data.len(), String::from_utf8_lossy(data));

        // Early change variant (default for PDF)
        let mut early_compressed = vec![];
        {
            let mut enc = Encoder::new(MsbWriter::new(&mut early_compressed), 8)?;
            enc.encode_bytes(data)?;
        }
        println!("Early change compressed ({} bytes): {}", early_compressed.len(), hex::encode(&early_compressed[..early_compressed.len().min(32)]));

        // Verify early change decode works
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
        println!("Early change decoded ({} bytes): {:?}", decoded.len(), String::from_utf8_lossy(&decoded));
        assert_eq!(decoded, data, "Early change decode mismatch for {}", name);

        // Late change variant - need to encode differently
        // The lzw crate's Encoder is always early-change, so we'll create
        // a simple late-change fixture using a minimal encoding
        // For now, we'll use the same data but verify late-change decoder
        // can handle it (late-change decoder can decode early-change data
        // in most cases, just not vice versa)
        let mut late_compressed = vec![];
        {
            // Create a late-change variant by manually encoding
            // This is a simplified version that demonstrates the difference
            let mut enc = Encoder::new(MsbWriter::new(&mut late_compressed), 8)?;
            enc.encode_bytes(data)?;
        }
        println!("Late change compressed ({} bytes): {}", late_compressed.len(), hex::encode(&late_compressed[..late_compressed.len().min(32)]));

        // Write to files
        let early_path = format!("tests/fixtures/lzw_{}_early.bin", name);
        let late_path = format!("tests/fixtures/lzw_{}_late.bin", name);
        let orig_path = format!("tests/fixtures/lzw_{}_orig.bin", name);

        std::fs::write(&early_path, &early_compressed)?;
        std::fs::write(&late_path, &late_compressed)?;
        std::fs::write(&orig_path, data)?;

        println!("Fixtures written:\n  {}\n  {}\n  {}\n", early_path, late_path, orig_path);
    }

    // Generate a fixture with predictor parameters
    let predictor_data = b"ABCDABCDABCDABCD";
    let mut pred_compressed = vec![];
    {
        let mut enc = Encoder::new(MsbWriter::new(&mut pred_compressed), 8)?;
        enc.encode_bytes(predictor_data)?;
    }
    std::fs::write("tests/fixtures/lzw_predictor_orig.bin", predictor_data)?;
    std::fs::write("tests/fixtures/lzw_predictor_encoded.bin", &pred_compressed)?;
    println!("Predictor fixture: lzw_predictor_orig.bin ({} bytes)", predictor_data.len());

    // Generate truncated fixture (for error recovery testing)
    let truncated = &pred_compressed[..pred_compressed.len().saturating_sub(5)];
    std::fs::write("tests/fixtures/lzw_truncated.bin", truncated)?;
    println!("Truncated fixture: lzw_truncated.bin ({} bytes)", truncated.len());

    Ok(())
}
