//! Generate unmapped glyph test fixture for comprehensive Unicode recovery testing.
//!
//! Run with: cargo run --bin gen_unmapped_comprehensive
//!
//! This creates the unmapped-comprehensive.pdf fixture as specified in
//! notes/bf-68f9i-design.md, testing all 4 failure modes of the Unicode
//! recovery cascade plus success path verification.

use anyhow::{Context, Result};
use lopdf::dictionary;
use lopdf::{Dictionary, Object, Document};
use std::fs::File;
use std::io::Write;

fn create_simple_page_with_font(
    content: &[u8],
    font_dict: Dictionary,
    doc: &mut Document,
) -> lopdf::ObjectId {
    let mut page_dict = Dictionary::new();
    page_dict.set("Type", "Page");
    page_dict.set("MediaBox", Object::Array(vec![
        Object::Real(0.0), Object::Real(0.0),
        Object::Real(612.0), Object::Real(792.0)
    ]));
    page_dict.set("Resources", dictionary! {
        "Font" => dictionary! {
            "F1" => font_dict
        }
    });

    let content_stream_id = doc.new_object_id();
    doc.objects.insert(content_stream_id, Object::Stream(lopdf::Stream::new(
        dictionary! {},
        content.to_vec()
    )));
    page_dict.set("Contents", Object::Reference(content_stream_id));

    doc.add_object(page_dict)
}

/// Create the unmapped-comprehensive.pdf fixture as specified in notes/bf-68f9i-design.md.
///
/// This fixture includes:
/// - 6 unmapped glyphs (testing failure path): g001, g002, g003, CustomA, CustomB, NotAGlyph, glyph_0041
/// - 3 mapped glyphs (testing success path): A, B, space
/// - Single-page PDF with simple text layout
/// - Type1 font with custom encoding (no embedding)
fn create_unmapped_comprehensive_pdf() -> Result<()> {
    let mut doc = Document::with_version("1.4");

    // Font dictionary with custom /Differences encoding
    // Codes 0-9 map to specific glyph names per the design spec
    let font_dict = dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "UnmappedTestFont",
        "Encoding" => dictionary! {
            "Type" => "Encoding",
            "Differences" => Object::Array(vec![
                Object::Integer(0),  // Start at code 0
                // Unmapped glyphs (7 total) - these will NOT resolve through Level 2 AGL
                Object::Name(b"g001".to_vec()),       // code 0  -> PUA glyph
                Object::Name(b"g002".to_vec()),       // code 1  -> PUA glyph
                Object::Name(b"g003".to_vec()),       // code 2  -> PUA glyph
                Object::Name(b"CustomA".to_vec()),    // code 3  -> custom encoding
                Object::Name(b"CustomB".to_vec()),    // code 4  -> custom encoding
                Object::Name(b"NotAGlyph".to_vec()),   // code 5  -> orphaned glyph
                Object::Name(b"glyph_0041".to_vec()),  // code 6  -> non-AGL algorithmic
                // Mapped glyphs (3 total) - standard AGL entries
                Object::Name(b"A".to_vec()),           // code 7  -> standard AGL
                Object::Name(b"B".to_vec()),           // code 8  -> standard AGL
                Object::Name(b"space".to_vec()),       // code 9  -> standard AGL
            ])
        }
    };

    // Content stream with 3 lines as per design spec
    // Line 1 (y=700): Three PUA glyphs (codes 0, 1, 2)
    // Line 2 (y=680): Custom and orphaned glyphs (codes 3, 4, 5, 6)
    // Line 3 (y=660): Mapped AGL glyphs (codes 7, 8, 9)
    let content = b"BT
/F1 12 Tf
50 700 Td
<000102> Tj
50 680 Td
<03040506> Tj
50 660 Td
<070809> Tj
ET";

    let page_id = create_simple_page_with_font(content, font_dict, &mut doc);

    // Create pages dict
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", "Pages");
    pages_dict.set("Count", Object::Integer(1));
    pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    let pages_id = doc.add_object(pages_dict);

    // Update page parent
    if let Some(Object::Dictionary(ref mut page_dict)) = doc.objects.get_mut(&page_id) {
        page_dict.set("Parent", Object::Reference(pages_id));
    }

    // Create catalog
    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", "Catalog");
    catalog_dict.set("Pages", Object::Reference(pages_id));

    let catalog_id = doc.add_object(catalog_dict);
    doc.trailer.set("Root", Object::Reference(catalog_id));

    // Save PDF
    let pdf_path = "tests/fixtures/encoding/unmapped-comprehensive.pdf";
    doc.save(pdf_path)
        .with_context(|| format!("Failed to create PDF: {}", pdf_path))?;
    println!("Created: {}", pdf_path);

    // Create ground truth .txt file
    // Expected: 7 × U+FFFD + "AB " (for mapped glyphs A, B, space)
    let txt_path = "tests/fixtures/encoding/unmapped-comprehensive.txt";
    let mut txt_file = File::create(txt_path)
        .with_context(|| format!("Failed to create ground truth: {}", txt_path))?;

    // Line 1: 3 U+FFFD for g001, g002, g003
    for _ in 0..3 {
        write!(txt_file, "\u{FFFD}")?;
    }
    writeln!(txt_file)?;

    // Line 2: 4 U+FFFD for CustomA, CustomB, NotAGlyph, glyph_0041
    for _ in 0..4 {
        write!(txt_file, "\u{FFFD}")?;
    }
    writeln!(txt_file)?;

    // Line 3: "AB " for A, B, space
    writeln!(txt_file, "AB ")?;

    println!("Created: {} (7 × U+FFFD + \"AB \")", txt_path);

    Ok(())
}

fn main() -> Result<()> {
    println!("Generating unmapped glyph test fixture...");
    println!("{}", "=".repeat(70));

    // Ensure output directory exists
    std::fs::create_dir_all("tests/fixtures/encoding")
        .context("Failed to create fixtures directory")?;

    println!("\n[1/1] Creating unmapped-comprehensive.pdf...");
    println!("- 6 unmapped glyphs: g001, g002, g003, CustomA, CustomB, NotAGlyph, glyph_0041");
    println!("- 3 mapped glyphs: A, B, space");
    println!("- 3-line layout with custom Type1 encoding");
    create_unmapped_comprehensive_pdf()?;

    println!("\n{}", "=".repeat(70));
    println!("Unmapped glyph fixture generated successfully!");
    println!("\nFixtures created:");
    println!("  tests/fixtures/encoding/unmapped-comprehensive.pdf");
    println!("  tests/fixtures/encoding/unmapped-comprehensive.txt");

    Ok(())
}
