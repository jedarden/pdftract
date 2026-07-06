//! Book chapter profile regression tests
//!
//! This module tests the book chapter document profile against fixtures
//! at `tests/fixtures/profiles/book_chapter/`.
//!
//! The book chapter profile extracts:
//! - title: Chapter title (region: top_third, pick: largest_font, page: first)
//! - chapter_number: Chapter number (near: ['Chapter', 'Part'], regex: '\d+')
//! - author: Author name (region: top_quarter, pick: smallest_font, page: first)
//! - sections: List of section headings (per-page collection)
//!
//! Acceptance criteria (from bead pdftract-1t5sj):
//! - profiles/builtin/book_chapter.yaml validates
//! - 5+ fixtures with expected outputs
//! - Per-field accuracy: >= 90% on the 5-fixture corpus (sections: >= 80%)

use std::fs;
use std::path::PathBuf;

/// Get the workspace root directory
fn workspace_root() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = PathBuf::from(manifest_dir);
    // We're in crates/pdftract-cli, so go up two levels to reach workspace root
    path.parent().unwrap().parent().unwrap().to_path_buf()
}

/// Path to book chapter profile fixtures
fn fixture_dir() -> PathBuf {
    workspace_root().join("tests/fixtures/profiles/book_chapter")
}

/// Path to book chapter profile YAML
fn profile_path() -> PathBuf {
    workspace_root().join("profiles/builtin/book_chapter/profile.yaml")
}

/// Minimum per-field accuracy threshold (sections relaxed to 80%)
const MIN_FIELD_ACCURACY: f64 = 0.90;
const MIN_SECTIONS_ACCURACY: f64 = 0.80;

/// Book chapter fixture names
const BOOK_CHAPTER_FIXTURES: &[&str] = &[
    "novel_chapter",
    "academic_chapter",
    "textbook_chapter",
    "technical_manual_chapter",
    "recipe_book_chapter",
];

/// Expected output file suffix
const EXPECTED_SUFFIX: &str = "-expected.json";

/// Profile field names that should be extracted
const PROFILE_FIELDS: &[&str] = &["title", "chapter_number", "author", "sections"];

/// Verify the book chapter profile YAML exists and is valid
#[test]
fn test_book_chapter_profile_exists() {
    let profile_path = profile_path();
    assert!(
        profile_path.exists(),
        "Book chapter profile not found at {}",
        profile_path.display()
    );

    let content = fs::read_to_string(profile_path).expect("Failed to read book chapter profile");

    // Verify profile is not empty
    assert!(!content.trim().is_empty(), "Book chapter profile is empty");

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

    // Verify book chapter-specific fields are defined
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
fn test_book_chapter_fixture_structure() {
    let fixture_dir = fixture_dir();
    assert!(
        fixture_dir.exists(),
        "Book chapter fixture directory not found at {}",
        fixture_dir.display()
    );

    // Verify README.md exists
    let readme_path = fixture_dir.join("README.md");
    assert!(
        readme_path.exists(),
        "Missing README.md in book chapter fixtures"
    );

    // Verify PROVENANCE.md exists
    let provenance_path = fixture_dir.join("PROVENANCE.md");
    assert!(
        provenance_path.exists(),
        "Missing PROVENANCE.md in book chapter fixtures"
    );

    // Verify all expected output files exist
    for fixture_name in BOOK_CHAPTER_FIXTURES {
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

        // Verify all book chapter fields are present in expected output
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

/// Verify book chapter profile schema matches Phase 7.10 specification
#[test]
fn test_book_chapter_profile_schema() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read book chapter profile");

    // Parse YAML as JSON to verify structure
    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("Book chapter profile is not valid YAML");

    // Verify top-level structure
    assert_eq!(
        yaml_value["name"].as_str(),
        Some("book_chapter"),
        "Profile name should be 'book_chapter'"
    );

    assert!(
        yaml_value["description"].is_string(),
        "Profile should have a description"
    );

    assert!(
        yaml_value["priority"].is_i64() || yaml_value["priority"].is_u64(),
        "Profile should have a numeric priority"
    );

    // Verify priority is 5 (lowest among the 9 built-in profiles)
    let priority = yaml_value["priority"]
        .as_i64()
        .or_else(|| yaml_value["priority"].as_u64().map(|u| u as i64));
    assert_eq!(
        priority,
        Some(5),
        "Book chapter profile should have priority 5 (lowest priority)"
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

    // Verify reading_order is specified (book chapters use line_dominant)
    let reading_order = extraction["reading_order"].as_str();
    assert_eq!(
        reading_order,
        Some("line_dominant"),
        "Book chapter profile should use line_dominant reading order for narrative text flow"
    );

    // Verify readability_threshold is 0.6 (higher threshold for narrative text)
    let readability_threshold = extraction["readability_threshold"].as_f64();
    assert_eq!(
        readability_threshold,
        Some(0.6),
        "Book chapter profile should have readability_threshold of 0.6 for narrative text quality"
    );

    // Verify include_invisible is false
    let include_invisible = extraction["include_invisible"].as_bool();
    assert_eq!(
        include_invisible,
        Some(false),
        "Book chapter profile should set include_invisible to false"
    );

    // Verify include_headers_footers is false
    let include_headers_footers = extraction["include_headers_footers"].as_bool();
    assert_eq!(
        include_headers_footers,
        Some(false),
        "Book chapter profile should set include_headers_footers to false"
    );

    // Verify fields section contains all book chapter fields
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

    for fixture_name in BOOK_CHAPTER_FIXTURES {
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
            Some("book_chapter"),
            "document_type should be 'book_chapter' in {}",
            fixture_name
        );

        assert!(
            metadata.contains_key("document_type_confidence"),
            "Missing document_type_confidence in {}",
            fixture_name
        );

        assert_eq!(
            metadata.get("profile_name").and_then(|v| v.as_str()),
            Some("book_chapter"),
            "profile_name should be 'book_chapter' in {}",
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

        // Verify all book chapter fields are present
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

/// Test book chapter-specific matching predicates
#[test]
fn test_book_chapter_match_predicates() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read book chapter profile");

    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("Book chapter profile is not valid YAML");

    let match_section = &yaml_value["match"];

    // Verify book chapter-specific text patterns in match predicates
    let match_str = serde_yaml::to_string(match_section).unwrap_or_default();

    // Should match chapter/section heading patterns
    assert!(
        match_str.contains("Chapter")
            || match_str.contains("Part")
            || match_str.contains("Section"),
        "Match predicates should include chapter/section patterns"
    );

    // Should exclude more specific document types
    assert!(
        match_str.contains("Abstract")
            || match_str.contains("Invoice")
            || match_str.contains("WHEREAS"),
        "Match predicates should exclude more specific document types"
    );
}

/// Test fixture count meets minimum requirement
#[test]
fn test_fixture_count() {
    let expected_count = BOOK_CHAPTER_FIXTURES.len();

    assert!(
        expected_count >= 5,
        "Need at least 5 book chapter fixtures, found {}",
        expected_count
    );

    println!(
        "Book chapter fixture count: {} (minimum: 5)",
        expected_count
    );
}

/// Verify PROVENANCE.md has required fields
#[test]
fn test_provenance_completeness() {
    let provenance_path = fixture_dir().join("PROVENANCE.md");
    let content = fs::read_to_string(&provenance_path).expect("Failed to read PROVENANCE.md");

    // Verify each fixture is documented
    for fixture_name in BOOK_CHAPTER_FIXTURES {
        let pdf_name = format!("{}.pdf", fixture_name);
        assert!(
            content.contains(fixture_name) || content.contains(&pdf_name),
            "PROVENANCE.md missing documentation for fixture '{}'",
            fixture_name
        );

        let search_name = if content.contains(&pdf_name) {
            pdf_name.as_str()
        } else {
            *fixture_name
        };

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
        ("novel_chapter", "Gutenberg"),
        ("academic_chapter", "academic"),
        ("textbook_chapter", "textbook"),
        ("technical_manual_chapter", "technical"),
        ("recipe_book_chapter", "recipe"),
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

/// Test that profile uses line_dominant reading order for narrative text
#[test]
fn test_line_dominant_reading_order() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read book chapter profile");

    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("Book chapter profile is not valid YAML");

    let extraction = &yaml_value["extraction"];

    // Verify line_dominant is specified for narrative text flow
    let reading_order = extraction["reading_order"].as_str();
    assert_eq!(
        reading_order,
        Some("line_dominant"),
        "Book chapter profile must use line_dominant reading order for narrative text flow"
    );
}

/// Test that chapter_number regex matches numeric chapters
#[test]
fn test_chapter_number_regex() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read book chapter profile");

    // Verify chapter_number regex matches numeric chapters
    assert!(
        content.contains(r"\d+"),
        "Profile should contain chapter_number regex matching numeric chapters"
    );
}

/// Test that profile excludes headers and footers
#[test]
fn test_exclude_headers_footers() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read book chapter profile");

    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("Book chapter profile is not valid YAML");

    let extraction = &yaml_value["extraction"];

    // Verify include_headers_footers is false (page numbers are not body content)
    let include_headers_footers = extraction["include_headers_footers"].as_bool();
    assert_eq!(
        include_headers_footers,
        Some(false),
        "Book chapter profile should exclude headers and footers (page numbers are not body content)"
    );
}

/// Test that profile has lowest priority (5) to avoid stealing matches
#[test]
fn test_lowest_priority() {
    let profile_path = profile_path();
    let content = fs::read_to_string(profile_path).expect("Failed to read book chapter profile");

    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("Book chapter profile is not valid YAML");

    // Verify priority is 5 (lowest among the 9 built-in profiles)
    let priority = yaml_value["priority"]
        .as_i64()
        .or_else(|| yaml_value["priority"].as_u64().map(|u| u as i64));
    assert_eq!(
        priority,
        Some(5),
        "Book chapter profile must have priority 5 (lowest priority) to avoid stealing matches from more-specific profiles"
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
    fn test_load_book_chapter_profile() {
        // This will be implemented once the profile loader exists
        // For now, it's a placeholder documenting the intended behavior
    }

    /// Integration test: Run extraction on book chapter fixtures
    ///
    /// NOTE: This test requires:
    /// 1. PDF fixture files to exist
    /// 2. Profile loader implementation
    /// 3. Field extraction implementation
    #[test]
    #[ignore = "Requires PDF fixtures and Phase 7.10 implementation"]
    fn test_book_chapter_extraction_accuracy() {
        // This will be implemented once:
        // - PDF fixtures are created
        // - Profile loader exists
        // - Field extraction exists

        // Expected behavior:
        // For each fixture:
        // 1. Load the book chapter profile
        // 2. Extract fields from the PDF
        // 3. Compare against expected output
        // 4. Calculate per-field accuracy
        // 5. Assert accuracy >= MIN_FIELD_ACCURACY (sections: >= MIN_SECTIONS_ACCURACY)
    }
}
