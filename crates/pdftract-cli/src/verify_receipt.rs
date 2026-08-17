//! Verify-receipt subcommand implementation.
//!
//! This module provides the CLI for verifying receipts against PDFs.
//! The verification protocol checks fingerprint, bbox IoU, and content hash.

use anyhow::{Context, Result};
use clap::Args;
use pdftract_core::document::{self};
use pdftract_core::receipts::verifier::{exit_code, VerificationResult};
use pdftract_core::receipts::Receipt;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

/// Verify a receipt against a PDF file.
#[derive(Args)]
pub struct VerifyReceiptCommand {
    /// Path to the PDF file to verify against
    #[arg(value_name = "FILE.pdf")]
    pub pdf_path: PathBuf,

    /// Path to the receipt JSON file, or "-" for stdin
    #[arg(value_name = "RECEIPT.json")]
    pub receipt_path: PathBuf,

    /// Read receipt from stdin (alternative to "-")
    #[arg(long, conflicts_with = "receipt_path")]
    pub stdin: bool,

    /// Receipt JSON as inline string (alternative to file path)
    #[arg(long, conflicts_with = "receipt_path", conflicts_with = "stdin")]
    pub inline: Option<String>,

    /// Output machine-readable JSON result
    #[arg(long)]
    pub json: bool,

    /// Suppress human-readable output (exit code only)
    #[arg(long, conflicts_with = "json")]
    pub quiet: bool,

    /// PDF password (INSECURE: rejected unless PDFTRACT_INSECURE_CLI_PASSWORD=1)
    #[arg(long)]
    pub password: Option<String>,

    /// Read password from stdin (one line, terminated by newline)
    #[arg(long, conflicts_with = "password")]
    pub password_stdin: bool,
}

impl VerifyReceiptCommand {
    /// Emit a warning if password is provided but not yet supported.
    ///
    /// TODO: Implement password support for encrypted PDFs.
    /// This is a placeholder for future work.
    fn warn_password_not_supported(&self) {
        if self.password_stdin || self.password.is_some() {
            eprintln!("Warning: Password support for encrypted PDFs is not yet implemented.");
            eprintln!("The verification will proceed without password handling.");
        }
    }
}

/// JSON output format for verification results.
#[derive(serde::Serialize)]
struct VerificationJsonOutput {
    status: String,
    pdf_fingerprint: String,
    page_index: usize,
    best_iou: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_content_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actual_content_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Run the verify-receipt command.
///
/// This function implements the full verification flow:
/// 1. Load and parse the receipt
/// 2. Check version compatibility
/// 3. Compute PDF fingerprint
/// 4. Extract spans from the target page
/// 5. Run verification protocol
/// 6. Output result and exit with appropriate code
pub fn run_verify_receipt(cmd: VerifyReceiptCommand) -> Result<()> {
    // Step 1: Load receipt
    let receipt = load_receipt(&cmd)?;

    // Step 2: Check version compatibility
    let binary_version = env!("CARGO_PKG_VERSION");
    if let Err(e) = pdftract_core::receipts::verifier::check_version_compatibility(
        &receipt.extraction_version,
        binary_version,
    ) {
        eprintln!("Error: {}", e);
        eprintln!(
            "Install pdftract v{} to verify this receipt",
            receipt.extraction_version
        );
        std::process::exit(exit_code::EXTRACTION_FAILED);
    }

    // Warn if patch version differs
    if let (Some((rmaj, rmin, rpatch)), Some((bmaj, bmin, bpatch))) = (
        pdftract_core::receipts::verifier::parse_semver(&receipt.extraction_version),
        pdftract_core::receipts::verifier::parse_semver(binary_version),
    ) {
        if rmaj == bmaj && rmin == bmin && rpatch != bpatch {
            eprintln!(
                "Warning: Receipt created with v{}.{}.{}, verifying with v{}.{}.{}. \
                 Verification should succeed, but small behavioral differences may exist.",
                rmaj, rmin, rpatch, bmaj, bmin, bpatch
            );
        }
    }

    // Step 3: Compute PDF fingerprint
    let actual_fingerprint = match document::compute_pdf_fingerprint(&cmd.pdf_path) {
        Ok(fp) => fp,
        Err(e) => {
            if !cmd.json && !cmd.quiet {
                eprintln!("Error: Failed to compute PDF fingerprint: {}", e);
            }
            std::process::exit(exit_code::EXTRACTION_FAILED);
        }
    };

    // Step 4: Extract spans from the target page
    let spans = match document::extract_spans_from_page(&cmd.pdf_path, receipt.page_index) {
        Ok(spans) => spans,
        Err(e) => {
            if !cmd.json && !cmd.quiet {
                eprintln!(
                    "Error: Failed to extract spans from page {}: {}",
                    receipt.page_index, e
                );
            }
            std::process::exit(exit_code::EXTRACTION_FAILED);
        }
    };

    // Step 5: Run verification protocol
    let result =
        pdftract_core::receipts::verifier::verify_receipt(&receipt, &spans, &actual_fingerprint);

    // Step 6: Output result
    output_result(&result, &receipt, &actual_fingerprint, &cmd);

    // Step 7: Exit with appropriate code
    std::process::exit(result.exit_code());
}

/// Load the receipt from file, stdin, or inline string.
fn load_receipt(cmd: &VerifyReceiptCommand) -> Result<Receipt> {
    let receipt_json = if let Some(inline) = &cmd.inline {
        inline.clone()
    } else if cmd.stdin || cmd.receipt_path.to_string_lossy() == "-" {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("Failed to read receipt from stdin")?;
        buffer
    } else {
        fs::read_to_string(&cmd.receipt_path)
            .with_context(|| format!("Failed to read receipt from {:?}", cmd.receipt_path))?
    };

    let receipt: Receipt =
        serde_json::from_str(&receipt_json).context("Failed to parse receipt JSON")?;
    Ok(receipt)
}

/// Output the verification result in the requested format.
fn output_result(
    result: &VerificationResult,
    receipt: &Receipt,
    actual_fingerprint: &str,
    cmd: &VerifyReceiptCommand,
) {
    if cmd.json {
        // JSON output
        let output = match result {
            VerificationResult::Ok {
                best_iou,
                actual_content_hash,
            } => {
                let expected_hash = receipt.content_hash.clone();
                VerificationJsonOutput {
                    status: "ok".to_string(),
                    pdf_fingerprint: actual_fingerprint.to_string(),
                    page_index: receipt.page_index,
                    best_iou: *best_iou,
                    expected_content_hash: Some(expected_hash),
                    actual_content_hash: Some(actual_content_hash.clone()),
                    error: None,
                }
            }
            VerificationResult::FingerprintMismatch { expected, actual } => {
                VerificationJsonOutput {
                    status: "fingerprint_mismatch".to_string(),
                    pdf_fingerprint: actual.clone(),
                    page_index: receipt.page_index,
                    best_iou: 0.0,
                    expected_content_hash: Some(expected.clone()),
                    actual_content_hash: Some(actual.clone()),
                    error: Some(format!("Expected fingerprint {}, got {}", expected, actual)),
                }
            }
            VerificationResult::BboxMismatch {
                best_iou,
                threshold,
            } => VerificationJsonOutput {
                status: "bbox_mismatch".to_string(),
                pdf_fingerprint: actual_fingerprint.to_string(),
                page_index: receipt.page_index,
                best_iou: *best_iou,
                expected_content_hash: None,
                actual_content_hash: None,
                error: Some(format!(
                    "No span meets IoU threshold {} (best IoU: {:.3})",
                    threshold, best_iou
                )),
            },
            VerificationResult::ContentMismatch {
                best_iou,
                expected_hash,
                actual_hash,
            } => VerificationJsonOutput {
                status: "content_mismatch".to_string(),
                pdf_fingerprint: actual_fingerprint.to_string(),
                page_index: receipt.page_index,
                best_iou: *best_iou,
                expected_content_hash: Some(expected_hash.clone()),
                actual_content_hash: Some(actual_hash.clone()),
                error: Some(format!(
                    "Content hash mismatch: expected {}, got {}",
                    expected_hash, actual_hash
                )),
            },
        };

        println!("{}", serde_json::to_string(&output).unwrap());
    } else if !cmd.quiet {
        // Human-readable output
        match result {
            VerificationResult::Ok {
                best_iou,
                actual_content_hash,
            } => {
                println!(
                    "Receipt verified: {} page {} bbox [{}, {}, {}, {}]",
                    receipt.pdf_fingerprint,
                    receipt.page_index,
                    receipt.bbox[0],
                    receipt.bbox[1],
                    receipt.bbox[2],
                    receipt.bbox[3]
                );
                println!(
                    "Best-match span IoU: {:.3}, content_hash: {}",
                    best_iou, actual_content_hash
                );
            }
            VerificationResult::FingerprintMismatch { expected, actual } => {
                eprintln!("Error: PDF fingerprint mismatch");
                eprintln!("  Expected: {}", expected);
                eprintln!("  Actual:   {}", actual);
                eprintln!();
                eprintln!("The receipt was created for a different PDF file.");
            }
            VerificationResult::BboxMismatch {
                best_iou,
                threshold,
            } => {
                eprintln!(
                    "Error: Bbox mismatch (no span meets {}% IoU threshold)",
                    threshold * 100.0
                );
                eprintln!("  Best IoU: {:.3}%", best_iou * 100.0);
                eprintln!(
                    "  Receipt bbox: [{}, {}, {}, {}]",
                    receipt.bbox[0], receipt.bbox[1], receipt.bbox[2], receipt.bbox[3]
                );
                eprintln!();
                eprintln!(
                    "No text span on page {} matches the receipt's bounding box.",
                    receipt.page_index
                );
            }
            VerificationResult::ContentMismatch {
                best_iou,
                expected_hash,
                actual_hash,
            } => {
                eprintln!("Error: Content hash mismatch");
                eprintln!("  Best-match IoU: {:.3}%", best_iou * 100.0);
                eprintln!("  Expected hash: {}", expected_hash);
                eprintln!("  Actual hash:   {}", actual_hash);
                eprintln!();
                eprintln!(
                    "The text at the receipt's location has changed since the receipt was created."
                );
            }
        }
    }
}
