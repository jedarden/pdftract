//! Generate scanned PDF fixtures from ground truth text files.
//!
//! Run with: cargo run --bin gen_scanned_fixtures
//!
//! This creates proper 300 DPI PDFs for OCR testing with clean text rendering.

use anyhow::{Context, Result};
use printpdf::{BuiltinFont, Mm, PdfDocument, PdfLayerReference, PdfPageReference};
use std::fs;
use std::path::Path;

// Point to MM conversion: 1 point = 1/72 inch ≈ 0.3528 mm
fn pt_to_mm(pt: f32) -> Mm {
    Mm(pt as f64 * 25.4 / 72.0)
}

struct FixtureSpec {
    name: &'static str,
    dir: &'static str,
    font: BuiltinFont,
    font_size: u8,
    line_spacing: f32, // in points
    margins: Margins,
}

struct Margins {
    left: f32,   // points
    top: f32,    // points
    right: f32,  // points
    bottom: f32, // points
}

const FIXTURES: &[FixtureSpec] = &[
    FixtureSpec {
        name: "receipt-300dpi",
        dir: "tests/fixtures/scanned/receipt",
        font: BuiltinFont::HelveticaBold,
        font_size: 10,
        line_spacing: 14.0,
        margins: Margins { left: 36.0, top: 36.0, right: 36.0, bottom: 36.0 }, // 0.5" = 36pt
    },
    FixtureSpec {
        name: "invoice-300dpi",
        dir: "tests/fixtures/scanned/documents",
        font: BuiltinFont::Helvetica,
        font_size: 11,
        line_spacing: 16.0,
        margins: Margins { left: 54.0, top: 54.0, right: 54.0, bottom: 54.0 }, // 0.75" = 54pt
    },
    FixtureSpec {
        name: "form-300dpi",
        dir: "tests/fixtures/scanned/documents",
        font: BuiltinFont::Helvetica,
        font_size: 11,
        line_spacing: 18.0,
        margins: Margins { left: 54.0, top: 54.0, right: 54.0, bottom: 54.0 }, // 0.75" = 54pt
    },
    FixtureSpec {
        name: "doc-10page-300dpi",
        dir: "tests/fixtures/scanned/multi-page",
        font: BuiltinFont::TimesRoman,
        font_size: 12,
        line_spacing: 18.0,
        margins: Margins { left: 72.0, top: 54.0, right: 72.0, bottom: 54.0 }, // 1" left/right, 0.75" top/bottom
    },
];

fn main() -> Result<()> {
    println!("Generating scanned fixture PDFs...");
    println!("{}", "=".repeat(60));

    for fixture in FIXTURES {
        generate_fixture(fixture)?;
    }

    println!("{}", "=".repeat(60));
    println!("Done! All PDF fixtures generated.");

    Ok(())
}

fn generate_fixture(spec: &FixtureSpec) -> Result<()> {
    println!("Generating: {}", spec.name);

    let txt_path = Path::new(spec.dir).join(format!("{}.txt", spec.name));
    let pdf_path = Path::new(spec.dir).join(format!("{}.pdf", spec.name));

    // Read ground truth text
    let text = fs::read_to_string(&txt_path)
        .with_context(|| format!("Failed to read ground truth: {}", txt_path.display()))?;

    // Create PDF (Letter size: 8.5" x 11" = 215.9mm x 279.4mm)
    let (doc, page1, layer1) = PdfDocument::new(
        format!("{} - OCR Test Fixture", spec.name),
        Mm(216.0),
        Mm(279.0),
        "Layer 1",
    );

    // Add font
    let font = doc.add_builtin_font(spec.font);

    // Get the first page and layer
    let mut current_page = doc.get_page(page1);
    let mut current_layer = current_page.get_layer(layer1);

    // Page dimensions in points (1" = 72pt, Letter = 8.5" x 11" = 612pt x 792pt)
    let page_height_pt = 792.0;
    let mut y_pos_pt = page_height_pt - spec.margins.top;

    // Process text line by line
    let lines: Vec<&str> = text.lines().collect();

    for line in lines {
        // Check if we need a new page
        if y_pos_pt < spec.margins.bottom as f64 + spec.line_spacing as f64 {
            // Add new page
            let (new_page, new_layer) = doc.add_page(
                Mm(216.0),
                Mm(279.0),
                format!("Page {}", current_page.len() + 1),
            );
            current_page = doc.get_page(new_page);
            current_layer = current_page.get_layer(new_layer);
            y_pos_pt = page_height_pt - spec.margins.top;
        }

        // Draw the line
        current_layer.use_text(
            line,
            spec.font_size as f64,
            pt_to_mm(spec.margins.left),
            pt_to_mm(y_pos_pt as f32),
            &font,
        );

        y_pos_pt -= spec.line_spacing as f64;
    }

    // Save PDF using the correct API
    let pdf_bytes = doc.save_to(Vec::new())
        .with_context(|| "Failed to save PDF to memory")?;

    fs::write(&pdf_path, pdf_bytes)
        .with_context(|| format!("Failed to write PDF: {}", pdf_path.display()))?;

    println!("  Created: {}", pdf_path.display());
    println!("  Success: {}", spec.name);

    Ok(())
}
