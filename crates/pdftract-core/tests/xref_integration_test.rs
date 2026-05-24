//! Integration tests for PDF xref resolution.
//!
//! This module runs integration tests against a corpus of PDF fixtures
//! covering various xref structures and edge cases.

mod xref_helpers;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use pdftract_core::diagnostics::Diagnostic;
use pdftract_core::parser::stream::{MemorySource, PdfSource};
use pdftract_core::parser::xref::{
    detect_linearization, forward_scan_xref, load_xref_linearized, load_xref_with_prev_chain,
    merge_hybrid, parse_traditional_xref, parse_xref_stream, XrefEntry, XrefSection,
};

/// Fixture directory containing the test PDF files.
const FIXTURE_DIR: &str = "../../tests/xref/fixtures";

/// Expected JSON file extension.
const EXPECTED_EXT: &str = ".expected.json";

/// Environment variable to enable golden file blessing.
const BLESS_ENV: &str = "BLESS";

/// Test result structure for golden file comparison.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct XrefTestResult {
    /// The xref entries parsed from the fixture.
    entries: HashMap<String, XrefEntryJson>,
    /// The trailer dictionary (simplified for JSON serialization).
    trailer: Option<serde_json::Value>,
    /// Diagnostics emitted during parsing.
    diagnostics: Vec<DiagnosticJson>,
}

/// JSON representation of an XrefEntry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(tag = "type")]
enum XrefEntryJson {
    #[serde(rename = "free")]
    Free { next_free: u32, gen_nr: u16 },
    #[serde(rename = "in_use")]
    InUse { offset: u64, gen_nr: u16 },
    #[serde(rename = "compressed")]
    Compressed { obj_stm_nr: u32, index: u32 },
}

/// JSON representation of a diagnostic.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DiagnosticJson {
    code: String,
    byte_offset: Option<u64>,
    message: String,
}

impl From<&Diagnostic> for DiagnosticJson {
    fn from(diag: &Diagnostic) -> Self {
        DiagnosticJson {
            code: format!("{:?}", diag.code),
            byte_offset: diag.byte_offset,
            message: diag.message.to_string(),
        }
    }
}

/// Load a PDF fixture and parse its xref structure.
///
/// This function attempts all four xref parsing strategies:
/// 1. Traditional xref table
/// 2. Xref stream
/// 3. Hybrid file (traditional + stream)
/// 4. Forward scan fallback
///
/// For files with /Prev chains, it traverses the full chain.
/// For linearized files, it merges first-page and full xrefs.
fn parse_fixture_xref(fixture_path: &Path) -> XrefSection {
    // Read the entire file into memory
    let data = fs::read(fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {:?}: {}", fixture_path, e));

    let source = MemorySource::new(data);

    // Find startxref offset
    let startxref = find_startxref(&source);

    // Check for linearized PDF
    let lin_info = detect_linearization(&source);

    let result = if let Some(info) = lin_info {
        // Linearized file: load and merge first-page and full xrefs
        load_xref_linearized(&source, &info, startxref)
    } else {
        // Non-linearized: load with /Prev chain support
        load_xref_with_prev_chain(&source, startxref)
    };

    // If traditional parsing failed, try forward scan as last resort
    if result.entries.is_empty() && result.trailer.is_none() {
        forward_scan_xref(&source, false)
    } else {
        result
    }
}

/// Find the startxref offset in a PDF file.
///
/// Scans the last 1KB of the file for the startxref keyword.
fn find_startxref(source: &MemorySource) -> u64 {
    let file_len = source.len().unwrap_or(0);
    if file_len < 1024 {
        return 0;
    }

    // Read the last 1KB
    let scan_start = file_len.saturating_sub(1024);
    let tail_data = source
        .read_at(scan_start, (file_len - scan_start) as usize)
        .unwrap_or_default();

    // Convert to string and search for startxref
    let tail_str = String::from_utf8_lossy(&tail_data);

    // Find "startxref" keyword
    let startxref_pos = tail_str.find("startxref").unwrap_or_else(|| {
        // If not found, return 0 to trigger fallback strategies
        return 0;
    });

    // Parse the offset after "startxref"
    let after_startxref = &tail_str[startxref_pos + "startxref".len()..];
    let offset_str = after_startxref.split_whitespace().next().unwrap_or("0");

    let offset: u64 = offset_str.parse().unwrap_or(0);

    // Adjust for the scan start offset
    if offset == 0 {
        scan_start
    } else {
        offset
    }
}

/// Compare parsed xref result against golden file.
fn compare_with_golden(fixture_path: &Path, result: &XrefSection) -> Result<(), String> {
    let golden_path = fixture_path.with_extension(EXPECTED_EXT.trim_start_matches('.'));

    // Check if we should bless (overwrite) the golden file
    let bless = std::env::var(BLESS_ENV).is_ok();

    if bless {
        // Write/update the golden file
        let golden = XrefTestResult {
            entries: convert_xref_entries(&result.entries),
            trailer: result.trailer.as_ref().map(|t| {
                // Simplified trailer serialization - just count keys
                let key_count = t.keys().count();
                serde_json::json!({ "key_count": key_count })
            }),
            diagnostics: result
                .diagnostics
                .iter()
                .map(DiagnosticJson::from)
                .collect(),
        };

        let golden_json = serde_json::to_string_pretty(&golden)
            .map_err(|e| format!("Failed to serialize golden: {}", e))?;

        fs::write(&golden_path, golden_json)
            .map_err(|e| format!("Failed to write golden file {:?}: {}", golden_path, e))?;

        eprintln!("Blessed golden file: {:?}", golden_path);
        return Ok(());
    }

    // Read and compare with existing golden file
    if !golden_path.exists() {
        return Err(format!(
            "Golden file not found: {:?}. Run with {}=1 to create it.",
            golden_path, BLESS_ENV
        ));
    }

    let golden_json = fs::read_to_string(&golden_path)
        .map_err(|e| format!("Failed to read golden file {:?}: {}", golden_path, e))?;

    let golden: XrefTestResult = serde_json::from_str(&golden_json)
        .map_err(|e| format!("Failed to parse golden file {:?}: {}", golden_path, e))?;

    // Compare entries
    let result_entries = convert_xref_entries(&result.entries);

    if golden.entries != result_entries {
        return Err(format!(
            "Xref entries mismatch.\nExpected: {:#?}\nActual: {:#?}",
            golden.entries, result_entries
        ));
    }

    // Compare diagnostics (only count, not exact messages which may vary)
    if golden.diagnostics.len() != result.diagnostics.len() {
        return Err(format!(
            "Diagnostic count mismatch.\nExpected: {} diagnostics\nActual: {} diagnostics\n{:?}",
            golden.diagnostics.len(),
            result.diagnostics.len(),
            result.diagnostics
        ));
    }

    Ok(())
}

/// Helper function to convert XrefEntry map to JSON-serializable format.
fn convert_xref_entries(
    entries: &std::collections::HashMap<u32, XrefEntry>,
) -> HashMap<String, XrefEntryJson> {
    entries
        .iter()
        .map(|(k, v)| {
            let key = k.to_string();
            let json = match v {
                XrefEntry::Free { next_free, gen_nr } => XrefEntryJson::Free {
                    next_free: *next_free,
                    gen_nr: *gen_nr,
                },
                XrefEntry::InUse { offset, gen_nr } => XrefEntryJson::InUse {
                    offset: *offset,
                    gen_nr: *gen_nr,
                },
                XrefEntry::Compressed { obj_stm_nr, index } => XrefEntryJson::Compressed {
                    obj_stm_nr: *obj_stm_nr,
                    index: *index,
                },
            };
            (key, json)
        })
        .collect()
}

/// Test all fixtures in the fixture directory.
#[test]
fn test_xref_fixtures() {
    let fixture_dir = Path::new(FIXTURE_DIR);

    if !fixture_dir.exists() {
        eprintln!(
            "Warning: Fixture directory {:?} does not exist. Skipping tests.",
            fixture_dir
        );
        return;
    }

    let entries = fs::read_dir(fixture_dir)
        .unwrap_or_else(|e| panic!("Failed to read fixture directory {:?}: {}", fixture_dir, e));

    for entry in entries {
        let entry = entry.unwrap_or_else(|e| panic!("Failed to read directory entry: {}", e));
        let path = entry.path();

        // Skip directories and non-PDF files
        if path.is_dir() || path.extension().and_then(|s| s.to_str()) != Some("pdf") {
            continue;
        }

        let fixture_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        eprintln!("Testing fixture: {}", fixture_name);

        // Parse the fixture
        let result = parse_fixture_xref(&path);

        // Compare with golden (or bless if BLESS=1)
        if let Err(e) = compare_with_golden(&path, &result) {
            panic!("Fixture {} failed: {}", fixture_name, e);
        }
    }
}

/// Test that the forward scan fallback recovers objects from truncated files.
#[test]
fn test_forward_scan_recovery() {
    // This test will use the truncated_after_xref.pdf fixture
    let fixture_path = Path::new(FIXTURE_DIR).join("truncated_after_xref.pdf");

    if !fixture_path.exists() {
        eprintln!(
            "Warning: Fixture {:?} does not exist. Skipping test.",
            fixture_path
        );
        return;
    }

    let result = parse_fixture_xref(&fixture_path);

    // Should have recovered some entries via forward scan
    assert!(
        !result.entries.is_empty(),
        "Forward scan should recover some xref entries"
    );

    // Should emit XREF_REPAIRED diagnostic
    use pdftract_core::diagnostics::DiagCode;
    use xref_helpers::assert_diagnostic;
    assert_diagnostic(&result.diagnostics, DiagCode::XrefRepaired);
}

/// Test that /Prev chain depth limit is enforced.
#[test]
fn test_prev_chain_depth_limit() {
    let fixture_path = Path::new(FIXTURE_DIR).join("deep_prev_chain.pdf");

    if !fixture_path.exists() {
        eprintln!(
            "Warning: Fixture {:?} does not exist. Skipping test.",
            fixture_path
        );
        return;
    }

    let result = parse_fixture_xref(&fixture_path);

    // Should emit STRUCT_DEPTH_EXCEEDED diagnostic
    use pdftract_core::diagnostics::DiagCode;
    use xref_helpers::assert_diagnostic;
    assert_diagnostic(&result.diagnostics, DiagCode::StructDepthExceeded);
}

/// Test that circular /Prev references are detected.
#[test]
fn test_circular_prev_detection() {
    let fixture_path = Path::new(FIXTURE_DIR).join("circular_prev.pdf");

    if !fixture_path.exists() {
        eprintln!(
            "Warning: Fixture {:?} does not exist. Skipping test.",
            fixture_path
        );
        return;
    }

    let result = parse_fixture_xref(&fixture_path);

    // Should emit STRUCT_CIRCULAR_REF diagnostic
    use pdftract_core::diagnostics::DiagCode;
    use xref_helpers::assert_diagnostic;
    assert_diagnostic(&result.diagnostics, DiagCode::StructCircularRef);
}
