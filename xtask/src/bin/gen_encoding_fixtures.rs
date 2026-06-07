//! Generate encoding test fixtures for Levels 2-4 Unicode recovery.
//!
//! Run with: cargo run --bin gen_encoding_fixtures
//!
//! This creates PDF fixtures that test the four-level Unicode recovery cascade:
//! - Level 1: ToUnicode CMap (confidence = 1.0)
//! - Level 2: Encoding vector + AGL (confidence = 0.9)
//! - Level 3: Font fingerprint cache (confidence = 0.85)
//! - Level 4: Glyph shape recognition (confidence = 0.7)
//!
//! These fixtures specifically test cases where ToUnicode is ABSENT,
//! forcing the extractor to use Levels 2-4 for Unicode recovery.

use anyhow::{Context, Result};
use lopdf::dictionary;
use lopdf::{Dictionary, Object, Document, ObjectId};
use std::fs::File;
use std::io::Write;

fn create_simple_page_with_font(
    content: &[u8],
    font_dict: Dictionary,
    doc: &mut Document,
) -> ObjectId {
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

/// Create a PDF using Standard 14 font (Helvetica) with NO ToUnicode CMap.
/// This exercises Level 2 (AGL) Unicode recovery.
///
/// Standard 14 fonts like Helvetica have well-known glyph names that map
/// cleanly through the Adobe Glyph List (AGL). Without ToUnicode, the
/// extractor must fall back to Level 2.
fn create_agl_only_pdf() -> Result<()> {
    let mut doc = Document::with_version("1.4");

    // Font dictionary: Helvetica is a Standard 14 font
    // No /ToUnicode, no /Encoding specified (uses built-in encoding)
    let font_dict = dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica"
    };

    // Content stream: "Hello World" using standard encoding
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(Hello World!) Tj\nET\n";
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
    let pdf_path = "tests/fixtures/encoding/agl-only.pdf";
    doc.save(pdf_path)
        .with_context(|| format!("Failed to create PDF: {}", pdf_path))?;
    println!("Created: {}", pdf_path);

    // Create ground truth .txt file
    let txt_path = "tests/fixtures/encoding/agl-only.txt";
    let mut txt_file = File::create(txt_path)
        .with_context(|| format!("Failed to create ground truth: {}", txt_path))?;
    writeln!(txt_file, "Hello World!")?;
    println!("Created: {}", txt_path);

    Ok(())
}

/// Create a PDF with a custom encoding dictionary using non-AGL glyph names.
/// This exercises Level 4 (shape recognition) since:
/// - No ToUnicode CMap
/// - Custom glyph names not in AGL
/// - No embedded font program (no Level 3 fingerprint possible)
///
/// Expected: U+FFFD replacement characters for unmapped glyphs.
fn create_no_mapping_pdf() -> Result<()> {
    let mut doc = Document::with_version("1.4");

    // Font dictionary with custom /Differences encoding
    // Using arbitrary glyph names that are NOT in AGL
    let font_dict = dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => dictionary! {
            "Type" => "Encoding",
            "Differences" => Object::Array(vec![
                Object::Integer(32),  // Start at space (code 32)
                // Non-AGL glyph names - these will NOT resolve through Level 2
                Object::Name(b"g001".to_vec()),  // code 32 -> "g001" (not in AGL)
                Object::Name(b"g002".to_vec()),  // code 33 -> "g002" (not in AGL)
                Object::Name(b"g003".to_vec()),  // code 34 -> "g003" (not in AGL)
                Object::Name(b"g004".to_vec()),  // code 35 -> "g004" (not in AGL)
                Object::Name(b"g005".to_vec()),  // code 36 -> "g005" (not in AGL)
                Object::Name(b"g006".to_vec()),  // code 37 -> "g006" (not in AGL)
                Object::Name(b"g007".to_vec()),  // code 38 -> "g007" (not in AGL)
                Object::Name(b"g008".to_vec()),  // code 39 -> "g008" (not in AGL)
                Object::Name(b"g009".to_vec()),  // code 40 -> "g009" (not in AGL)
                Object::Name(b"g010".to_vec()),  // code 41 -> "g010" (not in AGL)
            ])
        }
    };

    // Content stream: "Test FFFFD" where each letter uses the custom encoding
    // String content: codes 32-41 map to g001-g010
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(\x20\x21\x22\x23\x24) Tj\nET\n";
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
    let pdf_path = "tests/fixtures/encoding/no-mapping.pdf";
    doc.save(pdf_path)
        .with_context(|| format!("Failed to create PDF: {}", pdf_path))?;
    println!("Created: {}", pdf_path);

    // Create ground truth .txt file
    // Expected: All U+FFFD since no mapping exists
    let txt_path = "tests/fixtures/encoding/no-mapping.txt";
    let mut txt_file = File::create(txt_path)
        .with_context(|| format!("Failed to create ground truth: {}", txt_path))?;
    // Write 5 U+FFFD characters (one for each glyph code)
    for _ in 0..5 {
        write!(txt_file, "\u{FFFD}")?;
    }
    writeln!(txt_file)?;
    println!("Created: {} (expected: all U+FFFD)", txt_path);

    Ok(())
}

/// Create a PDF with a subsetted TrueType font that has no ToUnicode.
/// This exercises Level 3 (font fingerprint) if the embedded font
/// matches a known fingerprint in the database.
///
/// NOTE: This is a placeholder. A true Level 3 test requires:
/// - An embedded font program (/FontFile2, /FontFile3)
/// - A font that exists in the fingerprint database
/// - The database (build/font-fingerprints.json) must be populated
///
/// For now, this fixture documents the structure needed for Level 3 testing.
fn create_fingerprint_match_pdf_placeholder() -> Result<()> {
    let mut doc = Document::with_version("1.4");

    // Font dictionary with embedded TrueType program
    // In a real Level 3 test, this would include:
    // - /FontFile2 stream with actual font data
    // - A font that's in build/font-fingerprints.json
    //
    // For this placeholder, we use the structure without actual font data
    let font_dict = dictionary! {
        "Type" => "Font",
        "Subtype" => "TrueType",
        "BaseFont" => "SubsetFont+ABCDE",
        "FirstChar" => 32,
        "LastChar" => 126,
        "Widths" => Object::Array(vec![
            Object::Integer(500); 95  // Placeholder widths for codes 32-126
        ]),
        "FontDescriptor" => dictionary! {
            "Type" => "FontDescriptor",
            "FontName" => "SubsetFont+ABCDE",
            "Flags" => Object::Integer(32),  // Symbolic font
            "FontBBox" => Object::Array(vec![
                Object::Integer(0), Object::Integer(0),
                Object::Integer(1000), Object::Integer(1000)
            ]),
            "ItalicAngle" => Object::Integer(0),
            "Ascent" => Object::Integer(1000),
            "Descent" => Object::Integer(-200),
            "CapHeight" => Object::Integer(700),
            "StemV" => Object::Integer(80),
            // FontFile2 would go here with actual font data:
            // "FontFile2" => Object::Reference(font_stream_id)
        }
    };

    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(Placeholder for Level 3 fingerprint test) Tj\nET\n";
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
    let pdf_path = "tests/fixtures/encoding/fingerprint-match.pdf";
    doc.save(pdf_path)
        .with_context(|| format!("Failed to create PDF: {}", pdf_path))?;
    println!("Created: {} (placeholder - needs embedded font for Level 3)", pdf_path);

    // Create placeholder ground truth
    let txt_path = "tests/fixtures/encoding/fingerprint-match.txt";
    let mut txt_file = File::create(txt_path)
        .with_context(|| format!("Failed to create ground truth: {}", txt_path))?;
    writeln!(txt_file, "Placeholder for Level 3 fingerprint test")?;
    writeln!(txt_file, "# NOTE: This fixture requires build/font-fingerprints.json")?;
    writeln!(txt_file, "# and an embedded font matching a database entry.")?;
    println!("Created: {} (placeholder)", txt_path);

    Ok(())
}

/// Create a PDF with Type 3 font glyphs that have no ToUnicode.
/// This exercises Level 4 (shape recognition) for Type 3 fonts.
///
/// Type 3 fonts define glyphs as PDF content streams in /CharProcs.
/// Without ToUnicode, the extractor must rasterize each glyph and
/// match against the shape database.
fn create_type3_shape_match_pdf() -> Result<()> {
    let mut doc = Document::with_version("1.4");

    // Type 3 font dictionary
    let font_dict = dictionary! {
        "Type" => "Font",
        "Subtype" => "Type3",
        "FontBBox" => Object::Array(vec![
            Object::Integer(0), Object::Integer(0),
            Object::Integer(1000), Object::Integer(1000)
        ]),
        "FontMatrix" => Object::Array(vec![
            Object::Real(0.001), Object::Real(0.0),
            Object::Real(0.0), Object::Real(0.001),
            Object::Real(0.0), Object::Real(0.0)
        ]),
        "Encoding" => dictionary! {
            "Type" => "Encoding",
            "Differences" => Object::Array(vec![
                Object::Integer(32),  // Start at code 32
                Object::Name(b"custom_glyph_a".to_vec()),  // Non-AGL name
                Object::Name(b"custom_glyph_b".to_vec()),  // Non-AGL name
                Object::Name(b"custom_glyph_c".to_vec()),  // Non-AGL name
            ])
        },
        "CharProcs" => dictionary! {
            "custom_glyph_a" => Object::Reference(1),  // Placeholder for glyph stream
            "custom_glyph_b" => Object::Reference(2),  // Placeholder for glyph stream
            "custom_glyph_c" => Object::Reference(3),  // Placeholder for glyph stream
        }
    };

    // Create simple glyph content streams (draw basic shapes)
    // These are placeholders - real Type 3 glyphs would draw actual letter shapes
    let glyph_streams = vec![
        b"q 0 0 500 500 re f Q",  // Rectangle (placeholder for "A")
        b"q 0 0 500 500 re f Q",  // Rectangle (placeholder for "B")
        b"q 0 0 500 500 re f Q",  // Rectangle (placeholder for "C")
    ];

    let mut char_proc_refs = vec![];
    for (i, glyph_content) in glyph_streams.iter().enumerate() {
        let stream_id = doc.new_object_id();
        doc.objects.insert(stream_id, Object::Stream(lopdf::Stream::new(
            dictionary! {},
            glyph_content.to_vec()
        )));
        char_proc_refs.push(Object::Reference(stream_id));
    }

    // Update font dict with actual stream references
    let font_dict = dictionary! {
        "Type" => "Font",
        "Subtype" => "Type3",
        "FontBBox" => Object::Array(vec![
            Object::Integer(0), Object::Integer(0),
            Object::Integer(1000), Object::Integer(1000)
        ]),
        "FontMatrix" => Object::Array(vec![
            Object::Real(0.001), Object::Real(0.0),
            Object::Real(0.0), Object::Real(0.001),
            Object::Real(0.0), Object::Real(0.0)
        ]),
        "Encoding" => dictionary! {
            "Type" => "Encoding",
            "Differences" => Object::Array(vec![
                Object::Integer(32),
                Object::Name(b"custom_glyph_a".to_vec()),
                Object::Name(b"custom_glyph_b".to_vec()),
                Object::Name(b"custom_glyph_c".to_vec()),
            ])
        },
        "CharProcs" => dictionary! {
            "custom_glyph_a" => char_proc_refs[0].clone(),
            "custom_glyph_b" => char_proc_refs[1].clone(),
            "custom_glyph_c" => char_proc_refs[2].clone(),
        }
    };

    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(\x20\x21\x22) Tj\nET\n";  // Codes 32, 33, 34
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
    let pdf_path = "tests/fixtures/encoding/type3-shape-match.pdf";
    doc.save(pdf_path)
        .with_context(|| format!("Failed to create PDF: {}", pdf_path))?;
    println!("Created: {} (Type3 with placeholder glyphs)", pdf_path);

    // Create placeholder ground truth
    let txt_path = "tests/fixtures/encoding/type3-shape-match.txt";
    let mut txt_file = File::create(txt_path)
        .with_context(|| format!("Failed to create ground truth: {}", txt_path))?;
    writeln!(txt_file, "# Type3 font shape match test")?;
    writeln!(txt_file, "# Glyphs are placeholder rectangles - not shape-matchable")?;
    writeln!(txt_file, "# Real Level 4 test requires actual letter shapes in CharProcs")?;
    println!("Created: {} (placeholder)", txt_path);

    Ok(())
}

/// Create a comprehensive test PDF with mixed encoding scenarios.
/// This PDF contains multiple text runs, each testing different aspects
/// of the Unicode recovery cascade.
fn create_mixed_scenarios_pdf() -> Result<()> {
    let mut doc = Document::with_version("1.4");

    // Scenario 1: Standard 14 font (Helvetica) - exercises Level 2 AGL
    let font1 = dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica"
    };

    let mut page_dict = Dictionary::new();
    page_dict.set("Type", "Page");
    page_dict.set("MediaBox", Object::Array(vec![
        Object::Real(0.0), Object::Real(0.0),
        Object::Real(612.0), Object::Real(792.0)
    ]));

    // Multiple fonts for different scenarios
    page_dict.set("Resources", dictionary! {
        "Font" => dictionary! {
            "F1" => font1,  // Helvetica - Level 2 AGL
            "F2" => dictionary! {  // Custom encoding - Level 4
                "Type" => "Font",
                "Subtype" => "Type1",
                "BaseFont" => "Helvetica",
                "Encoding" => dictionary! {
                    "Type" => "Encoding",
                    "Differences" => Object::Array(vec![
                        Object::Integer(65),  // Start at 'A'
                        Object::Name(b"glyph01".to_vec()),  // Not in AGL
                        Object::Name(b"glyph02".to_vec()),  // Not in AGL
                        Object::Name(b"glyph03".to_vec()),  // Not in AGL
                    ])
                }
            }
        }
    });

    // Content stream with multiple test scenarios
    let content = b"BT
/F1 12 Tf 100 700 Td (AGL Test: Hello World!) Tj
/F1 12 Tf 100 680 Td (Numbers: 0123456789) Tj
/F1 12 Tf 100 660 Td (Punctuation: .,!?;:) Tj
/F2 12 Tf 100 640 Td (NoMappingTest) Tj
ET";

    let content_stream_id = doc.new_object_id();
    doc.objects.insert(content_stream_id, Object::Stream(lopdf::Stream::new(
        dictionary! {},
        content.to_vec()
    )));
    page_dict.set("Contents", Object::Reference(content_stream_id));

    let page_id = doc.add_object(page_dict);

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
    let pdf_path = "tests/fixtures/encoding/mixed-scenarios.pdf";
    doc.save(pdf_path)
        .with_context(|| format!("Failed to create PDF: {}", pdf_path))?;
    println!("Created: {}", pdf_path);

    // Create ground truth .txt file
    let txt_path = "tests/fixtures/encoding/mixed-scenarios.txt";
    let mut txt_file = File::create(txt_path)
        .with_context(|| format!("Failed to create ground truth: {}", txt_path))?;
    writeln!(txt_file, "AGL Test: Hello World!")?;
    writeln!(txt_file, "Numbers: 0123456789")?;
    writeln!(txt_file, "Punctuation: .,!?;:")?;
    // The "NoMappingTest" uses non-AGL glyph names - expect U+FFFD
    for _ in 0..13 {
        write!(txt_file, "\u{FFFD}")?;
    }
    writeln!(txt_file)?;
    println!("Created: {}", txt_path);

    Ok(())
}

fn main() -> Result<()> {
    println!("Generating encoding test fixtures for Unicode recovery (Levels 2-4)...");
    println!("{}", "=".repeat(70));

    // Ensure output directory exists
    std::fs::create_dir_all("tests/fixtures/encoding")
        .context("Failed to create fixtures directory")?;

    println!("\n[1/5] Creating agl-only.pdf (Level 2: AGL lookup)...");
    create_agl_only_pdf()?;

    println!("\n[2/5] Creating no-mapping.pdf (Level 4 failure: no valid mapping)...");
    create_no_mapping_pdf()?;

    println!("\n[3/5] Creating fingerprint-match.pdf (Level 3: font fingerprint)...");
    create_fingerprint_match_pdf_placeholder()?;

    println!("\n[4/5] Creating type3-shape-match.pdf (Level 4: Type3 shape recognition)...");
    create_type3_shape_match_pdf()?;

    println!("\n[5/5] Creating mixed-scenarios.pdf (comprehensive test)...");
    create_mixed_scenarios_pdf()?;

    println!("\n{}", "=".repeat(70));
    println!("All encoding fixtures generated successfully!");
    println!("\nFixtures created:");
    println!("  tests/fixtures/encoding/agl-only.pdf");
    println!("  tests/fixtures/encoding/no-mapping.pdf");
    println!("  tests/fixtures/encoding/fingerprint-match.pdf (placeholder)");
    println!("  tests/fixtures/encoding/type3-shape-match.pdf (placeholder)");
    println!("  tests/fixtures/encoding/mixed-scenarios.pdf");
    println!("\nNOTE: fingerprint-match.pdf and type3-shape-match.pdf are placeholders.");
    println!("      Full Level 3/4 support requires embedded fonts and shape database.");

    Ok(())
}
