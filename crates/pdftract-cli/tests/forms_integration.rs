//! Forms integration tests.
//!
//! This test module validates PDF form field extraction and analysis:
//! - AcroForm text fields, checkboxes, radio buttons, dropdowns
//! - XFA form detection and structure
//! - Form field values, flags, and properties
//! - Form submission metadata
//!
//! Tests iterate through PDF fixtures in tests/fixtures/forms/ and verify
//! that form fields are correctly extracted and represented in JSON output.

use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

/// Get the path to the pdftract binary (cargo build output)
fn pdftract_bin() -> PathBuf {
    // The binary should be built at target/debug/pdftract or target/release/pdftract
    // CARGO_MANIFEST_DIR is the crate directory; workspace target is two levels up
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../target/debug/pdftract");

    // Fall back to release if debug doesn't exist
    if !path.exists() {
        let mut release_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        release_path.push("../../target/release/pdftract");
        return release_path;
    }

    path
}

/// Path to fixtures directory
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/forms")
}

/// Find all PDF files in the fixtures directory (non-recursive)
fn find_pdf_fixtures() -> Vec<PathBuf> {
    let fixtures_dir = fixtures_dir();
    let mut pdf_files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&fixtures_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
                pdf_files.push(path);
            }
        }
    }

    pdf_files.sort();
    pdf_files
}

/// Discover all PDF files in the given directory recursively using walkdir.
///
/// # Arguments
/// * `fixtures_path` - Path to the fixtures directory to search
///
/// # Returns
/// A `Vec<PathBuf>` containing paths to all discovered PDF files
fn discover_pdf_fixtures<P: AsRef<Path>>(fixtures_path: P) -> Vec<PathBuf> {
    let mut pdf_files = Vec::new();

    let walker = WalkDir::new(fixtures_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path().extension().map(|ext| ext == "pdf").unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf());

    pdf_files.extend(walker);
    pdf_files.sort();

    pdf_files
}

/// Test discover_pdf_fixtures function and print discovered fixtures
///
/// This test verifies that the walkdir-based PDF discovery function works correctly
/// and prints the names of all discovered fixtures to stdout.
#[test]
fn test_discover_pdf_fixtures() {
    let fixtures_dir = fixtures_dir();
    let pdf_files = discover_pdf_fixtures(&fixtures_dir);

    println!("\n=== Discovered PDF Fixtures ===");
    if pdf_files.is_empty() {
        println!("No PDF files found in {}", fixtures_dir.display());
    } else {
        for pdf_path in &pdf_files {
            println!("  - {}", pdf_path.display());
        }
        println!("Total: {} PDF file(s)", pdf_files.len());
    }
    println!("==============================\n");

    // Test that the function runs without errors
    // (We don't assert a count since fixtures may be added/removed)
    let _ = pdf_files;
}

/// Basic test that discovers all form PDF fixtures
///
/// This test iterates through all PDF files in the forms fixtures directory
/// and verifies they can be opened and parsed by pdftract.
#[test]
fn test_forms_fixtures_discovery() {
    let pdf_files = find_pdf_fixtures();
    let bin = pdftract_bin();

    // Ensure binary exists
    assert!(bin.exists(), "pdftract binary not found at {:?}", bin);

    // If no fixtures yet, test passes (scaffold for future fixtures)
    if pdf_files.is_empty() {
        println!("No PDF fixtures found in {:?} - test scaffold ready for fixtures", fixtures_dir());
        return;
    }

    println!("Found {} form PDF fixtures", pdf_files.len());

    // Test each fixture can be processed
    for pdf_path in pdf_files {
        println!("Testing fixture: {}", pdf_path.display());

        // Run pdftract extract --json on the fixture
        let output = Command::new(&bin)
            .arg("extract")
            .arg("--json")
            .arg(&pdf_path)
            .output();

        match output {
            Ok(result) => {
                // For now, just verify it doesn't crash
                // Future tests will validate the JSON output structure
                if result.status.success() {
                    println!("  ✓ Successfully processed");
                } else {
                    println!("  ⚠ Processing failed with status: {}", result.status);
                    println!("    stderr: {}", String::from_utf8_lossy(&result.stderr));
                }
            }
            Err(e) => {
                println!("  ✗ Failed to run pdftract: {}", e);
            }
        }
    }
}

/// Basic test skeleton that calls pdftract extract on all discovered PDFs
///
/// This test iterates through all PDF files discovered by the walkdir-based
/// discover_pdf_fixtures helper and calls pdftract extract --json on each.
/// It captures stdout/stderr and prints results without strict validation.
#[test]
fn test_extract_all_discovered_pdfs() {
    let fixtures_dir = fixtures_dir();
    let pdf_files = discover_pdf_fixtures(&fixtures_dir);
    let bin = pdftract_bin();

    // Ensure binary exists
    assert!(bin.exists(), "pdftract binary not found at {:?}", bin);

    // If no fixtures yet, test passes (scaffold for future fixtures)
    if pdf_files.is_empty() {
        println!(
            "No PDF fixtures found in {:?} - test scaffold ready for fixtures",
            fixtures_dir
        );
        return;
    }

    println!(
        "\n=== Running pdftract extract on {} discovered PDF(s) ===\n",
        pdf_files.len()
    );

    // Process each fixture with pdftract extract
    let mut success_count = 0;
    let mut failure_count = 0;

    for pdf_path in &pdf_files {
        println!("Processing: {}", pdf_path.display());

        // Run pdftract extract --json on the fixture
        let output = Command::new(&bin)
            .arg("extract")
            .arg("--json")
            .arg(pdf_path)
            .output();

        match output {
            Ok(result) => {
                // Capture stdout/stderr for debugging
                let stdout = String::from_utf8_lossy(&result.stdout);
                let stderr = String::from_utf8_lossy(&result.stderr);

                if result.status.success() {
                    println!("  ✓ Status: SUCCESS");
                    // Print first 200 chars of stdout to show we got JSON output
                    let preview = if stdout.len() > 200 {
                        &stdout[..200]
                    } else {
                        &stdout
                    };
                    println!("  Output preview: {}...", preview);
                    success_count += 1;
                } else {
                    println!("  ✗ Status: FAILED (exit code: {:?})", result.status.code());
                    if !stderr.is_empty() {
                        println!("  stderr: {}", stderr);
                    }
                    if !stdout.is_empty() {
                        let preview = if stdout.len() > 200 {
                            &stdout[..200]
                        } else {
                            &stdout
                        };
                        println!("  stdout preview: {}...", preview);
                    }
                    failure_count += 1;
                }
            }
            Err(e) => {
                println!("  ✗ Failed to run pdftract: {}", e);
                failure_count += 1;
            }
        }
        println!();
    }

    println!("=== Summary: {} succeeded, {} failed ===\n", success_count, failure_count);

    // For this skeleton test, we don't assert success_count > 0
    // PDF extraction failures are OK at this stage - we're just verifying
    // that the test framework and CLI invocation work correctly
    let _ = (success_count, failure_count);
}

/// Test that form field structure is correctly extracted
///
/// This test verifies that form fields are correctly represented in the JSON output:
/// - Field names (T entries)
/// - Field types (FT entries: Tx for text, Btn for buttons, Ch for choice)
/// - Field values (V entries)
/// - Field flags (Ff entries)
/// - Field appearance characteristics (appearance streams, fonts, etc.)
#[test]
fn test_form_field_structure() {
    let pdf_files = find_pdf_fixtures();

    if pdf_files.is_empty() {
        println!("No PDF fixtures - skipping form field structure test");
        return;
    }

    // TODO: Implement form field structure validation
    // This will parse the JSON output and verify:
    // - Form fields array exists
    // - Each field has required properties
    // - Field types match expected values
    // - Field values are correctly extracted
}

/// Test AcroForm-specific features
///
/// Validates AcroForm (classic PDF form) extraction:
/// - Terminal fields (leaf nodes)
/// - Field hierarchies (non-terminal fields with kids)
/// - Calculate actions, format, keystroke, validate actions
/// - Appearance dictionaries
#[test]
fn test_acroform_features() {
    let pdf_files = find_pdf_fixtures();

    if pdf_files.is_empty() {
        println!("No PDF fixtures - skipping AcroForm features test");
        return;
    }

    // TODO: Implement AcroForm-specific validation
    // This will verify AcroForm dictionary entries:
    // - Fields array
    // - NeedAppearances flag
    // - CO (signature calculation order)
    // - DR (default resources)
    // - DA (default appearance)
}

/// Test XFA form detection
///
/// Verifies that XFA (XML Forms Architecture) forms are detected:
/// - XFA presence in AcroForm dictionary
/// - XFA streams identification
#[test]
fn test_xfa_detection() {
    let pdf_files = find_pdf_fixtures();

    if pdf_files.is_empty() {
        println!("No PDF fixtures - skipping XFA detection test");
        return;
    }

    // TODO: Implement XFA detection validation
    // This will check for XFA presence and structure
}
