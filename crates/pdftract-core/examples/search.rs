//! Example: Search for text patterns across a PDF.
//!
//! Demonstrates pattern matching across extracted text. This example
//! shows how to search for a regex pattern and report matches with page
//! numbers and bounding boxes.
//!
//! Usage:
//!   cargo run --example search -- tests/fixtures/sample.pdf "invoice"

use anyhow::Result;
use pdftract_core::{extract_pdf, ExtractionOptions};
use regex::Regex;
use std::env;
use std::path::Path;

struct Match {
    page_number: u32,
    text: String,
    bbox: [f64; 4],
}

fn main() -> Result<()> {
    // Get PDF path and pattern from command line
    let args: Vec<String> = env::args().collect();
    let pdf_path = args.get(1).map(|s| s.as_str()).unwrap_or("tests/fixtures/sample.pdf");
    let pattern = args.get(2).map(|s| s.as_str()).unwrap_or("the");

    // Compile regex pattern (case-insensitive by default)
    let regex = Regex::new(&format!("(?i){}", pattern))?;

    // Extract with default options
    let options = ExtractionOptions::default();
    let result = extract_pdf(Path::new(pdf_path), &options)?;

    // Search across all pages
    let mut matches = Vec::new();

    for page in &result.pages {
        for span in &page.spans {
            if regex.is_match(&span.text) {
                matches.push(Match {
                    page_number: page.page_number,
                    text: span.text.clone(),
                    bbox: span.bbox,
                });
            }
        }
    }

    // Print results
    if matches.is_empty() {
        println!("No matches found for pattern: {}", pattern);
    } else {
        println!("Found {} matches for pattern: {}", matches.len(), pattern);
        println!();

        for m in &matches {
            println!("Page {}: \"{}\"", m.page_number, m.text);
            println!("  Bbox: [{}, {}, {}, {}]", m.bbox[0], m.bbox[1], m.bbox[2], m.bbox[3]);
            println!();
        }
    }

    Ok(())
}
