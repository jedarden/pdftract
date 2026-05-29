//! Quick debug test for failing stream decoder fixtures.

use pdftract_core::parser::stream::{
    FlateDecoder, LZWDecoder, ASCII85Decoder, normalize_filter_name, StreamDecoder,
};
use pdftract_core::parser::object::{PdfObject, PdfDict};
use indexmap::IndexMap;

#[test]
fn test_decoder_debug() {
    // Test LZW decoder
    println!("Testing LZW decoder...");
    let lzw_input = std::fs::read("tests/stream_decoder/fixtures/lzw_early_change_0.bin").unwrap();
    println!("LZW input: {:02x?}", lzw_input);

    let mut counter = 0u64;
    let mut params = IndexMap::new();
    params.insert("/EarlyChange".into(), PdfObject::Integer(0));
    let params_obj = PdfObject::Dict(Box::new(params));

    let result = LZWDecoder.decode(&lzw_input, Some(&params_obj), &mut counter, pdftract_core::parser::stream::DEFAULT_MAX_DECOMPRESS_BYTES);
    match &result {
        Ok(data) => println!("LZW output: {:02x?}", data),
        Err(e) => println!("LZW error: {}", e),
    }

    // Test ASCII85 decoder
    println!("\nTesting ASCII85 decoder...");
    let a85_input = std::fs::read("tests/stream_decoder/fixtures/filter_array_a85_then_flate.bin").unwrap();
    println!("ASCII85 input (first 50 bytes): {:02x?}", &a85_input[..a85_input.len().min(50)]);

    let mut counter = 0u64;
    let result = ASCII85Decoder.decode(&a85_input, None, &mut counter, pdftract_core::parser::stream::DEFAULT_MAX_DECOMPRESS_BYTES);
    match &result {
        Ok(data) => {
            println!("ASCII85 decoded (first 50 bytes): {:02x?}", &data[..data.len().min(50)]);
            println!("ASCII85 decoded as string: {:?}", String::from_utf8_lossy(data));
        }
        Err(e) => println!("ASCII85 error: {}", e),
    }

    // Test Flate decoder with PNG predictor
    println!("\nTesting Flate decoder with PNG predictor...");
    let flate_input = std::fs::read("tests/stream_decoder/fixtures/flate_png_pred15_all_six.bin").unwrap();
    println!("Flate input (first 50 bytes): {:02x?}", &flate_input[..flate_input.len().min(50)]);

    let mut counter = 0u64;
    let mut params = IndexMap::new();
    params.insert("/Predictor".into(), PdfObject::Integer(15));
    params.insert("/Columns".into(), PdfObject::Integer(8));
    params.insert("/Colors".into(), PdfObject::Integer(1));
    params.insert("/BitsPerComponent".into(), PdfObject::Integer(8));
    let params_obj = PdfObject::Dict(Box::new(params));

    let result = FlateDecoder.decode(&flate_input, Some(&params_obj), &mut counter, pdftract_core::parser::stream::DEFAULT_MAX_DECOMPRESS_BYTES);
    match &result {
        Ok(data) => {
            println!("Flate output (first 50 bytes): {:02x?}", &data[..data.len().min(50)]);
            println!("Flate output as string: {:?}", String::from_utf8_lossy(data));
        }
        Err(e) => println!("Flate error: {}", e),
    }
}
