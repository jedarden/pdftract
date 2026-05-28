use pdftract_core::parser::stream::{FlateDecoder, StreamDecoder};
use pdftract_core::parser::object::{PdfObject, PdfDict};
use indexmap::IndexMap;

fn main() {
    let input = vec![0x78, 0x9c, 0xe3, 0x0e, 0x92, 0xe5, 0xd8, 0xf9, 0x8f, 0x81, 0x81, 0x81, 0x07, 0x88, 0x19, 0x81, 0x98, 0x81, 0x37, 0x88, 0x9f, 0xe5, 0x1e, 0x48, 0x84, 0x2f, 0x08, 0x2a, 0xc2, 0x15, 0x94, 0x5f, 0x6e, 0xa2, 0x07, 0x04, 0xfc, 0x40, 0x86, 0x29, 0x88, 0x01, 0x00, 0xf0, 0xe0, 0x09, 0x58];
    
    let mut dict = IndexMap::new();
    dict.insert("/Predictor".into(), PdfObject::Integer(15));
    dict.insert("/Columns".into(), PdfObject::Integer(8));
    dict.insert("/Colors".into(), PdfObject::Integer(1));
    dict.insert("/BitsPerComponent".into(), PdfObject::Integer(8));
    let params = PdfObject::Dict(Box::new(dict));
    
    let mut counter = 0u64;
    let result = FlateDecoder.decode(&input, Some(&params), &mut counter, 100_000_000);
    
    match result {
        Ok(output) => {
            println!("Decoded: {:02x?}", output);
            println!("Decoded ASCII: {:?}", String::from_utf8_lossy(&output));
            println!("Length: {}", output.len());
        }
        Err(e) => println!("Error: {:?}", e),
    }
}
