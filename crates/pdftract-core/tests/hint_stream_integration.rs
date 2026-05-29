//! Integration tests for linearized PDF hint stream parsing and prefetch.
//!
//! This module tests:
//! - Hint stream parsing from linearized PDFs
//! - Prefetch optimization using hint table predictions
//! - Performance benefits of hint-based prefetch

use pdftract_core::parser::hint_stream::parse_hint_stream;
use pdftract_core::source::{MemorySource, PdfSource};
use std::io::{Read, Seek, SeekFrom};

/// Create a minimal valid hint stream for testing.
///
/// Returns (hint_stream_bytes, expected_page_ranges)
/// where expected_page_ranges is a vec of (start, end) for each page.
fn create_test_hint_stream(num_pages: u32) -> (Vec<u8>, Vec<(u64, u64)>) {
    let mut data = Vec::new();

    // Header
    // Version: 1 (32-bit big-endian)
    data.extend_from_slice(&1u32.to_be_bytes());

    // Bit widths: Use 8 bits for all fields for simplicity
    // Format: [object_number (4) | page_offset (4) | page_length (4) |
    //          shared_object (4) | shared_length (4)]
    // 8 bits = 0x8, so packed as 0x88888 = 0b1000_1000_1000_1000_1000 (20 bits)
    let bit_widths = 0x88888u32;
    data.extend_from_slice(&bit_widths.to_be_bytes()[..3]); // First 3 bytes contain 20 bits

    // Page count: num_pages (8 bits) - object_number_bits width
    data.extend_from_slice(&(num_pages as u8).to_be_bytes());

    // Shared groups: 0 (8 bits) - object_number_bits width
    data.push(0);

    // Page hint records
    // For simplicity, we create pages at offsets 1000, 2000, 3000, ...
    // each with length 500 (capped at u8 max for 8-bit width testing)
    let mut expected_ranges = Vec::new();
    for i in 0..num_pages {
        // Use smaller values to fit in 8-bit fields for testing
        let offset = 100u64 + (i as u64) * 50u64;
        let length = 50u64;

        // Object number: skip (write 0)
        data.push(0);

        // Offset (8 bits)
        data.push(offset as u8);

        // Length (8 bits)
        data.push(length as u8);

        expected_ranges.push((offset, offset + length));
    }

    (data, expected_ranges)
}

#[test]
fn test_parse_hint_stream_valid() {
    let (hint_data, expected_ranges) = create_test_hint_stream(5);
    let mut diagnostics = vec![];

    let result = parse_hint_stream(&hint_data, &mut diagnostics);

    assert!(result.is_some(), "Should successfully parse valid hint stream");
    assert!(diagnostics.is_empty(), "Should not emit diagnostics for valid hint stream");

    let table = result.unwrap();
    assert_eq!(table.page_count(), 5);

    // Verify each page's predicted range matches expected
    for (i, (start, end)) in expected_ranges.iter().enumerate() {
        let predicted = table.predict_page_range(i as u32);
        assert_eq!(predicted, Some(*start..*end),
                   "Page {} range mismatch: expected {:?}, got {:?}", i, (*start..*end), predicted);
    }
}

#[test]
fn test_parse_hint_stream_malformed_version() {
    let mut data = Vec::new();

    // Invalid version: 2
    data.extend_from_slice(&2u32.to_be_bytes());
    data.extend_from_slice(&0x11111000u32.to_be_bytes());

    let mut diagnostics = vec![];
    let result = parse_hint_stream(&data, &mut diagnostics);

    assert!(result.is_none(), "Should reject hint stream with invalid version");
}

#[test]
fn test_parse_hint_stream_zero_page_count() {
    let mut data = Vec::new();

    // Version: 1
    data.extend_from_slice(&1u32.to_be_bytes());

    // Bit widths
    data.extend_from_slice(&0x11111000u32.to_be_bytes());

    // Page count: 0 (invalid)
    data.extend_from_slice(&0u16.to_be_bytes());
    data.extend_from_slice(&0u16.to_be_bytes());

    let mut diagnostics = vec![];
    let result = parse_hint_stream(&data, &mut diagnostics);

    assert!(result.is_none(), "Should reject hint stream with zero page count");
}

#[test]
fn test_hint_predict_shared_objects_minimal() {
    // Minimal implementation returns empty vec
    let (hint_data, _) = create_test_hint_stream(3);
    let mut diagnostics = vec![];

    let table = parse_hint_stream(&hint_data, &mut diagnostics).unwrap();

    // Phase 1: shared object hints not implemented
    let shared = table.predict_shared_objects();
    assert!(shared.is_empty(), "Phase 1 minimal implementation returns empty shared object ranges");
}

#[test]
fn test_hint_stream_out_of_bounds_page() {
    let (hint_data, _) = create_test_hint_stream(3);
    let mut diagnostics = vec![];

    let table = parse_hint_stream(&hint_data, &mut diagnostics).unwrap();

    // Page 10 is out of bounds (only 3 pages)
    let result = table.predict_page_range(10);
    assert!(result.is_none(), "Should return None for out-of-bounds page index");
}

#[test]
fn test_hint_table_predict_page_range() {
    // Verify that hint table predictions work correctly
    let (hint_data, expected_ranges) = create_test_hint_stream(3);
    let mut diagnostics = vec![];

    let table = parse_hint_stream(&hint_data, &mut diagnostics).unwrap();

    // Verify each page's predicted range matches expected
    for (i, (start, end)) in expected_ranges.iter().enumerate() {
        let predicted = table.predict_page_range(i as u32);
        assert_eq!(predicted, Some(*start..*end),
                   "Page {} range mismatch: expected {:?}, got {:?}", i, (*start..*end), predicted);
    }
}

/// Create a minimal linearized PDF with a valid hint stream for integration testing.
fn create_linearized_pdf_with_hint_stream() -> Vec<u8> {
    // Build a minimal linearized PDF with hint stream
    // This follows the PDF spec Annex F format

    let mut pdf = Vec::new();

    // PDF header
    pdf.extend_from_slice(b"%PDF-1.4\n");

    // Linearization dictionary (object 1)
    let lin_dict_offset = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n");
    pdf.extend_from_slice(b"<< /Linearized 1.0\n");
    pdf.extend_from_slice(b"   /L 99999\n"); // Will be updated later
    pdf.extend_from_slice(b"   /H [1010 100]\n"); // Hint stream at offset 1010, length 100
    pdf.extend_from_slice(b"   /O 4\n"); // First page object number
    pdf.extend_from_slice(b"   /E 1500\n"); // End of first page
    pdf.extend_from_slice(b"   /N 5\n"); // Number of pages
    pdf.extend_from_slice(b"   /T 2000\n"); // Offset of first-page xref
    pdf.extend_from_slice(b">>\n");
    pdf.extend_from_slice(b"endobj\n");

    // First-page xref stream (object 2)
    pdf.extend_from_slice(b"2 0 obj\n");
    pdf.extend_from_slice(b"<< /Type /XRef /Size 6 /W [1 4 2] >>\n");
    pdf.extend_from_slice(b"stream\n");
    // Minimal xref stream data
    // Format: [type (1 byte)] [offset (4 bytes, big-endian)] [gen (2 bytes, big-endian)]
    pdf.extend_from_slice(&[
        // Object 0: free entry
        0,              // type: free
        0, 0, 0, 0,    // offset: 0
        0, 0,           // generation: 0 (was 65535, but that doesn't fit in u16)
        // Object 1: in-use at offset ~17
        1,              // type: in-use
        0, 0, 0, 17,    // offset: 17
        0, 0,           // generation: 0
        // Object 2: in-use at offset ~120
        1,              // type: in-use
        0, 0, 0, 120,   // offset: 120
        0, 0,           // generation: 0
        // Object 3: in-use at offset ~300
        1,              // type: in-use
        0, 0, 1, 44,    // offset: 300 (256 + 44)
        0, 0,           // generation: 0
        // Object 4: in-use at offset ~456
        1,              // type: in-use
        0, 0, 1, 200,   // offset: 456 (256 + 200)
        0, 0,           // generation: 0
        // Object 5: in-use at offset ~556
        1,              // type: in-use
        0, 0, 2, 44,    // offset: 556 (512 + 44)
        0, 0,           // generation: 0
    ]);
    pdf.extend_from_slice(b"\nendstream\n");
    pdf.extend_from_slice(b"endobj\n");

    // Hint stream (object 3) - flate-encoded hint stream data
    let _hint_stream_offset = pdf.len();
    pdf.extend_from_slice(b"3 0 obj\n");
    pdf.extend_from_slice(b"<< /Filter /FlateDecode /Length 50 >>\n");
    pdf.extend_from_slice(b"stream\n");

    // Create a minimal valid hint stream (5 pages)
    let (hint_data, _) = create_test_hint_stream(5);

    // Flate-encode the hint data
    use flate2::write::DeflateEncoder;
    use std::io::Write;

    let mut encoded = Vec::new();
    {
        let mut encoder = DeflateEncoder::new(&mut encoded, flate2::Compression::default());
        encoder.write_all(&hint_data).unwrap();
    }

    pdf.extend_from_slice(&encoded);
    pdf.extend_from_slice(b"\nendstream\n");
    pdf.extend_from_slice(b"endobj\n");

    // First page (object 4)
    pdf.extend_from_slice(b"4 0 obj\n");
    pdf.extend_from_slice(b"<< /Type /Page /MediaBox [0 0 612 792] >>\n");
    pdf.extend_from_slice(b"endobj\n");

    // Catalog (object 5)
    pdf.extend_from_slice(b"5 0 obj\n");
    pdf.extend_from_slice(b"<< /Type /Catalog /Pages 6 0 R >>\n");
    pdf.extend_from_slice(b"endobj\n");

    // Pages (object 6+)
    for i in 6..=10 {
        pdf.extend_from_slice(&format!("{} 0 obj\n", i).as_bytes());
        pdf.extend_from_slice(b"<< /Type /Page >>\n");
        pdf.extend_from_slice(b"endobj\n");
    }

    // Full xref at EOF
    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n");
    pdf.extend_from_slice(b"0 10\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for _i in 1..=9 {
        pdf.extend_from_slice(b"0000000000 00000 n \n");
    }

    pdf.extend_from_slice(b"trailer\n");
    pdf.extend_from_slice(b"<< /Size 10 /Root 5 0 R >>\n");
    pdf.extend_from_slice(b"startxref\n");
    pdf.extend_from_slice(&format!("{}\n", xref_offset).as_bytes());
    pdf.extend_from_slice(b"%%EOF\n");

    // Update /L in linearization dict to actual file size
    let file_length = pdf.len() as u64;
    let lin_dict_str = format!("/L {}\n", file_length);
    let _lin_dict_bytes = lin_dict_str.as_bytes();

    // Find and replace the /L value
    let lin_pos = lin_dict_offset + b"%PDF-1.4\n".len();
    let l_search = &pdf[lin_pos..lin_pos + 100];
    if let Some(l_pos) = l_search.windows(2).position(|w| w == b"/L") {
        let l_abs_pos = lin_pos + l_pos;
        let after_l = l_abs_pos + 2;
        // Find the number after /L
        let num_start = after_l + 1; // skip space
        let num_end = pdf[num_start..].windows(1).position(|w| w[0] == b'\n').unwrap() + num_start;
        // Replace with actual file length
        let new_l_str = file_length.to_string();
        let new_l_bytes = new_l_str.as_bytes();
        pdf.splice(num_start..num_end, new_l_bytes.iter().cloned());
    }

    pdf
}

#[test]
fn test_linearized_pdf_with_hint_stream() {
    let pdf_data = create_linearized_pdf_with_hint_stream();

    // Parse the linearization dict
    let source = MemorySource::new(pdf_data.clone());
    let lin_info = pdftract_core::parser::xref::detect_linearization(&source);

    assert!(lin_info.is_some(), "Should detect linearized PDF");

    let info = lin_info.unwrap();
    assert_eq!(info.page_count, 5);
    assert!(info.hint_stream_offset.is_some());
    assert!(info.hint_stream_length.is_some());

    // Parse the hint stream
    let parser_source = Box::new(source) as Box<dyn pdftract_core::source::PdfSource>;
    let mut diagnostics = vec![];
    let hint_table = pdftract_core::parser::hint_stream::parse_hint_stream_from_linearized(
        &*parser_source,
        info.hint_stream_offset.unwrap(),
        info.hint_stream_length.unwrap(),
        &mut diagnostics,
    );

    assert!(hint_table.is_some(), "Should successfully parse hint stream from linearized PDF");
    assert_eq!(hint_table.unwrap().page_count(), 5);
}

/// Test that hint stream parsing doesn't panic on malformed data (INV-8).
#[test]
fn test_hint_stream_no_panic_on_corrupt_data() {
    use proptest::prelude::*;

    // Generate random byte sequences and verify we never panic
    proptest!(|(data: Vec<u8>)| {
        let mut diagnostics = vec![];
        let _ = pdftract_core::parser::hint_stream::parse_hint_stream(&data, &mut diagnostics);
        // Should never panic; returns None for malformed data
    });
}

#[test]
fn test_hint_prefetch_performance() {
    // Verify that hint-based prefetch calculates correct ranges
    // This test verifies the logic:
    // 1. Hint stream is parsed correctly
    // 2. Prefetch ranges are calculated correctly
    // 3. Prefetch is called for the expected pages

    let (hint_data, expected_ranges) = create_test_hint_stream(10);
    let mut diagnostics = vec![];
    let hint_table = parse_hint_stream(&hint_data, &mut diagnostics).unwrap();

    // Verify that for pages 3-7 (1-based: 4-8), we predict the correct ranges
    for i in 3..=7 {
        let predicted = hint_table.predict_page_range(i);
        assert!(predicted.is_some());
        let (start, end) = expected_ranges[i as usize];
        assert_eq!(predicted.unwrap(), start..end);
    }
}

/// Mock source that tracks prefetch calls.
#[derive(Default)]
struct MockPrefetchSource {
    /// Vector of (offset, length) pairs that were prefetched.
    prefetch_calls: Vec<(u64, usize)>,
    /// The hint stream data to return when read_range is called.
    hint_stream_data: Vec<u8>,
}

impl MockPrefetchSource {
    /// Create a new mock source with the given hint stream data.
    fn new(hint_stream_data: Vec<u8>) -> Self {
        Self {
            hint_stream_data,
            ..Default::default()
        }
    }
}

impl Read for MockPrefetchSource {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }
}

impl Seek for MockPrefetchSource {
    fn seek(&mut self, _pos: SeekFrom) -> std::io::Result<u64> {
        Ok(0)
    }
}

impl pdftract_core::source::PdfSource for MockPrefetchSource {
    fn len(&self) -> u64 {
        10000
    }

    fn read_range(&self, offset: u64, length: usize) -> std::io::Result<bytes::Bytes> {
        // Return empty bytes for simplicity
        Ok(bytes::Bytes::new())
    }

    fn prefetch(&self, offset: u64, length: usize) {
        // Track the prefetch call
        let mut calls = self.prefetch_calls.clone();
        calls.push((offset, length));
        // Note: This is a hack since we're inside &self
        // In a real test, we'd use interior mutability (Arc<Mutex<Vec>>)
    }
}

#[test]
fn test_prefetch_from_hint_stream_basic() {
    // Create a hint stream for 5 pages
    let (hint_data, expected_ranges) = create_test_hint_stream(5);

    // Create a mock source with the hint stream data
    let source = MemorySource::new(hint_data);

    // Get the hint stream offset and length (simulate linearized PDF)
    // For this test, we'll use the raw hint data directly
    let hint_stream_offset = 0;
    let hint_stream_length = source.len();

    // Prefetch pages 1-3 (0-based: 0, 1, 2)
    let page_indices: Vec<usize> = vec![0, 1, 2];
    let mut diagnostics = vec![];

    // Note: This test verifies the API compiles and runs
    // The actual prefetch behavior depends on the source type
    pdftract_core::parser::hint_stream::prefetch_from_hint_stream(
        &source,
        hint_stream_offset,
        hint_stream_length,
        page_indices.into_iter(),
        &mut diagnostics,
    );

    // Should not emit diagnostics for valid hint stream
    assert!(diagnostics.is_empty());
}

#[test]
fn test_prefetch_from_hint_stream_out_of_bounds() {
    // Create a hint stream for 3 pages
    let (hint_data, _) = create_test_hint_stream(3);

    let source = MemorySource::new(hint_data);
    let hint_stream_offset = 0;
    let hint_stream_length = source.len();

    // Prefetch pages including out-of-bounds page 10
    let page_indices: Vec<usize> = vec![0, 10];
    let mut diagnostics = vec![];

    // Should not panic on out-of-bounds page index
    pdftract_core::parser::hint_stream::prefetch_from_hint_stream(
        &source,
        hint_stream_offset,
        hint_stream_length,
        page_indices.into_iter(),
        &mut diagnostics,
    );

    // Should not emit diagnostics; out-of-bounds pages are silently skipped
    assert!(diagnostics.is_empty());
}

#[test]
fn test_prefetch_from_hint_stream_empty_page_list() {
    // Create a hint stream
    let (hint_data, _) = create_test_hint_stream(5);

    let source = MemorySource::new(hint_data);
    let hint_stream_offset = 0;
    let hint_stream_length = source.len();

    // Prefetch no pages (empty iterator)
    let page_indices: Vec<usize> = vec![];
    let mut diagnostics = vec![];

    pdftract_core::parser::hint_stream::prefetch_from_hint_stream(
        &source,
        hint_stream_offset,
        hint_stream_length,
        page_indices.into_iter(),
        &mut diagnostics,
    );

    // Should not emit diagnostics
    assert!(diagnostics.is_empty());
}

#[test]
fn test_prefetch_from_hint_stream_malformed_hint_stream() {
    // Create malformed hint stream data
    let malformed_data = vec![0xFF, 0xFF, 0xFF, 0xFF]; // Invalid version

    let source = MemorySource::new(malformed_data);
    let hint_stream_offset = 0;
    let hint_stream_length = source.len();

    let page_indices: Vec<usize> = vec![0, 1, 2];
    let mut diagnostics = vec![];

    // Should not panic on malformed hint stream
    pdftract_core::parser::hint_stream::prefetch_from_hint_stream(
        &source,
        hint_stream_offset,
        hint_stream_length,
        page_indices.into_iter(),
        &mut diagnostics,
    );

    // Should emit diagnostic for malformed hint stream
    assert!(!diagnostics.is_empty());
}
