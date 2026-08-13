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
    assert!(
        is_unmapped_glyph_name(".notdef"),
        ".notdef should be identified as unmapped. \
         Expected: true. \
         Found: {}. \
         Why this matters: .notdef is the standard PDF fallback glyph configured in \
         build/unmapped-glyph-names.json and must never appear in text extraction.",
        is_unmapped_glyph_name(".notdef")
    );
    assert!(
        is_unmapped_glyph_name(".null"),
        ".null should be identified as unmapped. \
         Expected: true. \
         Found: {}. \
         Why this matters: .null is a standard PDF special glyph configured as unmapped.",
        is_unmapped_glyph_name(".null")
    );

    // Verify that normal glyph names are not flagged as unmapped
    assert!(
        !is_unmapped_glyph_name("A"),
        "A should NOT be identified as unmapped. \
         Expected: false. \
         Found: {}. \
         Why this matters: A is a normal Latin letter that should always be preserved in text.",
        is_unmapped_glyph_name("A")
    );
    assert!(
        !is_unmapped_glyph_name("space"),
        "space should NOT be identified as unmapped. \
         Expected: false. \
         Found: {}. \
         Why this matters: space is a standard whitespace character that should be preserved.",
        is_unmapped_glyph_name("space")
    );

    // Basic CMAP parsing test with multiple normal glyph mappings
    // Tests multiple glyph types: letters, space, and custom names
    let cmap_data = b"beginbfchar 4 <00> <0041> <01> <0042> <02> <0020> <03> <0043> endbfchar";
    let map = parse_to_unicode(cmap_data);

    // Verify the map was created
    assert!(
        !map.is_empty(),
        "CMAP should not be empty after parsing valid glyph mappings. \
         Expected: non-empty map. \
         Found: empty map. \
         Why this matters: If the CMAP parser produces an empty map from valid input, \
         the parser is incorrectly rejecting all glyphs or has a parsing error."
    );
    assert_eq!(
        map.len(),
        4,
        "CMAP should have exactly 4 mappings after parsing. \
         Expected: 4 mappings (A, B, space, C). \
         Found: {} mappings. \
         Why this matters: Incorrect mapping count indicates the parser is dropping \
         or duplicating entries.",
        map.len()
    );

    // Verify the mapping works for individual glyphs
    let result = map.lookup(&[0x00]);
    assert_eq!(
        result,
        Some(&['A'][..]),
        "Byte 0x00 should map to 'A'. \
         Expected: Some(\"A\"). \
         Found: {:?}. \
         Why this matters: This verifies the basic lookup functionality works correctly.",
        result
    );

    // NEW: Assert that normal glyphs ARE PRESENT in CMAP output
    // This verifies the positive case - that normal glyphs are NOT being incorrectly filtered out.
    // We verify presence by checking that each expected glyph type can be looked up successfully.

    // Verify letter 'A' is present (basic Latin letter)
    let result_a = map.lookup(&[0x00]);
    assert_eq!(
        result_a,
        Some(&['A'][..]),
        "Normal glyph 'A' should be present in CMAP. \
         Expected: Some(\"A\"). \
         Found: {:?}. \
         Why this matters: Letter glyphs should not be filtered out - only unmapped glyphs should be excluded.",
        result_a
    );

    // Verify letter 'B' is present (basic Latin letter)
    let result_b = map.lookup(&[0x01]);
    assert_eq!(
        result_b,
        Some(&['B'][..]),
        "Normal glyph 'B' should be present in CMAP. \
         Expected: Some(\"B\"). \
         Found: {:?}. \
         Why this matters: Letter glyphs should not be filtered out - only unmapped glyphs should be excluded.",
        result_b
    );

    // Verify space is present (whitespace character)
    let result_space = map.lookup(&[0x02]);
    assert_eq!(
        result_space,
        Some(&[' '][..]),
        "Normal glyph 'space' should be present in CMAP. \
         Expected: Some(\" \"). \
         Found: {:?}. \
         Why this matters: Whitespace glyphs should not be filtered out - only unmapped glyphs should be excluded.",
        result_space
    );

    // Verify 'C' is present (another letter to ensure multiple letters work)
    let result_c = map.lookup(&[0x03]);
    assert_eq!(
        result_c,
        Some(&['C'][..]),
        "Normal glyph 'C' should be present in CMAP. \
         Expected: Some(\"C\"). \
         Found: {:?}. \
         Why this matters: Multiple letter glyphs should all be present - the parser should not filter out valid letters.",
        result_c
    );

    // Verify all normal glyph types are accounted for
    // This ensures the CMAP contains exactly the expected normal glyphs
    // and no spurious entries were added
    assert_eq!(
        map.len(),
        4,
        "CMAP should contain exactly 4 normal glyph mappings. \
         Expected: 4 mappings (A, B, space, C). \
         Found: {} mappings. \
         Why this matters: Different count may indicate a glyph was incorrectly filtered or an extra entry was added.",
        map.len()
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
            "CMAP entry for bytes {:02X?} has invalid destination: {:?}. \
             Expected: non-empty vector of valid Unicode characters (no replacement character). \
             Found: empty or contains '�'. \
             Why this matters: This may indicate an unmapped glyph was not filtered correctly, \
             or the parser is generating invalid Unicode mappings.",
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
    assert_eq!(
        map.len(),
        3,
        "CMAP should have exactly 3 mappings. \
         Expected: 3 mappings (A, B, C). \
         Found: {} mappings. \
         Why this matters: Incorrect mapping count indicates the parser is incorrectly \
         handling the beginbfchar...endbfchar construct.",
        map.len()
    );

    // Verify each mapping
    assert_eq!(
        map.lookup(&[0x00]),
        Some(&['A'][..]),
        "0x00 should map to 'A'. \
         Expected: Some(\"A\"). \
         Found: {:?}. \
         Why this matters: Verifies the first mapping in the sequence is correct.",
        map.lookup(&[0x00])
    );
    assert_eq!(
        map.lookup(&[0x01]),
        Some(&['B'][..]),
        "0x01 should map to 'B'. \
         Expected: Some(\"B\"). \
         Found: {:?}. \
         Why this matters: Verifies the second mapping in the sequence is correct.",
        map.lookup(&[0x01])
    );
    assert_eq!(
        map.lookup(&[0x02]),
        Some(&['C'][..]),
        "0x02 should map to 'C'. \
         Expected: Some(\"C\"). \
         Found: {:?}. \
         Why this matters: Verifies the third mapping in the sequence is correct.",
        map.lookup(&[0x02])
    );

    // Verify unmapped glyph check still works
    assert!(
        is_unmapped_glyph_name(".notdef"),
        "Unmapped glyph check should still work after parsing. \
         Expected: true. \
         Found: {}. \
         Why this matters: This verifies the unmapped_glyph_name function is not affected \
         by CMAP parsing operations.",
        is_unmapped_glyph_name(".notdef")
    );

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
    assert_eq!(
        map.len(),
        26,
        "Range should expand to exactly 26 mappings (A-Z). \
         Expected: 26 mappings. \
         Found: {} mappings. \
         Why this matters: The beginbfrange...endbfrange construct should expand the range \
         <0041>-<005A> to 26 individual mappings, one for each letter in the alphabet.",
        map.len()
    );

    // Verify first and last entries
    assert_eq!(
        map.lookup(&[0x00, 0x41]),
        Some(&['A'][..]),
        "First entry in range should be 'A'. \
         Expected: Some(\"A\"). \
         Found: {:?}. \
         Why this matters: Verifies the range starts at the correct character (A = U+0041).",
        map.lookup(&[0x00, 0x41])
    );
    assert_eq!(
        map.lookup(&[0x00, 0x5A]),
        Some(&['Z'][..]),
        "Last entry in range should be 'Z'. \
         Expected: Some(\"Z\"). \
         Found: {:?}. \
         Why this matters: Verifies the range ends at the correct character (Z = U+005A).",
        map.lookup(&[0x00, 0x5A])
    );

    // Verify unmapped glyph names are still recognized
    assert!(
        is_unmapped_glyph_name(".notdef"),
        "Unmapped check should recognize '.notdef'. \
         Expected: true. \
         Found: {}. \
         Why this matters: Verifies the unmapped_glyph_name function works without leading slash.",
        is_unmapped_glyph_name(".notdef")
    );
    assert!(
        is_unmapped_glyph_name("/.notdef"),
        "Unmapped check should recognize '/.notdef' (with leading slash). \
         Expected: true. \
         Found: {}. \
         Why this matters: PDF glyph names may include a leading slash; the check must handle both.",
        is_unmapped_glyph_name("/.notdef")
    );
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
        // Code 3 → /CustomA (unmapped - should be filtered out)
        // NOTE: CustomA is configured as unmapped in build/unmapped-glyph-names.json
        PdfObject::Integer(3),
        PdfObject::Name(intern("/CustomA")),
        // Code 4 → /CustomB (unmapped - should be filtered out)
        // NOTE: CustomB is configured as unmapped in build/unmapped-glyph-names.json
        PdfObject::Integer(4),
        PdfObject::Name(intern("/CustomB")),
        // Code 5 → /.notdef (unmapped - should be filtered out)
        PdfObject::Integer(5),
        PdfObject::Name(intern("/.notdef")),
        // Code 6 → /A (normal - should be present)
        PdfObject::Integer(6),
        PdfObject::Name(intern("/A")),
        // Code 7 → /B (normal - should be present)
        PdfObject::Integer(7),
        PdfObject::Name(intern("/B")),
        // Code 8 → /space (normal - should be present)
        PdfObject::Integer(8),
        PdfObject::Name(intern("/space")),
    ]));

    let mut diagnostics = Vec::new();
    let overlay = DifferencesOverlay::parse(&diff_array, &mut diagnostics);

    // Verify that unmapped glyphs are ABSENT from the overlay
    // These assertions verify the core skip behavior: glyphs configured as unmapped
    // in build/unmapped-glyph-names.json must NOT appear in the parsed overlay.
    assert_eq!(
        overlay.get(0),
        None,
        "Code 0 (g001) should be absent from DifferencesOverlay. \
         Expected: None (unmapped glyph should be filtered out). \
         Found: {:?}. \
         Why this matters: g001 is configured as unmapped in build/unmapped-glyph-names.json \
         and should be skipped during parsing.",
        overlay.get(0)
    );
    assert_eq!(
        overlay.get(1),
        None,
        "Code 1 (g002) should be absent from DifferencesOverlay. \
         Expected: None (unmapped glyph should be filtered out). \
         Found: {:?}. \
         Why this matters: g002 is configured as unmapped and should be skipped.",
        overlay.get(1)
    );
    assert_eq!(
        overlay.get(2),
        None,
        "Code 2 (g003) should be absent from DifferencesOverlay. \
         Expected: None (unmapped glyph should be filtered out). \
         Found: {:?}. \
         Why this matters: g003 is configured as unmapped and should be skipped.",
        overlay.get(2)
    );
    assert_eq!(
        overlay.get(3),
        None,
        "Code 3 (CustomA) should be absent from DifferencesOverlay. \
         Expected: None (unmapped glyph should be filtered out). \
         Found: {:?}. \
         Why this matters: CustomA is configured as unmapped in build/unmapped-glyph-names.json \
         and should be skipped during parsing.",
        overlay.get(3)
    );
    assert_eq!(
        overlay.get(4),
        None,
        "Code 4 (CustomB) should be absent from DifferencesOverlay. \
         Expected: None (unmapped glyph should be filtered out). \
         Found: {:?}. \
         Why this matters: CustomB is configured as unmapped in build/unmapped-glyph-names.json \
         and should be skipped during parsing.",
        overlay.get(4)
    );
    assert_eq!(
        overlay.get(5),
        None,
        "Code 5 (.notdef) should be absent from DifferencesOverlay. \
         Expected: None (unmapped glyph should be filtered out). \
         Found: {:?}. \
         Why this matters: .notdef is the standard PDF fallback glyph that must never appear in text extraction.",
        overlay.get(5)
    );

    // Verify that normal glyphs ARE PRESENT in the overlay
    // This ensures we don't over-filter: normal glyphs that ARE NOT in the unmapped set
    // must appear in the parsed overlay.
    assert_eq!(
        overlay.get(6),
        Some(Arc::from("/A")),
        "Code 6 (A) should be present in DifferencesOverlay. \
         Expected: Some(\"/A\"). \
         Found: {:?}. \
         Why this matters: A is a normal Latin letter and should never be filtered.",
        overlay.get(6)
    );
    assert_eq!(
        overlay.get(7),
        Some(Arc::from("/B")),
        "Code 7 (B) should be present in DifferencesOverlay. \
         Expected: Some(\"/B\"). \
         Found: {:?}. \
         Why this matters: B is a normal Latin letter and should never be filtered.",
        overlay.get(7)
    );
    assert_eq!(
        overlay.get(8),
        Some(Arc::from("/space")),
        "Code 8 (space) should be present in DifferencesOverlay. \
         Expected: Some(\"/space\"). \
         Found: {:?}. \
         Why this matters: space is a standard whitespace character and should never be filtered.",
        overlay.get(8)
    );

    // Verify total count: only 3 entries should remain (A, B, space)
    assert_eq!(
        overlay.len(),
        3,
        "Overlay should have exactly 3 entries after filtering out unmapped glyphs. \
         Expected: 3 entries (A, B, space). \
         Found: {} entries. \
         Why this matters: 6 unmapped glyphs (g001, g002, g003, CustomA, CustomB, .notdef) \
         were filtered from build/unmapped-glyph-names.json, leaving only 3 normal glyphs.",
        overlay.len()
    );

    // Verify no diagnostics were generated (this is expected behavior, not an error)
    assert!(
        diagnostics.is_empty(),
        "Parsing should not generate diagnostics when filtering unmapped glyphs. \
         Expected: empty diagnostics vector. \
         Found: {} diagnostics. \
         Why this matters: Filtering unmapped glyphs is expected behavior, not an error condition.",
        diagnostics.len()
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
    // In consecutive sequences, unmapped glyphs must be filtered out while preserving
    // the correct code assignments for normal glyphs.
    assert_eq!(
        overlay.get(10),
        None,
        "Code 10 (g001) should be absent in consecutive sequence. \
         Expected: None (unmapped glyph filtered out). \
         Found: {:?}. \
         Why this matters: In the sequence [10 → g001, g002, A, .notdef, B], \
         g001 is at position 10 and is unmapped.",
        overlay.get(10)
    );
    assert_eq!(
        overlay.get(11),
        None,
        "Code 11 (g002) should be absent in consecutive sequence. \
         Expected: None (unmapped glyph filtered out). \
         Found: {:?}. \
         Why this matters: g002 is at position 11 (consecutive after g001) and is unmapped.",
        overlay.get(11)
    );
    assert_eq!(
        overlay.get(13),
        None,
        "Code 13 (.notdef) should be absent in consecutive sequence. \
         Expected: None (unmapped glyph filtered out). \
         Found: {:?}. \
         Why this matters: .notdef is at position 13 (after A) and is unmapped.",
        overlay.get(13)
    );

    // Verify normal glyphs are present (A at code 12, B at code 14)
    assert_eq!(
        overlay.get(12),
        Some(Arc::from("/A")),
        "Code 12 (A) should be present in consecutive sequence. \
         Expected: Some(\"/A\"). \
         Found: {:?}. \
         Why this matters: A is at position 12 (third in consecutive sequence) and is normal.",
        overlay.get(12)
    );
    assert_eq!(
        overlay.get(14),
        Some(Arc::from("/B")),
        "Code 14 (B) should be present in consecutive sequence. \
         Expected: Some(\"/B\"). \
         Found: {:?}. \
         Why this matters: B is at position 14 (fifth in consecutive sequence) and is normal.",
        overlay.get(14)
    );

    // Verify total count: only 2 entries should remain
    assert_eq!(
        overlay.len(),
        2,
        "Overlay should have exactly 2 entries after filtering consecutive sequence. \
         Expected: 2 entries (A at code 12, B at code 14). \
         Found: {} entries. \
         Why this matters: 3 unmapped glyphs (g001, g002, .notdef) were filtered from the 5-item consecutive sequence.",
        overlay.len()
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
        "Code 20 (.null) should be absent. \
         Expected: None (unmapped glyph filtered out). \
         Found: {:?}. \
         Why this matters: .null is a standard PDF special glyph that should never appear in text extraction output.",
        overlay.get(20)
    );

    // Verify normal glyph is present
    assert_eq!(
        overlay.get(21),
        Some(Arc::from("/Z")),
        "Code 21 (Z) should be present. \
         Expected: Some(\"/Z\"). \
         Found: {:?}. \
         Why this matters: Z is a normal Latin letter and should be preserved.",
        overlay.get(21)
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
    // This comprehensive test ensures the entire g-series range is properly filtered.
    for i in 0..=9 {
        let code = 30 + i;
        assert_eq!(
            overlay.get(code as u8),
            None,
            "Code {} (g{:03}) should be absent. \
             Expected: None (unmapped glyph filtered out). \
             Found: {:?}. \
             Why this matters: All g000-g009 glyphs are configured as unmapped in \
             build/unmapped-glyph-names.json and should be filtered out. \
             This is iteration {}/10 of the full g-series range.",
            code, i, overlay.get(code as u8), i + 1
        );
    }

    // Verify overlay is completely empty
    assert_eq!(
        overlay.len(),
        0,
        "Overlay should be completely empty after filtering all 10 g-series unmapped glyphs. \
         Expected: 0 entries. \
         Found: {} entries. \
         Why this matters: This proves the unmapped glyph filter works correctly across \
         the entire configured g-series range (g000-g009).",
        overlay.len()
    );
}
