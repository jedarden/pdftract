use pdftract_core::parser::stream::FlateDecoder;
use pdftract_core::parser::object::PdfObject;
use indexmap::IndexMap;

fn main() {
    let input = std::fs::read("tests/stream_decoder/fixtures/flate_png_pred15_all_six.bin")
        .expect("fixture should exist");
    let expected = std::fs::read("tests/stream_decoder/fixtures/flate_png_pred15_all_six.expected")
        .expect("expected should exist");

    // Create PNG predictor params: predictor=15, columns=8, colors=1, bits=8
    let mut dict = IndexMap::new();
    dict.insert("/Predictor".into(), PdfObject::Integer(15));
    dict.insert("/Columns".into(), PdfObject::Integer(8));
    dict.insert("/Colors".into(), PdfObject::Integer(1));
    dict.insert("/BitsPerComponent".into(), PdfObject::Integer(8));
    let params = PdfObject::Dict(Box::new(dict));

    let mut counter = 0u64;
    let result = FlateDecoder.decode(&input, Some(&params), &mut counter, u64::MAX);

    match result {
        Ok(decoded) => {
            eprintln!("Expected: {:?}", String::from_utf8_lossy(&expected));
            eprintln!("Got:      {:?}", String::from_utf8_lossy(&decoded));
            eprintln!("Expected bytes: {:?}", expected);
            eprintln!("Got bytes:      {:?}", decoded.as_slice());

            if decoded == expected.as_slice() {
                eprintln!("SUCCESS: Output matches!");
            } else {
                eprintln!("FAILURE: Output does not match!");
                for (i, (exp, got)) in expected.iter().zip(decoded.iter()).enumerate() {
                    if exp != got {
                        eprintln!("  Mismatch at byte {}: expected 0x{:02x}, got 0x{:02x}", i, exp, got);
                    }
                }
                if decoded.len() != expected.len() {
                    eprintln!("  Length mismatch: expected {}, got {}", expected.len(), decoded.len());
                }
            }
        }
        Err(e) => {
            eprintln!("Decode error: {}", e);
        }
    }
}
