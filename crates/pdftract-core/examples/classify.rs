//! Example: Classify PDF document type.
//!
//! Demonstrates page-level classification to determine the extraction
//! path (Vector, Scanned, Hybrid, or BrokenVector). This is useful for
//! deciding whether OCR is needed and understanding the document's structure.
//!
//! Note: Document-type classification (invoice, receipt, etc.) requires the
//! `profiles` feature. This example shows page-level classification which
//! is always available.
//!
//! Usage:
//!   cargo run --example classify -- tests/fixtures/sample.pdf

use anyhow::Result;
use pdftract_core::{extract_pdf, ExtractionOptions};
use std::env;
use std::path::Path;
use std::collections::HashMap;

fn main() -> Result<()> {
    // Get PDF path from command line, or use a default
    let args: Vec<String> = env::args().collect();
    let pdf_path = args.get(1).map(|s| s.as_str()).unwrap_or("tests/fixtures/sample.pdf");

    // Extract with default options
    let options = ExtractionOptions::default();
    let result = extract_pdf(Path::new(pdf_path), &options)?;

    // Classify pages by type
    let mut page_types: HashMap<String, usize> = HashMap::new();

    println!("Page Classification:");
    println!();

    for page in &result.pages {
        let page_type = page.page_type.as_deref().unwrap_or("unknown");

        // Count by type
        *page_types.entry(page_type.to_string()).or_insert(0) += 1;

        println!("Page {}: {}", page.page_number, page_type);
    }

    // Print summary
    println!();
    println!("Summary:");
    for (ptype, count) in page_types.iter() {
        println!("  {}: {} pages", ptype, count);
    }

    // Provide guidance based on classification
    println!();
    println!("Extraction Guidance:");
    if page_types.contains_key("scanned") || page_types.contains_key("mixed") {
        println!("  - Consider enabling OCR for scanned/mixed pages");
        println!("  - Use ExtractionOptions {{ ocr_languages: vec![\"eng\".to_string()], ..Default::default() }}");
    }
    if page_types.contains_key("broken_vector") {
        println!("  - Some pages have invisible text; OCR may help");
    }
    if page_types.contains_key("vector") {
        println!("  - Vector text extraction is sufficient");
    }

    Ok(())
}
