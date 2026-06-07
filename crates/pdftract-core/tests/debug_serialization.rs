// Quick test to understand serialization format
use pdftract_core::fingerprint::canonicalize::serialize_dict_canonical;
use pdftract_core::parser::object::{PdfDict, PdfObject};
use std::sync::Arc;

#[test]
fn debug_serialization() {
    let mut dict = PdfDict::new();
    dict.insert(Arc::from("/Z"), PdfObject::Integer(3));
    dict.insert(Arc::from("/A"), PdfObject::Integer(1));
    dict.insert(Arc::from("/M"), PdfObject::Integer(2));

    let bytes = serialize_dict_canonical(&dict);
    println!("serialize_dict_canonical output: {}", String::from_utf8_lossy(&bytes));
    println!("bytes: {:?}", bytes);
}
