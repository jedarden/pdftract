use pdftract_core::parser::stream::LZWDecoder;
use pdftract_core::parser::object::{PdfObject, PdfDict};
use indexmap::IndexMap;
use std::sync::Arc;

#[test]
fn debug_lzw_fixtures() {
    let data = [0x08, 0x80, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x57, 0x6f, 0x72, 0x6c, 0x64];
    
    println!("Testing LZW early_change=1 (default)");
    let mut counter = 0;
    let result = LZWDecoder.decode(&data, None, &mut counter, 1000000);
    println!("Result: {:?}", result);
    if let Ok(bytes) = result {
        println!("Decoded: {:?}", bytes);
        println!("Decoded as string: {:?}", String::from_utf8(bytes.clone()));
    }
    
    println!("\nTesting LZW early_change=0");
    let mut counter2 = 0;
    let mut params = IndexMap::new();
    params.insert(Arc::from("/EarlyChange"), PdfObject::Integer(0));
    let result2 = LZWDecoder.decode(&data, Some(&PdfObject::Dict(Box::new(params))), &mut counter2, 1000000);
    println!("Result: {:?}", result2);
    if let Ok(bytes) = result2 {
        println!("Decoded: {:?}", bytes);
        println!("Decoded as string: {:?}", String::from_utf8(bytes.clone()));
    }
}
