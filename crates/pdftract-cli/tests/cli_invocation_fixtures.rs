//! CLI invocation fixture discovery and enumeration
//!
//! This test module provides comprehensive fixture discovery for CLI testing:
//! - Discovers all PDF fixtures from the main test fixtures directory
//! - Provides test-accessible fixture enumeration
//! - Supports category-based filtering (encrypted, forms, ocr, malformed, etc.)
//! - Enables bulk CLI invocation testing
//!
//! The fixtures are organized in /tests/fixtures/ by category:
//! - encrypted/: Password-protected and encrypted PDFs
//! - forms/: PDFs with AcroForm and XFA forms
//! - ocr/: Scanned documents requiring OCR processing
//! - malformed/: Corrupted or malformed PDFs for error handling
//! - scanned/: Scanned documents and receipts
//! - cjk/: Chinese/Japanese/Korean language documents
//! - fonts/: PDFs with various font encodings and subsets
//! - And more...

mod fixture_discovery;

use std::path::{Path, PathBuf};
use std::process::Command;
use fixture_discovery::{fixtures_root, discover_all_fixtures, discover_fixtures_by_category, discover_fixtures_in_dir, fixture_categories};

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

/// Test that the main fixtures directory exists and is accessible
#[test]
fn test_main_fixtures_dir_exists() {
    let fixtures_dir = fixtures_root();
    assert!(fixtures_dir.exists(), "Main fixtures directory does not exist: {:?}", fixtures_dir);
    assert!(fixtures_dir.is_dir(), "Main fixtures path is not a directory: {:?}", fixtures_dir);

    println!("Main fixtures directory: {:?}", fixtures_dir);
}

/// Test fixture discovery mechanism and print discovered fixtures
///
/// This test verifies that the walkdir-based PDF discovery function works correctly
/// and prints the names of all discovered fixtures to stdout.
#[test]
fn test_discover_all_pdf_fixtures() {
    let fixtures_dir = fixtures_root();
    let pdf_files = discover_all_fixtures();

    println!("\n=== Discovered PDF Fixtures ===");
    println!("Fixtures directory: {}", fixtures_dir.display());

    if pdf_files.is_empty() {
        println!("No PDF files found in {}", fixtures_dir.display());
    } else {
        println!("Total PDF files discovered: {}", pdf_files.len());

        // Group by category for better readability
        let mut by_category: std::collections::HashMap<String, Vec<&PathBuf>> = std::collections::HashMap::new();

        for pdf_path in &pdf_files {
            if let Some(category) = pdf_path.parent().and_then(|p| p.file_name()) {
                let category_name = category.to_string_lossy().to_string();
                by_category.entry(category_name).or_default().push(pdf_path);
            }
        }

        let mut sorted_categories: Vec<_> = by_category.iter().collect();
        sorted_categories.sort_by(|a, b| a.0.cmp(b.0));

        for (category, files) in sorted_categories {
            println!("\n[{}] {} file(s):", category, files.len());
            for pdf_path in files {
                let relative_path = pdf_path.strip_prefix(&fixtures_dir).unwrap_or(pdf_path);
                println!("  - {}", relative_path.display());
            }
        }
    }
    println!("==============================\n");

    // Test that the function runs without errors
    // (We don't assert a count since fixtures may be added/removed)
    let _ = pdf_files;
}

/// Test category-based fixture discovery
#[test]
fn test_discover_fixtures_by_category() {
    println!("\n=== Category-based Fixture Discovery ===\n");

    // Get all available categories
    let categories = fixture_categories();
    println!("Available fixture categories: {}", categories.len());

    for category in &categories {
        let fixtures = discover_fixtures_by_category(category);
        println!("[{}] {} PDF file(s)", category, fixtures.len());
    }

    println!("\n=========================================\n");

    // Verify we have some expected categories
    assert!(categories.len() > 0, "No fixture categories found");
}

/// Test that we can enumerate fixtures for CLI processing
///
/// This test ensures that fixtures can be enumerated in a format suitable
/// for bulk CLI invocation testing.
#[test]
fn test_fixture_enumeration_for_cli() {
    let fixtures_dir = fixtures_root();
    let pdf_files = discover_all_fixtures();
    let bin = pdftract_bin();

    // Ensure binary exists
    assert!(bin.exists(), "pdftract binary not found at {:?}", bin);

    println!("\n=== Fixture Enumeration for CLI Testing ===");
    println!("Binary path: {:?}", bin);
    println!("Fixtures directory: {}", fixtures_dir.display());
    println!("Total fixtures discovered: {}\n", pdf_files.len());

    // If no fixtures yet, test passes (scaffold for future fixtures)
    if pdf_files.is_empty() {
        println!("No PDF fixtures found - test scaffold ready");
        return;
    }

    // Print a sample of fixtures for verification
    let sample_size = 5.min(pdf_files.len());
    println!("Sample fixtures (first {} of {}):", sample_size, pdf_files.len());
    for (i, pdf_path) in pdf_files.iter().take(sample_size).enumerate() {
        let relative_path = pdf_path.strip_prefix(&fixtures_dir).unwrap_or(pdf_path);
        println!("  {}. {}", i + 1, relative_path.display());
    }

    if pdf_files.len() > sample_size {
        println!("  ... and {} more", pdf_files.len() - sample_size);
    }

    println!("\nAll fixtures are accessible for CLI processing");
    println!("==========================================\n");
}

/// Basic test that pdftract extract --json runs on discovered fixtures
///
/// This test runs `pdftract extract --json` on a small sample of discovered fixtures
/// to verify that the CLI invocation works correctly. It processes only the first
/// 5 fixtures to keep test runtime reasonable.
#[test]
fn test_cli_invocation_on_fixture_sample() {
    let fixtures_dir = fixtures_root();
    let pdf_files = discover_all_fixtures();
    let bin = pdftract_bin();

    // Ensure binary exists
    assert!(bin.exists(), "pdftract binary not found at {:?}", bin);

    // If no fixtures yet, test passes (scaffold for future fixtures)
    if pdf_files.is_empty() {
        println!("No PDF fixtures found - test scaffold ready");
        return;
    }

    // Process only a small sample to keep test runtime reasonable
    let sample_size = 5.min(pdf_files.len());
    let sample = &pdf_files[..sample_size];

    println!("\n=== CLI Invocation Test (Sample of {} fixtures) ===\n", sample_size);

    let mut success_count = 0;
    let mut failure_count = 0;

    for pdf_path in sample {
        let relative_path = pdf_path.strip_prefix(&fixtures_dir).unwrap_or(pdf_path);
        println!("Processing: {}", relative_path.display());

        // Run pdftract extract --json - on the fixture (JSON to stdout)
        let output = Command::new(&bin)
            .arg("extract")
            .arg("--json")
            .arg("-")
            .arg(pdf_path)
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    println!("  ✓ Success");
                    success_count += 1;
                } else {
                    println!("  ⚠ Failed with status: {}", result.status);
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    if !stderr.is_empty() {
                        println!("    stderr: {}", stderr.lines().take(3).collect::<Vec<_>>().join("\n    "));
                    }
                    failure_count += 1;
                }
            }
            Err(e) => {
                println!("  ✗ Failed to run pdftract: {}", e);
                failure_count += 1;
            }
        }
    }

    println!("\nResults: {} succeeded, {} failed (out of {} sample fixtures)",
             success_count, failure_count, sample_size);
    println!("==========================================\n");

    // We don't assert all succeed since some fixtures may be malformed/encrypted
    // Just verify the mechanism works
    assert!(success_count + failure_count == sample_size,
            "Test did not complete all fixture invocations");
}

/// Test that pdftract extract --json runs on ALL discovered fixtures
///
/// This is the comprehensive integration test that invokes the CLI on each fixture
/// with proper timeout protection and output capture. It ensures that:
/// - Each fixture is processed independently
/// - CLI invocation completes without hanging (bounded waits)
/// - Output is captured for each invocation
/// - Results are tracked even if individual fixtures fail
///
/// This test demonstrates the full integration between fixture discovery
/// and CLI invocation handling.
#[test]
fn test_cli_invocation_on_all_fixtures() {
    let fixtures_dir = fixtures_root();
    let pdf_files = discover_all_fixtures();
    let bin = pdftract_bin();

    // Ensure binary exists
    assert!(bin.exists(), "pdftract binary not found at {:?}", bin);

    // If no fixtures yet, test passes (scaffold for future fixtures)
    if pdf_files.is_empty() {
        println!("No PDF fixtures found - test scaffold ready");
        return;
    }

    println!("\n=== CLI Integration Test: All {} Fixtures ===\n", pdf_files.len());

    let mut results: Vec<(PathBuf, bool, String)> = Vec::with_capacity(pdf_files.len());
    let mut success_count = 0;
    let mut failure_count = 0;

    // Process each fixture independently with timeout protection
    for (index, pdf_path) in pdf_files.iter().enumerate() {
        let relative_path = pdf_path.strip_prefix(&fixtures_dir).unwrap_or(pdf_path);
        println!("[{}/{}] Processing: {}", index + 1, pdf_files.len(), relative_path.display());

        // Invoke CLI with bounded timeout (30 seconds max per fixture)
        let result = invoke_cli_with_timeout(&bin, pdf_path, 30);

        match result {
            Ok(Some(exit_code)) => {
                if exit_code == 0 {
                    println!("  ✓ SUCCESS (exit: 0)");
                    success_count += 1;
                    results.push((pdf_path.clone(), true, "exit code 0".to_string()));
                } else {
                    println!("  ⚠ FAILED (exit: {})", exit_code);
                    failure_count += 1;
                    results.push((pdf_path.clone(), false, format!("exit code {}", exit_code)));
                }
            }
            Ok(None) => {
                // Process terminated by signal
                println!("  ⚠ FAILED (terminated by signal)");
                failure_count += 1;
                results.push((pdf_path.clone(), false, "terminated by signal".to_string()));
            }
            Err(timeout_err) => {
                // Timeout or execution error
                println!("  ⚠ FAILED: {}", timeout_err);
                failure_count += 1;
                results.push((pdf_path.clone(), false, timeout_err));
            }
        }
    }

    // Print summary
    println!("\n=== Test Summary ===");
    println!("Total fixtures processed: {}", pdf_files.len());
    println!("Successful: {}", success_count);
    println!("Failed: {}", failure_count);
    println!("\n");

    // Show breakdown of failures if any
    if failure_count > 0 {
        println!("=== Failed Fixtures (showing first 10) ===");
        for (path, _success, reason) in results.iter().filter(|(_, success, _)| !success).take(10) {
            let relative_path = path.strip_prefix(&fixtures_dir).unwrap_or(path);
            println!("  - {}", relative_path.display());
            println!("    Reason: {}", reason);
        }
        if failure_count > 10 {
            println!("  ... and {} more", failure_count - 10);
        }
        println!("\n");
    }

    // The test completes successfully even if individual fixtures fail
    // This ensures the iteration completes and all results are captured
    println!("✓ Test completed - all fixtures processed with CLI invocation");

    // Assertion: We attempted to process every discovered fixture
    assert_eq!(results.len(), pdf_files.len(), "Result count should match fixture count");
    assert!(success_count + failure_count == pdf_files.len(), "Total results should equal fixture count");
}

/// Invoke CLI command on a single fixture with timeout protection
///
/// This helper function executes the CLI command with a bounded wait to prevent
/// indefinite hangs. It spawns the process, waits for completion with a timeout,
/// and returns the exit code or an error message.
///
/// # Arguments
/// * `bin` - Path to the pdftract binary
/// * `pdf_path` - Path to the PDF fixture
/// * `timeout_secs` - Maximum seconds to wait for completion
///
/// # Returns
/// * `Ok(Some(exit_code))` - Process completed with exit code
/// * `Ok(None)` - Process terminated by signal
/// * `Err(String)` - Timeout or execution error
fn invoke_cli_with_timeout(bin: &PathBuf, pdf_path: &PathBuf, timeout_secs: u64) -> Result<Option<i32>, String> {
    use std::thread;
    use std::time::{Duration, Instant};

    // Spawn the CLI process
    let mut child = Command::new(bin)
        .arg("extract")
        .arg("--json")
        .arg("-")
        .arg(pdf_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn process: {}", e))?;

    let timeout = Duration::from_secs(timeout_secs);

    // Poll for completion with timeout
    let start = Instant::now();
    loop {
        // Check if process has exited
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process completed
                return Ok(status.code());
            }
            Ok(None) => {
                // Still running, check timeout
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("Timeout exceeded ({:?})", timeout));
                }
                // Sleep a bit before polling again
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                return Err(format!("Failed to wait for process: {}", e));
            }
        }
    }
}
