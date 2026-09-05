//! Structural tests for the no-mapping.pdf unmapped-glyph fixture.
//!
//! Verifies that `tools/generate_encoding_fixtures.py` emits the fixture described by
//! notes/bf-68f9i-design.md: seven glyph names that must survive all four levels of
//! the Unicode fallback chain (no ToUnicode, absent from the AGL, non-algorithmic,
//! no embedded font to fingerprint) plus three standard AGL names as a control group.
//!
//! These checks inspect the fixture bytes and the AGL resolver directly, so they stay
//! valid independently of whether text extraction currently succeeds end to end.

use pdftract_core::font::agl::unicode_for_glyph_name;

/// The fixture's character-code table, in /Differences order.
///
/// Mirrors `NO_MAPPING_GLYPHS` in tools/generate_encoding_fixtures.py; the two must
/// stay in agreement because the fixture is generated from that table.
const UNMAPPED_GLYPHS: &[(&str, u32)] = &[
    ("g001", 0xFFFD),
    ("g002", 0xFFFD),
    ("g003", 0xFFFD),
    ("CustomA", 0xFFFD),
    ("CustomB", 0xFFFD),
    ("NotAGlyph", 0xFFFD),
    ("glyph_0041", 0xFFFD),
];

const MAPPED_GLYPHS: &[(&str, u32)] = &[("A", 0x41), ("B", 0x42), ("space", 0x20)];

fn fixture_path() -> String {
    format!(
        "{}/../../tests/fixtures/encoding/no-mapping.pdf",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn fixture_bytes() -> Vec<u8> {
    std::fs::read(fixture_path()).expect("failed to read no-mapping.pdf fixture")
}

#[test]
fn fixture_has_all_glyph_names_in_differences_array() {
    let pdf = fixture_bytes();
    let text = String::from_utf8_lossy(&pdf);

    // The /Differences array must carry the base code followed by every glyph name,
    // in table order, so code N resolves to the Nth name.
    let expected = "/Differences [0 /g001 /g002 /g003 /CustomA /CustomB /NotAGlyph \
                    /glyph_0041 /A /B /space]";
    assert!(
        text.contains(expected),
        "Fixture /Differences array does not match the design table. \
         Expected to contain: {expected}. \
         Why this matters: these glyph names are the fixture's whole purpose — without \
         them the no-mapping fixture exercises nothing and the GLYPH_UNMAPPED path is \
         never reached."
    );
}

#[test]
fn fixture_declares_no_tounicode_cmap() {
    let pdf = fixture_bytes();
    let text = String::from_utf8_lossy(&pdf);

    assert!(
        !text.contains("/ToUnicode"),
        "Fixture declares a /ToUnicode CMap. \
         Why this matters: Level 1 recovery reads /ToUnicode before the encoding \
         dictionary, so its presence would give every unmapped glyph a Unicode value \
         and the fixture would no longer test the fallback chain at all."
    );
}

#[test]
fn fixture_content_stream_shows_all_ten_character_codes() {
    let pdf = fixture_bytes();
    let text = String::from_utf8_lossy(&pdf);

    // Character codes 0-9 as hex string show operators, one per design-doc line.
    for hex in ["<000102>", "<03040506>", "<070809>"] {
        assert!(
            text.contains(&format!("{hex} Tj")),
            "Content stream is missing show operator `{hex} Tj`. \
             Why this matters: the fixture must actually place codes 0-9 on the page; \
             a missing operator means those glyph names are never selected and the \
             fixture under-covers the unmapped set."
        );
    }
}

#[test]
fn fixture_font_is_type1_and_not_embedded() {
    let pdf = fixture_bytes();
    let text = String::from_utf8_lossy(&pdf);

    assert!(
        text.contains("/Subtype /Type1"),
        "Fixture font is not /Type1. \
         Why this matters: the design doc specifies Type1 precisely so the font is not \
         embedded; an embedded font would open the Level 3 fingerprint path."
    );
    assert!(
        !text.contains("/FontDescriptor"),
        "Fixture font carries a /FontDescriptor. \
         Why this matters: a descriptor implies an embedded font program, which would \
         give Level 3 a hash to fingerprint and let it recover the unmapped glyphs."
    );
}

#[test]
fn unmapped_glyph_names_have_no_unicode_representation() {
    for (name, _) in UNMAPPED_GLYPHS {
        assert_eq!(
            unicode_for_glyph_name(name),
            None,
            "Glyph /{name} resolved to a Unicode value but is specified as unmapped. \
             Why this matters: the fixture's unmapped names must be absent from the \
             Adobe Glyph List and match no algorithmic convention, otherwise Level 2 \
             would recover them and the GLYPH_UNMAPPED diagnostic would not fire."
        );
    }
}

#[test]
fn mapped_control_glyphs_resolve_through_agl() {
    for (name, codepoint) in MAPPED_GLYPHS {
        let resolved = unicode_for_glyph_name(name).map(|c| c as u32);
        assert_eq!(
            resolved,
            Some(*codepoint),
            "Control glyph /{name} did not resolve through the AGL to U+{codepoint:04X}. \
             Why this matters: the control group proves the fixture distinguishes \
             recoverable from unrecoverable names; if these stop resolving, the fixture \
             can no longer tell a working resolver from a broken one."
        );
    }
}

#[test]
fn non_agl_algorithmic_prefix_is_rejected() {
    // `glyph_0041` looks algorithmic but uses a prefix outside the two conventions the
    // parser accepts (uniXXXX / uXXXXXX), so it must not resolve.
    assert_eq!(
        unicode_for_glyph_name("glyph_0041"),
        None,
        "Glyph /glyph_0041 resolved via the algorithmic path. \
         Why this matters: only `uni` and `u` prefixes are algorithmic; accepting \
         arbitrary prefixes would let non-AGL names silently acquire Unicode values \
         and mask genuinely unmapped glyphs."
    );
}
