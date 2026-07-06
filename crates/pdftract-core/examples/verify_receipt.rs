//! Example: Verify a citation receipt against a PDF.
//!
//! Demonstrates receipt verification, which confirms that extracted text
//! originated from a specific region in a specific PDF.
//!
//! Usage:
//!   cargo run --example verify_receipt -- tests/fixtures/sample.pdf receipt.json

use anyhow::Result;
use pdftract_core::document::{compute_pdf_fingerprint, extract_spans_from_page};
use pdftract_core::receipts::verifier::{verify_receipt, VerificationResult};
use pdftract_core::receipts::Receipt;
use std::env;
use std::fs;
use std::path::Path;

fn main() -> Result<()> {
    // Get paths from command line
    let args: Vec<String> = env::args().collect();
    let pdf_path = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("tests/fixtures/sample.pdf");
    let receipt_path = args.get(2).map(|s| s.as_str()).unwrap_or("receipt.json");

    // Load receipt
    let receipt_data = fs::read_to_string(receipt_path)?;
    let receipt: Receipt = serde_json::from_str(&receipt_data)?;

    println!("Verifying receipt:");
    println!("  PDF fingerprint: {}", receipt.pdf_fingerprint);
    println!("  Page index: {}", receipt.page_index);
    println!(
        "  Bbox: [{}, {}, {}, {}]",
        receipt.bbox[0], receipt.bbox[1], receipt.bbox[2], receipt.bbox[3]
    );
    println!("  Content hash: {}", receipt.content_hash);
    println!();

    // Compute PDF fingerprint
    let actual_fingerprint = compute_pdf_fingerprint(Path::new(pdf_path))?;

    if actual_fingerprint != receipt.pdf_fingerprint {
        println!("FAILED: Fingerprint mismatch");
        println!("  Expected: {}", receipt.pdf_fingerprint);
        println!("  Actual:   {}", actual_fingerprint);
        return Ok(());
    }

    // Extract spans from the target page
    let spans = extract_spans_from_page(Path::new(pdf_path), receipt.page_index)?;

    // Verify receipt
    let result = verify_receipt(&receipt, &spans, &actual_fingerprint);

    match result {
        VerificationResult::Ok {
            best_iou,
            actual_content_hash,
        } => {
            println!("VERIFIED: Receipt is valid");
            println!("  Best IoU: {:.3}", best_iou);
            println!("  Content hash: {}", actual_content_hash);
        }
        VerificationResult::BboxMismatch {
            best_iou,
            threshold,
        } => {
            println!("FAILED: Bbox mismatch");
            println!("  Best IoU: {:.3}", best_iou);
            println!("  Required: {:.3}", threshold);
        }
        VerificationResult::ContentMismatch {
            best_iou,
            expected_hash,
            actual_hash,
        } => {
            println!("FAILED: Content hash mismatch");
            println!("  Best IoU: {:.3}", best_iou);
            println!("  Expected: {}", expected_hash);
            println!("  Actual:   {}", actual_hash);
        }
        VerificationResult::FingerprintMismatch { expected, actual } => {
            println!("FAILED: Fingerprint mismatch");
            println!("  Expected: {}", expected);
            println!("  Actual:   {}", actual);
        }
    }

    Ok(())
}
