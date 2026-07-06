//! Example: Stream PDF extraction as NDJSON.
//!
//! Demonstrates memory-efficient streaming extraction using
//! `extract_pdf_ndjson`, which writes each page as a newline-delimited
//! JSON object immediately after extraction. This keeps memory usage
//! bounded regardless of document size.
//!
//! Usage:
//!   cargo run --example extract_stream -- tests/fixtures/sample.pdf

use anyhow::Result;
use pdftract_core::{extract_pdf_ndjson, ExtractionOptions};
use std::env;
use std::io::{self, BufWriter};
use std::path::Path;

fn main() -> Result<()> {
    // Get PDF path from command line, or use a default
    let args: Vec<String> = env::args().collect();
    let pdf_path = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("tests/fixtures/sample.pdf");

    // Extract with default options, streaming to stdout
    let options = ExtractionOptions::default();
    let stdout = BufWriter::new(io::stdout());
    let metadata = extract_pdf_ndjson(Path::new(pdf_path), &options, stdout)?;

    // Print summary to stderr (so it doesn't mix with NDJSON output)
    eprintln!("Extraction complete:");
    eprintln!("  Pages: {}", metadata.page_count);
    eprintln!("  Spans: {}", metadata.span_count);
    eprintln!("  Blocks: {}", metadata.block_count);
    eprintln!("  Errors: {}", metadata.error_count);

    if let Some(algo) = metadata.reading_order_algorithm {
        eprintln!("  Reading order: {}", algo);
    }

    // Print diagnostics if any
    for diag in &metadata.diagnostics {
        eprintln!("  Diagnostic: {}", diag);
    }

    Ok(())
}
