//! Form profile regression tests
//!
//! This module tests the form document profile against fixtures
//! at `tests/fixtures/profiles/form/`.
//!
//! The form profile is DEGENERATE - it has NO field extractors.
//! Per plan line 3045: "form has no field extractor; the form_fields
//! output from Phase 7.4 is surfaced separately in extraction output".
//!
//! Acceptance criteria (from bead pdftract-596dz):
//! - profiles/builtin/form.yaml validates
//! - 5+ fixtures with expected outputs
//! - metadata.profile_fields is empty (degenerate profile)
//! - output.form_fields is populated (when Phase 7.4 is integrated)

use std::fs;
use std::path::{Path, PathBuf};

/// Get the workspace root directory
fn workspace_root() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = PathBuf::from(manifest_dir);
    // We're in crates/pdftract-cli, so go up two levels to reach workspace root
    path.parent().unwrap().parent().unwrap().to_path_buf()
}

/// Path to form profile fixtures
fn fixture_dir() -> PathBuf {
    workspace_root().join("tests/fixtures/profiles/form")
}

/// Path to form profile YAML
fn profile_path() -> PathBuf {
    workspace_root().join("profiles/builtin/form/profile.yaml")
}

/// Form fixture names
const FORM_FIXTURES: &[&str] = &["irs_1040", "w2", "i9", "expense_report", "intake_form"];

/// Expected output file suffix
const EXPECTED_SUFFIX: &str = "-expected.json";

/// Verify the form profile YAML exists and is valid
#[test]
fn test_form_profile_exists() {
    let profile_path = profile_path();
    assert!(
        profile_path.exists(),
        "Form profile not found at {}",
        profile_path.display()
    );

    let content = fs::read_to_string(profile_path).expect("Failed to read form profile");

    // Verify profile is not empty
    assert!(!content.trim().is_empty(), "Form profile is empty");

    // Verify required top-level keys exist
    assert!(content.contains("name:"), "Profile missing 'name' key");
    assert!(
        content.contains("description:"),
        "Profile missing 'description' key"
    );
    assert!(
        content.contains("priority:"),
        "Profile missing 'priority' key"
    );
    assert!(
        content.contains("threshold:"),
        "Profile missing 'threshold' key"
    );
    assert!(
        content.contains("predicates:"),
        "Profile missing 'predicates' key"
    );

    // Verify form profile has type: form
    assert!(content.contains("type:"), "Profile missing 'type' key");
    assert!(content.contains("form"), "Profile type should be 'form'");
}

/// Verify all fixture directories exist with expected outputs
#[test]
fn test_form_fixture_structure() {
    let fixture_dir = fixture_dir();
    assert!(
        fixture_dir.exists(),
        "Form fixture directory not found at {}",
        fixture_dir.display()
    );

    // Verify README.md exists
    let readme_path = fixture_dir.join("README.md");
    assert!(readme_path.exists(), "Missing README.md in form fixtures");

    // Verify PROVENANCE.md exists
    let provenance_path = fixture_dir.join("PROVENANCE.md");
    assert!(
        provenance_path.exists(),
        "Missing PROVENANCE.md in form fixtures"
    );

    // Verify all expected output files exist
    for fixture_name in FORM_FIXTURES {
        let expected_path = fixture_dir.join(format!("{}{}", fixture_name, EXPECTED_SUFFIX));
        assert!(
            expected_path.exists(),
            "Missing expected output for fixture '{}': {}",
            fixture_name,
            expected_path.display()
        );

        // Verify expected output is valid JSON
        let content = fs::read_to_string(&expected_path).expect("Failed to read expected output");

        let _: serde_json::Value = serde_json::from_str(&content).expect(&format!(
            "Expected output is not valid JSON: {}",
            expected_path.display()
        ));

        // Verify expected output has required structure
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Check metadata.document_type is "form"
        let document_type = json.pointer("/metadata/document_type").expect(&format!(
            "Missing /metadata/document_type in {}",
            expected_path.display()
        ));
        assert_eq!(
            document_type.as_str(),
            Some("form"),
            "Document type should be 'form' in {}",
            expected_path.display()
        );

        // Check metadata.profile_name is "form"
        let profile_name = json.pointer("/metadata/profile_name").expect(&format!(
            "Missing /metadata/profile_name in {}",
            expected_path.display()
        ));
        assert_eq!(
            profile_name.as_str(),
            Some("form"),
            "Profile name should be 'form' in {}",
            expected_path.display()
        );

        // CRITICAL: Check metadata.profile_fields is empty (degenerate profile)
        let profile_fields = json.pointer("/metadata/profile_fields").expect(&format!(
            "Missing /metadata/profile_fields in {}",
            expected_path.display()
        ));

        let obj = profile_fields
            .as_object()
            .expect("profile_fields is not an object");

        assert!(
            obj.is_empty(),
            "Form profile should have empty profile_fields (degenerate profile) in {}",
            expected_path.display()
        );

        // Verify document_type_confidence is present and valid
        let confidence = json
            .pointer("/metadata/document_type_confidence")
            .expect(&format!(
                "Missing /metadata/document_type_confidence in {}",
                expected_path.display()
            ));

        assert!(
            confidence.as_f64().is_some(),
            "document_type_confidence should be a number in {}",
            expected_path.display()
        );

        let conf_value = confidence.as_f64().unwrap();
        assert!(
            conf_value >= 0.0 && conf_value <= 1.0,
            "document_type_confidence should be between 0 and 1 in {}",
            expected_path.display()
        );
    }
}

/// Verify form profile schema matches Phase 7.10 specification
#[test]
fn test_form_profile_schema() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read form profile");

    // Parse YAML as JSON to verify structure
    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("Form profile is not valid YAML");

    // Verify top-level structure
    assert_eq!(
        yaml_value["name"].as_str(),
        Some("Form Document"),
        "Profile name should be 'Form Document'"
    );

    assert!(
        yaml_value["description"].is_string(),
        "Profile should have a description"
    );

    assert!(
        yaml_value["threshold"].is_number(),
        "Profile should have a numeric threshold"
    );

    // Verify type is "form"
    assert_eq!(
        yaml_value["type"].as_str(),
        Some("form"),
        "Profile type should be 'form'"
    );

    // Verify predicates exist
    assert!(
        yaml_value["predicates"].is_sequence(),
        "Profile should have predicates array"
    );

    let predicates = yaml_value["predicates"].as_sequence().unwrap();
    assert!(
        !predicates.is_empty(),
        "Profile should have at least one predicate"
    );

    // Verify form-specific predicates
    // - structural_has_form_field (weight 0.4)
    // - text_contains "form" (weight 0.2)
    // - page_count_in_range 1-10 (weight 0.15)
    // - text_contains "application" (weight 0.15)
    // - text_contains "please complete" (weight 0.1)

    let predicate_kinds: Vec<String> = predicates
        .iter()
        .filter_map(|p| {
            p.get("kind")
                .and_then(|k| k.as_str().map(|s| s.to_string()))
        })
        .collect();

    assert!(
        predicate_kinds.contains(&"structural_has_form_field".to_string()),
        "Form profile should have structural_has_form_field predicate"
    );

    assert!(
        predicate_kinds.contains(&"text_contains".to_string()),
        "Form profile should have text_contains predicate"
    );

    assert!(
        predicate_kinds.contains(&"page_count_in_range".to_string()),
        "Form profile should have page_count_in_range predicate"
    );
}

/// Verify form profile degenerate behavior (no field extractors)
#[test]
fn test_form_profile_is_degenerate() {
    // This test verifies that the form profile has no field extractors,
    // which is the expected degenerate behavior per plan line 3045.

    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read form profile");

    // The classification profile (profile.yaml) doesn't have fields,
    // but the extraction profile (classification/form.yaml) should have
    // profile_fields: {} (empty object)

    let extraction_profile_path =
        workspace_root().join("profiles/builtin/classification/form.yaml");

    assert!(
        extraction_profile_path.exists(),
        "Extraction profile not found at {}",
        extraction_profile_path.display()
    );

    let extraction_content =
        fs::read_to_string(extraction_profile_path).expect("Failed to read extraction profile");

    // Parse YAML to verify profile_fields is empty
    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&extraction_content).expect("Extraction profile is not valid YAML");

    let profile_fields = &yaml_value["profile_fields"];

    // serde_yaml::Value uses is_mapping() for objects
    assert!(
        profile_fields.is_mapping(),
        "profile_fields should be a mapping/object"
    );

    // Check if the mapping is empty
    let is_empty = if let Some(mapping) = profile_fields.as_mapping() {
        mapping.is_empty()
    } else {
        false
    };

    assert!(
        is_empty,
        "Form profile should have empty profile_fields (degenerate profile)"
    );

    // Verify form_fields_integration: true is present
    assert!(
        extraction_content.contains("form_fields_integration: true"),
        "Form profile should have form_fields_integration: true"
    );

    // Verify reading_order: line_dominant
    assert!(
        extraction_content.contains("reading_order: line_dominant"),
        "Form profile should have reading_order: line_dominant"
    );
}

/// Verify README.md mentions degenerate profile behavior
#[test]
fn test_form_readme_mentions_degenerate() {
    let readme_path = fixture_dir().join("README.md");
    let content = fs::read_to_string(&readme_path).expect("Failed to read README.md");

    // Verify README explains that form is a degenerate profile
    assert!(
        content.contains("degenerate"),
        "README should mention that the form profile is degenerate"
    );

    assert!(
        content.contains("profile_fields: {{}}"),
        "README should show empty profile_fields"
    );

    assert!(
        content.contains("NO field extractors"),
        "README should explain that there are no field extractors"
    );
}
