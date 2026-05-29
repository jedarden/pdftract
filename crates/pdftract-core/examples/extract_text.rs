//! Example: Extract plain text from a PDF.
//!
//! Demonstrates text extraction using `extract_pdf` followed by
//! `serialize_page_text` to produce human-readable plain text output.
//!
//! Usage:
//!   cargo run --example extract_text -- tests/fixtures/sample.pdf

use anyhow::Result;
use pdftract_core::{extract_pdf, text::serialize_page_text, ExtractionOptions, TextOptions};
use std::env;
use std::path::Path;

fn main() -> Result<()> {
    // Get PDF path from command line, or use a default
    let args: Vec<String> = env::args().collect();
    let pdf_path = args.get(1).map(|s| s.as_str()).unwrap_or("tests/fixtures/sample.pdf");

    // Extract with default options
    let options = ExtractionOptions::default();
    let result = extract_pdf(Path::new(pdf_path), &options)?;

    // Convert to plain text
    let text_options = TextOptions::default();

    for page in &result.pages {
        // Print page separator
        println!("=== Page {} ===", page.page_number);

        // Serialize page text from blocks and spans
        let page_text = serialize_page_text(&page.blocks, &page.spans, &text_options);

        println!("{}", page_text);
        println!(); // Blank line between pages
    }

    Ok(())
}
