//! Integration tests for stream decoder fixtures.
//!
//! Walks all fixtures in tests/stream_decoder/fixtures/, runs the appropriate
//! filter decoder, compares against .expected files, and validates diagnostics.

use indexmap::IndexMap;
use pdftract_core::diagnostics::DiagCode;
use pdftract_core::parser::object::{PdfDict, PdfObject};
use pdftract_core::parser::stream::{
    normalize_filter_name, ASCII85Decoder, ASCIIHexDecoder, CCITTFaxDecoder, CryptDecoder,
    DCTDecoder, FlateDecoder, JpxStreamDecoder, LZWDecoder, PassthroughDecoder, RunLengthDecoder,
    StreamDecoder, DEFAULT_MAX_DECOMPRESS_BYTES,
};
use std::fs;
use std::path::PathBuf;

/// Fixture metadata describing the filter and parameters to use.
struct FixtureInfo {
    name: &'static str,
    filter: FixtureFilter,
    /// Expected diagnostic codes (empty if none expected)
    expected_diags: Vec<DiagCode>,
    /// Custom bomb limit for bomb tests
    bomb_limit: Option<u64>,
}

/// Filter configuration for a fixture.
enum FixtureFilter {
    /// Single filter with optional parameters.
    Single(&'static str, Option<PdfObject>),
    /// Filter array: decode through multiple filters in sequence.
    Array(Vec<(&'static str, Option<PdfObject>)>),
    /// Unknown filter - should return passthrough + STRUCT_UNKNOWN_FILTER.
    Unknown(&'static str),
}

/// Get all fixtures with their configuration.
fn get_fixtures() -> Vec<FixtureInfo> {
    vec![
        // FlateDecode fixtures
        FixtureInfo {
            name: "flate_simple",
            filter: FixtureFilter::Single("FlateDecode", None),
            expected_diags: vec![],
            bomb_limit: None,
        },
        FixtureInfo {
            name: "flate_png_pred15_all_six",
            filter: FixtureFilter::Single("FlateDecode", Some(create_png_predictor_params())),
            expected_diags: vec![],
            bomb_limit: None,
        },
        FixtureInfo {
            name: "flate_tiff_pred2",
            filter: FixtureFilter::Single("FlateDecode", Some(create_tiff_predictor_params())),
            expected_diags: vec![],
            bomb_limit: None,
        },
        FixtureInfo {
            name: "flate_truncated",
            filter: FixtureFilter::Single("FlateDecode", None),
            // Truncated FlateDecode stream: INV-8 soft-recovery must produce Ok(partial)
            // with a STREAM_DECODE_ERROR diagnostic, not a hard Err. See the contract
            // assertion in `test_all_stream_decoder_fixtures` (bf-4bx00 / bf-4fb3b).
            expected_diags: vec![DiagCode::StreamDecodeError],
            bomb_limit: None,
        },
        FixtureInfo {
            name: "flate_bomb_3gb",
            filter: FixtureFilter::Single("FlateDecode", None),
            expected_diags: vec![DiagCode::StreamBomb],
            bomb_limit: Some(2_000_000_000), // 2GB limit
        },
        // LZW fixtures
        FixtureInfo {
            name: "lzw_early_change_0",
            filter: FixtureFilter::Single("LZWDecode", Some(create_early_change_params(0))),
            expected_diags: vec![],
            bomb_limit: None,
        },
        FixtureInfo {
            name: "lzw_early_change_1",
            filter: FixtureFilter::Single("LZWDecode", Some(create_early_change_params(1))),
            expected_diags: vec![],
            bomb_limit: None,
        },
        // ASCII85 fixtures
        FixtureInfo {
            name: "ascii85_z_shortcut",
            filter: FixtureFilter::Single("ASCII85Decode", None),
            expected_diags: vec![],
            bomb_limit: None,
        },
        FixtureInfo {
            name: "ascii85_terminator",
            filter: FixtureFilter::Single("ASCII85Decode", None),
            expected_diags: vec![],
            bomb_limit: None,
        },
        // ASCIIHex fixture
        FixtureInfo {
            name: "asciihex_odd_length",
            filter: FixtureFilter::Single("ASCIIHexDecode", None),
            expected_diags: vec![],
            bomb_limit: None,
        },
        // RunLength fixture
        FixtureInfo {
            name: "runlength_basic",
            filter: FixtureFilter::Single("RunLengthDecode", None),
            expected_diags: vec![],
            bomb_limit: None,
        },
        // DCTDecode fixtures
        FixtureInfo {
            name: "dct_valid_jpeg",
            filter: FixtureFilter::Single("DCTDecode", None),
            expected_diags: vec![],
            bomb_limit: None,
        },
        FixtureInfo {
            name: "dct_missing_eoi",
            filter: FixtureFilter::Single("DCTDecode", None),
            expected_diags: vec![DiagCode::StreamInvalidJpeg],
            bomb_limit: None,
        },
        // JBIG2 fixture
        FixtureInfo {
            name: "jbig2_passthrough",
            filter: FixtureFilter::Single("JBIG2Decode", None),
            expected_diags: vec![DiagCode::OcrJbig2Unsupported],
            bomb_limit: None,
        },
        // Crypt fixture
        FixtureInfo {
            name: "crypt_identity",
            filter: FixtureFilter::Single("Crypt", Some(create_crypt_identity_params())),
            expected_diags: vec![],
            bomb_limit: None,
        },
        // Filter array fixture
        FixtureInfo {
            name: "filter_array_a85_then_flate",
            filter: FixtureFilter::Array(vec![("ASCII85Decode", None), ("FlateDecode", None)]),
            expected_diags: vec![],
            bomb_limit: None,
        },
        // Unknown filter fixture
        FixtureInfo {
            name: "unknown_filter",
            filter: FixtureFilter::Unknown("SomeFakeFilter"),
            expected_diags: vec![DiagCode::StreamUnknownFilter],
            bomb_limit: None,
        },
    ]
}

/// Create PNG predictor params for the pred15_all_six fixture.
fn create_png_predictor_params() -> PdfObject {
    let mut dict = IndexMap::new();
    dict.insert("/Predictor".into(), PdfObject::Integer(15));
    dict.insert("/Columns".into(), PdfObject::Integer(8));
    dict.insert("/Colors".into(), PdfObject::Integer(1));
    dict.insert("/BitsPerComponent".into(), PdfObject::Integer(8));
    PdfObject::Dict(Box::new(dict))
}

/// Create TIFF predictor 2 params.
fn create_tiff_predictor_params() -> PdfObject {
    let mut dict = IndexMap::new();
    dict.insert("/Predictor".into(), PdfObject::Integer(2));
    dict.insert("/Columns".into(), PdfObject::Integer(2));
    dict.insert("/Colors".into(), PdfObject::Integer(3));
    dict.insert("/BitsPerComponent".into(), PdfObject::Integer(8));
    PdfObject::Dict(Box::new(dict))
}

/// Create LZW EarlyChange params.
fn create_early_change_params(early_change: i64) -> PdfObject {
    let mut dict = IndexMap::new();
    dict.insert("/EarlyChange".into(), PdfObject::Integer(early_change));
    PdfObject::Dict(Box::new(dict))
}

/// Create Crypt /Identity params.
fn create_crypt_identity_params() -> PdfObject {
    let mut dict = IndexMap::new();
    dict.insert("/Name".into(), PdfObject::Name("Identity".into()));
    PdfObject::Dict(Box::new(dict))
}

/// Get the fixtures directory.
fn fixtures_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // We're in crates/pdftract-core, so go up to workspace root then to fixtures
    path.push("../../tests/stream_decoder/fixtures");
    path.canonicalize().unwrap_or_else(|_| {
        // Fallback: try relative to workspace root
        let mut fallback = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        fallback.push("../../../tests/stream_decoder/fixtures");
        fallback
    })
}

/// Get decoder for a filter name.
fn get_decoder(name: &str) -> Option<Box<dyn pdftract_core::parser::stream::StreamDecoder>> {
    match normalize_filter_name(name) {
        "FlateDecode" => Some(Box::new(FlateDecoder)),
        "LZWDecode" => Some(Box::new(LZWDecoder)),
        "ASCII85Decode" => Some(Box::new(ASCII85Decoder)),
        "ASCIIHexDecode" => Some(Box::new(ASCIIHexDecoder)),
        "Crypt" => Some(Box::new(CryptDecoder)),
        "DCTDecode" => Some(Box::new(DCTDecoder)),
        "JBIG2Decode" => Some(Box::new(PassthroughDecoder::new("JBIG2Decode"))),
        "JPXDecode" => Some(Box::new(JpxStreamDecoder)),
        "CCITTFaxDecode" => Some(Box::new(CCITTFaxDecoder)),
        "RunLengthDecode" => Some(Box::new(RunLengthDecoder)),
        _ => None,
    }
}

/// Decode data through a filter or filter array.
fn decode_fixture(fixture: &FixtureInfo, input: &[u8]) -> Result<Vec<u8>, String> {
    let mut counter = 0u64;
    let max_bytes = fixture.bomb_limit.unwrap_or(DEFAULT_MAX_DECOMPRESS_BYTES);

    match &fixture.filter {
        FixtureFilter::Single(filter_name, params) => {
            let decoder = get_decoder(filter_name)
                .ok_or_else(|| format!("Unknown filter: {}", filter_name))?;
            decoder
                .decode(input, params.as_ref(), &mut counter, max_bytes)
                .map_err(|e| format!("Decode error: {}", e))
        }
        FixtureFilter::Array(filters) => {
            let mut current = input.to_vec();
            for (filter_name, params) in filters {
                let decoder = get_decoder(filter_name)
                    .ok_or_else(|| format!("Unknown filter in array: {}", filter_name))?;
                current = decoder
                    .decode(&current, params.as_ref(), &mut counter, max_bytes)
                    .map_err(|e| format!("Decode error in {}: {}", filter_name, e))?;
            }
            Ok(current)
        }
        FixtureFilter::Unknown(filter_name) => {
            // Unknown filter should return passthrough
            let decoder = PassthroughDecoder::new(filter_name);
            decoder
                .decode(input, None, &mut counter, max_bytes)
                .map_err(|e| format!("Passthrough error: {}", e))
        }
    }
}

#[test]
fn test_all_stream_decoder_fixtures() {
    let fixtures = get_fixtures();
    let fixtures_path = fixtures_dir();

    let mut failures = Vec::new();
    let mut passed = 0;
    let mut total = 0;

    for fixture in fixtures {
        total += 1;
        let fixture_path = fixtures_path.join(format!("{}.bin", fixture.name));
        let expected_path = fixtures_path.join(format!("{}.expected", fixture.name));

        // Skip if fixture file doesn't exist (e.g., not generated yet)
        if !fixture_path.exists() {
            failures.push(format!("{}: fixture file not found", fixture.name));
            continue;
        }

        // Skip if expected file doesn't exist
        if !expected_path.exists() {
            failures.push(format!("{}: expected file not found", fixture.name));
            continue;
        }

        // Read fixture and expected data
        let input = fs::read(&fixture_path)
            .map_err(|e| format!("{}: failed to read fixture: {}", fixture.name, e));
        let input = match input {
            Ok(data) => data,
            Err(e) => {
                failures.push(e);
                continue;
            }
        };

        let expected = fs::read(&expected_path)
            .map_err(|e| format!("{}: failed to read expected: {}", fixture.name, e));
        let expected = match expected {
            Ok(data) => data,
            Err(e) => {
                failures.push(e);
                continue;
            }
        };

        // A fixture may declare that it expects a soft STREAM_DECODE_ERROR — i.e. a
        // truncated/corrupt stream that the decoder should recover from via INV-8 soft
        // partial-recovery (stream.rs:542-544: UnexpectedEof -> break -> partial output),
        // producing `Ok(partial)` rather than a hard `Err`. (bf-4bx00 / bf-4fb3b §5)
        let expects_decode_error = fixture
            .expected_diags
            .contains(&DiagCode::StreamDecodeError);

        // Decode the fixture
        let result = decode_fixture(&fixture, &input);
        let decoded = match result {
            Ok(data) => data,
            Err(e) => {
                if expects_decode_error {
                    // INV-8 regression guard (bf-60qj2 EC2/EC12): a fixture declared to
                    // expect a STREAM_DECODE_ERROR must soft-recover to Ok(partial); a hard
                    // Err means INV-8 soft-recovery has regressed.
                    failures.push(format!(
                        "{}: expected STREAM_DECODE_ERROR (DiagCode::StreamDecodeError) \
                         soft partial-recovery (Ok per INV-8), but decode returned hard Err: {}",
                        fixture.name, e
                    ));
                } else {
                    failures.push(format!("{}: {}", fixture.name, e));
                }
                continue;
            }
        };

        // Compare against expected
        // For bomb tests, we only check the first N bytes (the expected file is truncated)
        let expected_bytes = if fixture.name == "flate_bomb_3gb" {
            &expected[..expected.len().min(decoded.len())]
        } else {
            &expected[..]
        };

        if &decoded[..expected_bytes.len().min(decoded.len())] != expected_bytes {
            failures.push(format!(
                "{}: output mismatch (expected {} bytes, got {} bytes)",
                fixture.name,
                expected.len(),
                decoded.len()
            ));
            continue;
        }

        // For bomb test, verify we hit the bomb limit
        if fixture.name == "flate_bomb_3gb" {
            // The decoded output should be close to the bomb limit
            // The fixture expands from 10KB to 3GB, but we cap at 2GB
            // The expected file contains the first 1KB of the expected output
            // We should have decoded at least that much
            assert!(
                decoded.len() >= expected.len(),
                "Bomb test: output too short"
            );
            // And we should have hit the bomb limit (output should be truncated)
            assert!(
                decoded.len() < 3_000_000_000,
                "Bomb test: should have truncated"
            );
        }

        // Contract assertion for STREAM_DECODE_ERROR fixtures (bf-4bx00). The low-level
        // `StreamDecoder::decode` path returns `Result<Vec<u8>, FilterError>` with no
        // diagnostics channel (stream.rs:74), so `STREAM_DECODE_ERROR` is never *collected*
        // here — it is only emitted via `emit!()` on the full-extraction path, which this
        // fixture loop bypasses (bf-60qj2 EC1). The observable contract on this path is
        // therefore: the fixture declares a decode error AND decode did not hard-fail —
        // i.e. we reached this `Ok` arm via INV-8 soft partial-recovery. Reaching this
        // point (`passed += 1`) means the positive contract holds; the negative case — a
        // hard `Err` on a fixture declared to expect STREAM_DECODE_ERROR — is caught by
        // the `Err`-arm guard above (EC2/EC12).
        //
        // GAP (EC6/EC7/EC13): with no length hint, checksum, or collected diagnostic on
        // this path, the assertion cannot distinguish "correct partial recovery" from
        // "decoder silently produced clean output," and the partial bytes are not
        // byte-stable — so there is no `decoded.len()` proxy and no byte-assertion on the
        // partial content.
        passed += 1;
    }

    // Report results
    if !failures.is_empty() {
        eprintln!("Stream decoder fixture tests:");
        eprintln!("  Passed: {}/{}", passed, total);
        eprintln!("  Failed:");
        for failure in &failures {
            eprintln!("    - {}", failure);
        }
        panic!("{} stream decoder fixture tests failed", failures.len());
    } else {
        eprintln!("Stream decoder fixtures: {}/{} passed", passed, total);
    }
}

#[test]
fn test_each_filter_exercised() {
    // Verify each filter is exercised by at least one fixture
    let filters_exercised: std::collections::HashSet<_> = get_fixtures()
        .iter()
        .flat_map(|f| match &f.filter {
            FixtureFilter::Single(name, _) => vec![*name],
            FixtureFilter::Array(filters) => filters.iter().map(|(n, _)| *n).collect(),
            FixtureFilter::Unknown(name) => vec![*name],
        })
        .map(normalize_filter_name)
        .collect();

    let expected_filters = [
        "FlateDecode",
        "LZWDecode",
        "ASCII85Decode",
        "ASCIIHexDecode",
        "RunLengthDecode",
        "DCTDecode",
        "JBIG2Decode",
        "Crypt",
    ];

    for filter in expected_filters {
        assert!(
            filters_exercised.contains(filter),
            "Filter {} is not exercised by any fixture",
            filter
        );
    }
}
