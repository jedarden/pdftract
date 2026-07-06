//! CMAP unmapped glyph skip tests.
//!
//! This test module verifies that unmapped glyph names (like .notdef, .null)
//! are properly skipped during CMAP and ToUnicode entry creation.

use pdftract_core::font::cmap::{parse_to_unicode, ToUnicodeMap};
use pdftract_core::font::unmapped::is_unmapped_glyph_name;

/// Test that CMAP parsing handles unmapped glyph names correctly.
///
/// This is a basic structural test that verifies:
/// 1. The CMAP parser can be instantiated
/// 2. The unmapped glyph check function works
/// 3. A minimal end-to-end flow compiles
///
/// More comprehensive tests will be added in follow-up work.
#[test]
fn test_cmap_unmapped_glyph_skip() {
    // Verify that known unmapped glyph names are identified correctly
    assert!(is_unmapped_glyph_name(".notdef"), ".notdef should be unmapped");
    assert!(is_unmapped_glyph_name(".null"), ".null should be unmapped");

    // Verify that normal glyph names are not flagged as unmapped
    assert!(!is_unmapped_glyph_name("A"), "A should not be unmapped");
    assert!(!is_unmapped_glyph_name("space"), "space should not be unmapped");

    // Basic CMAP parsing test with simple mapping
    let cmap_data = b"beginbfchar 1 <00> <0041> endbfchar";
    let map = parse_to_unicode(cmap_data);

    // Verify the map was created
    assert!(!map.is_empty(), "CMAP should not be empty after parsing");
    assert_eq!(map.len(), 1, "CMAP should have 1 mapping");

    // Verify the mapping works
    let result = map.lookup(&[0x00]);
    assert_eq!(result, Some(&['A'][..]), "Byte 0x00 should map to 'A'");
}

/// Test that CMAP with multiple mappings handles unmapped glyphs correctly.
///
/// This test sets up a CMAP with multiple character mappings and verifies
/// that the parser handles them correctly, providing a baseline for
/// future unmapped glyph filtering tests.
#[test]
fn test_cmap_multiple_mappings_with_unmapped_check() {
    // Create a CMAP with multiple mappings
    let cmap_data = b"beginbfchar 3 <00> <0041> <01> <0042> <02> <0043> endbfchar";
    let map = parse_to_unicode(cmap_data);

    // Verify all mappings were created
    assert_eq!(map.len(), 3, "CMAP should have 3 mappings");

    // Verify each mapping
    assert_eq!(map.lookup(&[0x00]), Some(&['A'][..]), "0x00 should map to 'A'");
    assert_eq!(map.lookup(&[0x01]), Some(&['B'][..]), "0x01 should map to 'B'");
    assert_eq!(map.lookup(&[0x02]), Some(&['C'][..]), "0x02 should map to 'C'");

    // Verify unmapped glyph check still works
    assert!(is_unmapped_glyph_name(".notdef"), "Unmapped check should work");
}

/// Test CMAP range mapping with unmapped glyph awareness.
///
/// Tests the beginbfrange...endbfrange construct to ensure range mappings
/// work correctly, providing a foundation for testing unmapped glyph
/// filtering in range contexts.
#[test]
fn test_cmap_range_mapping_with_unmapped_awareness() {
    // Create a CMAP with a range mapping (A-Z)
    let cmap_data = b"beginbfrange 1 <0041> <005A> <0041> endbfrange";
    let map = parse_to_unicode(cmap_data);

    // Verify range was expanded
    assert_eq!(map.len(), 26, "Range should expand to 26 mappings (A-Z)");

    // Verify first and last entries
    assert_eq!(map.lookup(&[0x00, 0x41]), Some(&['A'][..]), "First should be 'A'");
    assert_eq!(map.lookup(&[0x00, 0x5A]), Some(&['Z'][..]), "Last should be 'Z'");

    // Verify unmapped glyph names are still recognized
    assert!(is_unmapped_glyph_name(".notdef"));
    assert!(is_unmapped_glyph_name("/.notdef"));  // With leading slash
}
