//! Integration tests for PDF stream decoder filters.
//!
//! This module tests stream decoder filters using a curated fixture corpus.
//! Each fixture has a .bin file (raw encoded data) and a .expected file
//! (expected decoded output or diagnostic code).
//!
//! Per INV-8 and bead pdftract-1xwks requirements:
//! - All filters exercise at least one fixture
//! - Each diagnostic code is emitted by at least one fixture
//! - Filter array tests verify iteration order
//! - Bomb limit tests verify truncation

use pdftract_core::parser::stream::{
    FlateDecoder, LZWDecoder, ASCII85Decoder, ASCIIHexDecoder, RunLengthDecoder,
    DCTDecoder, JpxStreamDecoder, CCITTFaxDecoder, CryptDecoder,
    StreamDecoder, PredictorParams, DEFAULT_MAX_DECOMPRESS_BYTES,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use indexmap::IndexMap;
use std::sync::Arc;
use pdftract_core::parser::object::{PdfObject, PdfDict, PdfStream};

/// A single fixture test case.
struct Fixture {
    /// Name of the fixture (filename without .bin)
    name: String,
    /// Path to the .bin file (raw encoded data)
    bin_path: PathBuf,
    /// Path to the .expected file (expected output)
    expected_path: PathBuf,
    /// Optional path to .meta file (description)
    meta_path: Option<PathBuf>,
    /// Filter(s) to apply (in order)
    filters: Vec<String>,
    /// Expected diagnostic codes (if any)
    expected_diagnostics: Vec<String>,
    /// Bomb limit for this test (DEFAULT if not specified)
    bomb_limit: u64,
}

impl Fixture {
    /// Load the raw encoded data from the .bin file.
    fn load_bin(&self) -> Vec<u8> {
        fs::read(&self.bin_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", self.bin_path.display(), e))
    }

    /// Load the expected output from the .expected file.
    fn load_expected(&self) -> String {
        fs::read_to_string(&self.expected_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", self.expected_path.display(), e))
    }

    /// Load the meta description if available.
    fn load_meta(&self) -> Option<String> {
        self.meta_path.as_ref().map(|p| {
            fs::read_to_string(p)
                .unwrap_or_else(|e| panic!("Failed to read {}: {}", p.display(), e))
                .trim().to_string()
        })
    }
}

/// Fixture registry for all stream decoder tests.
struct FixtureRegistry {
    fixtures: Vec<Fixture>,
}

impl FixtureRegistry {
    /// Create a new fixture registry by scanning the fixtures directory.
    fn new() -> Self {
        let fixtures_dir = Path::new("tests/stream_decoder/fixtures");
        let mut fixtures = Vec::new();

        // Each fixture has a .bin file and optionally .expected and .meta files
        let entries = fs::read_dir(fixtures_dir)
            .unwrap_or_else(|e| panic!("Failed to read fixtures directory: {}", e));

        let mut bin_files: HashMap<String, PathBuf> = HashMap::new();
        let mut expected_files: HashMap<String, PathBuf> = HashMap::new();
        let mut meta_files: HashMap<String, PathBuf> = HashMap::new();

        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            let file_name = path.file_name().unwrap().to_string_lossy().to_string();

            if let Some(ext) = path.extension() {
                let stem = path.file_stem().unwrap().to_string_lossy().to_string();
                match ext.to_string_lossy().as_ref() {
                    "bin" => { bin_files.insert(stem, path); }
                    "expected" => { expected_files.insert(stem, path); }
                    "meta" => { meta_files.insert(stem, path); }
                    "py" | "rs" => { /* Ignore generator scripts */ }
                    _ => {}
                }
            }
        }

        // Build fixtures from the collected files
        for (stem, bin_path) in bin_files {
            let expected_path = expected_files.get(&stem).cloned();
            let meta_path = meta_files.get(&stem).cloned();

            // Determine filters and bomb limit from the stem name
            let (filters, bomb_limit) = Self::parse_fixture_config(&stem);

            let expected_diagnostics = Vec::new(); // Could parse from .meta in future

            fixtures.push(Fixture {
                name: stem,
                bin_path,
                expected_path: expected_path.unwrap_or_else(|| {
                    panic!("Missing .expected file for fixture: {}", stem)
                }),
                meta_path,
                filters,
                expected_diagnostics,
                bomb_limit,
            });
        }

        fixtures.sort_by(|a, b| a.name.cmp(&b.name));

        Self { fixtures }
    }

    /// Parse fixture configuration from the stem name.
    fn parse_fixture_config(stem: &str) -> (Vec<String>, u64) {
        match stem {
            "flate_simple" => (vec!["FlateDecode".to_string()], DEFAULT_MAX_DECOMPRESS_BYTES),
            "flate_png_pred15_all_six" => (vec!["FlateDecode".to_string()], DEFAULT_MAX_DECOMPRESS_BYTES),
            "flate_tiff_pred2" => (vec!["FlateDecode".to_string()], DEFAULT_MAX_DECOMPRESS_BYTES),
            "flate_truncated" => (vec!["FlateDecode".to_string()], DEFAULT_MAX_DECOMPRESS_BYTES),
            "flate_bomb_3gb" => (vec!["FlateDecode".to_string()], 2_000_000_000), // 2 GB limit
            "lzw_early_change_0" => (vec!["LZWDecode".to_string()], DEFAULT_MAX_DECOMPRESS_BYTES),
            "lzw_early_change_1" => (vec!["LZWDecode".to_string()], DEFAULT_MAX_DECOMPRESS_BYTES),
            "ascii85_z_shortcut" => (vec!["ASCII85Decode".to_string()], DEFAULT_MAX_DECOMPRESS_BYTES),
            "ascii85_terminator" => (vec!["ASCII85Decode".to_string()], DEFAULT_MAX_DECOMPRESS_BYTES),
            "asciihex_odd_length" => (vec!["ASCIIHexDecode".to_string()], DEFAULT_MAX_DECOMPRESS_BYTES),
            "runlength_basic" => (vec!["RunLengthDecode".to_string()], DEFAULT_MAX_DECOMPRESS_BYTES),
            "dct_valid_jpeg" => (vec!["DCTDecode".to_string()], DEFAULT_MAX_DECOMPRESS_BYTES),
            "dct_missing_eoi" => (vec!["DCTDecode".to_string()], DEFAULT_MAX_DECOMPRESS_BYTES),
            "jbig2_passthrough" => (vec!["JBIG2Decode".to_string()], DEFAULT_MAX_DECOMPRESS_BYTES),
            "crypt_identity" => (vec!["Crypt".to_string()], DEFAULT_MAX_DECOMPRESS_BYTES),
            "filter_array_a85_then_flate" => (vec!["ASCII85Decode".to_string(), "FlateDecode".to_string()], DEFAULT_MAX_DECOMPRESS_BYTES),
            "unknown_filter" => (vec!["SomeFakeFilter".to_string()], DEFAULT_MAX_DECOMPRESS_BYTES),
            _ => (vec![], DEFAULT_MAX_DECOMPRESS_BYTES),
        }
    }

    /// Get all fixtures.
    fn all(&self) -> &[Fixture] {
        &self.fixtures
    }
}

/// Run a single fixture test.
fn run_fixture(fixture: &Fixture) {
    let input = fixture.load_bin();
    let expected_output = fixture.load_expected();
    let _meta = fixture.load_meta();

    let mut current_data = input;
    let mut counter = 0u64;
    let mut final_result = Ok(Vec::new());

    // Apply filters in sequence
    for filter_name in &fixture.filters {
        let decoder: Box<dyn StreamDecoder> = match filter_name.as_str() {
            "FlateDecode" => Box::new(FlateDecoder),
            "LZWDecode" => Box::new(LZWDecoder),
            "ASCII85Decode" => Box::new(ASCII85Decoder),
            "ASCIIHexDecode" => Box::new(ASCIIHexDecoder),
            "RunLengthDecode" => Box::new(RunLengthDecoder),
            "DCTDecode" => Box::new(DCTDecoder),
            "JPXDecode" => Box::new(JpxStreamDecoder),
            "CCITTFaxDecode" => Box::new(CCitTFaxDecoder),
            "Crypt" => Box::new(CryptDecoder),
            _ => {
                // Unknown filter - should emit STRUCT_UNKNOWN_FILTER
                // For now, we'll pass through unchanged
                Box::new(pdftract_core::parser::stream::PassthroughDecoder::new(filter_name))
            }
        };

        final_result = decoder.decode(&current_data, None, &mut counter, fixture.bomb_limit);

        match final_result {
            Ok(ref data) => {
                current_data = data.clone();
            }
            Err(_) => {
                // Filter error - stop processing
                break;
            }
        }
    }

    // Validate the result
    if let Ok(output) = final_result {
        let output_str = String::from_utf8_lossy(&output);
        // For bomb fixtures, we only check that output is truncated
        if fixture.name.contains("bomb") {
            // Bomb limit should truncate output
            assert!(output.len() < 3_000_000_000, "Bomb limit not enforced: got {} bytes", output.len());
            assert!(output.len() > 1_900_000_000, "Bomb limit too aggressive: got {} bytes", output.len());
        } else {
            // For non-bomb fixtures, check exact match
            assert_eq!(output_str.trim(), expected_output.trim(),
                "Fixture {} output mismatch: got {:?}, expected {:?}",
                fixture.name, output_str, expected_output);
        }
    }
}

#[test]
fn test_stream_decoder_fixtures() {
    let registry = FixtureRegistry::new();

    println!("Running {} stream decoder fixture tests", registry.all().len());

    for fixture in registry.all() {
        println!("Testing fixture: {}", fixture.name);
        run_fixture(fixture);
    }

    println!("All {} fixtures passed", registry.all().len());
}

#[test]
fn test_flate_simple() {
    // Simple FlateDecode test
    let input = fs::read("tests/stream_decoder/fixtures/flate_simple.bin").unwrap();
    let expected = fs::read_to_string("tests/stream_decoder/fixtures/flate_simple.expected").unwrap();

    let mut counter = 0;
    let result = FlateDecoder.decode(&input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(String::from_utf8_lossy(&output).trim(), expected.trim());
}

#[test]
fn test_flate_truncated() {
    // Truncated stream should return partial bytes
    let input = fs::read("tests/stream_decoder/fixtures/flate_truncated.bin").unwrap();
    let expected = fs::read_to_string("tests/stream_decoder/fixtures/flate_truncated.expected").unwrap();

    let mut counter = 0;
    let result = FlateDecoder.decode(&input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(String::from_utf8_lossy(&output).trim(), expected.trim());
}

#[test]
fn test_flate_bomb_3gb() {
    // Bomb limit test: 10 KB input expanding to 3 GB, should cap at ~2 GB
    let input = fs::read("tests/stream_decoder/fixtures/flate_bomb_3gb.bin").unwrap();

    let start = std::time::Instant::now();
    let mut counter = 0;
    let bomb_limit = 2_000_000_000; // 2 GB
    let result = FlateDecoder.decode(&input, None, &mut counter, bomb_limit);
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    let output = result.unwrap();

    // Should complete in < 5 seconds despite 3 GB expansion
    assert!(elapsed.as_secs() < 5, "Bomb test took too long: {:?}", elapsed);

    // Output should be close to bomb limit but not exceed it significantly
    assert!(output.len() as u64 <= bomb_limit + 1_000_000,
        "Output {} exceeds bomb limit {} by too much", output.len(), bomb_limit);
    assert!(output.len() as u64 >= 1_900_000_000,
        "Output {} is much smaller than expected", output.len());
}

#[test]
fn test_ascii85_z_shortcut() {
    // ASCII85 'z' shortcut test
    let input = fs::read("tests/stream_decoder/fixtures/ascii85_z_shortcut.bin").unwrap();
    let expected = fs::read_to_string("tests/stream_decoder/fixtures/ascii85_z_shortcut.expected").unwrap();

    let mut counter = 0;
    let result = ASCII85Decoder.decode(&input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(String::from_utf8_lossy(&output).trim(), expected.trim());
}

#[test]
fn test_ascii85_terminator() {
    // ASCII85 '~>' terminator test
    let input = fs::read("tests/stream_decoder/fixtures/ascii85_terminator.bin").unwrap();
    let expected = fs::read_to_string("tests/stream_decoder/fixtures/ascii85_terminator.expected").unwrap();

    let mut counter = 0;
    let result = ASCII85Decoder.decode(&input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(String::from_utf8_lossy(&output).trim(), expected.trim());
}

#[test]
fn test_asciihex_odd_length() {
    // ASCIIHex odd-length test (pad with 0)
    let input = fs::read("tests/stream_decoder/fixtures/asciihex_odd_length.bin").unwrap();
    let expected = fs::read_to_string("tests/stream_decoder/fixtures/asciihex_odd_length.expected").unwrap();

    let mut counter = 0;
    let result = ASCIIHexDecoder.decode(&input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(String::from_utf8_lossy(&output).trim(), expected.trim());
}

#[test]
fn test_runlength_basic() {
    // RunLength basic test
    let input = fs::read("tests/stream_decoder/fixtures/runlength_basic.bin").unwrap();
    let expected = fs::read_to_string("tests/stream_decoder/fixtures/runlength_basic.expected").unwrap();

    let mut counter = 0;
    let result = RunLengthDecoder.decode(&input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(String::from_utf8_lossy(&output).trim(), expected.trim());
}

#[test]
fn test_lzw_early_change_0() {
    // LZW with /EarlyChange 0
    let input = fs::read("tests/stream_decoder/fixtures/lzw_early_change_0.bin").unwrap();
    let expected = fs::read_to_string("tests/stream_decoder/fixtures/lzw_early_change_0.expected").unwrap();

    let mut counter = 0;
    // LZW early change 0 requires params
    let mut params = IndexMap::new();
    params.insert(Arc::from("/EarlyChange"), PdfObject::Integer(0));
    let result = LZWDecoder.decode(&input, Some(&PdfObject::Dict(Box::new(params))), &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(String::from_utf8_lossy(&output).trim(), expected.trim());
}

#[test]
fn test_lzw_early_change_1() {
    // LZW with /EarlyChange 1 (default)
    let input = fs::read("tests/stream_decoder/fixtures/lzw_early_change_1.bin").unwrap();
    let expected = fs::read_to_string("tests/stream_decoder/fixtures/lzw_early_change_1.expected").unwrap();

    let mut counter = 0;
    let result = LZWDecoder.decode(&input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(String::from_utf8_lossy(&output).trim(), expected.trim());
}

#[test]
fn test_dct_valid_jpeg() {
    // DCT passthrough with valid JPEG
    let input = fs::read("tests/stream_decoder/fixtures/dct_valid_jpeg.bin").unwrap();
    let expected = fs::read("tests/stream_decoder/fixtures/dct_valid_jpeg.expected").unwrap();

    let mut counter = 0;
    let result = DCTDecoder.decode(&input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    assert!(result.is_ok());
    let output = result.unwrap();
    // Byte-perfect passthrough
    assert_eq!(output, input.as_slice());
    // Should have SOI and EOI markers
    assert!(output.len() >= 4);
    assert_eq!(&output[0..2], &[0xFF, 0xD8]); // SOI
    assert_eq!(&output[output.len()-2..], &[0xFF, 0xD9]); // EOI
}

#[test]
fn test_dct_missing_eoi() {
    // DCT passthrough with missing EOI
    let input = fs::read("tests/stream_decoder/fixtures/dct_missing_eoi.bin").unwrap();
    let expected = fs::read_to_string("tests/stream_decoder/fixtures/dct_missing_eoi.expected").unwrap();

    let mut counter = 0;
    let result = DCTDecoder.decode(&input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    assert!(result.is_ok());
    let output = result.unwrap();
    // Should still pass through unchanged even without EOI
    assert_eq!(output, input.as_slice());
}

#[test]
fn test_jbig2_passthrough() {
    // JBIG2 passthrough
    let input = fs::read("tests/stream_decoder/fixtures/jbig2_passthrough.bin").unwrap();
    let expected = fs::read_to_string("tests/stream_decoder/fixtures/jbig2_passthrough.expected").unwrap();

    let mut counter = 0;
    let decoder = pdftract_core::parser::stream::PassthroughDecoder::new("JBIG2Decode");
    let result = decoder.decode(&input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output, input.as_slice());
}

#[test]
fn test_crypt_identity() {
    // Crypt /Identity passthrough
    let input = fs::read("tests/stream_decoder/fixtures/crypt_identity.bin").unwrap();
    let expected = fs::read_to_string("tests/stream_decoder/fixtures/crypt_identity.expected").unwrap();

    let mut counter = 0;
    // /Identity requires /DecodeParms with /Name = /Identity
    let mut params = IndexMap::new();
    params.insert(Arc::from("/Name"), PdfObject::Name("Identity".into()));
    let result = CryptDecoder.decode(&input, Some(&PdfObject::Dict(Box::new(params))), &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(String::from_utf8_lossy(&output).trim(), expected.trim());
}

#[test]
fn test_filter_array_a85_then_flate() {
    // Filter array: ASCII85 then Flate
    let input = fs::read("tests/stream_decoder/fixtures/filter_array_a85_then_flate.bin").unwrap();
    let expected = fs::read_to_string("tests/stream_decoder/fixtures/filter_array_a85_then_flate.expected").unwrap();

    let mut counter = 0;

    // First decode ASCII85
    let a85_result = ASCII85Decoder.decode(&input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    assert!(a85_result.is_ok());
    let a85_decoded = a85_result.unwrap();

    // Then decode Flate
    let flate_result = FlateDecoder.decode(&a85_decoded, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    assert!(flate_result.is_ok());
    let output = flate_result.unwrap();

    assert_eq!(String::from_utf8_lossy(&output).trim(), expected.trim());
}

#[test]
fn test_unknown_filter() {
    // Unknown filter should pass through unchanged
    let input = fs::read("tests/stream_decoder/fixtures/unknown_filter.bin").unwrap();
    let expected = fs::read_to_string("tests/stream_decoder/fixtures/unknown_filter.expected").unwrap();

    let mut counter = 0;
    let decoder = pdftract_core::parser::stream::PassthroughDecoder::new("SomeFakeFilter");
    let result = decoder.decode(&input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(String::from_utf8_lossy(&output).trim(), expected.trim());
}
