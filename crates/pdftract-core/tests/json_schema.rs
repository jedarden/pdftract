//! JSON Schema validation tests for PDF extraction output.
//!
//! These tests verify that extraction output conforms to the published
//! JSON Schema at docs/schema/v1.0/pdftract.schema.json.
//!
//! The schema validator catches regressions where code changes emit
//! fields not in the schema or omit required fields, breaking downstream
//! clients that rely on schema compatibility.
//!
//! # Test fixtures
//!
//! Fixtures are located in tests/fixtures/json_schema/. Each PDF file
//! should have a corresponding .expected.json file with the known-good
//! extraction output for regression testing. If the .expected.json is
//! missing, the test will still validate against the schema but won't
//! catch semantic regressions.
//!
//! # Adding new fixtures
//!
//! 1. Place the PDF in tests/fixtures/json_schema/
//! 2. Run `pdftract extract -o expected.json <pdf>` to generate output
//! 3. Rename expected.json to <name>.expected.json
//! 4. Commit both files

use std::fs;
use std::path::PathBuf;

use pdftract_core::extract::{extract_pdf, result_to_json};
use pdftract_core::options::ExtractionOptions;
use serde_json::{json, Value};

/// The JSON Schema for pdftract extraction output v1.0.
///
/// Loaded from the committed schema file, not regenerated on-the-fly.
/// Schema regeneration is a separate CI gate (pdftract-2qw5j).
const SCHEMA_JSON: &str = include_str!("../../../docs/schema/v1.0/pdftract.schema.json");

/// Compiled JSON Schema validator.
///
/// Initialized once and reused across all tests for efficiency.
static SCHEMA: once_cell::sync::Lazy<jsonschema::Validator> =
    once_cell::sync::Lazy::new(|| {
        let schema: Value = serde_json::from_str(SCHEMA_JSON)
            .expect("Schema file is valid JSON");
        jsonschema::validator_for(&schema)
            .expect("Schema is valid JSON Schema Draft 2020-12")
    });

/// Format a validation error into a human-readable message with path.
fn format_validation_error(error: &jsonschema::ValidationError) -> String {
    format!("  - Path '{}': {:?}", error.instance_path, error.kind)
}

/// A single test fixture for JSON schema validation.
struct Fixture {
    /// Fixture name (filename without extension)
    name: String,
    /// Path to the PDF fixture file
    pdf_path: PathBuf,
    /// Path to the expected JSON output (if exists)
    expected_path: Option<PathBuf>,
}

impl Fixture {
    /// Load all fixtures from the fixtures directory.
    ///
    /// Scans tests/fixtures/json_schema/ for *.pdf files and
    /// builds fixture objects with corresponding .expected.json
    /// paths if they exist.
    fn load_all() -> Vec<Self> {
        let fixtures_dir = PathBuf::from("tests/fixtures/json_schema");
        let mut fixtures = Vec::new();

        // Create fixtures directory if it doesn't exist
        if !fixtures_dir.exists() {
            fs::create_dir_all(&fixtures_dir)
                .expect("Failed to create fixtures directory");
        }

        // Scan for PDF files
        let entries = fs::read_dir(&fixtures_dir)
            .unwrap_or_else(|e| panic!("Failed to read fixtures directory: {}", e));

        for entry in entries {
            let entry = entry.expect("Failed to read directory entry");
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
                let name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .expect("Invalid PDF filename")
                    .to_string();

                let expected_path = path.with_extension("expected.json");
                let expected_path = if expected_path.exists() {
                    Some(expected_path)
                } else {
                    None
                };

                fixtures.push(Fixture {
                    name,
                    pdf_path: path,
                    expected_path,
                });
            }
        }

        // Sort by name for deterministic test order
        fixtures.sort_by(|a, b| a.name.cmp(&b.name));

        fixtures
    }

    /// Validate this fixture against the JSON schema.
    ///
    /// Extracts the PDF, serializes to JSON, and validates against
    /// the schema. If expected.json exists, also validates that
    /// extraction output is semantically identical.
    fn validate(&self) {
        println!("Validating fixture: {}", self.name);

        // Extract PDF to ExtractionResult
        let extraction_result = extract_pdf(
            &self.pdf_path,
            &ExtractionOptions::default(),
        ).unwrap_or_else(|e| panic!("Failed to extract fixture {}: {}", self.name, e));

        // Convert to JSON
        let json_value = result_to_json(&extraction_result);
        let json_str = serde_json::to_string_pretty(&json_value)
            .unwrap_or_else(|e| panic!("Failed to serialize fixture {} to JSON: {}", self.name, e));

        // Validate against schema (collect all errors for comprehensive report)
        let errors: Vec<_> = SCHEMA.iter_errors(&json_value).collect();

        if !errors.is_empty() {
            // Collect all validation errors for a comprehensive report
            let error_details: Vec<String> = errors
                .iter()
                .map(|e| format!("  - Path '{}': {:?}", e.instance_path, e.kind))
                .collect();

            panic!(
                "\n=== JSON Schema Validation Failed ===\n\
                 Fixture: {}\n\
                 Schema violations:\n{}\n\
                 Output JSON:\n{}\n\
                 ====================================\n",
                self.name,
                error_details.join("\n"),
                json_str
            );
        }

        // If expected.json exists, validate semantic equivalence
        if let Some(ref expected_path) = self.expected_path {
            let expected_str = fs::read_to_string(expected_path)
                .unwrap_or_else(|e| panic!("Failed to read expected.json for {}: {}", self.name, e));

            let expected: Value = serde_json::from_str(&expected_str)
                .unwrap_or_else(|e| panic!("Failed to parse expected.json for {}: {}", self.name, e));

            // Deep equality check for semantic equivalence
            if expected != json_value {
                println!("\n=== Semantic Mismatch ===");
                println!("Fixture: {}", self.name);
                println!("Expected: {}", serde_json::to_string_pretty(&expected).unwrap());
                println!("Actual: {}", json_str);
                println!("========================\n");
                panic!("Fixture {} output does not match expected.json", self.name);
            }
        }
    }
}

#[test]
fn test_all_fixtures_validate_against_schema() {
    let fixtures = Fixture::load_all();

    if fixtures.is_empty() {
        println!("No fixtures found in tests/fixtures/json_schema/");
        println!("Create at least one fixture PDF to enable schema validation tests.");
        return;
    }

    println!("Running JSON schema validation on {} fixtures", fixtures.len());

    for fixture in &fixtures {
        fixture.validate();
    }

    println!("All {} fixtures validated successfully", fixtures.len());
}

#[test]
fn test_schema_itself_is_valid() {
    // Verify the schema file is valid JSON Schema Draft 2020-12
    let schema: Value = serde_json::from_str(SCHEMA_JSON)
        .expect("Schema file is valid JSON");

    // validator_for should succeed if schema is valid
    let _compiled = jsonschema::validator_for(&schema)
        .expect("Schema is valid JSON Schema Draft 2020-12");

    // Verify top-level structure
    assert!(
        schema.get("$schema").is_some(),
        "Schema must declare $schema version"
    );
    assert!(
        schema.get("$id").is_some(),
        "Schema must declare $id"
    );
    assert!(
        schema.get("properties").is_some(),
        "Schema must have properties object"
    );

    println!("Schema file is valid JSON Schema Draft 2020-12");
}

#[test]
fn test_schema_has_required_document_level_fields() {
    let schema: Value = serde_json::from_str(SCHEMA_JSON).unwrap();
    let properties = schema.get("properties")
        .and_then(|p| p.as_object())
        .expect("Schema properties must be an object");

    // Verify required document-level fields exist
    let required_fields = vec![
        "schema_version",
        "metadata",
        "pages",
        "errors",
        "extraction_quality",
    ];

    for field in required_fields {
        assert!(
            properties.contains_key(field),
            "Schema must have document-level field: {}",
            field
        );
    }

    // Verify required fields are marked as required
    let required = schema.get("required")
        .and_then(|r| r.as_array())
        .expect("Schema must have required array");

    assert!(
        required.iter().any(|v| v == "schema_version"),
        "schema_version must be required"
    );
    assert!(
        required.iter().any(|v| v == "metadata"),
        "metadata must be required"
    );

    println!("Schema has all required document-level fields");
}

#[test]
fn test_schema_page_json_structure() {
    let schema: Value = serde_json::from_str(SCHEMA_JSON).unwrap();

    // Navigate to PageJson definition
    let page_json = schema.get("$defs")
        .and_then(|defs| defs.get("PageJson"))
        .expect("Schema must define PageJson");

    let page_props = page_json.get("properties")
        .and_then(|p| p.as_object())
        .expect("PageJson must have properties");

    // Verify critical page fields exist
    let required_page_fields = vec![
        "page_index",
        "page_number",
        "width",
        "height",
        "rotation",
        "type",
    ];

    for field in required_page_fields {
        assert!(
            page_props.contains_key(field),
            "PageJson must have field: {}",
            field
        );
    }

    // Verify arrays with default values
    let array_fields = vec!["spans", "blocks", "tables", "annotations"];
    for field in array_fields {
        let field_def = page_props.get(field)
            .expect(format!("PageJson must have field: {}", field).as_str());
        assert!(
            field_def.get("type").and_then(|t| t.as_str()) == Some("array"),
            "PageJson.{} must be an array",
            field
        );
    }

    println!("PageJson structure is valid");
}

#[test]
fn test_schema_span_json_structure() {
    let schema: Value = serde_json::from_str(SCHEMA_JSON).unwrap();

    // Navigate to SpanJson definition
    let span_json = schema.get("$defs")
        .and_then(|defs| defs.get("SpanJson"))
        .expect("Schema must define SpanJson");

    let span_props = span_json.get("properties")
        .and_then(|p| p.as_object())
        .expect("SpanJson must have properties");

    // Verify critical span fields exist
    let required_span_fields = vec![
        "text",
        "bbox",
        "font",
        "size",
    ];

    for field in required_span_fields {
        assert!(
            span_props.contains_key(field),
            "SpanJson must have field: {}",
            field
        );
    }

    println!("SpanJson structure is valid");
}

#[test]
fn test_synthetic_output_validates() {
    // Create a minimal valid JSON structure and verify it validates
    // This tests that the schema itself is correctly structured
    let json_value = json!({
        "schema_version": "1.0",
        "metadata": {
            "page_count": 1,
            "is_tagged": false,
            "is_encrypted": false,
            "contains_javascript": false,
            "contains_xfa": false,
            "ocg_present": false,
            "conformance": "none",
            "javascript_actions": []
        },
        "outline": [],
        "threads": [],
        "attachments": [],
        "signatures": [],
        "form_fields": [],
        "links": [],
        "pages": [{
            "page_index": 0,
            "page_number": 1,
            "width": 612.0,
            "height": 792.0,
            "rotation": 0,
            "type": "text",
            "spans": [],
            "blocks": [],
            "tables": [],
            "annotations": []
        }],
        "extraction_quality": {
            "overall_quality": "none"
        },
        "errors": []
    });

    let errors: Vec<_> = SCHEMA.iter_errors(&json_value).collect();

    if !errors.is_empty() {
        let error_details: Vec<String> = errors
            .iter()
            .map(|e| format!("  - Path '{}': {:?}", e.instance_path, e.kind))
            .collect();
        panic!(
            "Minimal JSON failed schema validation:\n{}\nJSON:\n{}",
            error_details.join("\n"),
            serde_json::to_string_pretty(&json_value).unwrap()
        );
    }

    println!("Minimal JSON validates successfully");
}

#[test]
#[ignore = "Diagnostic test - run with cargo test -- --ignored"]
fn debug_list_available_fixtures() {
    let fixtures = Fixture::load_all();

    if fixtures.is_empty() {
        println!("No fixtures found in tests/fixtures/json_schema/");
    } else {
        println!("Available fixtures ({} total):", fixtures.len());
        for fixture in &fixtures {
            let has_expected = if fixture.expected_path.is_some() { " [has expected.json]" } else { "" };
            println!("  - {}{}", fixture.name, has_expected);
        }
    }
}
