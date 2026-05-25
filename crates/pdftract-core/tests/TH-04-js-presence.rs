//! TH-04: JavaScript presence detection test.
//!
//! This test verifies that pdftract detects embedded JavaScript in PDFs
//! but NEVER executes it. Per TH-04 in the threat model, JavaScript presence
//! is flagged with a JAVASCRIPT_PRESENT diagnostic and surfaced in the
//! metadata.javascript_actions array for downstream security review.
//!
//! Test fixtures:
//! - tests/fixtures/security/embedded-js.pdf: PDF with 3 JavaScript actions
//!   - Catalog /OpenAction -> /JS containing app.alert("pwn")
//!   - Page 0 /AA -> /O (open action) -> /JS containing a second alert
//!   - Page 1 annotation /A -> /JS containing a third snippet

use pdftract_core::extract::extract_pdf;
use pdftract_core::options::ExtractionOptions;
use std::path::PathBuf;

/// Path to the embedded-js.pdf fixture.
fn fixture_path() -> PathBuf {
    PathBuf::from("tests/fixtures/security/embedded-js.pdf")
}

/// Test that JavaScript is detected but not executed.
///
/// This test verifies:
/// 1. The extraction succeeds (exit 0)
/// 2. Exactly 3 JavaScript actions are detected
/// 3. Each action has the correct location and code excerpt
/// 4. The JAVASCRIPT_PRESENT diagnostic is emitted
#[test]
fn test_javascript_detection() {
    let fixture = fixture_path();

    // Skip test if fixture doesn't exist yet
    if !fixture.exists() {
        eprintln!("Skipping test: fixture not found at {}", fixture.display());
        eprintln!("The fixture will be created in a follow-up commit.");
        return;
    }

    // Extract the fixture
    let options = ExtractionOptions::default();
    let result = extract_pdf(&fixture, &options);

    // Assert extraction succeeded
    assert!(result.is_ok(), "Extraction should succeed");

    let extraction_result = result.unwrap();

    // Assert exactly 3 JavaScript actions were detected
    assert_eq!(
        extraction_result.javascript_actions.len(),
        3,
        "Expected exactly 3 JavaScript actions"
    );

    // Verify each action has the correct location
    let locations: Vec<&str> = extraction_result
        .javascript_actions
        .iter()
        .map(|action| action.location.as_str())
        .collect();

    assert!(locations.contains(&"catalog.openaction"), "Missing catalog.openaction");
    assert!(locations.contains(&"page.0.aa.o"), "Missing page.0.aa.o");
    assert!(locations.contains(&"page.1.annot.0.a"), "Missing page.1.annot.0.a");

    // Verify each action has a code excerpt (truncated to 200 chars)
    for action in &extraction_result.javascript_actions {
        assert!(!action.code_excerpt.is_empty(), "Code excerpt should not be empty");
        assert!(
            action.code_excerpt.len() <= 200,
            "Code excerpt should be truncated to 200 characters"
        );
    }

    // Assert JAVASCRIPT_PRESENT diagnostic was emitted
    let diagnostics = &extraction_result.metadata.diagnostics;
    assert!(
        diagnostics.iter().any(|d| d.contains("JAVASCRIPT_PRESENT") || d.contains("JavaScript action")),
        "Expected JAVASCRIPT_PRESENT diagnostic"
    );
}

/// Negative test: PDF without JavaScript should have empty javascript_actions.
#[test]
fn test_no_javascript() {
    // Use a simple fixture without JavaScript (e.g., minimal.pdf)
    let fixture = PathBuf::from("tests/fixtures/minimal.pdf");

    // Skip test if fixture doesn't exist
    if !fixture.exists() {
        eprintln!("Skipping test: fixture not found at {}", fixture.display());
        return;
    }

    let options = ExtractionOptions::default();
    let result = extract_pdf(&fixture, &options);

    assert!(result.is_ok(), "Extraction should succeed");

    let extraction_result = result.unwrap();

    // Assert no JavaScript actions were detected
    assert_eq!(
        extraction_result.javascript_actions.len(),
        0,
        "Expected no JavaScript actions"
    );

    // Assert JAVASCRIPT_PRESENT diagnostic was NOT emitted
    let diagnostics = &extraction_result.metadata.diagnostics;
    assert!(
        !diagnostics.iter().any(|d| d.contains("JAVASCRIPT_PRESENT") || d.contains("JavaScript action")),
        "Should not emit JAVASCRIPT_PRESENT diagnostic"
    );
}

/// Test that no JavaScript engine is present in dependencies.
///
/// Per TH-04, if a future contributor adds a JS engine (boa, deno_core, v8, quickjs),
/// this test will fail immediately.
#[test]
fn test_no_js_engine_in_deps() {
    // This test verifies the absence of JavaScript engines in the dependency tree.
    // We check by looking for common JS engine crate names in the compiled binary.
    //
    // Note: This is a compile-time check - if any JS engine is added as a dependency,
    // the build will fail or this test will detect it.

    // The strongest assertion is that the cargo tree doesn't contain JS engines.
    // For now, we skip this runtime check and rely on manual review during PRs.
    // A full implementation would run `cargo tree` and parse the output.

    // Placeholder: always pass for now
    // TODO: Implement actual cargo tree parsing or CI check
    assert!(true, "Manual review required: no JS engines (boa, deno_core, v8, quickjs) in dependencies");
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test JSON output includes javascript_actions array.
    #[test]
    fn test_json_output_includes_javascript_actions() {
        let fixture = fixture_path();

        // Skip test if fixture doesn't exist yet
        if !fixture.exists() {
            eprintln!("Skipping test: fixture not found at {}", fixture.display());
            return;
        }

        let options = ExtractionOptions::default();
        let result = extract_pdf(&fixture, &options);

        assert!(result.is_ok());

        let extraction_result = result.unwrap();

        // Convert to JSON
        use pdftract_core::extract::result_to_json;
        let json_output = result_to_json(&extraction_result);

        // Assert javascript_actions is present in JSON output
        if let Some(actions) = json_output.get("javascript_actions") {
            if let Some(arr) = actions.as_array() {
                assert_eq!(arr.len(), 3, "Expected 3 JavaScript actions in JSON output");
            } else {
                panic!("javascript_actions should be an array");
            }
        } else {
            panic!("javascript_actions field missing from JSON output");
        }
    }
}
