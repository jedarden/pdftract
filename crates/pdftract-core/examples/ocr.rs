//! Example: OCR-enabled extraction for scanned PDFs.
//!
//! Demonstrates text extraction with OCR fallback for scanned documents
//! where no vector text is available.
//!
//! Requires the "ocr" feature to be enabled (and Tesseract installed).
//!
//! Usage:
//!   cargo run --example ocr --features ocr -- tests/sdk-conformance/fixtures/misc/01.pdf

use anyhow::Result;
use pdftract_core::{extract_pdf, ExtractionOptions};
use std::env;
use std::path::Path;

fn main() -> Result<()> {
    // This example requires the OCR feature
    #[cfg(not(feature = "ocr"))]
    {
        eprintln!("Error: This example requires the 'ocr' feature.");
        eprintln!("Run with: cargo run --example ocr --features ocr -- <pdf-path>");
        eprintln!();
        eprintln!("The OCR feature also requires Tesseract to be installed on your system.");
        eprintln!("See: https://github.com/tesseract-ocr/tesseract");
        std::process::exit(1);
    }

    #[cfg(feature = "ocr")]
    {
        // Get PDF path from command line, or use a default
        let args: Vec<String> = env::args().collect();
        let pdf_path = args
            .get(1)
            .map(|s| s.as_str())
            .unwrap_or("tests/sdk-conformance/fixtures/misc/01.pdf");

        // Extract with OCR enabled
        let options = ExtractionOptions {
            ocr_language: vec!["eng".to_string()],
            ..Default::default()
        };

        let result = extract_pdf(Path::new(pdf_path), &options)?;

        // Print extraction results
        println!("Extracted {} pages", result.pages.len());

        for (i, page) in result.pages.iter().enumerate() {
            println!("=== Page {} ===", i + 1);
            println!(
                "  Dimensions: {} x {}",
                page.width.unwrap_or(0.0),
                page.height.unwrap_or(0.0)
            );
            println!("  Spans: {}", page.spans.len());
            println!("  Blocks: {}", page.blocks.len());

            // Show a preview of extracted text
            let preview: String = page
                .spans
                .iter()
                .map(|s| s.text.clone())
                .collect::<Vec<_>>()
                .join(" ");

            let preview_preview = if preview.len() > 200 {
                format!("{}...", &preview[..200])
            } else {
                preview
            };

            println!("  Text preview: {}", preview_preview);
            println!();
        }

        Ok(())
    }
}
