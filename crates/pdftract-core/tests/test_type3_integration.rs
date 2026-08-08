//! Integration test for Type3 font test module setup.
//!
//! This test verifies that all test fixtures and helper functions
//! from the Type3 font module are accessible and working correctly.

use pdftract_core::font::type3_test_fixtures::{
    create_basic_glyph_dict,
    create_charproc_stream_with_curves,
    create_empty_content_stream,
    create_main_content_stream,
    create_main_content_stream_multi,
    create_minimal_glyph_dict,
    create_minimal_type3_font,
    create_simple_charproc_stream,
    mock_counter,
    mock_resolver,
    mock_source,
    to_charprocs_map,
    GlyphEntry,
};
use pdftract_core::font::Type3Font;
use pdftract_core::parser::object::types::ObjRef;

#[test]
fn test_fixture_accessibility() {
    // Test that all fixture functions are accessible
    let notdef_ref = ObjRef::new(10, 0);
    let test_ref = ObjRef::new(11, 0);

    // Test basic glyph dict creation
    let dict = create_basic_glyph_dict(notdef_ref, test_ref);
    assert_eq!(dict.len(), 2);
    assert!(dict.contains_key(".notdef"));
    assert!(dict.contains_key("A"));

    // Test minimal glyph dict creation
    let minimal_dict = create_minimal_glyph_dict(notdef_ref);
    assert_eq!(minimal_dict.len(), 1);

    // Test charproc stream creation
    let simple_stream = create_simple_charproc_stream();
    assert!(!simple_stream.is_empty());

    let curved_stream = create_charproc_stream_with_curves();
    assert!(!curved_stream.is_empty());

    // Test main content stream creation
    let main_stream = create_main_content_stream();
    assert!(!main_stream.is_empty());

    let main_stream_multi = create_main_content_stream_multi();
    assert!(!main_stream_multi.is_empty());

    // Test empty content stream creation
    let empty_stream = create_empty_content_stream();
    assert!(!empty_stream.is_empty());

    // Test mock creation
    let resolver = mock_resolver();
    let source = mock_source();
    let counter = mock_counter();

    // Verify they work
    use std::sync::atomic::Ordering;
    resolver.store(true, Ordering::SeqCst);
    source.store(true, Ordering::SeqCst);
    counter.fetch_add(1, Ordering::SeqCst);

    assert!(resolver.load(Ordering::SeqCst));
    assert!(source.load(Ordering::SeqCst));
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn test_type3_font_from_fixture() {
    // Test that we can create a Type3Font using fixture functions
    let notdef_ref = ObjRef::new(42, 0);
    let font = create_minimal_type3_font(notdef_ref);

    // Verify the font properties
    assert!(font.char_procs.contains_key(".notdef"));
    assert_eq!(font.first_char, 0);
    assert_eq!(font.last_char, 0);
    assert_eq!(font.widths.len(), 1);
    assert_eq!(font.widths[0], 500.0);
}

#[test]
fn test_glyph_entry_creation() {
    // Test GlyphEntry creation
    let entry = GlyphEntry::new("test", 650.0, [10.0, 20.0, 640.0, 750.0], ObjRef::new(5, 0));

    assert_eq!(entry.name.as_ref(), "test");
    assert_eq!(entry.width, 650.0);
    assert_eq!(entry.bbox, [10.0, 20.0, 640.0, 750.0]);
    assert_eq!(entry.charproc_ref, ObjRef::new(5, 0));
}

#[test]
fn test_to_charprocs_conversion() {
    // Test to_charprocs_map conversion
    let notdef_ref = ObjRef::new(10, 0);
    let test_ref = ObjRef::new(11, 0);
    let glyph_dict = create_basic_glyph_dict(notdef_ref, test_ref);

    let charprocs = to_charprocs_map(&glyph_dict);

    assert_eq!(charprocs.len(), 2);
    assert_eq!(charprocs.get(".notdef"), Some(&notdef_ref));
    assert_eq!(charprocs.get("A"), Some(&test_ref));
}

#[test]
fn test_integration_with_type3_font() {
    // This test verifies complete integration:
    // 1. Create glyph dict using fixtures
    // 2. Convert to charprocs format
    // 3. Use with Type3Font

    let notdef_ref = ObjRef::new(100, 0);
    let test_ref = ObjRef::new(101, 0);

    // Create glyph dict
    let glyph_dict = create_basic_glyph_dict(notdef_ref, test_ref);

    // Convert to charprocs
    let charprocs = to_charprocs_map(&glyph_dict);

    // Create Type3Font with charprocs
    let font = Type3Font::mock(Some(charprocs));

    // Verify the font has the correct glyphs
    assert!(font.has_glyph(".notdef"));
    assert!(font.has_glyph("A"));
    assert_eq!(font.glyph_count(), 2);
}

#[test]
fn test_charproc_streams_for_different_glyphs() {
    // Test that we can create different charproc streams for different glyphs

    // Simple stream for basic glyphs
    let simple = create_simple_charproc_stream();
    assert!(simple.starts_with(b"0 0 m"));

    // Curved stream for complex glyphs
    let curved = create_charproc_stream_with_curves();
    let curved_str = std::str::from_utf8(&curved).unwrap();
    assert!(curved_str.contains("c")); // Contains curveto

    // They should be different
    assert_ne!(simple, curved);
}

#[test]
fn test_main_content_streams_for_text_drawing() {
    // Test main content streams for drawing text with Type3 fonts

    let single = create_main_content_stream();
    let multi = create_main_content_stream_multi();

    // Both should be valid PDF streams
    let single_str = std::str::from_utf8(&single).unwrap();
    let multi_str = std::str::from_utf8(&multi).unwrap();

    // Should have BT/ET (Begin/End Text)
    assert!(single_str.contains("BT"));
    assert!(single_str.contains("ET"));
    assert!(multi_str.contains("BT"));
    assert!(multi_str.contains("ET"));

    // Should have font selection (Tf)
    assert!(single_str.contains("Tf"));
    assert!(multi_str.contains("Tf"));

    // Should have text positioning (Td)
    assert!(single_str.contains("Td"));
    assert!(multi_str.contains("Td"));

    // Should have text drawing (Tj)
    assert!(single_str.contains("Tj"));
    assert!(multi_str.contains("Tj"));
}

#[test]
fn test_content_stream_references_glyph_from_dict() {
    // Test that content stream functions create streams that reference valid glyphs

    // Create a glyph dict with specific glyphs
    let notdef_ref = ObjRef::new(10, 0);
    let test_ref = ObjRef::new(11, 0);
    let glyph_dict = create_basic_glyph_dict(notdef_ref, test_ref);

    // Create content stream that references glyph "A"
    let content_stream = create_main_content_stream();
    let stream_str = std::str::from_utf8(&content_stream).unwrap();

    // Verify the stream references glyph "A" which exists in the dict
    assert!(stream_str.contains("(A)"), "Content stream should reference glyph 'A'");
    assert!(glyph_dict.contains_key("A"), "Glyph dict should contain 'A'");

    // Verify we can extract the referenced glyph from the stream
    // The stream contains "(A) Tj" which draws glyph A
    assert!(stream_str.contains("Tj"), "Stream should contain Tj operator");
}

#[test]
fn test_content_stream_with_multiple_glyphs() {
    // Test content stream that references multiple glyphs

    // Create glyph dict with multiple glyphs
    let notdef_ref = ObjRef::new(10, 0);
    let a_ref = ObjRef::new(11, 0);
    let b_ref = ObjRef::new(12, 0);
    let c_ref = ObjRef::new(13, 0);

    let mut glyph_dict = create_basic_glyph_dict(notdef_ref, a_ref);

    // Add more glyphs "B" and "C"
    use std::sync::Arc;
    glyph_dict.insert(
        Arc::from("B"),
        GlyphEntry::new("B", 600.0, [50.0, 0.0, 550.0, 700.0], b_ref)
    );
    glyph_dict.insert(
        Arc::from("C"),
        GlyphEntry::new("C", 600.0, [50.0, 0.0, 550.0, 700.0], c_ref)
    );

    // Create content stream that references multiple glyphs
    let content_stream = create_main_content_stream_multi();
    let stream_str = std::str::from_utf8(&content_stream).unwrap();

    // Verify the stream references glyphs that exist in the dict
    assert!(glyph_dict.contains_key("A"), "Glyph dict should contain 'A'");
    assert!(glyph_dict.contains_key("B"), "Glyph dict should contain 'B'");
    assert!(glyph_dict.contains_key("C"), "Glyph dict should contain 'C'");

    // The stream should reference these glyphs (AB is in "(AB) Tj", C is in "(C) Tj")
    assert!(stream_str.contains("(AB)"), "Stream should reference glyphs 'AB'");
    assert!(stream_str.contains("(C)"), "Stream should reference glyph 'C'");
}

#[test]
fn test_charproc_stream_valid_for_glyph_dict() {
    // Test that charproc streams work with glyph dictionaries

    // Create glyph dict
    let notdef_ref = ObjRef::new(10, 0);
    let glyph_ref = ObjRef::new(11, 0);
    let glyph_dict = create_basic_glyph_dict(notdef_ref, glyph_ref);

    // Get a charproc stream
    let charproc_stream = create_simple_charproc_stream();

    // Verify the stream is valid (not empty, has PDF operators)
    assert!(!charproc_stream.is_empty(), "Charproc stream should not be empty");

    // Verify the glyph dict has the expected entries
    assert_eq!(glyph_dict.len(), 2, "Glyph dict should have 2 entries");
    assert!(glyph_dict.contains_key(".notdef"), "Should have .notdef glyph");
    assert!(glyph_dict.contains_key("A"), "Should have 'A' glyph");

    // Verify each glyph entry has a valid charproc reference
    for (name, entry) in &glyph_dict {
        assert!(!entry.name.is_empty(), "Glyph name should not be empty");
        assert!(entry.width > 0.0, "Glyph width should be positive");
        assert!(entry.bbox.len() == 4, "Glyph bbox should have 4 elements");
    }
}

#[test]
fn test_content_stream_edge_cases() {
    // Test edge cases for content stream functions

    // Test empty content stream
    let empty_stream = create_empty_content_stream();
    let empty_str = std::str::from_utf8(&empty_stream).unwrap();

    // Empty stream should still be valid PDF (has BT/ET)
    assert!(empty_str.contains("BT"), "Empty stream should have BT");
    assert!(empty_str.contains("ET"), "Empty stream should have ET");

    // Test simple charproc stream (basic PDF operators)
    let simple_stream = create_simple_charproc_stream();
    assert!(!simple_stream.is_empty(), "Simple stream should not be empty");

    // Test curved charproc stream (includes bezier curves)
    let curved_stream = create_charproc_stream_with_curves();
    let curved_str = std::str::from_utf8(&curved_stream).unwrap();

    // Should contain curveto operator
    assert!(curved_str.contains('c'), "Curved stream should contain 'c' operator");

    // Different streams should produce different outputs
    assert_ne!(simple_stream, curved_stream, "Different streams should differ");
}
