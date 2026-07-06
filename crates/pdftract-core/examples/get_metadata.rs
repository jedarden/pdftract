//! Example: Extract PDF metadata without full page content.
//!
//! Demonstrates lightweight metadata extraction by parsing only the
//! document catalog, trailer, and page tree. This is faster than full
//! extraction for use cases that only need document info.
//!
//! Note: This example shows how to extract metadata from the full result.
//! For true metadata-only extraction (parsing without content streams),
//! use the `pdftract extract --metadata-only` CLI command or the
//! document module's metadata extraction functions.
//!
//! Usage:
//!   cargo run --example get_metadata -- tests/fixtures/sample.pdf

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

    // Print metadata
    println!("PDF Metadata:");
    println!("  Fingerprint: {}", result.fingerprint);
    println!("  Page count: {}", result.metadata.page_count);
    println!("  Total spans: {}", result.metadata.span_count);
    println!("  Total blocks: {}", result.metadata.block_count);
    println!(
        "  Receipts mode: {}",
        result.metadata.receipts_mode.as_str()
    );

    if let Some(algo) = result.metadata.reading_order_algorithm {
        println!("  Reading order: {}", algo);
    }

    if result.metadata.error_count > 0 {
        println!("  Error count: {}", result.metadata.error_count);
    }

    // Print diagnostics
    if !result.metadata.diagnostics.is_empty() {
        println!("\nDiagnostics:");
        for diag in &result.metadata.diagnostics {
            println!("  - {}", diag);
        }
    }

    // Print signatures
    if !result.signatures.is_empty() {
        println!("\nDigital Signatures:");
        for sig in &result.signatures {
            println!("  - Field: {}", sig.field_name);
            if !sig.signer_name.is_empty() {
                println!("    Signer: {}", sig.signer_name);
            }
            if let Some(date) = &sig.signing_date {
                println!("    Date: {}", date);
            }
            println!("    Status: {}", sig.validation_status);
        }
    }

    // Print form fields
    if !result.form_fields.is_empty() {
        println!("\nForm Fields: {}", result.form_fields.len());
    }

    // Print links
    if !result.links.is_empty() {
        println!("\nLinks: {}", result.links.len());
    }

    // Print attachments
    if !result.attachments.is_empty() {
        println!("\nAttachments:");
        for attachment in &result.attachments {
            println!("  - {} ({} bytes)", attachment.name, attachment.size);
        }
    }

    Ok(())
}
