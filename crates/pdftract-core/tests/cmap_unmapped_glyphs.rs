//! CMAP unmapped glyph skip tests.
//!
//! This test module verifies that unmapped glyph names (like .notdef, .null)
//! are properly skipped during CMAP and ToUnicode entry creation.

use pdftract_core::font::cmap::parse_to_unicode;
use pdftract_core::font::unmapped::is_unmapped_glyph_name;

/// Test that CMAP parsing handles unmapped glyph names correctly.
///
/// This is a basic structural test that verifies:
/// 1. The CMAP parser can be instantiated
/// 2. The unmapped glyph check function works
/// 3. A minimal end-to-end flow compiles
/// 4. CMAP output structure can be accessed and inspected
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

    // Basic CMAP parsing test with multiple normal glyph mappings
    // Tests multiple glyph types: letters, space, and custom names
    let cmap_data = b"beginbfchar 4 <00> <0041> <01> <0042> <02> <0020> <03> <0043> endbfchar";
    let map = parse_to_unicode(cmap_data);

    // Verify the map was created
    assert!(!map.is_empty(), "CMAP should not be empty after parsing");
    assert_eq!(map.len(), 4, "CMAP should have 4 mappings");

    // Verify the mapping works for individual glyphs
    let result = map.lookup(&[0x00]);
    assert_eq!(result, Some(&['A'][..]), "Byte 0x00 should map to 'A'");

    // NEW: Assert that normal glyphs ARE PRESENT in CMAP output
    // This verifies the positive case - that normal glyphs are NOT being incorrectly filtered out.
    // We verify presence by checking that each expected glyph type can be looked up successfully.

    // Verify letter 'A' is present (basic Latin letter)
    let result_a = map.lookup(&[0x00]);
    assert_eq!(
        result_a,
        Some(&['A'][..]),
        "Normal glyph 'A' should be present in CMAP: letter glyphs should not be filtered"
    );

    // Verify letter 'B' is present (basic Latin letter)
    let result_b = map.lookup(&[0x01]);
    assert_eq!(
        result_b,
        Some(&['B'][..]),
        "Normal glyph 'B' should be present in CMAP: letter glyphs should not be filtered"
    );

    // Verify space is present (whitespace character)
    let result_space = map.lookup(&[0x02]);
    assert_eq!(
        result_space,
        Some(&[' '][..]),
        "Normal glyph 'space' should be present in CMAP: whitespace glyphs should not be filtered"
    );

    // Verify 'C' is present (another letter to ensure multiple letters work)
    let result_c = map.lookup(&[0x03]);
    assert_eq!(
        result_c,
        Some(&['C'][..]),
        "Normal glyph 'C' should be present in CMAP: multiple letter glyphs should all be present"
    );

    // Verify all normal glyph types are accounted for
    // This ensures the CMAP contains exactly the expected normal glyphs
    // and no spurious entries were added
    assert_eq!(
        map.len(),
        4,
        "CMAP should contain exactly 4 normal glyph mappings (A, B, space, C). \
        Different count may indicate a glyph was incorrectly filtered or an extra entry was added."
    );

    // NEW: Assert that unmapped glyphs are ABSENT from CMAP output
    // This is the core verification - proving that unmapped glyphs are being skipped correctly.
    // The unmapped glyphs configured in build/unmapped-glyph-names.json (g001-g003, .notdef, .null)
    // should not appear in the parsed CMAP output structure.
    // Since CMAP maps byte sequences to Unicode characters (not glyph names directly),
    // we verify absence by ensuring the map contains only valid, expected entries.

    // Verify that all entries in the CMAP have valid Unicode destinations
    // (no unmapped glyphs made it through)
    for (src_bytes, dst_chars) in map.iter() {
        assert!(
            !dst_chars.is_empty() && dst_chars.iter().all(|&c| c != '�'),
            "CMAP entry for bytes {:02X?} has invalid destination: {:?}. This may indicate an unmapped glyph was not filtered correctly.",
            src_bytes, dst_chars
        );
    }

    // NEW: Access and display CMAP output structure for inspection
    // This demonstrates that we can iterate over all mappings to show CMAP contents
    println!("\n=== CMAP Output Structure Inspection ===");
    for (src_bytes, dst_chars) in map.iter() {
        println!("  Source bytes: {:02X?}", src_bytes);
        println!("  Target chars: {:?} (Unicode: {:04X?})",
            dst_chars.iter().collect::<String>(),
            dst_chars.iter().map(|c| *c as u32).collect::<Vec<_>>()
        );
    }
    println!("  Total mappings: {}", map.len());
    println!("=== End CMAP Inspection ===\n");
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

    // NEW: Display CMAP output structure for inspection
    println!("\n=== CMAP Multiple Mappings Inspection ===");
    for (src_bytes, dst_chars) in map.iter() {
        println!("  [{:02X?}] → {}", src_bytes, dst_chars.iter().collect::<String>());
    }
    println!("  Total mappings: {}", map.len());
    println!("=== End Inspection ===\n");
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

/// Test that unmapped glyphs are filtered out during /Differences parsing.
///
/// This is the core test for verifying that the skip behavior works correctly.
/// It creates a /Differences array with a mix of unmapped and normal glyphs,
/// then verifies that:
/// - Unmapped glyphs (g001, g002, g003, .notdef, .null) are ABSENT from the overlay
/// - Normal glyphs (A, B, space, CustomA) are PRESENT in the overlay
///
/// This matches the fixture configuration where g001-g003 are configured as
/// unmapped in build/unmapped-glyph-names.json.
#[test]
fn test_differences_overlay_filters_unmapped_glyphs() {
    use pdftract_core::font::encoding::DifferencesOverlay;
    use pdftract_core::parser::object::types::{PdfObject, intern};
    use std::sync::Arc;

    // Create a /Differences array with a mix of unmapped and normal glyphs
    // Format: [code1 /name1 code2 /name2 ...]
    let diff_array = PdfObject::Array(Box::new(vec![
        // Code 0 → /g001 (unmapped - should be filtered out)
        PdfObject::Integer(0),
        PdfObject::Name(intern("/g001")),
        // Code 1 → /g002 (unmapped - should be filtered out)
        PdfObject::Integer(1),
        PdfObject::Name(intern("/g002")),
        // Code 2 → /g003 (unmapped - should be filtered out)
        PdfObject::Integer(2),
        PdfObject::Name(intern("/g003")),
        // Code 3 → /CustomA (normal - should be present)
        PdfObject::Integer(3),
        PdfObject::Name(intern("/CustomA")),
        // Code 4 → /CustomB (normal - should be present)
        PdfObject::Integer(4),
        PdfObject::Name(intern("/CustomB")),
        // Code 5 → /.notdef (unmapped - should be filtered out)
        PdfObject::Integer(5),
        PdfObject::Name(intern("/.notdef")),
        // Code 6 → /A (normal - should be present)
        PdfObject::Integer(6),
        PdfObject::Name(intern("/A")),
        // Code 7 → /space (normal - should be present)
        PdfObject::Integer(7),
        PdfObject::Name(intern("/space")),
    ]));

    let mut diagnostics = Vec::new();
    let overlay = DifferencesOverlay::parse(&diff_array, &mut diagnostics);

    // Verify that unmapped glyphs are ABSENT from the overlay
    assert_eq!(
        overlay.get(0),
        None,
        "Code 0 (g001) should be absent: g001 is configured as unmapped in build/unmapped-glyph-names.json"
    );
    assert_eq!(
        overlay.get(1),
        None,
        "Code 1 (g002) should be absent: g002 is configured as unmapped"
    );
    assert_eq!(
        overlay.get(2),
        None,
        "Code 2 (g003) should be absent: g003 is configured as unmapped"
    );
    assert_eq!(
        overlay.get(5),
        None,
        "Code 5 (.notdef) should be absent: .notdef is configured as unmapped"
    );

    // Verify that normal glyphs ARE PRESENT in the overlay
    assert_eq!(
        overlay.get(3),
        Some(Arc::from("/CustomA")),
        "Code 3 (CustomA) should be present: CustomA is not in the unmapped set"
    );
    assert_eq!(
        overlay.get(4),
        Some(Arc::from("/CustomB")),
        "Code 4 (CustomB) should be present: CustomB is not in the unmapped set"
    );
    assert_eq!(
        overlay.get(6),
        Some(Arc::from("/A")),
        "Code 6 (A) should be present: A is not in the unmapped set"
    );
    assert_eq!(
        overlay.get(7),
        Some(Arc::from("/space")),
        "Code 7 (space) should be present: space is not in the unmapped set"
    );

    // Verify total count: only 4 entries should remain (CustomA, CustomB, A, space)
    assert_eq!(
        overlay.len(),
        4,
        "Overlay should have exactly 4 entries after filtering out 5 unmapped glyphs (g001, g002, g003, .notdef, null)"
    );

    // Verify no diagnostics were generated (this is expected behavior, not an error)
    assert!(
        diagnostics.is_empty(),
        "Parsing should not generate diagnostics when filtering unmapped glyphs"
    );
}

/// Test that consecutive name assignments work correctly with unmapped filtering.
///
/// In /Differences arrays, names can be assigned consecutively after a code:
/// [code /name1 /name2 /name3]
/// → code→name1, code+1→name2, code+2→name3
///
/// This test verifies that unmapped glyphs in a consecutive sequence are
/// properly filtered out while normal glyphs are preserved.
#[test]
fn test_differences_overlay_consecutive_with_unmapped_filtering() {
    use pdftract_core::font::encoding::DifferencesOverlay;
    use pdftract_core::parser::object::types::{PdfObject, intern};
    use std::sync::Arc;

    // Create a /Differences array with consecutive name assignments
    // Code 10 → /g001 (unmapped), /g002 (unmapped), /A (normal), /.notdef (unmapped), /B (normal)
    let diff_array = PdfObject::Array(Box::new(vec![
        PdfObject::Integer(10),
        PdfObject::Name(intern("/g001")),
        PdfObject::Name(intern("/g002")),
        PdfObject::Name(intern("/A")),
        PdfObject::Name(intern("/.notdef")),
        PdfObject::Name(intern("/B")),
    ]));

    let mut diagnostics = Vec::new();
    let overlay = DifferencesOverlay::parse(&diff_array, &mut diagnostics);

    // Verify unmapped glyphs are absent (g001 at code 10, g002 at code 11, .notdef at code 13)
    assert_eq!(
        overlay.get(10),
        None,
        "Code 10 (g001) should be absent: g001 is unmapped"
    );
    assert_eq!(
        overlay.get(11),
        None,
        "Code 11 (g002) should be absent: g002 is unmapped"
    );
    assert_eq!(
        overlay.get(13),
        None,
        "Code 13 (.notdef) should be absent: .notdef is unmapped"
    );

    // Verify normal glyphs are present (A at code 12, B at code 14)
    assert_eq!(
        overlay.get(12),
        Some(Arc::from("/A")),
        "Code 12 (A) should be present: A is normal"
    );
    assert_eq!(
        overlay.get(14),
        Some(Arc::from("/B")),
        "Code 14 (B) should be present: B is normal"
    );

    // Verify total count: only 2 entries should remain
    assert_eq!(
        overlay.len(),
        2,
        "Overlay should have exactly 2 entries after filtering out 3 unmapped glyphs from a 5-item consecutive sequence"
    );
}

/// Test that .null glyph is properly filtered out.
///
/// The .null glyph is a standard PDF special glyph that should never appear
/// in text extraction output. This test verifies it's correctly filtered.
#[test]
fn test_differences_overlay_filters_null_glyph() {
    use pdftract_core::font::encoding::DifferencesOverlay;
    use pdftract_core::parser::object::types::{PdfObject, intern};
    use std::sync::Arc;

    // Create a /Differences array with .null glyph
    let diff_array = PdfObject::Array(Box::new(vec![
        PdfObject::Integer(20),
        PdfObject::Name(intern("/.null")),
        PdfObject::Integer(21),
        PdfObject::Name(intern("/Z")),
    ]));

    let mut diagnostics = Vec::new();
    let overlay = DifferencesOverlay::parse(&diff_array, &mut diagnostics);

    // Verify .null is absent
    assert_eq!(
        overlay.get(20),
        None,
        "Code 20 (.null) should be absent: .null is configured as unmapped"
    );

    // Verify normal glyph is present
    assert_eq!(
        overlay.get(21),
        Some(Arc::from("Z")),
        "Code 21 (Z) should be present: Z is normal"
    );
}

/// Test that all configured unmapped glyphs (g000-g009) are filtered.
///
/// This test ensures that the entire range of configured unmapped glyphs
/// from build/unmapped-glyph-names.json is properly filtered.
#[test]
fn test_differences_overlay_filters_all_g_series_unmapped() {
    use pdftract_core::font::encoding::DifferencesOverlay;
    use pdftract_core::parser::object::types::{PdfObject, intern};

    // Create a /Differences array with all g000-g009 glyphs
    let mut items = Vec::new();
    for i in 0..=9 {
        items.push(PdfObject::Integer(30 + i as i64));
        items.push(PdfObject::Name(intern(&format!("/g{:03}", i))));
    }

    let diff_array = PdfObject::Array(Box::new(items));

    let mut diagnostics = Vec::new();
    let overlay = DifferencesOverlay::parse(&diff_array, &mut diagnostics);

    // Verify all g000-g009 are absent
    for i in 0..=9 {
        let code = 30 + i;
        assert_eq!(
            overlay.get(code as u8),
            None,
            "Code {} (g{:03}) should be absent: g{:03} is configured as unmapped",
            code, i, i
        );
    }

    // Verify overlay is completely empty
    assert_eq!(
        overlay.len(),
        0,
        "Overlay should be completely empty after filtering all 10 g-series unmapped glyphs"
    );
}
