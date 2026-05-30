use std::time::Instant;

// Minimal test to check if FlateDecode bomb limit works
fn main() {
    let bomb_data = std::fs::read("tests/stream_decoder/fixtures/flate_bomb_3gb.bin")
        .expect("Failed to read bomb fixture");

    println!("Bomb fixture size: {} bytes", bomb_data.len());

    let start = Instant::now();
    let mut counter = 0;
    let bomb_limit = 1_000_000_000; // 1 GB

    // Try to decode with flate2 directly first
    println!("Testing with flate2 ZlibDecoder...");

    use flate2::read::ZlibDecoder;
    let mut decoder = ZlibDecoder::new(&bomb_data[..]);
    let mut output = Vec::new();
    let mut chunk = [0u8; 64 * 1024];
    let mut total_bytes = 0u64;

    loop {
        match decoder.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                total_bytes += n as u64;
                if total_bytes > bomb_limit {
                    println!("  Hit bomb limit after {} bytes", total_bytes);
                    break;
                }
                if output.len() < 10_000_000 {
                    output.extend_from_slice(&chunk[..n]);
                }
            }
            Err(e) => {
                println!("  Decode error: {}", e);
                break;
            }
        }
    }

    let elapsed = start.elapsed();
    println!("  Decoded {} bytes in {:?}", total_bytes, elapsed);
    println!("  First 100 bytes of output: {:02x?}", &output[..100.min(output.len())]);
}

fn read(_buf: &mut [u8]) -> std::io::Result<usize> {
    Ok(0)
}
