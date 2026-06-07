// Quick test to understand serialization format
use pdftract_core::fingerprint::canonicalize::{serialize_dict_canonical, serialize_object_canonical};
use pdftract_core::types::objects::{PdfDict, PdfObject};
use std::sync::Arc;

fn main() {
    let mut dict = PdfDict::new();
    dict.insert(Arc::from("/Z"), PdfObject::Integer(3));
    dict.insert(Arc::from("/A"), PdfObject::Integer(1));
    dict.insert(Arc::from("/M"), PdfObject::Integer(2));

    let bytes = serialize_dict_canonical(&dict);
    println!("serialize_dict_canonical output: {}", String::from_utf8_lossy(&bytes));
    println!("bytes: {:?}", bytes);

    println!("\n--- serialize_object_canonical ---");
    let mut result = Vec::new();
    serialize_object_canonical(&mut result, &PdfObject::Dict(Box::new(dict)));
    println!("serialize_object_canonical output: {}", String::from_utf8_lossy(&result));
    println!("bytes: {:?}", result);
}
