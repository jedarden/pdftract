//! Quick test to verify bomb limit works correctly
use std::time::Instant;

#[test]
fn test_bomb_limit_simple() {
    let bomb_data = std::fs::read("tests/stream_decoder/fixtures/flate_bomb_3gb.bin")
        .expect("Failed to read bomb fixture");

    println!("Bomb fixture size: {} bytes", bomb_data.len());

    let start = Instant::now();
    let mut counter = 0;
    let bomb_limit = 1_000_000_000; // 1 GB

    use pdftract_core::parser::stream::FlateDecoder;
    let result = FlateDecoder.decode(&bomb_data, None, &mut counter, bomb_limit);

    let elapsed = start.elapsed();
    println!("Decode completed in {:?}", elapsed);

    assert!(result.is_ok());
    let output = result.unwrap();
    println!("Output size: {} bytes", output.len());

    // Should complete in < 5 seconds
    assert!(elapsed.as_secs() < 5, "Bomb test took too long: {:?}", elapsed);

    // Output should be truncated near the limit
    assert!(output.len() as u64 <= bomb_limit + 1_000_000,
        "Output {} exceeds bomb limit {} by too much", output.len(), bomb_limit);
    assert!(output.len() as u64 >= 900_000_000,
        "Output {} is much smaller than expected", output.len());
}
