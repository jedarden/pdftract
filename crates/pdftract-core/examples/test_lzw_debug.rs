use pdftract_core::parser::stream::{LZWDecoder, DEFAULT_MAX_DECOMPRESS_BYTES, StreamDecoder};
use indexmap::IndexMap;
use pdftract_core::parser::object::PdfObject;

fn main() {
    let input = vec![0x08, 0x80, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x57, 0x6f, 0x72, 0x6c, 0x64];
    
    let mut dict = IndexMap::new();
    dict.insert("/EarlyChange".into(), PdfObject::Integer(0));
    let params = PdfObject::Dict(Box::new(dict));
    
    let mut counter = 0;
    let result = LZWDecoder.decode(&input, Some(&params), &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    
    match result {
        Ok(data) => {
            println!("Success! Decoded {} bytes", data.len());
            println!("Decoded: {:?}", String::from_utf8_lossy(&data));
            println!("Hex: {:02x?}", data);
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}
