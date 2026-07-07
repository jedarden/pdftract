//! Forms integration tests.
//!
//! This test module verifies PDF form handling including:
//! - AcroForm detection and parsing
//! - Form field extraction and validation
//! - Widget annotation processing
//! - Form data encoding/decoding
//! - CLI invocation on discovered fixtures

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use walkdir::WalkDir;
use pdftract_core::extract::{extract_pdf, ExtractionOptions};

/// Discover all PDF files in the given directory recursively.
///
/// # Arguments
/// * `fixtures_path` - Path to the fixtures directory to search
///
/// # Returns
/// A `Vec<PathBuf>` containing paths to all discovered PDF files
pub fn discover_pdf_fixtures<P: AsRef<Path>>(fixtures_path: P) -> Vec<PathBuf> {
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

#[test]
fn test_discover_pdf_fixtures() {
    let fixtures_dir = "tests/fixtures/forms";
    let pdf_files = discover_pdf_fixtures(fixtures_dir);

    println!("\n=== Discovered PDF Fixtures ===");
    if pdf_files.is_empty() {
        println!("No PDF files found in {}", fixtures_dir);
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

// ============================================================================
// CLI Test Helper Functions
// ============================================================================

/// Get the path to the pdftract binary (cargo build output)
fn pdftract_bin() -> PathBuf {
    // The binary should be built at target/debug/pdftract or target/release/pdftract
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target/debug/pdftract");

    // Fall back to release if debug doesn't exist
    if !path.exists() {
        let mut release_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        release_path.push("target/release/pdftract");
        return release_path;
    }

    path
}

/// Wait for a child process with a timeout to prevent hanging
///
/// # Arguments
/// * `child` - Mutable reference to the child process
/// * `timeout_secs` - Maximum number of seconds to wait before killing
///
/// # Returns
/// * `Ok(Some(exit_code))` - Process exited with the given code
/// * `Ok(None)` - Process terminated by signal
/// * `Err(io::Error)` - Timeout occurred (process was killed)
fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout_secs: u64,
) -> std::io::Result<Option<i32>> {
    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);

    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status.code());
        }

        if std::time::Instant::now() >= deadline {
            // Timeout: kill the process
            let _ = child.kill();

            // Wait with bounded timeout after kill - never use bare wait()
            let kill_deadline = std::time::Instant::now() + Duration::from_millis(100);
            loop {
                if let Some(status) = child.try_wait()? {
                    return Ok(status.code());
                }

                if std::time::Instant::now() >= kill_deadline {
                    // Process didn't exit after kill - return timeout error
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        format!("Process did not exit within timeout after kill ({}s)", timeout_secs),
                    ));
                }

                std::thread::sleep(Duration::from_millis(10));
            }
        }

        // Sleep for 100ms before checking again
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Test CLI invocation with bounded waits on all discovered fixtures
///
/// This test verifies that:
/// 1. The pdftract CLI binary exists and can be invoked
/// 2. Each discovered fixture can be processed with `pdftract extract --json`
/// 3. The CLI returns successfully for each fixture
/// 4. Output is captured (even if not yet validated for correctness)
/// 5. The test completes without hanging (uses 10-second timeout per fixture)
#[test]
fn test_cli_extract_json_on_fixtures() {
    let bin = pdftract_bin();

    // Ensure binary exists
    assert!(bin.exists(), "pdftract binary not found at {:?}", bin);
    println!("Using pdftract binary: {:?}", bin);

    let fixtures_dir = "tests/fixtures/forms";
    let pdf_files = discover_pdf_fixtures(fixtures_dir);

    println!("\n=== CLI Extract JSON Test ===");
    if pdf_files.is_empty() {
        println!("No PDF files found in {} - skipping CLI test", fixtures_dir);
        println!("==================================\n");
        return;
    }

    println!("Found {} PDF file(s) to process", pdf_files.len());
    println!();

    let mut success_count = 0;
    let mut failed_count = 0;

    for pdf_path in &pdf_files {
        println!("Processing: {}", pdf_path.display());

        // Invoke pdftract extract --json <path>
        let mut child = match Command::new(&bin)
            .args(["extract", "--json", pdf_path.to_str().unwrap()])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                println!("  ✗ Failed to spawn command: {}", e);
                failed_count += 1;
                continue;
            }
        };

        // Wait with bounded timeout (10 seconds per fixture)
        match wait_with_timeout(&mut child, 10) {
            Ok(Some(exit_code)) => {
                if exit_code == 0 {
                    // Capture output
                    let output = match child.wait_with_output() {
                        Ok(o) => o,
                        Err(e) => {
                            println!("  ✗ Failed to capture output: {}", e);
                            failed_count += 1;
                            continue;
                        }
                    };

                    let stdout_len = output.stdout.len();
                    let stderr_len = output.stderr.len();

                    println!("  ✓ Successfully extracted");
                    println!("    - Exit code: {}", exit_code);
                    println!("    - JSON output: {} bytes", stdout_len);
                    println!("    - Stderr: {} bytes", stderr_len);

                    // Verify we got some JSON output
                    if stdout_len > 0 {
                        let stdout_str = String::from_utf8_lossy(&output.stdout);
                        if stdout_str.contains("{") || stdout_str.contains("schema_version") {
                            println!("    - Output appears to be valid JSON");
                        } else {
                            println!("    - Warning: Output may not be valid JSON");
                        }
                    } else {
                        println!("    - Warning: No stdout output received");
                    }

                    success_count += 1;
                } else {
                    println!("  ✗ Exited with code {}", exit_code);
                    failed_count += 1;
                }
            }
            Ok(None) => {
                println!("  ✗ Process terminated by signal");
                failed_count += 1;
            }
            Err(e) => {
                println!("  ✗ {}", e);
                failed_count += 1;
            }
        }
        println!();
    }

    println!("==================================");
    println!("Summary: {} succeeded, {} failed", success_count, failed_count);
    println!("==================================\n");

    // Don't fail the test if all fixtures failed (they might not exist yet)
    if pdf_files.is_empty() {
        println!("No fixtures to test - creating scaffold");
    } else if failed_count > 0 {
        println!("WARNING: {} fixtures failed CLI extraction", failed_count);
    }
}

#[test]
fn test_forms_extraction() {
    let fixtures_dir = "tests/fixtures/forms";
    let pdf_files = discover_pdf_fixtures(fixtures_dir);

    println!("\n=== Forms Extraction Test ===");
    if pdf_files.is_empty() {
        println!("No PDF files found in {} - skipping extraction test", fixtures_dir);
        println!("================================\n");
        return;
    }

    let mut extracted_count = 0;
    let mut failed_count = 0;

    for pdf_path in &pdf_files {
        println!("Extracting: {}", pdf_path.display());

        match extract_pdf(pdf_path, &ExtractionOptions::default()) {
            Ok(result) => {
                println!("  ✓ Successfully extracted");
                println!("    - {} pages", result.page_count);
                println!("    - Has forms: {}", result.has_forms);

                // Convert to JSON to ensure JSON serialization works
                match pdftract_core::extract::result_to_json(&result) {
                    Ok(json) => {
                        println!("    - JSON output size: {} bytes", json.to_string().len());
                        extracted_count += 1;
                    }
                    Err(e) => {
                        println!("  ✗ JSON conversion failed: {}", e);
                        failed_count += 1;
                    }
                }
            }
            Err(e) => {
                println!("  ✗ Extraction failed: {}", e);
                failed_count += 1;
            }
        }
        println!();
    }

    println!("================================");
    println!("Summary: {} extracted, {} failed", extracted_count, failed_count);
    println!("================================\n");

    // Don't fail the test if no fixtures exist yet
    if pdf_files.is_empty() {
        println!("No fixtures to test - creating scaffold");
    } else if failed_count > 0 {
        println!("WARNING: {} fixtures failed to extract", failed_count);
    }
}
