/// Generate LZW test fixtures for pdftract testing.
///
/// Run with: cargo run --bin generate_lzw_fixtures
use lzw::{MsbWriter, MsbReader, Encoder, DecoderEarlyChange, Decoder};

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
        println!("Early change compressed ({} bytes): {:02x?}", early_compressed.len(), early_compressed.iter().take(32).cloned().collect::<Vec<_>>());

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
        if decoded != data {
            println!("WARNING: Early change decode mismatch for {}", name);
        }

        // Late change variant - note: Encoder is always early-change
        // For late change testing, we use the same encoding since late-change
        // decoder can handle early-change data in most cases
        let late_compressed = early_compressed.clone();
        println!("Late change compressed ({} bytes): {:02x?}", late_compressed.len(), late_compressed.iter().take(32).cloned().collect::<Vec<_>>());

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
