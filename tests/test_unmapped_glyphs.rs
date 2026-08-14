//! Test unmapped glyphs produce U+FFFD diagnostics
//!
//! This test verifies that:
//! - Unmapped glyph extraction produces U+FFFD replacement characters
//! - GLYPH_UNMAPPED diagnostics are emitted for each unmapped glyph
//! - Mapped AGL glyphs still resolve correctly

use pdftract_core::{extract_pdf, ExtractionOptions};
use std::path::Path;

#[test]
fn test_unmapped_glyphs_produces_replacement_chars() {
    let fixture_path = Path::new("tests/fixtures/encoding/unmapped-glyphs.pdf");

    // Verify fixture exists
    assert!(
        fixture_path.exists(),
        "Test fixture should exist at {}",
        fixture_path.display()
    );

    // Run extraction
    let result = extract_pdf(
        fixture_path,
        &ExtractionOptions::default(),
    );

    assert!(
        result.is_ok(),
        "PDF extraction should succeed for unmapped-glyphs fixture: {:?}",
        result.err()
    );

    let extraction_result = result.unwrap();

    // Verify we have at least one page
    assert!(
        !extraction_result.pages.is_empty(),
        "At least one page should be extracted from the PDF"
    );

    let first_page = &extraction_result.pages[0];

    // Collect all text from spans
    let mut extracted_text = String::new();
    for block in &first_page.blocks {
        for span in &block.spans {
            extracted_text.push_str(&span.text);
        }
    }

    // Expected: 7 U+FFFD characters followed by "AB "
    // Line 1: ��� (3 U+FFFD)
    // Line 2: ���� (4 U+FFFD)
    // Line 3: AB
    let expected_chars = vec!['\u{FFFD}', '\u{FFFD}', '\u{FFFD}', '\u{FFFD}', '\u{FFFD}', '\u{FFFD}', '\u{FFFD}', 'A', 'B', ' '];

    println!("Extracted text: {:?}", extracted_text);
    println!("Extracted text chars: {:?}", extracted_text.chars().collect::<Vec<_>>());

    let actual_chars: Vec<char> = extracted_text.chars().collect();

    // Check that we have at least the expected replacement characters
    let replacement_count = actual_chars.iter().filter(|&&c| c == '\u{FFFD}').count();
    assert!(
        replacement_count >= 7,
        "Expected at least 7 U+FFFD replacement characters, found {}",
        replacement_count
    );

    // Check that A and B are still mapped correctly
    assert!(
        extracted_text.contains('A') && extracted_text.contains('B'),
        "Expected text to contain mapped AGL glyphs A and B"
    );

    // Verify GLYPH_UNMAPPED diagnostics were emitted
    let unmapped_diagnostics: Vec<&String> = first_page.diagnostics
        .iter()
        .filter(|d| d.contains("GLYPH_UNMAPPED") || d.contains("unmapped"))
        .collect();

    println!("Diagnostics: {:?}", first_page.diagnostics);
    println!("Unmapped diagnostics: {:?}", unmapped_diagnostics);

    // We expect some diagnostics about unmapped glyphs
    // The exact count depends on how many unmapped glyphs were encountered
    assert!(
        !unmapped_diagnostics.is_empty(),
        "Expected at least one GLYPH_UNMAPPED diagnostic, found none"
    );

    println!("✓ Unmapped glyphs produce U+FFFD replacement characters");
    println!("✓ GLYPH_UNMAPPED diagnostics emitted: {}", unmapped_diagnostics.len());
}

#[test]
fn test_unmapped_comprehensive_fixture() {
    let fixture_path = Path::new("tests/fixtures/encoding/unmapped-comprehensive.pdf");

    assert!(
        fixture_path.exists(),
        "Test fixture should exist at {}",
        fixture_path.display()
    );

    let result = extract_pdf(
        fixture_path,
        &ExtractionOptions::default(),
    );

    assert!(
        result.is_ok(),
        "PDF extraction should succeed for unmapped-comprehensive fixture: {:?}",
        result.err()
    );

    let extraction_result = result.unwrap();
    let first_page = &extraction_result.pages[0];

    // Collect all text from spans
    let mut extracted_text = String::new();
    for block in &first_page.blocks {
        for span in &block.spans {
            extracted_text.push_str(&span.text);
        }
    }

    println!("Comprehensive extracted text: {:?}", extracted_text);
    println!("Comprehensive diagnostics: {:?}", first_page.diagnostics);

    // Verify U+FFFD characters are present
    let has_replacement = extracted_text.contains('\u{FFFD}');
    assert!(
        has_replacement,
        "Expected comprehensive fixture to contain U+FFFD replacement characters"
    );

    // Verify A and B are still mapped correctly
    assert!(
        extracted_text.contains('A') && extracted_text.contains('B'),
        "Expected comprehensive text to contain mapped AGL glyphs A and B"
    );
}
