use pdftract_core::parser::stream::{ASCII85Decoder, StreamDecoder, DEFAULT_MAX_DECOMPRESS_BYTES};

fn main() {
    // Test ascii85_terminator fixture
    let input = b"<~<+U,m\n\t~>";
    let mut counter = 0;
    let result = ASCII85Decoder.decode(input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    println!("Input: {:?}", input);
    println!("Result: {:?}", result);
    
    if let Ok(output) = result {
        println!("Output bytes: {:?}", output);
        println!("Output string: {:?}", String::from_utf8_lossy(&output));
    }
}
