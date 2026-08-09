//! Test verification for extract_content_stream_bytes function
//!
//! This test verifies that the extract_content_stream_bytes function:
//! 1. Extracts bytes from stream dictionary
//! 2. Extracts bytes from direct byte array (String)
//! 3. Extracts bytes from byte array (Array of integers 0-255)
//! 4. Handles both compressed and uncompressed streams
//! 5. Returns raw bytes without executing/drawing

use pdftract_core::content_stream::extract_content_stream_bytes;
use pdftract_core::parser::lexer::Lexer;
use pdftract_core::parser::object::{PdfDict, PdfObject, PdfStream};
use pdftract_core::parser::stream::{ExtractionOptions, PdfSource};
use std::sync::Arc;

/// Minimal PdfSource implementation for testing
struct TestPdfSource {
    data: Vec<u8>,
}

impl TestPdfSource {
    fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl PdfSource for TestPdfSource {
    fn read_at(&self, offset: u64, len: usize) -> Result<Vec<u8>, std::io::Error> {
        let start = offset as usize;
        let end = (start + len).min(self.data.len());
        if start >= self.data.len() {
            Ok(Vec::new())
        } else {
            Ok(self.data[start..end].to_vec())
        }
    }

    fn total_len(&self) -> u64 {
        self.data.len() as u64
    }
}

#[test]
fn test_extract_from_direct_string() {
    // Test Case 1: Direct byte array (String object)
    let obj = PdfObject::String(Box::new(b"BT (Hello) Tj ET".to_vec()));
    let bytes = extract_content_stream_bytes(&obj, None, None, None).unwrap();
    assert_eq!(bytes, b"BT (Hello) Tj ET");
}

#[test]
fn test_extract_from_byte_array() {
    // Test Case 2: Byte array (Array of integers 0-255)
    let byte_values: Vec<PdfObject> = vec
![
        72u8, 101, 108, 108, 111 // "Hello"
    ].into_iter().map(|b| PdfObject::Integer(b as i64)).collect();
    let obj = PdfObject::Array(Box::new(byte_values));
    let bytes = extract_content_stream_bytes(&obj, None, None, None).unwrap();
    assert_eq!(bytes, b"Hello");
}

#[test]
fn test_extract_from_uncompressed_stream() {
    // Test Case 3: Uncompressed stream
    let content = b"BT /F1 12 Tf 100 700 Td (Hello) Tj ET".to_vec();

    // Create a minimal stream dictionary
    let mut dict = PdfDict::new();
    dict.insert("/Length".into(), PdfObject::Integer(content.len() as i64));

    // Create a stream that starts at offset 0
    let stream = PdfStream::new(dict, 0, Some(content.len() as u64));
    let obj = PdfObject::Stream(Box::new(stream));

    // Create source with the content
    let source = TestPdfSource::new(content);
    let opts = ExtractionOptions {
        max_decompress_bytes: 1_000_000,
        password: None,
    };
    let mut counter = 0u64;

    let bytes = extract_content_stream_bytes(&obj, Some(&source), Some(&opts), Some(&mut counter)).unwrap();
    assert_eq!(bytes, b"BT /F1 12 Tf 100 700 Td (Hello) Tj ET");
}

#[test]
fn test_extract_from_compressed_stream() {
    // Test Case 4: Compressed stream (FlateDecode)
    use std::io::Write;

    // Create some content
    let content = b"BT /F1 12 Tf 100 700 Td (Compressed) Tj ET";

    // Compress it using flate
    let mut compressed = Vec::new();
    {
        let mut encoder = flate2::write::DeflateEncoder::new(&mut compressed, flate2::Compression::default());
        encoder.write_all(content).unwrap();
        encoder.finish().unwrap();
    }

    // Create stream dictionary with FlateDecode filter
    let mut dict = PdfDict::new();
    dict.insert("/Length".into(), PdfObject::Integer(compressed.len() as i64));
    dict.insert("/Filter".into(), PdfObject::Name(Arc::from("FlateDecode")));

    // Create stream
    let stream = PdfStream::new(dict, 0, Some(compressed.len() as u64));
    let obj = PdfObject::Stream(Box::new(stream));

    // Create source with compressed data
    let source = TestPdfSource::new(compressed);
    let opts = ExtractionOptions {
        max_decompress_bytes: 1_000_000,
        password: None,
    };
    let mut counter = 0u64;

    let bytes = extract_content_stream_bytes(&obj, Some(&source), Some(&opts), Some(&mut counter)).unwrap();
    assert_eq!(bytes, content);
}

#[test]
fn test_extract_from_invalid_type() {
    // Test Case 5: Invalid object type
    let obj = PdfObject::Integer(42);
    let result = extract_content_stream_bytes(&obj, None, None, None);
    assert!(result.is_err());
}

#[test]
fn test_extract_from_array_with_non_byte_values() {
    // Test Case 6: Array with values outside 0-255 range
    let arr = vec![
        PdfObject::Integer(72),
        PdfObject::Integer(300), // Invalid: > 255
        PdfObject::Integer(108),
    ];
    let obj = PdfObject::Array(Box::new(arr));
    let result = extract_content_stream_bytes(&obj, None, None, None);
    assert!(result.is_err());
}
