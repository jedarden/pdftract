//! Property-based tests for the PDF stream decoder.
//!
//! These tests verify that the stream decoder maintains its core invariants
//! across all possible inputs, following INV-8 (no panic at public boundary).

use pdftract_core::parser::stream::{
    FlateDecoder, ASCII85Decoder, ASCIIHexDecoder, LZWDecoder, RunLengthDecoder,
    DEFAULT_MAX_DECOMPRESS_BYTES,
};
use indexmap::IndexMap;
use pdftract_core::parser::object::{PdfObject, PdfDict, PdfStream};

/// Property: FlateDecoder never panics on random input.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_flate_decode_never_panics(
        data in proptest::collection::vec(proptest::num::u8::ANY, 0..100_000)
    ) {
        let mut counter = 0;
        // Any random input should not panic FlateDecode
        let _ = FlateDecoder.decode(&data, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    }
}

/// Property: FlateDecoder with predictor never panics on random input.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_flate_decode_with_predictor_never_panics(
        data in proptest::collection::vec(proptest::num::u8::ANY, 0..50_000),
        predictor in 1i32..16i32,
        columns in 1i32..100i32,
        colors in 1i32..5i32,
        bits_per_component in 1i32..17i32
    ) {
        let mut dict = IndexMap::new();
        dict.insert("/Predictor".into(), PdfObject::Integer(predictor as i64));
        dict.insert("/Columns".into(), PdfObject::Integer(columns as i64));
        dict.insert("/Colors".into(), PdfObject::Integer(colors as i64));
        dict.insert("/BitsPerComponent".into(), PdfObject::Integer(bits_per_component as i64));

        let params = Some(PdfObject::Dict(Box::new(dict)));
        let mut counter = 0;

        // Should not panic even with invalid predictor data
        let _ = FlateDecoder.decode(&data, params.as_ref(), &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    }
}

/// Property: FlateDecoder bomb limit enforcement never panics.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_flate_decode_bomb_limit_no_panic(
        data in proptest::collection::vec(proptest::num::u8::ANY, 0..100_000),
        bomb_limit in 0u64..1_000_000u64
    ) {
        let mut counter = 0;
        // Any bomb limit should not cause panic
        let _ = FlateDecoder.decode(&data, None, &mut counter, bomb_limit);
    }
}

/// Property: ASCII85Decoder never panics on random input.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_ascii85_decode_never_panics(
        data in proptest::collection::vec(proptest::num::u8::ANY, 0..100_000)
    ) {
        let mut counter = 0;
        // Any random input should not panic ASCII85Decode
        let _ = ASCII85Decoder.decode(&data, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    }
}

/// Property: ASCIIHexDecoder never panics on random input.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_asciihex_decode_never_panics(
        data in proptest::collection::vec(proptest::num::u8::ANY, 0..100_000)
    ) {
        let mut counter = 0;
        // Any random input should not panic ASCIIHexDecode
        let _ = ASCIIHexDecoder.decode(&data, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    }
}

/// Property: LZWDecoder never panics on random input.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_lzw_decode_never_panics(
        data in proptest::collection::vec(proptest::num::u8::ANY, 0..100_000)
    ) {
        let mut counter = 0;
        // Any random input should not panic LZWDecode
        let _ = LZWDecoder.decode(&data, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    }
}

/// Property: Decoded bytes never exceed bomb limit.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_decoded_bytes_within_bomb_limit(
        data in proptest::collection::vec(proptest::num::u8::ANY, 0..50_000),
        bomb_limit in 100u64..10_000u64
    ) {
        let mut counter = 0;
        let result = FlateDecoder.decode(&data, None, &mut counter, bomb_limit);

        prop_assert!(result.is_ok());
        let decoded = result.unwrap();

        // Decoded output should not exceed bomb limit
        prop_assert!((decoded.len() as u64) <= bomb_limit + 1000,
            "Decoded {} bytes exceeds bomb limit {} with significant margin",
            decoded.len(), bomb_limit);

        // Counter should also not exceed bomb limit significantly
        prop_assert!(counter <= bomb_limit + 1000,
            "Counter {} exceeds bomb limit {} with significant margin",
            counter, bomb_limit);
    }
}

/// Property: Empty input always produces empty output.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_empty_input_empty_output() {
        let empty: Vec<u8> = vec![];
        let mut counter = 0;

        let result = FlateDecoder.decode(&empty, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
        prop_assert!(result.is_ok());
        prop_assert_eq!(result.unwrap(), empty);

        let result = ASCII85Decoder.decode(&empty, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
        prop_assert!(result.is_ok());
        prop_assert_eq!(result.unwrap(), empty);

        let result = ASCIIHexDecoder.decode(&empty, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
        prop_assert!(result.is_ok());
        prop_assert_eq!(result.unwrap(), empty);
    }
}

/// Property: Zero bomb limit always produces empty output.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_zero_bomb_limit_empty_output(
        data in proptest::collection::vec(proptest::num::u8::ANY, 0..10_000)
    ) {
        let mut counter = 0;
        let bomb_limit: u64 = 0;

        let result = FlateDecoder.decode(&data, None, &mut counter, bomb_limit);
        prop_assert!(result.is_ok());
        prop_assert_eq!(result.unwrap().len(), 0);

        let result = ASCII85Decoder.decode(&data, None, &mut counter, bomb_limit);
        prop_assert!(result.is_ok());
        prop_assert_eq!(result.unwrap().len(), 0);
    }
}

/// Property: Decoder is idempotent for valid compressed data.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_valid_decode_reproducible(
        data in proptest::collection::vec(proptest::num::u8::ANY, 0..1000)
    ) {
        // Compress the data first
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&data).unwrap();
        let compressed = encoder.finish().unwrap();

        // Decode twice and compare
        let mut counter1 = 0;
        let result1 = FlateDecoder.decode(&compressed, None, &mut counter1, DEFAULT_MAX_DECOMPRESS_BYTES);

        let mut counter2 = 0;
        let result2 = FlateDecoder.decode(&compressed, None, &mut counter2, DEFAULT_MAX_DECOMPRESS_BYTES);

        prop_assert_eq!(result1, result2);
        prop_assert_eq!(counter1, counter2);
    }
}

/// Property: ASCII85 'z' shortcut always produces 4 zero bytes.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_ascii85_z_shortcut(
        prefix in proptest::collection::vec(proptest::num::u8::ANY, 0..100),
        suffix in proptest::collection::vec(proptest::num::u8::ANY, 0..100)
    ) {
        let mut input = prefix;
        input.push(b'z');
        input.extend_from_slice(&suffix);

        let mut counter = 0;
        let result = ASCII85Decoder.decode(&input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);

        prop_assert!(result.is_ok());
        // The 'z' should decode to 4 zeros
        let decoded = result.unwrap();
        prop_assert!(decoded.len() >= 4);
        prop_assert_eq!(&decoded[0..4], &[0u8; 4]);
    }
}

/// Property: PredictorParams from_pdf_object never panics.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_predictor_params_never_panics(
        predictor in proptest::option::of(1i32..20i32),
        columns in proptest::option::of(0i32..1000i32),
        colors in proptest::option::of(0i32::PROPTEST_MAXNUM(10i32)),
        bits_per_component in proptest::option::of(0i32..32i32)
    ) {
        use pdftract_core::parser::stream::PredictorParams;

        let mut dict = IndexMap::new();

        if let Some(p) = predictor {
            dict.insert("/Predictor".into(), PdfObject::Integer(p));
        }
        if let Some(c) = columns {
            dict.insert("/Columns".into(), PdfObject::Integer(c));
        }
        if let Some(c) = colors {
            dict.insert("/Colors".into(), PdfObject::Integer(c));
        }
        if let Some(b) = bits_per_component {
            dict.insert("/BitsPerComponent".into(), PdfObject::Integer(b));
        }

        let params = PredictorParams::from_pdf_object(Some(&PdfObject::Dict(Box::new(dict))));
        // Should never panic, may return None or Some
        match params {
            Some(_) | None => {},
        }
    }
}

/// Property: normalize_filter_name handles all strings without panicking.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_normalize_filter_name_no_panic(
        name in proptest::collection::vec(proptest::num::u8::ANY, 0..100)
    ) {
        use pdftract_core::parser::stream::normalize_filter_name;
        use std::ffi::CStr;

        // Try to create a string, skip invalid UTF-8
        if let Ok(s) = String::from_utf8(name.clone()) {
            let _ = normalize_filter_name(&s);
        }
    }
}

/// Property: Multiple filter decoders in sequence don't panic.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_multiple_filters_no_panic(
        data in proptest::collection::vec(proptest::num::u8::ANY, 0..50_000),
        num_filters in 0usize..5usize
    ) {
        let mut current = data.clone();
        let mut counter = 0;

        for i in 0..num_filters {
            // Alternate between different decoders
            let result = match i % 3 {
                0 => FlateDecoder.decode(&current, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES),
                1 => ASCII85Decoder.decode(&current, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES),
                _ => ASCIIHexDecoder.decode(&current, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES),
            };

            if result.is_ok() {
                current = result.unwrap();
            } else {
                // Hard error - stop decoding
                break;
            }
        }

        // If we get here without panic, the test passes
        prop_assert!(true);
    }
}

/// Property: Very large bomb limit doesn't cause issues.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_very_large_bomb_limit(
        data in proptest::collection::vec(proptest::num::u8::ANY, 0..10_000)
    ) {
        let mut counter = 0;
        let very_large_limit: u64 = u64::MAX / 2;

        let result = FlateDecoder.decode(&data, None, &mut counter, very_large_limit);
        // Should not panic even with near-maximum bomb limit
        prop_assert!(result.is_ok());
    }
}

/// Property: Decode result is always deterministic for same input.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_decode_deterministic(
        data in proptest::collection::vec(proptest::num::u8::ANY, 0..10_000)
    ) {
        let mut counter1 = 0;
        let result1 = FlateDecoder.decode(&data, None, &mut counter1, 1000);

        let mut counter2 = 0;
        let result2 = FlateDecoder.decode(&data, None, &mut counter2, 1000);

        prop_assert_eq!(result1, result2);
        prop_assert_eq!(counter1, counter2);
    }
}

/// Property: PdfStream with various filter arrays doesn't panic.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_pdfstream_filter_array_no_panic(
        filter_count in 0usize..5usize
    ) {
        let mut dict = IndexMap::new();

        if filter_count > 0 {
            let filters: Vec<PdfObject> = (0..filter_count)
                .map(|_| PdfObject::Name("FlateDecode".to_string()))
                .collect();
            dict.insert("/Filter".into(), PdfObject::Array(Box::new(filters)));
        }

        dict.insert("/Length".into(), PdfObject::Integer(100));

        let stream = PdfStream::new(dict, 0, Some(100));
        // Creating a stream should not panic
        prop_assert_eq!(stream.offset, 0);
        prop_assert_eq!(stream.length(), Some(100));
    }
}

/// Property: FlateDecode roundtrip - encode then decode produces original.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_flate_roundtrip(
        data in proptest::collection::vec(proptest::num::u8::ANY, 0..50_000)
    ) {
        use flate2::write::{ZlibEncoder, ZlibDecoder};
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

/// Property: ASCII85 roundtrip - encode then decode produces original.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_ascii85_roundtrip(
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

/// Property: Bomb limit enforced for varying decompression ratios.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_bomb_limit_enforced(
        // Seed for deterministic test
        seed in 0u64..1000u64,
        // Decompression ratio to test (1 = 1:1, 100 = 100:1)
        ratio in 10u32..1000u32,
        // Bomb limit in bytes
        bomb_limit in 100u64..100_000u64,
    ) {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        // Create a pattern that compresses well
        // Repeated pattern "AB" compresses at high ratio
        let repeat_count = ((ratio as usize) * 100).min(50_000);
        let mut pattern = Vec::with_capacity(repeat_count * 2);
        for _ in 0..repeat_count {
            pattern.push(b'A');
            pattern.push(b'B');
        }

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
            decoded.len() as u64 <= bomb_limit + 10_000,
            "Decoded {} bytes exceeds bomb limit {} by more than 10KB",
            decoded.len(),
            bomb_limit
        );

        // Counter should also be bounded
        prop_assert!(
            counter <= bomb_limit + 10_000,
            "Counter {} exceeds bomb limit {} by more than 10KB",
            counter,
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
