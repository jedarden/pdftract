//! Property-based tests for PDF stream decoder filters and filter pipelines.
//!
//! This module tests the core invariants of PDF stream decoding:
//! - No panic on any input (INV-8)
//! - Roundtrip correctness for encodable filters
//! - Bomb limit enforcement
//! - Filter pipeline ordering

use pdftract_core::parser::stream::{
    FlateDecoder, LZWDecoder, ASCII85Decoder, ASCIIHexDecoder, RunLengthDecoder,
    DCTDecoder, JpxStreamDecoder, CCITTFaxDecoder, CryptDecoder,
    DEFAULT_MAX_DECOMPRESS_BYTES,
};
use indexmap::IndexMap;
use pdftract_core::parser::object::{PdfObject, PdfDict};
use pdftract_core::diagnostics::DiagCode;

/// Property: Filter pipeline never panics on arbitrary input.
///
/// Tests each filter with random byte inputs to ensure INV-8 compliance.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_filter_pipeline_never_panics(
        filter in 0usize..8usize,
        data in proptest::collection::vec(proptest::num::u8::ANY, 0..100_000)
    ) {
        let mut counter = 0;

        // Test each filter type
        let result = match filter {
            0 => FlateDecoder.decode(&data, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES),
            1 => LZWDecoder.decode(&data, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES),
            2 => ASCII85Decoder.decode(&data, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES),
            3 => ASCIIHexDecoder.decode(&data, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES),
            4 => RunLengthDecoder.decode(&data, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES),
            5 => DCTDecoder.decode(&data, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES),
            6 => JpxStreamDecoder.decode(&data, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES),
            7 => CCITTFaxDecoder.decode(&data, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES),
            _ => unreachable!(),
        };

        // Should never panic - may return Ok or Err
        prop_assert!(result.is_ok() || result.is_err());
    }
}

/// Property: FlateDecode roundtrip - encode then decode produces original.
///
/// Uses flate2's ZlibEncoder to encode, then FlateDecoder to decode.
/// The output should be byte-identical to the input.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_flate_roundtrip(
        data in proptest::collection::vec(proptest::num::u8::ANY, 0..50_000)
    ) {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        // Encode with flate2 (zlib format)
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&data).unwrap();
        let encoded = encoder.finish().unwrap();

        // Decode with our FlateDecoder (handles zlib format)
        let mut counter = 0;
        let result = FlateDecoder.decode(&encoded, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);

        prop_assert!(result.is_ok());
        let decoded = result.unwrap();

        // Should round-trip perfectly
        prop_assert_eq!(decoded, data);
    }
}

/// Property: ASCII85Decode roundtrip - encode then decode produces original.
///
/// Uses a custom ASCII85 encoder to encode, then ASCII85Decoder to decode.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_a85_roundtrip(
        data in proptest::collection::vec(proptest::num::u8::ANY, 0..10_000)
    ) {
        let encoded = ascii85_encode(&data);

        // Decode with our ASCII85Decoder
        let mut counter = 0;
        let result = ASCII85Decoder.decode(&encoded, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);

        prop_assert!(result.is_ok());
        let decoded = result.unwrap();

        // Should round-trip perfectly
        prop_assert_eq!(decoded, data);
    }
}

/// Property: RunLengthDecode roundtrip - encode then decode produces original.
///
/// Uses a custom RunLength encoder following the PDF spec.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_runlength_roundtrip(
        data in proptest::collection::vec(proptest::num::u8::ANY, 0..10_000)
    ) {
        let encoded = runlength_encode(&data);

        // Decode with our RunLengthDecoder
        let mut counter = 0;
        let result = RunLengthDecoder.decode(&encoded, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);

        prop_assert!(result.is_ok());
        let decoded = result.unwrap();

        // Should round-trip perfectly
        prop_assert_eq!(decoded, data);
    }
}

/// Property: Bomb limit enforced for synthetic FlateDecode bombs.
///
/// Creates synthetic FlateDecode bombs of varying sizes and verifies
/// that the output is capped at max_decompress_bytes.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_bomb_limit_enforced(
        // Size of bomb in MB (10, 100, 1000)
        size_mb in 10usize..1000usize,
        // Bomb limit in bytes
        bomb_limit in 100_000u64..10_000_000_000u64,
    ) {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        // Create a pattern that compresses well (repeated bytes)
        // 1 MB of zeros compresses to ~1 KB
        let repeat_count = size_mb * 1024 * 1024;
        let pattern = vec![0u8; repeat_count.min(50_000_000)]; // Cap at 50MB to avoid timeout

        // Encode with flate2
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(&pattern).unwrap();
        let encoded = encoder.finish().unwrap();

        // Decode with bomb limit
        let mut counter = 0;
        let result = FlateDecoder.decode(&encoded, None, &mut counter, bomb_limit);

        prop_assert!(result.is_ok());
        let decoded = result.unwrap();

        // Output should not exceed bomb limit significantly
        // (allowing small margin for chunk processing)
        prop_assert!(
            decoded.len() as u64 <= bomb_limit + 100_000,
            "Decoded {} bytes exceeds bomb limit {} by more than 100KB",
            decoded.len(),
            bomb_limit
        );
    }
}

/// Helper: Encode bytes in ASCII85 format (Base85).
fn ascii85_encode(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len() / 4 * 5 + 10);
    result.push(b'<');
    result.push(b'~');

    let mut chunk = [0u8; 4];
    for (i, &byte) in data.iter().enumerate() {
        chunk[i % 4] = byte;

        if i % 4 == 3 || i == data.len() - 1 {
            // Process this chunk
            let chunk_len = if i == data.len() - 1 { (i % 4) + 1 } else { 4 };

            // Check for all zeros (use 'z' shortcut)
            if chunk_len == 4 && chunk.iter().all(|&b| b == 0) {
                result.push(b'z');
                chunk = [0; 4];
                continue;
            }

            // Convert to 32-bit number
            let value = u32::from_be_bytes(chunk);

            // Encode in base85
            for j in (0..5).rev() {
                let divisor = 85u32.pow(j as u32);
                let encoded_char = (value / divisor) % 85;
                result.push(encoded_char as u8 + 33);
            }
            chunk = [0; 4];
        }
    }

    result.push(b'~');
    result.push(b'>');
    result
}

/// Helper: Encode bytes using RunLength encoding (PDF spec).
fn runlength_encode(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut i = 0;

    while i < data.len() {
        // Look ahead for repeated bytes
        let current_byte = data[i];
        let mut repeat_count = 1;

        while i + repeat_count < data.len() && data[i + repeat_count] == current_byte && repeat_count < 127 {
            repeat_count += 1;
        }

        if repeat_count >= 3 {
            // Use run-length encoding for 3+ repeats
            // 257 - repeat_count = length byte
            let len_byte = (257 - repeat_count) as u8;
            result.push(len_byte);
            result.push(current_byte);
            i += repeat_count;
        } else {
            // Look ahead for non-repeating bytes
            let literal_start = i;
            let mut literal_len = 0;

            while i + literal_len < data.len() && literal_len < 127 {
                // Check if next byte would repeat (start of a run)
                if i + literal_len + 2 < data.len()
                    && data[i + literal_len] == data[i + literal_len + 1]
                    && data[i + literal_len] == data[i + literal_len + 2]
                {
                    break;
                }
                literal_len += 1;
            }

            // Encode as literal copy
            if literal_len > 0 {
                let len_byte = (literal_len - 1) as u8; // len+1 bytes -> len is len-1
                result.push(len_byte);
                result.extend_from_slice(&data[literal_start..literal_start + literal_len]);
                i += literal_len;
            } else {
                // Single byte as literal
                result.push(0); // len=0 means copy 1 byte
                result.push(current_byte);
                i += 1;
            }
        }
    }

    // End of data marker
    result.push(128);

    result
}
