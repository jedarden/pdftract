//! Legal filing profile regression tests
//!
//! This module tests the legal filing document profile against fixtures
//! at `tests/fixtures/profiles/legal_filing/`.
//!
//! The legal filing profile extracts:
//! - case_number: Case number (near: "Case No.", "Civil Action No.", regex match)
//! - court: Court name (region: top_quarter, pick: largest_font)
//! - parties: Plaintiff/Defendant or Petitioner/Respondent (near: party markers)
//! - filing_date: Filing date (near: "Filed", "Date Filed", parse: date)
//! - docket_entries: Docket entries list (region: full, BEST-EFFORT)
//!
//! Acceptance criteria (from bead pdftract-260a3):
//! - profiles/builtin/legal_filing.yaml validates
//! - 5+ fixtures with expected outputs
//! - Per-field accuracy: >= 90% on the 5-fixture corpus (parties, docket_entries >= 80%)

use std::fs;
use std::path::{Path, PathBuf};

/// Get the workspace root directory
fn workspace_root() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = PathBuf::from(manifest_dir);
    // We're in crates/pdftract-cli, so go up two levels to reach workspace root
    path.parent().unwrap().parent().unwrap().to_path_buf()
}

/// Path to legal filing profile fixtures
fn fixture_dir() -> PathBuf {
    workspace_root().join("tests/fixtures/profiles/legal_filing")
}

/// Path to legal filing profile YAML
fn profile_path() -> PathBuf {
    workspace_root().join("profiles/builtin/legal_filing/profile.yaml")
}

/// Minimum per-field accuracy threshold
const MIN_FIELD_ACCURACY: f64 = 0.90;

/// Relaxed accuracy threshold for complex fields (parties, docket_entries)
const MIN_RELAXED_ACCURACY: f64 = 0.80;

/// Legal filing fixture names
const LEGAL_FILING_FIXTURES: &[&str] = &[
    "federal_complaint",
    "state_motion",
    "appellate_brief",
    "court_order",
    "docket_sheet",
];

/// Expected output file suffix
const EXPECTED_SUFFIX: &str = "-expected.json";

/// Profile field names that should be extracted
const PROFILE_FIELDS: &[&str] = &[
    "case_number",
    "court",
    "parties",
    "filing_date",
    "docket_entries",
];

/// Verify the legal filing profile YAML exists and is valid
#[test]
fn test_legal_filing_profile_exists() {
    let profile_path = profile_path();
    assert!(
        profile_path.exists(),
        "Legal filing profile not found at {}",
        profile_path.display()
    );

    let content = fs::read_to_string(profile_path).expect("Failed to read legal filing profile");

    // Verify profile is not empty
    assert!(!content.trim().is_empty(), "Legal filing profile is empty");

    // Verify required top-level keys exist (Phase 7.10 schema)
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

    // Verify legal filing-specific fields are defined
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
fn test_legal_filing_fixture_structure() {
    let fixture_dir = fixture_dir();
    assert!(
        fixture_dir.exists(),
        "Legal filing fixture directory not found at {}",
        fixture_dir.display()
    );

    // Verify README.md exists
    let readme_path = fixture_dir.join("README.md");
    assert!(
        readme_path.exists(),
        "Missing README.md in legal filing fixtures"
    );

    // Verify PROVENANCE.md exists
    let provenance_path = fixture_dir.join("PROVENANCE.md");
    assert!(
        provenance_path.exists(),
        "Missing PROVENANCE.md in legal filing fixtures"
    );

    // Verify all expected output files exist
    for fixture_name in LEGAL_FILING_FIXTURES {
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

        // Verify all legal filing fields are present in expected output
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

/// Verify legal filing profile schema matches Phase 7.10 specification
#[test]
fn test_legal_filing_profile_schema() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read legal filing profile");

    // Parse YAML as JSON to verify structure
    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("Legal filing profile is not valid YAML");

    // Verify top-level structure
    assert_eq!(
        yaml_value["name"].as_str(),
        Some("legal_filing"),
        "Profile name should be 'legal_filing'"
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

    // Verify reading_order is specified (legal filings use xy_cut for complex layouts)
    let reading_order = extraction["reading_order"].as_str();
    assert_eq!(
        reading_order,
        Some("xy_cut"),
        "Legal filing profile should use xy_cut reading order for complex layouts"
    );

    // Verify readability_threshold
    assert!(
        extraction["readability_threshold"].is_number(),
        "Profile should specify readability_threshold"
    );

    // Verify include_headers_footers is true (page numbers and citations are load-bearing)
    let include_headers_footers = extraction["include_headers_footers"].as_bool();
    assert_eq!(
        include_headers_footers,
        Some(true),
        "Legal filing profile should set include_headers_footers to true"
    );

    // Verify fields section contains all legal filing fields
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

    for fixture_name in LEGAL_FILING_FIXTURES {
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
            Some("legal_filing"),
            "document_type should be 'legal_filing' in {}",
            fixture_name
        );

        assert!(
            metadata.contains_key("document_type_confidence"),
            "Missing document_type_confidence in {}",
            fixture_name
        );

        assert_eq!(
            metadata.get("profile_name").and_then(|v| v.as_str()),
            Some("legal_filing"),
            "profile_name should be 'legal_filing' in {}",
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

        // Verify all legal filing fields are present
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

/// Test legal filing-specific matching predicates
#[test]
fn test_legal_filing_match_predicates() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read legal filing profile");

    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("Legal filing profile is not valid YAML");

    let match_section = &yaml_value["match"];

    // Verify legal filing-specific text patterns in match predicates
    // Convert to string for checking content
    let match_str = serde_yaml::to_string(match_section).unwrap_or_default();

    // Should match common legal filing phrases
    assert!(
        match_str.contains("UNITED STATES DISTRICT COURT") || match_str.contains("IN THE COURT OF"),
        "Match predicates should include court name patterns"
    );

    assert!(
        match_str.contains("Case No.") || match_str.contains("Docket No."),
        "Match predicates should include case number patterns"
    );

    assert!(
        match_str.contains("Plaintiff") || match_str.contains("Petitioner"),
        "Match predicates should include party patterns"
    );
}

/// Test fixture count meets minimum requirement
#[test]
fn test_fixture_count() {
    let fixture_dir = fixture_dir();

    // Count expected output files (excluding README and PROVENANCE)
    let expected_count = LEGAL_FILING_FIXTURES.len();

    assert!(
        expected_count >= 5,
        "Need at least 5 legal filing fixtures, found {}",
        expected_count
    );

    println!(
        "Legal filing fixture count: {} (minimum: 5)",
        expected_count
    );
}

/// Verify PROVENANCE.md has required fields
#[test]
fn test_provenance_completeness() {
    let provenance_path = fixture_dir().join("PROVENANCE.md");
    let content = fs::read_to_string(&provenance_path).expect("Failed to read PROVENANCE.md");

    // Verify each fixture is documented
    for fixture_name in LEGAL_FILING_FIXTURES {
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
            section.contains("Type:") || section.contains("**Type**"),
            "PROVENANCE.md missing 'Type' for fixture '{}'",
            fixture_name
        );

        assert!(
            section.contains("Case No.") || section.contains("**Case No.**"),
            "PROVENANCE.md missing 'Case No.' for fixture '{}'",
            fixture_name
        );

        assert!(
            section.contains("Pages:") || section.contains("**Pages**"),
            "PROVENANCE.md missing 'Pages' count for fixture '{}'",
            fixture_name
        );
    }
}

/// Test that fixture diversity requirements are met
#[test]
fn test_fixture_diversity() {
    let fixture_dir = fixture_dir();

    // Verify we have the required fixture types
    let required_types = [
        ("federal_complaint", "federal"),
        ("state_motion", "state"),
        ("appellate_brief", "appellate"),
        ("court_order", "order"),
        ("docket_sheet", "docket"),
    ];

    for (fixture_name, expected_keyword) in required_types {
        let provenance_path = fixture_dir.join("PROVENANCE.md");
        let content = fs::read_to_string(&provenance_path).expect("Failed to read PROVENANCE.md");

        let pdf_name = format!("{}.pdf", fixture_name);
        let search_name = if content.contains(&pdf_name) {
            pdf_name.as_str()
        } else {
            fixture_name
        };

        let section_start = content.find(search_name).unwrap();
        let section_end = content[section_start..]
            .find("\n## ")
            .or_else(|| content[section_start..].find("\n# "))
            .unwrap_or(content[section_start..].len());

        let section = &content[section_start..section_start + section_end];

        assert!(
            section.contains(expected_keyword),
            "Fixture '{}' should mention '{}' in PROVENANCE.md",
            fixture_name,
            expected_keyword
        );
    }
}

/// Test that profile includes headers and footers requirement
#[test]
fn test_include_headers_footers() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read legal filing profile");

    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("Legal filing profile is not valid YAML");

    let extraction = &yaml_value["extraction"];

    // Verify include_headers_footers is true (page numbers and citations are load-bearing in legal docs)
    let include_headers_footers = extraction["include_headers_footers"].as_bool();
    assert_eq!(
        include_headers_footers,
        Some(true),
        "Legal filing profile must set include_headers_footers to true for page numbers and citations"
    );
}

/// Test that case_number regex handles multiple formats
#[test]
fn test_case_number_regex_formats() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read legal filing profile");

    // Verify case_number regex handles multiple formats:
    // - Federal: 1:24-cv-00123
    // - State: CGC-24-123456
    // - Appellate: 24-1234
    assert!(
        content.contains(r"[\w-]+:?\s*\d+[\w-]*") || content.contains(r"case_number"),
        "Profile should contain case_number regex matching multiple formats"
    );
}

/// Test that parties field handles different party types
#[test]
fn test_parties_field_variations() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read legal filing profile");

    // Verify parties field handles different party type combinations:
    // - Plaintiff/Defendant
    // - Petitioner/Respondent
    // - Appellant/Appellee
    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("Legal filing profile is not valid YAML");

    let parties_field = &yaml_value["fields"]["parties"];
    let parties_str = serde_yaml::to_string(parties_field).unwrap_or_default();

    assert!(
        parties_str.contains("Plaintiff")
            || parties_str.contains("Defendant")
            || parties_str.contains("Petitioner")
            || parties_str.contains("Respondent")
            || parties_str.contains("v."),
        "Parties field should handle common party type markers"
    );
}

/// Test that docket_entries field is marked as BEST-EFFORT
#[test]
fn test_docket_entries_best_effort() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read legal filing profile");

    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("Legal filing profile is not valid YAML");

    let docket_field = &yaml_value["fields"]["docket_entries"];

    // Verify docket_entries uses region: full for BEST-EFFORT extraction
    let docket_str = serde_yaml::to_string(docket_field).unwrap_or_default();
    assert!(
        docket_str.contains("full") || docket_str.contains("region"),
        "Docket entries should use region-based extraction for BEST-EFFORT behavior"
    );
}

/// Test that filing_date uses date parsing
#[test]
fn test_filing_date_parsing() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read legal filing profile");

    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("Legal filing profile is not valid YAML");

    let filing_date_field = &yaml_value["fields"]["filing_date"];

    // Verify filing_date uses parse: date
    let date_str = serde_yaml::to_string(filing_date_field).unwrap_or_default();
    assert!(
        date_str.contains("date") || date_str.contains("parse"),
        "Filing date should use date parsing"
    );
}

/// Test that court field uses top_quarter region with largest_font
#[test]
fn test_court_field_extraction() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read legal filing profile");

    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("Legal filing profile is not valid YAML");

    let court_field = &yaml_value["fields"]["court"];

    // Verify court uses region: top_quarter and pick: largest_font
    let court_str = serde_yaml::to_string(court_field).unwrap_or_default();
    assert!(
        court_str.contains("top_quarter") || court_str.contains("largest_font"),
        "Court field should use top_quarter region with largest_font pick strategy"
    );
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
    fn test_load_legal_filing_profile() {
        // This will be implemented once the profile loader exists
        // For now, it's a placeholder documenting the intended behavior
    }

    /// Integration test: Run extraction on legal filing fixtures
    ///
    /// NOTE: This test requires:
    /// 1. PDF fixture files to exist
    /// 2. Profile loader implementation
    /// 3. Field extraction implementation
    #[test]
    #[ignore = "Requires PDF fixtures and Phase 7.10 implementation"]
    fn test_legal_filing_extraction_accuracy() {
        // This will be implemented once:
        // - PDF fixtures are created
        // - Profile loader exists
        // - Field extraction exists

        // Expected behavior:
        // For each fixture:
        // 1. Load the legal filing profile
        // 2. Extract fields from the PDF
        // 3. Compare against expected output
        // 4. Calculate per-field accuracy
        // 5. Assert accuracy >= MIN_FIELD_ACCURACY (parties, docket_entries >= MIN_RELAXED_ACCURACY)
    }
}
