#[allow(unused_imports)]
use pdftract_core::parser::stream::{LZWDecoder, StreamDecoder};
use pdftract_core::parser::object::{PdfObject, PdfDict};
use indexmap::IndexMap;
use std::sync::Arc;

#[test]
fn test_lzw_debug() {
    // Test with lzw_early_change_0.bin data
    // 08 80 48 65 6c 6c 6f 57 6f 72 6c 64
    let input = vec![0x08, 0x80, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x57, 0x6f, 0x72, 0x6c, 0x64];
    
    let mut params = IndexMap::new();
    params.insert(Arc::from("/EarlyChange"), PdfObject::Integer(0));
    
    let mut counter = 0;
    let decoder = LZWDecoder;
    let result = decoder.decode(&input, Some(&PdfObject::Dict(Box::new(params))), &mut counter, u64::MAX);
    
    match result {
        Ok(data) => {
            println!("Decoded {} bytes: {:?}", data.len(), String::from_utf8_lossy(&data));
        }
        Err(e) => println!("Error: {:?}", e),
    }
}
