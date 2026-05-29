//! Example: Extract Markdown from a PDF.
//!
//! Demonstrates Markdown extraction using `page_to_markdown` to produce
//! GitHub Flavored Markdown with optional HTML comment anchors for
//! cite-back verification.
//!
//! Usage:
//!   cargo run --example extract_markdown -- tests/fixtures/sample.pdf

use anyhow::Result;
use pdftract_core::{extract_pdf, markdown::page_to_markdown, ExtractionOptions};
use std::env;
use std::path::Path;

fn main() -> Result<()> {
    // Get PDF path from command line, or use a default
    let args: Vec<String> = env::args().collect();
    let pdf_path = args.get(1).map(|s| s.as_str()).unwrap_or("tests/fixtures/sample.pdf");

    // Extract with default options
    let options = ExtractionOptions::default();
    let result = extract_pdf(Path::new(pdf_path), &options)?;

    for (i, page) in result.pages.iter().enumerate() {
        // Print page separator
        println!("## Page {}", page.page_number);
        println!();

        // Convert page to Markdown with anchors and page breaks
        let markdown = page_to_markdown(
            &page.blocks,
            &page.tables,
            i, // page_index
            true, // include_anchor
            true, // include_page_break
        );

        println!("{}", markdown);
        println!();
    }

    Ok(())
}
