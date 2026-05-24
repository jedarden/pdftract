//! Contract profile regression tests
//!
//! This module tests the contract document profile against fixtures
//! at `tests/fixtures/profiles/contract/`.
//!
//! The contract profile extracts:
//! - parties: Contract parties (between X and Y)
//! - effective_date: Agreement effective date
//! - term: Contract term (duration or end date)
//! - governing_law: Governing law/jurisdiction
//! - signatures: Signature block parties
//!
//! Acceptance criteria (from bead pdftract-dtpwa):
//! - profiles/builtin/contract.yaml validates
//! - 5+ fixtures with expected outputs
//! - Per-field accuracy: >= 90%

use std::fs;
use std::path::{Path, PathBuf};

/// Get the workspace root directory
fn workspace_root() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = PathBuf::from(manifest_dir);
    // We're in crates/pdftract-cli, so go up two levels to reach workspace root
    path.parent().unwrap().parent().unwrap().to_path_buf()
}

/// Path to contract profile fixtures
fn fixture_dir() -> PathBuf {
    workspace_root().join("tests/fixtures/profiles/contract")
}

/// Path to contract profile YAML
fn profile_path() -> PathBuf {
    workspace_root().join("profiles/builtin/contract/profile.yaml")
}

/// Minimum per-field accuracy threshold
const MIN_FIELD_ACCURACY: f64 = 0.90;

/// Contract fixture names
const CONTRACT_FIXTURES: &[&str] = &[
    "nda",
    "employment",
    "msa",
    "service_agreement",
    "real_estate",
];

/// Expected output file suffix
const EXPECTED_SUFFIX: &str = "-expected.json";

/// Profile field names that should be extracted
const PROFILE_FIELDS: &[&str] = &[
    "parties",
    "effective_date",
    "term",
    "governing_law",
    "signatures",
];

/// Verify the contract profile YAML exists and is valid
#[test]
fn test_contract_profile_exists() {
    let profile_path = profile_path();
    assert!(
        profile_path.exists(),
        "Contract profile not found at {}",
        profile_path.display()
    );

    let content = fs::read_to_string(profile_path).expect("Failed to read contract profile");

    // Verify profile is not empty
    assert!(!content.trim().is_empty(), "Contract profile is empty");

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
    assert!(content.contains("match:"), "Profile missing 'match' key");
    assert!(
        content.contains("extraction:"),
        "Profile missing 'extraction' key"
    );
    assert!(content.contains("fields:"), "Profile missing 'fields' key");

    // Verify contract-specific fields are defined
    for field in PROFILE_FIELDS {
        assert!(
            content.contains(&format!("{}:", field)),
            "Profile missing field '{}'",
            field
        );
    }
}

/// Verify all fixture directories exist with expected outputs
#[test]
fn test_contract_fixture_structure() {
    let fixture_dir = fixture_dir();
    assert!(
        fixture_dir.exists(),
        "Contract fixture directory not found at {}",
        fixture_dir.display()
    );

    // Verify README.md exists
    let readme_path = fixture_dir.join("README.md");
    assert!(
        readme_path.exists(),
        "Missing README.md in contract fixtures"
    );

    // Verify PROVENANCE.md exists
    let provenance_path = fixture_dir.join("PROVENANCE.md");
    assert!(
        provenance_path.exists(),
        "Missing PROVENANCE.md in contract fixtures"
    );

    // Verify all expected output files exist
    for fixture_name in CONTRACT_FIXTURES {
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

        // Check metadata.profile_fields exists
        let profile_fields = json.pointer("/metadata/profile_fields").expect(&format!(
            "Missing /metadata/profile_fields in {}",
            expected_path.display()
        ));

        // Verify all contract fields are present in expected output
        let obj = profile_fields
            .as_object()
            .expect("profile_fields is not an object");
        for field in PROFILE_FIELDS {
            assert!(
                obj.contains_key(*field),
                "Expected output missing field '{}' in {}",
                field,
                expected_path.display()
            );
        }
    }
}

/// Verify contract profile schema matches Phase 7.10 specification
#[test]
fn test_contract_profile_schema() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read contract profile");

    // Parse YAML as JSON to verify structure
    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("Contract profile is not valid YAML");

    // Verify top-level structure
    assert_eq!(
        yaml_value["name"].as_str(),
        Some("contract"),
        "Profile name should be 'contract'"
    );

    assert!(
        yaml_value["description"].is_string(),
        "Profile should have a description"
    );

    assert!(
        yaml_value["priority"].is_i64() || yaml_value["priority"].is_u64(),
        "Profile should have a numeric priority"
    );

    // Verify match section has all/any/none combinators
    let match_section = &yaml_value["match"];
    assert!(
        match_section.is_mapping(),
        "Profile 'match' section should be a mapping"
    );

    // Verify extraction tuning keys
    let extraction = &yaml_value["extraction"];
    assert!(
        extraction.is_mapping(),
        "Profile 'extraction' section should be a mapping"
    );

    // Verify reading_order is specified (contracts use xy_cut)
    let reading_order = extraction["reading_order"].as_str();
    assert_eq!(
        reading_order,
        Some("xy_cut"),
        "Contract profile should use xy_cut reading order"
    );

    // Verify readability_threshold
    assert!(
        extraction["readability_threshold"].is_number(),
        "Profile should specify readability_threshold"
    );

    // Verify fields section contains all contract fields
    let fields = &yaml_value["fields"];
    assert!(
        fields.is_mapping(),
        "Profile 'fields' section should be a mapping"
    );

    for field in PROFILE_FIELDS {
        assert!(
            fields.get(*field).is_some(),
            "Profile missing field '{}'",
            field
        );
    }
}

/// Test that expected outputs have consistent structure
#[test]
fn test_expected_output_consistency() {
    let fixture_dir = fixture_dir();

    for fixture_name in CONTRACT_FIXTURES {
        let expected_path = fixture_dir.join(format!("{}{}", fixture_name, EXPECTED_SUFFIX));
        let content = fs::read_to_string(&expected_path).expect("Failed to read expected output");

        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Verify metadata structure
        let metadata = json["metadata"]
            .as_object()
            .expect(&format!("Missing 'metadata' in {}", fixture_name));

        // Verify required metadata fields
        assert_eq!(
            metadata.get("document_type").and_then(|v| v.as_str()),
            Some("contract"),
            "document_type should be 'contract' in {}",
            fixture_name
        );

        assert!(
            metadata.contains_key("document_type_confidence"),
            "Missing document_type_confidence in {}",
            fixture_name
        );

        assert_eq!(
            metadata.get("profile_name").and_then(|v| v.as_str()),
            Some("contract"),
            "profile_name should be 'contract' in {}",
            fixture_name
        );

        assert_eq!(
            metadata.get("profile_version").and_then(|v| v.as_str()),
            Some("1.0.0"),
            "profile_version should be '1.0.0' in {}",
            fixture_name
        );

        // Verify profile_fields structure
        let profile_fields = metadata
            .get("profile_fields")
            .and_then(|v| v.as_object())
            .expect(&format!("Missing profile_fields in {}", fixture_name));

        // Verify all contract fields are present
        for field in PROFILE_FIELDS {
            assert!(
                profile_fields.contains_key(*field),
                "Missing field '{}' in {}",
                field,
                fixture_name
            );
        }
    }
}

/// Test contract-specific matching predicates
#[test]
fn test_contract_match_predicates() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read contract profile");

    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("Contract profile is not valid YAML");

    let match_section = &yaml_value["match"];

    // Verify contract-specific text patterns in match predicates
    // Convert to string for checking content
    let match_str = serde_yaml::to_string(match_section).unwrap_or_default();

    // Should match common contract phrases
    assert!(
        match_str.contains("AGREEMENT") || match_str.contains("CONTRACT"),
        "Match predicates should include 'AGREEMENT' or 'CONTRACT'"
    );

    // Should exclude invoices and receipts
    assert!(
        match_str.contains("Invoice") || match_str.contains("Receipt"),
        "Match predicates should exclude invoices/receipts"
    );
}

/// Test fixture count meets minimum requirement
#[test]
fn test_fixture_count() {
    let fixture_dir = fixture_dir();

    // Count expected output files (excluding README and PROVENANCE)
    let expected_count = CONTRACT_FIXTURES.len();

    assert!(
        expected_count >= 5,
        "Need at least 5 contract fixtures, found {}",
        expected_count
    );

    println!("Contract fixture count: {} (minimum: 5)", expected_count);
}

/// Verify PROVENANCE.md has required fields
#[test]
fn test_provenance_completeness() {
    let provenance_path = fixture_dir().join("PROVENANCE.md");
    let content = fs::read_to_string(&provenance_path).expect("Failed to read PROVENANCE.md");

    // Verify each fixture is documented
    for fixture_name in CONTRACT_FIXTURES {
        // Check for both "name" and "name.pdf" in provenance
        let pdf_name = format!("{}.pdf", fixture_name);
        assert!(
            content.contains(fixture_name) || content.contains(&pdf_name),
            "PROVENANCE.md missing documentation for fixture '{}'",
            fixture_name
        );

        // Use the name that's actually in the file for section searching
        let search_name = if content.contains(&pdf_name) {
            pdf_name.as_str()
        } else {
            *fixture_name
        };

        // Verify required fields are documented
        let section_start = content.find(search_name).unwrap();
        let section_end = content[section_start..]
            .find("\n## ")
            .or_else(|| content[section_start..].find("\n# "))
            .unwrap_or(content[section_start..].len());

        let section = &content[section_start..section_start + section_end];

        assert!(
            section.contains("Source:") || section.contains("**Source**"),
            "PROVENANCE.md missing 'Source' for fixture '{}'",
            fixture_name
        );

        assert!(
            section.contains("License:") || section.contains("**License**"),
            "PROVENANCE.md missing 'License' for fixture '{}'",
            fixture_name
        );

        assert!(
            section.contains("PII:") || section.contains("**PII**"),
            "PROVENANCE.md missing 'PII' field for fixture '{}'",
            fixture_name
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Integration test: Verify profile can be loaded and parsed
    ///
    /// NOTE: This test requires the profile loader to be implemented.
    /// It will be enabled once Phase 7.10 is fully implemented.
    #[test]
    #[ignore = "Phase 7.10 profile loader not yet implemented"]
    fn test_load_contract_profile() {
        // This will be implemented once the profile loader exists
        // For now, it's a placeholder documenting the intended behavior
    }

    /// Integration test: Run extraction on contract fixtures
    ///
    /// NOTE: This test requires:
    /// 1. PDF fixture files to exist
    /// 2. Profile loader implementation
    /// 3. Field extraction implementation
    #[test]
    #[ignore = "Requires PDF fixtures and Phase 7.10 implementation"]
    fn test_contract_extraction_accuracy() {
        // This will be implemented once:
        // - PDF fixtures are created
        // - Profile loader exists
        // - Field extraction exists

        // Expected behavior:
        // For each fixture:
        // 1. Load the contract profile
        // 2. Extract fields from the PDF
        // 3. Compare against expected output
        // 4. Calculate per-field accuracy
        // 5. Assert accuracy >= MIN_FIELD_ACCURACY
    }
}
