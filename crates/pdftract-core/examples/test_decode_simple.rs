use pdftract_core::parser::stream::{ASCII85Decoder, FlateDecoder, DEFAULT_MAX_DECOMPRESS_BYTES};

fn main() {
    let input = std::fs::read("/home/coding/pdftract/tests/stream_decoder/fixtures/filter_array_a85_then_flate.bin").unwrap();

    println!("=== Step 1: ASCII85 Decode ===");
    let mut counter = 0u64;
    match ASCII85Decoder.decode(&input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES) {
        Ok(decoded) => {
            println!("Success: {} bytes", decoded.len());
            println!("Hex (first 60): {}", hex::encode(&decoded[..decoded.len().min(60)]));
            println!("Counter after A85: {}", counter);

            println!("\n=== Step 2: Flate Decode ===");
            let mut counter2 = counter; // Start from where A85 left off
            println!("Counter before Flate: {}", counter2);
            println!("Max bytes: {}", DEFAULT_MAX_DECOMPRESS_BYTES);
            println!("Budget remaining: {}", DEFAULT_MAX_DECOMPRESS_BYTES - counter2);

            match FlateDecoder.decode(&decoded, None, &mut counter2, DEFAULT_MAX_DECOMPRESS_BYTES) {
                Ok(flated) => {
                    println!("Success: {} bytes", flated.len());
                    println!("Counter after Flate: {}", counter2);
                    if !flated.is_empty() {
                        println!("Text: {}", String::from_utf8_lossy(flated));
                    } else {
                        println!("Got empty bytes!");
                    }
                }
                Err(e) => println!("Error: {}", e),
            }
        }
        Err(e) => println!("A85 Error: {}", e),
    }
}
