use pdftract_core::parser::stream::{ASCII85Decoder, FlateDecoder, StreamDecoder, DEFAULT_MAX_DECOMPRESS_BYTES};

fn main() {
    // Test filter array: ASCII85 then Flate
    let input = std::fs::read("tests/stream_decoder/fixtures/filter_array_a85_then_flate.bin").unwrap();
    let expected = std::fs::read("tests/stream_decoder/fixtures/filter_array_a85_then_flate.expected").unwrap();
    
    println!("Input bytes: {:?}", input);
    println!("Expected: {:?}", String::from_utf8_lossy(&expected));
    
    let mut counter = 0;
    
    // First decode ASCII85
    let a85_result = ASCII85Decoder.decode(&input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    println!("ASCII85 decode result: {:?}", a85_result);
    
    match &a85_result {
        Ok(a85_decoded) => {
            println!("ASCII85 decoded bytes: {:?}", a85_decoded);
            println!("ASCII85 decoded length: {}", a85_decoded.len());
            
            // Then decode Flate
            let flate_result = FlateDecoder.decode(a85_decoded, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
            println!("Flate decode result: {:?}", flate_result);
            
            match &flate_result {
                Ok(flated) => {
                    println!("Final output: {:?}", String::from_utf8_lossy(flated));
                    println!("Final output bytes: {:02x?}", flated);
                }
                Err(e) => println!("Flate error: {:?}", e),
            }
        }
        Err(e) => println!("ASCII85 error: {:?}", e),
    }
}
