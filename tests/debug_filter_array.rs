//! Debug the filter_array_a85_then_flate fixture

use pdftract_core::parser::stream::{ASCII85Decoder, FlateDecoder, DEFAULT_MAX_DECOMPRESS_BYTES};
use std::fs;

#[test]
fn debug_filter_array_fixture() {
    let input = fs::read("tests/stream_decoder/fixtures/filter_array_a85_then_flate.bin").unwrap();
    
    println!("Input bytes (raw): {:?}", input);
    println!("Input string: {:?}", String::from_utf8_lossy(&input));
    
    let mut counter = 0;
    let result = ASCII85Decoder.decode(&input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    
    match result {
        Ok(decoded) => {
            println!("ASCII85 decoded: {} bytes", decoded.len());
            println!("First 20 bytes (hex): {:02x?}", &decoded[..20.min(decoded.len())]);
            
            // Now try flate
            let mut counter2 = 0;
            let flate_result = FlateDecoder.decode(&decoded, None, &mut counter2, DEFAULT_MAX_DECOMPRESS_BYTES);
            match flate_result {
                Ok(final_data) => {
                    println!("Flate decoded: {} bytes", final_data.len());
                    println!("Text: {}", String::from_utf8_lossy(&final_data));
                }
                Err(e) => println!("Flate error: {:?}", e),
            }
        }
        Err(e) => println!("ASCII85 error: {:?}", e),
    }
}
