//! Example: Full PDF extraction to structured JSON.
//!
//! Demonstrates the `extract_pdf` function which returns the complete
//! DocumentJson including pages, spans, blocks, tables, signatures,
//! form fields, links, and attachments.
//!
//! Usage:
//!   cargo run --example extract -- tests/fixtures/sample.pdf

use anyhow::Result;
use pdftract_core::{extract_pdf, ExtractionOptions};
use std::env;
use std::path::Path;

fn main() -> Result<()> {
    // Get PDF path from command line, or use a default
    let args: Vec<String> = env::args().collect();
    let pdf_path = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("tests/fixtures/sample.pdf");

    // Extract with default options
    let options = ExtractionOptions::default();
    let result = extract_pdf(Path::new(pdf_path), &options)?;

    // Print summary
    println!("Fingerprint: {}", result.fingerprint);
    println!("Pages: {}", result.metadata.page_count);
    println!("Total spans: {}", result.metadata.span_count);
    println!("Total blocks: {}", result.metadata.block_count);

    // Print per-page summary
    for page in &result.pages {
        println!(
            "Page {}: {} spans, {} blocks, {} tables",
            page.page_number,
            page.spans.len(),
            page.blocks.len(),
            page.tables.len()
        );

        // Show first few spans
        for (i, span) in page.spans.iter().take(3).enumerate() {
            println!("  Span {}: \"{}\"", i, span.text);
        }
    }

    // Additional metadata
    if !result.signatures.is_empty() {
        println!("\nSignatures: {}", result.signatures.len());
    }
    if !result.form_fields.is_empty() {
        println!("Form fields: {}", result.form_fields.len());
    }
    if !result.links.is_empty() {
        println!("Links: {}", result.links.len());
    }
    if !result.attachments.is_empty() {
        println!("Attachments: {}", result.attachments.len());
    }

    Ok(())
}
