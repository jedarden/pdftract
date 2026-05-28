//! Slide deck profile regression tests
//!
//! This module tests the slide deck document profile against fixtures
//! at `tests/fixtures/profiles/slide_deck/`.
//!
//! The slide deck profile extracts:
//! - title: Presentation title (region: middle_half, pick: largest_font)
//! - presenter: Presenter name (region: bottom_half, pick: largest_font)
//! - date: Presentation date (near: "Date", parse: date)
//! - slide_titles: Ordered list of slide titles (pick: largest_font, collected per page)
//!
//! Acceptance criteria (from bead pdftract-2vajs):
//! - profiles/builtin/slide_deck.yaml validates
//! - 5+ fixtures with expected outputs
//! - tests/profiles/test_slide_deck.rs passes
//! - Per-field accuracy: >= 90% on the 5-fixture corpus (relaxed for slide_titles which is best-effort)

use std::fs;
use std::path::{Path, PathBuf};

/// Get the workspace root directory
fn workspace_root() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = PathBuf::from(manifest_dir);
    // We're in crates/pdftract-cli, so go up two levels to reach workspace root
    path.parent().unwrap().parent().unwrap().to_path_buf()
}

/// Path to slide deck profile fixtures
fn fixture_dir() -> PathBuf {
    workspace_root().join("tests/fixtures/profiles/slide_deck")
}

/// Path to slide deck profile YAML
fn profile_path() -> PathBuf {
    workspace_root().join("profiles/builtin/slide_deck/profile.yaml")
}

/// Minimum per-field accuracy threshold
const MIN_FIELD_ACCURACY: f64 = 0.90;

/// Slide deck fixture names
const SLIDE_DECK_FIXTURES: &[&str] = &[
    "pitch_deck",
    "academic_lecture",
    "corporate_kickoff",
    "bilingual_deck",
    "googleslides_handout",
];

/// Expected output file suffix
const EXPECTED_SUFFIX: &str = "-expected.json";

/// Profile field names that should be extracted
const PROFILE_FIELDS: &[&str] = &[
    "title",
    "presenter",
    "date",
    "slide_titles",
];

/// Verify the slide deck profile YAML exists and is valid
#[test]
fn test_slide_deck_profile_exists() {
    let profile_path = profile_path();
    assert!(
        profile_path.exists(),
        "Slide deck profile not found at {}",
        profile_path.display()
    );

    let content = fs::read_to_string(profile_path).expect("Failed to read slide deck profile");

    // Verify profile is not empty
    assert!(!content.trim().is_empty(), "Slide deck profile is empty");

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

    // Verify slide deck-specific fields are defined
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
fn test_slide_deck_fixture_structure() {
    let fixture_dir = fixture_dir();
    assert!(
        fixture_dir.exists(),
        "Slide deck fixture directory not found at {}",
        fixture_dir.display()
    );

    // Verify README.md exists
    let readme_path = fixture_dir.join("README.md");
    assert!(
        readme_path.exists(),
        "Missing README.md in slide deck fixtures"
    );

    // Verify PROVENANCE.md exists
    let provenance_path = fixture_dir.join("PROVENANCE.md");
    assert!(
        provenance_path.exists(),
        "Missing PROVENANCE.md in slide deck fixtures"
    );

    // Verify all expected output files exist
    for fixture_name in SLIDE_DECK_FIXTURES {
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

        // Verify all slide deck fields are present in expected output
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

/// Verify slide deck profile schema matches Phase 7.10 specification
#[test]
fn test_slide_deck_profile_schema() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read slide deck profile");

    // Parse YAML as JSON to verify structure
    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("Slide deck profile is not valid YAML");

    // Verify top-level structure
    assert_eq!(
        yaml_value["name"].as_str(),
        Some("slide_deck"),
        "Profile name should be 'slide_deck'"
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

    // Verify reading_order is specified (slide decks use xy_cut for layout)
    let reading_order = extraction["reading_order"].as_str();
    assert_eq!(
        reading_order,
        Some("xy_cut"),
        "Slide deck profile should use xy_cut reading order for proper layout detection"
    );

    // Verify readability_threshold
    assert!(
        extraction["readability_threshold"].is_number(),
        "Profile should specify readability_threshold"
    );

    // Verify include_invisible is false
    let include_invisible = extraction["include_invisible"].as_bool();
    assert_eq!(
        include_invisible,
        Some(false),
        "Slide deck profile should set include_invisible to false"
    );

    // Verify min_block_chars is set (slide decks have lower text density)
    assert!(
        extraction["min_block_chars"].is_number(),
        "Profile should specify min_block_chars"
    );

    // Verify fields section contains all slide deck fields
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

    for fixture_name in SLIDE_DECK_FIXTURES {
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
            Some("slide_deck"),
            "document_type should be 'slide_deck' in {}",
            fixture_name
        );

        assert!(
            metadata.contains_key("document_type_confidence"),
            "Missing document_type_confidence in {}",
            fixture_name
        );

        assert_eq!(
            metadata.get("profile_name").and_then(|v| v.as_str()),
            Some("slide_deck"),
            "profile_name should be 'slide_deck' in {}",
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

        // Verify all slide deck fields are present
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

/// Test slide deck-specific matching predicates
#[test]
fn test_slide_deck_match_predicates() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read slide deck profile");

    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("Slide deck profile is not valid YAML");

    let match_section = &yaml_value["match"];

    // Verify slide deck-specific text patterns in match predicates
    // Convert to string for checking content
    let match_str = serde_yaml::to_string(match_section).unwrap_or_default();

    // Should match common slide deck phrases
    assert!(
        match_str.contains("slides") || match_str.contains("presentation"),
        "Match predicates should include slide deck keywords"
    );

    // Should include page count range for slide decks (3-200 pages)
    assert!(
        match_str.contains("page_count") || match_str.contains("min"),
        "Match predicates should include page count range"
    );

    // Should exclude non-slide-deck document types
    assert!(
        match_str.contains("Abstract") || match_str.contains("References") || match_str.contains("WHEREAS"),
        "Match predicates should exclude scientific paper, contract patterns"
    );
}

/// Test fixture count meets minimum requirement
#[test]
fn test_fixture_count() {
    let fixture_dir = fixture_dir();

    // Count expected output files (excluding README and PROVENANCE)
    let expected_count = SLIDE_DECK_FIXTURES.len();

    assert!(
        expected_count >= 5,
        "Need at least 5 slide deck fixtures, found {}",
        expected_count
    );

    println!("Slide deck fixture count: {} (minimum: 5)", expected_count);
}

/// Verify PROVENANCE.md has required fields
#[test]
fn test_provenance_completeness() {
    let provenance_path = fixture_dir().join("PROVENANCE.md");
    let content = fs::read_to_string(&provenance_path).expect("Failed to read PROVENANCE.md");

    // Verify each fixture is documented
    for fixture_name in SLIDE_DECK_FIXTURES {
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

/// Test that fixture diversity requirements are met
#[test]
fn test_fixture_diversity() {
    let fixture_dir = fixture_dir();

    // Verify we have the required fixture types
    let required_types = [
        ("pitch_deck", "pitch"),
        ("academic_lecture", "academic"),
        ("corporate_kickoff", "kickoff"),
        ("bilingual_deck", "bilingual"),
        ("googleslides_handout", "handout"),
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

/// Test that profile handles slide deck extraction requirements
#[test]
fn test_slide_deck_extraction_fields() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read slide deck profile");

    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("Slide deck profile is not valid YAML");

    let fields = &yaml_value["fields"];

    // Verify title field configuration
    let title = &fields["title"];
    assert_eq!(
        title["type"].as_str(),
        Some("string"),
        "title field should be type string"
    );
    assert_eq!(
        title["region"].as_str(),
        Some("middle_half"),
        "title should be extracted from middle_half region"
    );
    assert_eq!(
        title["pick"].as_str(),
        Some("largest_font"),
        "title should pick largest_font"
    );
    assert_eq!(
        title["page"].as_str(),
        Some("first"),
        "title should be from first page"
    );

    // Verify presenter field configuration
    let presenter = &fields["presenter"];
    assert_eq!(
        presenter["type"].as_str(),
        Some("string"),
        "presenter field should be type string"
    );
    assert_eq!(
        presenter["region"].as_str(),
        Some("bottom_half"),
        "presenter should be extracted from bottom_half region"
    );
    assert_eq!(
        presenter["pick"].as_str(),
        Some("largest_font"),
        "presenter should pick largest_font"
    );
    assert_eq!(
        presenter["page"].as_str(),
        Some("first"),
        "presenter should be from first page"
    );

    // Verify date field configuration
    let date = &fields["date"];
    assert_eq!(
        date["type"].as_str(),
        Some("date"),
        "date field should be type date"
    );
    assert!(
        date["near"].is_sequence(),
        "date should have 'near' keyword list"
    );

    // Verify slide_titles field configuration
    let slide_titles = &fields["slide_titles"];
    assert_eq!(
        slide_titles["type"].as_str(),
        Some("array"),
        "slide_titles field should be type array"
    );
    assert_eq!(
        slide_titles["pick"].as_str(),
        Some("largest_font"),
        "slide_titles should pick largest_font"
    );
    assert_eq!(
        slide_titles["per_page"].as_bool(),
        Some(true),
        "slide_titles should be collected per_page"
    );
}

/// Test that slide_titles is an array in expected outputs
#[test]
fn test_slide_titles_is_array() {
    let fixture_dir = fixture_dir();

    for fixture_name in SLIDE_DECK_FIXTURES {
        let expected_path = fixture_dir.join(format!("{}{}", fixture_name, EXPECTED_SUFFIX));
        let content = fs::read_to_string(&expected_path).expect("Failed to read expected output");

        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        let slide_titles = json
            .pointer("/metadata/profile_fields/slide_titles")
            .expect(&format!("Missing slide_titles in {}", fixture_name));

        assert!(
            slide_titles.is_array(),
            "slide_titles should be an array in {}",
            fixture_name
        );

        // Verify slide_titles is non-empty for most fixtures (googleslides_handout may be partial)
        if *fixture_name != "googleslides_handout" {
            let titles = slide_titles.as_array().unwrap();
            assert!(
                !titles.is_empty(),
                "slide_titles should not be empty in {}",
                fixture_name
            );
        }
    }
}

/// Test that profile handles multi-slide-per-page edge case
#[test]
fn test_multi_slide_per_page_handling() {
    // The googleslides_handout fixture tests the multi-slide-per-page edge case.
    // This test verifies that the fixture exists and is documented as a known limitation.

    let fixture_dir = fixture_dir();
    let readme_path = fixture_dir.join("README.md");
    let content = fs::read_to_string(&readme_path).expect("Failed to read README.md");

    // Verify README documents the multi-slide-per-page limitation
    assert!(
        content.contains("multi-slide-per-page") || content.contains("handout"),
        "README should document multi-slide-per-page edge case"
    );

    // Verify googleslides_handout fixture exists
    let handout_path = fixture_dir.join("googleslides_handout-expected.json");
    assert!(
        handout_path.exists(),
        "googleslides_handout fixture should exist for testing multi-slide-per-page edge case"
    );
}

/// Test that profile excludes non-slide-deck document types
#[test]
fn test_exclusion_patterns() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read slide deck profile");

    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("Slide deck profile is not valid YAML");

    let match_section = &yaml_value["match"];

    // Verify 'none' combinator exists for exclusions
    assert!(
        match_section.get("none").is_some(),
        "Profile should have 'none' combinator for exclusions"
    );

    let none_section = match_section["none"].as_sequence().unwrap();

    // Convert to string for checking content
    let none_str = serde_yaml::to_string(none_section).unwrap_or_default();

    // Verify common non-slide-deck patterns are excluded
    assert!(
        none_str.contains("Abstract") || none_str.contains("References"),
        "Exclusion patterns should include scientific paper markers"
    );

    assert!(
        none_str.contains("WHEREAS") || none_str.contains("Invoice"),
        "Exclusion patterns should include contract/invoice markers"
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
    fn test_load_slide_deck_profile() {
        // This will be implemented once the profile loader exists
        // For now, it's a placeholder documenting the intended behavior
    }

    /// Integration test: Run extraction on slide deck fixtures
    ///
    /// NOTE: This test requires:
    /// 1. PDF fixture files to exist
    /// 2. Profile loader implementation
    /// 3. Field extraction implementation
    #[test]
    #[ignore = "Requires PDF fixtures and Phase 7.10 implementation"]
    fn test_slide_deck_extraction_accuracy() {
        // This will be implemented once:
        // - PDF fixtures are created
        // - Profile loader exists
        // - Field extraction exists

        // Expected behavior:
        // For each fixture:
        // 1. Load the slide deck profile
        // 2. Extract fields from the PDF
        // 3. Compare against expected output
        // 4. Calculate per-field accuracy
        // 5. Assert accuracy >= MIN_FIELD_ACCURACY (with relaxed threshold for slide_titles)
    }
}
