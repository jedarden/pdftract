use pdftract_core::parser::stream::{
    FlateDecoder, LZWDecoder, ASCII85Decoder, ASCIIHexDecoder,
    RunLengthDecoder, DCTDecoder, JpxStreamDecoder, CCITTFaxDecoder,
    CryptDecoder, PassthroughDecoder, normalize_filter_name,
    StreamDecoder, DEFAULT_MAX_DECOMPRESS_BYTES,
};
use pdftract_core::parser::object::{PdfObject, PdfDict};
use pdftract_core::diagnostics::DiagCode;
use indexmap::IndexMap;
use std::path::PathBuf;
use std::fs;

fn main() {
    let fixtures = vec![
        ("flate_png_pred15_all_six", "FlateDecode", Some(create_png_predictor_params())),
        ("flate_truncated", "FlateDecode", None),
        ("lzw_early_change_0", "LZWDecode", Some(create_early_change_params(0))),
        ("lzw_early_change_1", "LZWDecode", Some(create_early_change_params(1))),
        ("ascii85_terminator", "ASCII85Decode", None),
    ];

    let fixtures_path = PathBuf::from("tests/stream_decoder/fixtures");

    for (name, filter_name, params) in fixtures {
        println!("\n=== {} ===", name);
        let bin_path = fixtures_path.join(format!("{}.bin", name));
        let expected_path = fixtures_path.join(format!("{}.expected", name));

        let input = fs::read(&bin_path).unwrap();
        let expected = fs::read(&expected_path).unwrap();

        println!("Input: {} bytes", input.len());
        println!("Expected: {} bytes", expected.len());
        println!("Expected hex: {:?}", hex::encode(&expected));

        let decoder = get_decoder(filter_name).unwrap();
        let mut counter = 0;
        let result = decoder.decode(&input, params.as_ref(), &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);

        match result {
            Ok(decoded) => {
                println!("Decoded: {} bytes", decoded.len());
                println!("Decoded hex: {:?}", hex::encode(&decoded));
                if decoded != expected.as_slice() {
                    println!("MISMATCH!");
                    // Show first difference
                    for (i, (&exp, &got)) in expected.iter().zip(decoded.iter()).enumerate() {
                        if exp != got {
                            println!("First difference at byte {}: expected 0x{:02x}, got 0x{:02x}", i, exp, got);
                            break;
                        }
                    }
                } else {
                    println!("MATCH!");
                }
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }

    // Test filter array
    println!("\n=== filter_array_a85_then_flate ===");
    let bin_path = fixtures_path.join("filter_array_a85_then_flate.bin");
    let expected_path = fixtures_path.join("filter_array_a85_then_flate.expected");
    let input = fs::read(&bin_path).unwrap();
    let expected = fs::read(&expected_path).unwrap();

    println!("Input: {} bytes", input.len());
    println!("Expected: {} bytes", expected.len());
    println!("Expected hex: {:?}", hex::encode(&expected));

    let mut current = input;
    let mut counter = 0;

    // First decode ASCII85
    let a85_decoder = ASCII85Decoder;
    match a85_decoder.decode(&current, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES) {
        Ok(decoded) => {
            println!("After ASCII85: {} bytes", decoded.len());
            println!("After ASCII85 hex: {:?}", hex::encode(&decoded));
            current = decoded;
        }
        Err(e) => {
            println!("ASCII85 error: {:?}", e);
            return;
        }
    }

    // Then decode Flate
    let flate_decoder = FlateDecoder;
    match flate_decoder.decode(&current, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES) {
        Ok(decoded) => {
            println!("After Flate: {} bytes", decoded.len());
            println!("After Flate hex: {:?}", hex::encode(&decoded));
            if decoded != expected.as_slice() {
                println!("MISMATCH!");
            } else {
                println!("MATCH!");
            }
        }
        Err(e) => {
            println!("Flate error: {:?}", e);
        }
    }
}

fn get_decoder(name: &str) -> Option<Box<dyn StreamDecoder>> {
    match normalize_filter_name(name) {
        "FlateDecode" => Some(Box::new(FlateDecoder)),
        "LZWDecode" => Some(Box::new(LZWDecoder)),
        "ASCII85Decode" => Some(Box::new(ASCII85Decoder)),
        "ASCIIHexDecode" => Some(Box::new(ASCIIHexDecoder)),
        "Crypt" => Some(Box::new(CryptDecoder)),
        "DCTDecode" => Some(Box::new(DCTDecoder)),
        "JBIG2Decode" => Some(Box::new(PassthroughDecoder::new("JBIG2Decode"))),
        "JPXDecode" => Some(Box::new(JpxStreamDecoder)),
        "CCITTFaxDecode" => Some(Box::new(CCITTFaxDecoder)),
        "RunLengthDecode" => Some(Box::new(RunLengthDecoder)),
        _ => None,
    }
}

fn create_png_predictor_params() -> PdfObject {
    let mut dict = IndexMap::new();
    dict.insert("/Predictor".into(), PdfObject::Integer(15));
    dict.insert("/Columns".into(), PdfObject::Integer(8));
    dict.insert("/Colors".into(), PdfObject::Integer(1));
    dict.insert("/BitsPerComponent".into(), PdfObject::Integer(8));
    PdfObject::Dict(Box::new(dict))
}

fn create_early_change_params(early_change: i64) -> PdfObject {
    let mut dict = IndexMap::new();
    dict.insert("/EarlyChange".into(), PdfObject::Integer(early_change));
    PdfObject::Dict(Box::new(dict))
}
