//! Example: Print the full `ExtractionResult` structure in debug format.
//!
//! Used by bf-61wg7 to inspect how extracted data and errors/diagnostics are
//! organized on the `ExtractionResult` struct.
//!
//! Usage:
//!   cargo run --example dump_extraction_result -- tests/fixtures/malformed/truncated-flate.pdf

use anyhow::Result;
use pdftract_core::{extract_pdf, ExtractionOptions};
use std::env;
use std::path::Path;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let pdf_path = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("tests/fixtures/malformed/truncated-flate.pdf");

    let options = ExtractionOptions::default();
    let result = extract_pdf(Path::new(pdf_path), &options)?;

    // Full pretty debug dump of the entire ExtractionResult tree.
    println!("===== ExtractionResult (pretty debug) =====");
    println!("{result:#?}");

    // Highlight where errors and diagnostics live.
    println!("\n===== Error / diagnostic locations =====");
    println!("metadata.error_count = {}", result.metadata.error_count);
    println!("metadata.diagnostics = {:?}", result.metadata.diagnostics);
    for page in &result.pages {
        println!("page[{}].error = {:?}", page.index, page.error);
    }

    Ok(())
}
