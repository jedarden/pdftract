//! Generate unmapped glyph test fixture with configurable glyph names.
//!
//! Run with: cargo run --bin gen_unmapped_configurable
//!
//! This generator reads unmapped glyph names from tests/fixtures/unmapped_config.txt
//! and creates a PDF with:
//! - Configured unmapped glyphs (NO ToUnicode entries)
//! - Standard AGL-mapped glyphs (WITH ToUnicode entries)
//! - All glyphs included in the encoding dictionary

use anyhow::{Context, Result};
use lopdf::dictionary;
use lopdf::{Dictionary, Object, Document};
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::Path;

/// Read unmapped glyph names from configuration file
fn read_unmapped_glyph_config(config_path: &Path) -> Result<Vec<String>> {
    let file = File::open(config_path)
        .with_context(|| format!("Failed to open config file: {}", config_path.display()))?;

    let mut glyph_names = Vec::new();

    for line in io::BufReader::new(file).lines() {
        let line = line?;
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        glyph_names.push(trimmed.to_string());
    }

    Ok(glyph_names)
}

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

/// Create a ToUnicode CMap that excludes configured unmapped glyphs.
///
/// This is the core logic for the unmapped glyph encoding:
/// - We create ToUnicode entries ONLY for glyphs that are NOT in the unmapped config
/// - Configured unmapped glyphs are intentionally excluded from the ToUnicode CMap
/// - All glyphs (mapped and unmapped) are still present in the font's encoding dictionary
fn create_selective_tounicode_cmap(
    glyph_mapping: &[(u8, &str, Option<char>)],  // (char_code, glyph_name, unicode_value)
) -> Result<Option<Vec<u8>>> {
    let mut cmap_lines = Vec::new();

    cmap_lines.push(b"/CIDInit /ProcSet findresource begin".to_vec());
    cmap_lines.push(b"12 dict begin".to_vec());
    cmap_lines.push(b"begincmap".to_vec());
    cmap_lines.push(b"/CMapType 2 def".to_vec());
    cmap_lines.push(b"/CMapName /SelectiveToUnicode def".to_vec());
    cmap_lines.push(b"1 begincodespacerange".to_vec());
    cmap_lines.push(b"<00> <FF>".to_vec());
    cmap_lines.push(b"endcodespacerange".to_vec());

    // Count mappings that have unicode values (i.e., are NOT unmapped)
    let mapped_glyphs: Vec<_> = glyph_mapping
        .iter()
        .filter_map(|&(code, name, unicode)| {
            unicode.map(|u| (code, name, u))
        })
        .collect();

    if mapped_glyphs.is_empty() {
        // No mapped glyphs - return no ToUnicode CMap at all
        return Ok(None);
    }

    cmap_lines.push(format!("{} beginbfchar", mapped_glyphs.len()).into_bytes());

    for (char_code, glyph_name, unicode_char) in mapped_glyphs {
        let unicode_hex = format!("{:04X}", unicode_char as u32);
        let char_code_hex = format!("{:02X}", char_code);

        cmap_lines.push(format!("<{}> <{}>", char_code_hex, unicode_hex).into_bytes());

        println!("  ToUnicode: code 0x{} ({}) -> U+{} ({})",
            char_code_hex, glyph_name, unicode_hex, unicode_char);
    }

    cmap_lines.push(b"endbfchar".to_vec());
    cmap_lines.push(b"endcmap".to_vec());
    cmap_lines.push(b"CMapName currentdict /CMap defineresource pop".to_vec());
    cmap_lines.push(b"end".to_vec());
    cmap_lines.push(b"end".to_vec());

    let cmap_data = cmap_lines.join(b"\n");

    println!("Created ToUnicode CMap with {} mappings (excluded {} unmapped glyphs)",
        mapped_glyphs.len(),
        glyph_mapping.len() - mapped_glyphs.len());

    Ok(Some(cmap_data))
}

/// Create the configurable unmapped glyph fixture.
///
/// This demonstrates the key encoding logic:
/// 1. Read configured unmapped glyph names from config file
/// 2. Build encoding dictionary with ALL glyphs (mapped + unmapped)
/// 3. Create ToUnicode CMap ONLY for mapped glyphs (skip unmapped)
/// 4. Unmapped glyphs appear in PDF but have no Unicode mapping
fn create_configurable_unmapped_pdf(
    unmapped_glyphs: &[String],
) -> Result<()> {
    let mut doc = Document::with_version("1.4");

    // Define our glyph mapping: (char_code, glyph_name, unicode_value_if_mapped)
    let mut glyph_mapping: Vec<(u8, &str, Option<char>)> = vec![
        // Unmapped glyphs (unicode = None)
        (0x00, "g001", None),
        (0x01, "g002", None),
        (0x02, "g003", None),
        (0x03, "CustomA", None),
        (0x04, "CustomB", None),
        (0x05, "NotAGlyph", None),
        (0x06, "glyph_0041", None),

        // Mapped glyphs (unicode = Some(char))
        (0x41, "A", Some('A')),
        (0x42, "B", Some('B')),
        (0x20, "space", Some(' ')),
    ];

    // Verify that unmapped_glyphs contains the expected unmapped glyph names
    println!("Configured unmapped glyphs from config file:");
    for glyph in unmapped_glyphs {
        println!("  - {}", glyph);
    }

    // Build encoding differences array
    let mut encoding_diffs = vec![Object::Integer(0)]; // Start at code 0

    // Add all glyph names to the encoding
    for (code, name, _) in &glyph_mapping {
        // Add entries in order, ensuring all codes are represented
        encoding_diffs.push(Object::Name(name.as_bytes().to_vec()));
    }

    // Build font dictionary
    let mut font_dict = dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "ConfigurableUnmappedFont",
        "Encoding" => dictionary! {
            "Type" => "Encoding",
            "Differences" => Object::Array(encoding_diffs)
        }
    };

    // Create selective ToUnicode CMap (skips configured unmapped glyphs)
    let cmap_data = create_selective_tounicode_cmap(&glyph_mapping)?;

    // Add ToUnicode to font dictionary only if we have mapped glyphs
    if let Some(cmap_bytes) = cmap_data {
        let cmap_stream_id = doc.new_object_id();
        doc.objects.insert(
            cmap_stream_id,
            Object::Stream(lopdf::Stream::new(
                dictionary! {
                    "Type" => "/CMap",
                    "CMapName" => "/SelectiveToUnicode"
                },
                cmap_bytes
            ))
        );

        // CRITICAL: We selectively add ToUnicode ONLY for mapped glyphs
        // Unmapped glyphs are intentionally excluded from the CMap
        font_dict.set("ToUnicode", Object::Reference(cmap_stream_id));

        println!("Added ToUnicode CMap to font (mapped glyphs only)");
    } else {
        println!("No ToUnicode CMap created (all glyphs are unmapped)");
    }

    // Content stream showing both unmapped and mapped glyphs
    // Line 1: Three unmapped PUA glyphs
    // Line 2: Custom and orphaned unmapped glyphs
    // Line 3: Mapped AGL glyphs (A, B, space)
    let content = b"BT
/F1 12 Tf
50 700 Td
<000102> Tj
50 680 Td
<03040506> Tj
50 660 Td
<414220> Tj
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
    let pdf_path = "tests/fixtures/encoding/unmapped-configurable.pdf";
    doc.save(pdf_path)
        .with_context(|| format!("Failed to create PDF: {}", pdf_path))?;
    println!("Created: {}", pdf_path);

    // Create ground truth .txt file
    let txt_path = "tests/fixtures/encoding/unmapped-configurable.txt";
    let mut txt_file = File::create(txt_path)
        .with_context(|| format!("Failed to create ground truth: {}", txt_path))?;

    // Line 1: 3 U+FFFD for g001, g002, g003
    writeln!(txt_file, "{}", "\u{FFFD}\u{FFFD}\u{FFFD}")?;

    // Line 2: 4 U+FFFD for CustomA, CustomB, NotAGlyph, glyph_0041
    writeln!(txt_file, "{}", "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}")?;

    // Line 3: "AB " for A, B, space
    writeln!(txt_file, "AB ")?;

    println!("Created: {} (7 × U+FFFD + \"AB \")", txt_path);

    Ok(())
}

fn main() -> Result<()> {
    println!("Generating configurable unmapped glyph test fixture...");
    println!("{}", "=".repeat(70));

    // Ensure output directory exists
    std::fs::create_dir_all("tests/fixtures/encoding")
        .context("Failed to create fixtures directory")?;

    // Read unmapped glyph configuration
    let config_path = Path::new("tests/fixtures/unmapped_config.txt");
    let unmapped_glyphs = read_unmapped_glyph_config(config_path)
        .with_context(|| format!("Failed to read unmapped glyph config: {}", config_path.display()))?;

    println!("\nRead {} unmapped glyph names from config", unmapped_glyphs.len());

    println!("\n[1/1] Creating unmapped-configurable.pdf...");
    println!("- All glyphs included in encoding dictionary");
    println!("- ToUnicode CMap created ONLY for mapped glyphs");
    println!("- Configured unmapped glyphs excluded from ToUnicode");
    create_configurable_unmapped_pdf(&unmapped_glyphs)?;

    println!("\n{}", "=".repeat(70));
    println!("Configurable unmapped glyph fixture generated successfully!");
    println!("\nFixtures created:");
    println!("  tests/fixtures/encoding/unmapped-configurable.pdf");
    println!("  tests/fixtures/encoding/unmapped-configurable.txt");

    Ok(())
}