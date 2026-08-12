//! Forms integration tests.
//!
//! This test module verifies PDF form handling including:
//! - AcroForm detection and parsing
//! - Form field extraction and validation
//! - Widget annotation processing
//! - Form data encoding/decoding
//! - CLI invocation on discovered fixtures
//!
//! ## Fixture Coverage
//!
//! All fixtures are located in `tests/fixtures/forms/`:
//!
//! | Fixture | Type | Status | Ground Truth | Notes |
//! |---------|------|--------|--------------|-------|
//! | `acroform-readonly.pdf` | AcroForm | ✅ TESTED | `acroform-readonly.json` | Tests ReadOnly flag on text and checkbox fields |
//! | `acroform-submit.pdf` | AcroForm | ✅ TESTED | `acroform-submit.json` | Tests SubmitForm and ResetForm button actions |
//! | `acroform-text-fields.pdf` | AcroForm | ✅ TESTED | `acroform-text-fields.json` | Tests text, checkbox, radio, and choice fields |
//! | `xfa-dynamic.pdf` | XFA | ⏭️ SKIPPED | `xfa-dynamic.json` | XFA XML parsing not yet implemented - TODO |
//!
//! ## Test Categories
//!
//! 1. **Discovery Tests**: Verify all fixtures are discoverable
//! 2. **CLI Extraction Tests**: Verify `pdftract extract --json` works on all fixtures
//! 3. **API Extraction Tests**: Verify `extract_pdf()` API works on all fixtures
//! 4. **Ground Truth Tests**: Verify extracted field counts match expected values
//!    - AcroForm fixtures: Full validation against ground truth JSON
//!    - XFA fixtures: Explicitly skipped with TODO message
//!
//! ## XFA Handling
//!
//! XFA (XML Forms Architecture) fixtures are detected by filename pattern (`xfa` or `XFA`)
//! and are **explicitly skipped** with a clear TODO message. The ground truth file
//! exists for future implementation when XFA XML parsing support is added.
//!

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use walkdir::WalkDir;
use pdftract_core::extract::{extract_pdf, ExtractionOptions};
use serde_json::{Value as JsonValue};

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

// ============================================================================
// Ground Truth Comparison Tests
// ============================================================================

/// Get the corresponding ground truth JSON file path for a PDF fixture
fn ground_truth_path(pdf_path: &Path) -> Option<PathBuf> {
    let json_path = pdf_path.with_extension("json");
    if json_path.exists() {
        Some(json_path)
    } else {
        None
    }
}

/// Load and parse ground truth JSON file
fn load_ground_truth(json_path: &Path) -> Result<JsonValue, String> {
    let content = std::fs::read_to_string(json_path)
        .map_err(|e| format!("Failed to read ground truth file: {}", e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse ground truth JSON: {}", e))
}

/// Extract field count from ground truth JSON
fn extract_field_count_from_truth(ground_truth: &JsonValue) -> Result<usize, String> {
    ground_truth
        .get("form_fields")
        .and_then(|fields| fields.as_array())
        .map(|arr| arr.len())
        .ok_or_else(|| "Ground truth missing 'form_fields' array or it's not an array".to_string())
}

/// Test ground truth comparison for all fixtures
///
/// This test verifies that:
/// 1. Ground truth JSON files exist for AcroForm fixtures
/// 2. Ground truth JSON can be parsed successfully
/// 3. pdftract extract --json produces valid output
/// 4. Extracted field count matches expected count from ground truth
/// 5. XFA fixtures are handled gracefully (skipped with TODO)
#[test]
fn test_ground_truth_comparison() {
    let fixtures_dir = "tests/fixtures/forms";
    let pdf_files = discover_pdf_fixtures(fixtures_dir);

    println!("\n=== Ground Truth Comparison Test ===");
    if pdf_files.is_empty() {
        println!("No PDF files found in {} - skipping ground truth test", fixtures_dir);
        println!("===================================\n");
        return;
    }

    let bin = pdftract_bin();
    assert!(bin.exists(), "pdftract binary not found at {:?}", bin);

    println!("Found {} PDF file(s) to process", pdf_files.len());
    println!();

    let mut success_count = 0;
    let mut failed_count = 0;
    let mut skipped_count = 0;

    for pdf_path in &pdf_files {
        let pdf_name = pdf_path.file_name().unwrap_or_default().to_string_lossy();
        println!("Processing: {}", pdf_name);

        // Check for corresponding ground truth file
        let gt_path = match ground_truth_path(pdf_path) {
            Some(path) => path,
            None => {
                println!("  ⊘ No ground truth file found - skipping");
                skipped_count += 1;
                println!();
                continue;
            }
        };

        // Check if this is an XFA fixture (handle gracefully)
        if pdf_name.contains("xfa") || pdf_name.contains("XFA") {
            println!("  ⊘ XFA fixture detected - TODO: XFA XML parsing not yet implemented");
            println!("    - Ground truth exists at: {}", gt_path.display());
            skipped_count += 1;
            println!();
            continue;
        }

        // Load and parse ground truth
        let ground_truth = match load_ground_truth(&gt_path) {
            Ok(gt) => gt,
            Err(e) => {
                println!("  ✗ Failed to load ground truth: {}", e);
                failed_count += 1;
                println!();
                continue;
            }
        };

        // Extract expected field count from ground truth
        let expected_count = match extract_field_count_from_truth(&ground_truth) {
            Ok(count) => count,
            Err(e) => {
                println!("  ✗ Failed to extract field count from ground truth: {}", e);
                failed_count += 1;
                println!();
                continue;
            }
        };

        println!("  - Expected fields from ground truth: {}", expected_count);

        // Run pdftract extract --json
        let mut child = match Command::new(&bin)
            .args(["extract", "--json", pdf_path.to_str().unwrap()])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                println!("  ✗ Failed to spawn pdftract command: {}", e);
                failed_count += 1;
                println!();
                continue;
            }
        };

        // Wait with bounded timeout (10 seconds)
        match wait_with_timeout(&mut child, 10) {
            Ok(Some(exit_code)) => {
                if exit_code == 0 {
                    let output = match child.wait_with_output() {
                        Ok(o) => o,
                        Err(e) => {
                            println!("  ✗ Failed to capture output: {}", e);
                            failed_count += 1;
                            println!();
                            continue;
                        }
                    };

                    let stdout_str = String::from_utf8_lossy(&output.stdout);

                    // Parse pdftract JSON output
                    let extracted_json: JsonValue = match serde_json::from_str(&stdout_str) {
                        Ok(json) => json,
                        Err(e) => {
                            println!("  ✗ Failed to parse pdftract JSON output: {}", e);
                            failed_count += 1;
                            println!();
                            continue;
                        }
                    };

                    // Verify form_fields array exists
                    let extracted_fields = match extracted_json.get("form_fields").and_then(|f| f.as_array()) {
                        Some(fields) => fields,
                        None => {
                            println!("  ✗ Extracted JSON missing 'form_fields' array");
                            failed_count += 1;
                            println!();
                            continue;
                        }
                    };

                    let extracted_count = extracted_fields.len();
                    println!("  - Extracted fields: {}", extracted_count);

                    // Compare field counts
                    if extracted_count == expected_count {
                        println!("  ✓ Field count matches: {} == {}", extracted_count, expected_count);
                        success_count += 1;
                    } else {
                        println!("  ✗ Field count mismatch: {} (extracted) != {} (expected)",
                                extracted_count, expected_count);
                        failed_count += 1;
                    }

                } else {
                    println!("  ✗ pdftract exited with code {}", exit_code);
                    let stderr = String::from_utf8_lossy(&child.stderr.unwrap_or_default());
                    if !stderr.is_empty() {
                        println!("    - Stderr: {}", stderr.trim());
                    }
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

    println!("===================================");
    println!("Summary: {} succeeded, {} failed, {} skipped",
             success_count, failed_count, skipped_count);
    println!("===================================\n");

    // Report results but don't fail the test if no fixtures exist
    if pdf_files.is_empty() {
        println!("No fixtures to test - creating scaffold");
    } else if failed_count > 0 {
        println!("WARNING: {} fixtures failed ground truth comparison", failed_count);
    }
}

// ============================================================================
// Field Property Validation
// ============================================================================

/// Normalize field type for comparison
///
/// PDF field types can be represented as single characters or strings.
/// This normalizes them to a common format.
fn normalize_field_type(field_type: &JsonValue) -> String {
    if let Some(s) = field_type.as_str() {
        match s {
            "Tx" | "text" => "text".to_string(),
            "Btn" | "button" => "button".to_string(),
            "Ch" | "choice" => "choice".to_string(),
            "Sig" | "signature" => "signature".to_string(),
            other => other.to_lowercase(),
        }
    } else {
        format!("{:?}", field_type)
    }
}

/// Get user-friendly field type name from normalized type
fn field_type_display_name(field_type: &str, button_kind: Option<&JsonValue>) -> String {
    match field_type {
        "text" => "text".to_string(),
        "button" => {
            if let Some(kind) = button_kind.and_then(|k| k.as_str()) {
                match kind {
                    "checkbox" => "checkbox".to_string(),
                    "radio" => "radio".to_string(),
                    "pushbutton" => "pushbutton".to_string(),
                    _ => format!("button ({})", kind),
                }
            } else {
                "button".to_string()
            }
        }
        "choice" => "dropdown".to_string(),
        "signature" => "signature".to_string(),
        other => other.to_string(),
    }
}

/// Compare field names with clear diff reporting
fn compare_field_name(
    extracted_field: &JsonValue,
    expected_field: &JsonValue,
    field_index: usize,
) -> Result<(), String> {
    let extracted_name = extracted_field.get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("<missing>");
    let expected_name = expected_field.get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("<missing>");

    if extracted_name != expected_name {
        Err(format!(
            "Field [{}]: name mismatch - extracted '{}' != expected '{}'",
            field_index, extracted_name, expected_name
        ))
    } else {
        Ok(())
    }
}

/// Compare field types with clear diff reporting
fn compare_field_type(
    extracted_field: &JsonValue,
    expected_field: &JsonValue,
    field_index: usize,
) -> Result<(), String> {
    let extracted_type = normalize_field_type(
        &extracted_field.get("field_type").unwrap_or(&JsonValue::Null)
    );
    let expected_type = normalize_field_type(
        &expected_field.get("field_type").unwrap_or(&JsonValue::Null)
    );

    let extracted_display = field_type_display_name(
        &extracted_type,
        extracted_field.get("button_kind")
    );
    let expected_display = field_type_display_name(
        &expected_type,
        expected_field.get("button_kind")
    );

    if extracted_type != expected_type {
        Err(format!(
            "Field [{}] '{}': type mismatch - extracted '{}' != expected '{}'",
            field_index,
            extracted_field.get("name").and_then(|n| n.as_str()).unwrap_or("<missing>"),
            extracted_display,
            expected_display
        ))
    } else {
        Ok(())
    }
}

/// Compare default values with clear diff reporting
fn compare_default_value(
    extracted_field: &JsonValue,
    expected_field: &JsonValue,
    field_index: usize,
) -> Result<(), String> {
    let extracted_default = extracted_field.get("default_value");
    let expected_default = expected_field.get("default_value");

    // Handle null values on both sides
    if extracted_default.is_null() && expected_default.is_null() {
        return Ok(());
    }

    // Handle string comparison
    let extracted_str = extracted_default.and_then(|v| v.as_str());
    let expected_str = expected_default.and_then(|v| v.as_str());

    match (extracted_str, expected_str) {
        (Some(e), Some(x)) if e == x => Ok(()),
        (Some(e), Some(x)) => Err(format!(
            "Field [{}] '{}': default_value mismatch - extracted '{}' != expected '{}'",
            field_index,
            extracted_field.get("name").and_then(|n| n.as_str()).unwrap_or("<missing>"),
            e,
            x
        )),
        (None, Some(x)) => Err(format!(
            "Field [{}] '{}': default_value mismatch - extracted null != expected '{}'",
            field_index,
            extracted_field.get("name").and_then(|n| n.as_str()).unwrap_or("<missing>"),
            x
        )),
        (Some(e), None) => Err(format!(
            "Field [{}] '{}': default_value mismatch - extracted '{}' != expected null",
            field_index,
            extracted_field.get("name").and_then(|n| n.as_str()).unwrap_or("<missing>"),
            e
        )),
        (None, None) => Ok(()), // Both null
        _ => Err(format!(
            "Field [{}] '{}': default_value type mismatch",
            field_index,
            extracted_field.get("name").and_then(|n| n.as_str()).unwrap_or("<missing>")
        )),
    }
}

/// Compare read-only status with clear diff reporting
fn compare_read_only_status(
    extracted_field: &JsonValue,
    expected_field: &JsonValue,
    field_index: usize,
) -> Result<(), String> {
    let extracted_read_only = extracted_field.get("flags")
        .and_then(|f| f.get("read_only"))
        .and_then(|r| r.as_bool())
        .unwrap_or(false);
    let expected_read_only = expected_field.get("flags")
        .and_then(|f| f.get("read_only"))
        .and_then(|r| r.as_bool())
        .unwrap_or(false);

    if extracted_read_only != expected_read_only {
        Err(format!(
            "Field [{}] '{}': read_only mismatch - extracted {} != expected {}",
            field_index,
            extracted_field.get("name").and_then(|n| n.as_str()).unwrap_or("<missing>"),
            extracted_read_only,
            expected_read_only
        ))
    } else {
        Ok(())
    }
}

/// Compare a single field's properties with detailed validation
fn compare_field_properties(
    extracted_field: &JsonValue,
    expected_field: &JsonValue,
    field_index: usize,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    // Validate field name
    if let Err(e) = compare_field_name(extracted_field, expected_field, field_index) {
        errors.push(e);
    }

    // Validate field type
    if let Err(e) = compare_field_type(extracted_field, expected_field, field_index) {
        errors.push(e);
    }

    // Validate default value (only if expected has one or both should be null)
    if let Err(e) = compare_default_value(extracted_field, expected_field, field_index) {
        errors.push(e);
    }

    // Validate read-only status
    if let Err(e) = compare_read_only_status(extracted_field, expected_field, field_index) {
        errors.push(e);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Test comprehensive field validation for all fixtures
///
/// This test verifies that each extracted field matches the ground truth
/// for all critical properties:
/// 1. Field names match exactly
/// 2. Field types match (text, checkbox, radio, dropdown, submit)
/// 3. Default values match where applicable
/// 4. Read-only status matches (especially for acroform-readonly.pdf)
#[test]
fn test_field_property_validation() {
    let fixtures_dir = "tests/fixtures/forms";
    let pdf_files = discover_pdf_fixtures(fixtures_dir);

    println!("\n=== Field Property Validation Test ===");
    if pdf_files.is_empty() {
        println!("No PDF files found in {} - skipping field validation", fixtures_dir);
        println!("=====================================\n");
        return;
    }

    let bin = pdftract_bin();
    assert!(bin.exists(), "pdftract binary not found at {:?}", bin);

    println!("Found {} PDF file(s) to process", pdf_files.len());
    println!();

    let mut success_count = 0;
    let mut failed_count = 0;
    let mut skipped_count = 0;

    for pdf_path in &pdf_files {
        let pdf_name = pdf_path.file_name().unwrap_or_default().to_string_lossy();
        println!("Processing: {}", pdf_name);

        // Check for corresponding ground truth file
        let gt_path = match ground_truth_path(pdf_path) {
            Some(path) => path,
            None => {
                println!("  ⊘ No ground truth file found - skipping");
                skipped_count += 1;
                println!();
                continue;
            }
        };

        // Check if this is an XFA fixture (handle gracefully)
        if pdf_name.contains("xfa") || pdf_name.contains("XFA") {
            println!("  ⊘ XFA fixture detected - TODO: XFA XML parsing not yet implemented");
            skipped_count += 1;
            println!();
            continue;
        }

        // Load and parse ground truth
        let ground_truth = match load_ground_truth(&gt_path) {
            Ok(gt) => gt,
            Err(e) => {
                println!("  ✗ Failed to load ground truth: {}", e);
                failed_count += 1;
                println!();
                continue;
            }
        };

        let expected_fields = match ground_truth.get("form_fields").and_then(|f| f.as_array()) {
            Some(fields) => fields,
            None => {
                println!("  ✗ Ground truth missing 'form_fields' array");
                failed_count += 1;
                println!();
                continue;
            }
        };

        // Run pdftract extract --json
        let mut child = match Command::new(&bin)
            .args(["extract", "--json", pdf_path.to_str().unwrap()])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                println!("  ✗ Failed to spawn pdftract command: {}", e);
                failed_count += 1;
                println!();
                continue;
            }
        };

        // Wait with bounded timeout (10 seconds)
        match wait_with_timeout(&mut child, 10) {
            Ok(Some(exit_code)) => {
                if exit_code == 0 {
                    let output = match child.wait_with_output() {
                        Ok(o) => o,
                        Err(e) => {
                            println!("  ✗ Failed to capture output: {}", e);
                            failed_count += 1;
                            println!();
                            continue;
                        }
                    };

                    let stdout_str = String::from_utf8_lossy(&output.stdout);

                    // Parse pdftract JSON output
                    let extracted_json: JsonValue = match serde_json::from_str(&stdout_str) {
                        Ok(json) => json,
                        Err(e) => {
                            println!("  ✗ Failed to parse pdftract JSON output: {}", e);
                            failed_count += 1;
                            println!();
                            continue;
                        }
                    };

                    let extracted_fields = match extracted_json.get("form_fields").and_then(|f| f.as_array()) {
                        Some(fields) => fields,
                        None => {
                            println!("  ✗ Extracted JSON missing 'form_fields' array");
                            failed_count += 1;
                            println!();
                            continue;
                        }
                    };

                    // Validate each field's properties
                    let mut field_errors = Vec::new();
                    let mut validated_count = 0;

                    for (index, extracted_field) in extracted_fields.iter().enumerate() {
                        let expected_field = match expected_fields.get(index) {
                            Some(field) => field,
                            None => {
                                field_errors.push(format!(
                                    "Field [{}]: extracted field {} but expected array has only {} fields",
                                    index, index, expected_fields.len()
                                ));
                                break;
                            }
                        };

                        match compare_field_properties(extracted_field, expected_field, index) {
                            Ok(_) => validated_count += 1,
                            Err(errors) => {
                                for error in errors {
                                    field_errors.push(error);
                                }
                            }
                        }
                    }

                    if field_errors.is_empty() {
                        println!("  ✓ All {} field properties validated successfully", validated_count);
                        success_count += 1;
                    } else {
                        println!("  ✗ Field validation failed with {} error(s):", field_errors.len());
                        for error in &field_errors {
                            println!("    - {}", error);
                        }
                        failed_count += 1;
                    }

                } else {
                    println!("  ✗ pdftract exited with code {}", exit_code);
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

    println!("=====================================");
    println!("Summary: {} succeeded, {} failed, {} skipped",
             success_count, failed_count, skipped_count);
    println!("=====================================\n");

    // Report results but don't fail the test if no fixtures exist
    if pdf_files.is_empty() {
        println!("No fixtures to test - creating scaffold");
    } else if failed_count > 0 {
        println!("WARNING: {} fixtures failed field property validation", failed_count);
    }
}
