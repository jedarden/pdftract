//! JSON Schema validation integration tests.
//!
//! These tests verify that pdftract extraction outputs conform to the
//! published JSON Schema at docs/schema/v1.0/pdftract.schema.json.
//!
//! Per bead pdftract-2rc4 (Phase 6.1.4), this is a regression guard:
//! any code change that emits a field not in the schema, or omits a
//! required one, fails CI.
//!
//! Test workflow:
//! 1. Walk tests/fixtures/json_schema/ for *.pdf inputs
//! 2. Extract each PDF to JSON using pdftract_core
//! 3. Validate the JSON against the bundled schema
//! 4. Fail on any validation errors
//!
//! Fixtures with expected JSON files (.expected.json) are verified for
//! exact match. Fixtures without expected files generate them for
//! manual review on first run.

use pdftract_core::extract::extract_pdf;
use pdftract_core::options::ExtractionOptions;
use std::fs;
use std::path::PathBuf;

/// Fixture directory for JSON schema validation tests
const FIXTURES_DIR: &str = "tests/fixtures/json_schema";

/// A single test fixture for JSON schema validation.
struct Fixture {
    name: String,
    pdf_path: PathBuf,
    expected_path: Option<PathBuf>,
}

impl Fixture {
    /// Load all fixtures from the fixtures directory.
    fn load_all() -> Vec<Self> {
        let fixtures_dir = PathBuf::from(FIXTURES_DIR);
        let mut fixtures = Vec::new();

        let entries = fs::read_dir(&fixtures_dir).unwrap_or_else(|e| {
            panic!(
                "Failed to read fixtures directory '{}': {}",
                FIXTURES_DIR, e
            )
        });

        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();

            // Only process PDF files
            if path.extension().and_then(|s| s.to_str()) != Some("pdf") {
                continue;
            }

            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            let expected_path = path.with_extension("expected.json");

            fixtures.push(Fixture {
                name,
                pdf_path: path,
                expected_path: if expected_path.exists() {
                    Some(expected_path)
                } else {
                    None
                },
            });
        }

        // Sort for deterministic test order
        fixtures.sort_by(|a, b| a.name.cmp(&b.name));
        fixtures
    }
}

/// Load the bundled JSON Schema for validation.
fn load_schema() -> jsonschema::Validator {
    let schema_json = include_str!("../../../docs/schema/v1.0/pdftract.schema.json");
    let schema: serde_json::Value =
        serde_json::from_str(schema_json).expect("Bundled schema is not valid JSON");
    jsonschema::validator_for(&schema).expect("Bundled schema is not valid JSON Schema")
}

/// Validate a JSON value against the schema.
///
/// Returns Ok(()) if validation passes, Err with error details otherwise.
fn validate_json(
    schema: &jsonschema::Validator,
    value: &serde_json::Value,
) -> Result<(), Vec<String>> {
    let result = schema.validate(value);
    match result {
        Ok(_) => Ok(()),
        Err(error) => {
            // If there's at least one error, collect all errors using iter_errors
            let error_details: Vec<String> = schema
                .iter_errors(value)
                .map(|e| {
                    let path = e.instance_path.to_string();
                    format!("{} {}", path, e)
                })
                .collect();
            Err(error_details)
        }
    }
}

/// Test a single fixture for schema compliance.
fn test_fixture(fixture: &Fixture) {
    println!("Testing fixture: {}", fixture.name);

    // Load the schema
    let schema = load_schema();

    // Extract PDF to JSON
    let extraction_result = extract_pdf(&fixture.pdf_path, &ExtractionOptions::default())
        .unwrap_or_else(|e| panic!("Failed to extract fixture '{}': {}", fixture.name, e));

    // Convert to JSON using the same serialization as the CLI
    let json_value = pdftract_core::extract::result_to_json(&extraction_result);

    // Validate against schema
    if let Err(validation_errors) = validate_json(&schema, &json_value) {
        panic!(
            "Fixture '{}' failed schema validation with {} error(s):\n{}",
            fixture.name,
            validation_errors.len(),
            validation_errors.join("\n")
        );
    }

    // If expected JSON exists, verify exact match (for regression detection)
    if let Some(ref expected_path) = fixture.expected_path {
        let expected_json = fs::read_to_string(expected_path).unwrap_or_else(|e| {
            panic!("Failed to read expected JSON for '{}': {}", fixture.name, e)
        });

        let expected_value: serde_json::Value = serde_json::from_str(&expected_json)
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to parse expected JSON for '{}': {}",
                    fixture.name, e
                )
            });

        if json_value != expected_value {
            // For helpful debugging, show a diff-like comparison
            let json_str = serde_json::to_string_pretty(&json_value).unwrap();
            eprintln!("=== JSON MISMATCH ===");
            eprintln!("Fixture: {}", fixture.name);
            eprintln!("Expected: {}", expected_path.display());
            eprintln!("\nActual output:\n{}", json_str);
            eprintln!("====================");

            // Write actual output to a .actual.json file for comparison
            let actual_path = expected_path.with_extension("actual.json");
            fs::write(&actual_path, json_str)
                .unwrap_or_else(|e| eprintln!("Warning: Failed to write actual JSON: {}", e));

            panic!(
                "Fixture '{}' output does not match expected JSON",
                fixture.name
            );
        }
    } else {
        // No expected file exists - generate it for manual review
        let expected_path = fixture.pdf_path.with_extension("expected.json");
        let json_str = serde_json::to_string_pretty(&json_value).unwrap();

        println!("No expected.json found - creating it:");
        println!("  File: {}", expected_path.display());
        fs::write(&expected_path, json_str)
            .unwrap_or_else(|e| eprintln!("Warning: Failed to write expected.json: {}", e));
    }
}

// Test functions for each fixture

#[test]
fn test_all_fixtures_schema_compliance() {
    let fixtures = Fixture::load_all();
    assert!(
        !fixtures.is_empty(),
        "No fixtures found in '{}'",
        FIXTURES_DIR
    );

    for fixture in &fixtures {
        test_fixture(fixture);
    }
}

// Individual test functions for common fixtures (useful for targeted runs)

#[test]
fn test_simple_invoice() {
    let fixture = Fixture {
        name: "simple_invoice".to_string(),
        pdf_path: PathBuf::from(format!("{}/simple_invoice.pdf", FIXTURES_DIR)),
        expected_path: Some(PathBuf::from(format!(
            "{}/simple_invoice.expected.json",
            FIXTURES_DIR
        ))),
    };
    if fixture.pdf_path.exists() {
        test_fixture(&fixture);
    }
}

#[test]
fn test_sample() {
    let fixture = Fixture {
        name: "sample".to_string(),
        pdf_path: PathBuf::from(format!("{}/sample.pdf", FIXTURES_DIR)),
        expected_path: Some(PathBuf::from(format!(
            "{}/sample.expected.json",
            FIXTURES_DIR
        ))),
    };
    if fixture.pdf_path.exists() {
        test_fixture(&fixture);
    }
}

#[test]
fn test_encrypted_rc4() {
    let fixture = Fixture {
        name: "EC-04-rc4-encrypted".to_string(),
        pdf_path: PathBuf::from(format!("{}/EC-04-rc4-encrypted.pdf", FIXTURES_DIR)),
        expected_path: Some(PathBuf::from(format!(
            "{}/EC-04-rc4-encrypted.expected.json",
            FIXTURES_DIR
        ))),
    };
    if fixture.pdf_path.exists() {
        test_fixture(&fixture);
    }
}

#[test]
fn test_encrypted_aes128() {
    let fixture = Fixture {
        name: "EC-05-aes128-encrypted".to_string(),
        pdf_path: PathBuf::from(format!("{}/EC-05-aes128-encrypted.pdf", FIXTURES_DIR)),
        expected_path: Some(PathBuf::from(format!(
            "{}/EC-05-aes128-encrypted.expected.json",
            FIXTURES_DIR
        ))),
    };
    if fixture.pdf_path.exists() {
        test_fixture(&fixture);
    }
}

#[test]
fn test_valid_minimal() {
    let fixture = Fixture {
        name: "valid-minimal".to_string(),
        pdf_path: PathBuf::from(format!("{}/valid-minimal.pdf", FIXTURES_DIR)),
        expected_path: Some(PathBuf::from(format!(
            "{}/valid-minimal.expected.json",
            FIXTURES_DIR
        ))),
    };
    if fixture.pdf_path.exists() {
        test_fixture(&fixture);
    }
}
