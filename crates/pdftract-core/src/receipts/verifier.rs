//! Receipt verification protocol.
//!
//! This module implements the verifier that validates receipts against
//! the original PDF. The verifier reproduces the extraction and checks:
//! 1. PDF fingerprint matches
//! 2. At least one span has bbox overlap >= 90% IoU
//! 3. That span's NFC-normalized SHA-256 equals the receipt's content_hash
//!
//! # Exit codes
//!
//! - 0: receipt verifies
//! - 10: pdf_fingerprint mismatch
//! - 11: bbox mismatch (no span meets 90% IoU threshold)
//! - 12: content_hash mismatch (best-IoU span's text differs)
//! - 1: extraction failed (PDF unreadable, encrypted without password, etc.)

use crate::receipts::Receipt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

/// IoU verification threshold (90%).
///
/// This threshold is calibrated to be robust against floating-point
/// round-tripping noise (0-2 point shifts) while detecting deliberate
/// bbox tampering. Per plan section 6.8 line 2388.
pub const IOU_VERIFICATION_THRESHOLD: f64 = 0.9;

/// Verification exit codes.
pub mod exit_code {
    pub const SUCCESS: i32 = 0;
    pub const FINGERPRINT_MISMATCH: i32 = 10;
    pub const BBOX_MISMATCH: i32 = 11;
    pub const CONTENT_MISMATCH: i32 = 12;
    pub const EXTRACTION_FAILED: i32 = 1;
}

/// Verification result.
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationResult {
    Ok {
        best_iou: f64,
        actual_content_hash: String,
    },
    FingerprintMismatch {
        expected: String,
        actual: String,
    },
    BboxMismatch {
        best_iou: f64,
        threshold: f64,
    },
    ContentMismatch {
        best_iou: f64,
        expected_hash: String,
        actual_hash: String,
    },
}

impl VerificationResult {
    /// Get the exit code for this result.
    pub fn exit_code(&self) -> i32 {
        match self {
            VerificationResult::Ok { .. } => exit_code::SUCCESS,
            VerificationResult::FingerprintMismatch { .. } => exit_code::FINGERPRINT_MISMATCH,
            VerificationResult::BboxMismatch { .. } => exit_code::BBOX_MISMATCH,
            VerificationResult::ContentMismatch { .. } => exit_code::CONTENT_MISMATCH,
        }
    }

    /// Check if verification succeeded.
    pub fn is_ok(&self) -> bool {
        matches!(self, VerificationResult::Ok { .. })
    }
}

/// Compute IoU (Intersection over Union) for two bounding boxes.
///
/// # Arguments
///
/// * `a` - First bbox [x0, y0, x1, y1]
/// * `b` - Second bbox [x0, y0, x1, y1]
///
/// # Returns
///
/// IoU value in [0.0, 1.0], where 1.0 means identical boxes.
pub fn iou(a: [f64; 4], b: [f64; 4]) -> f64 {
    let x0 = a[0].max(b[0]);
    let y0 = a[1].max(b[1]);
    let x1 = a[2].min(b[2]);
    let y1 = a[3].min(b[3]);

    // No overlap
    if x1 <= x0 || y1 <= y0 {
        return 0.0;
    }

    let inter = (x1 - x0) * (y1 - y0);
    let area_a = (a[2] - a[0]) * (a[3] - a[1]);
    let area_b = (b[2] - b[0]) * (b[3] - b[1]);

    // Guard against division by zero
    let union = area_a + area_b - inter;
    if union <= 0.0 {
        return 0.0;
    }

    inter / union
}

/// Compute the content hash for a piece of text (NFC-normalized SHA-256).
///
/// # Returns
///
/// A string in the format `"sha256:" + hex(SHA-256)`.
pub fn compute_content_hash(text: &str) -> String {
    let nfc: String = text.nfc().collect();
    let hash = Sha256::digest(nfc.as_bytes());
    format!("sha256:{}", hex::encode(hash))
}

/// Extract version components from a semver string.
///
/// # Returns
///
/// `(major, minor, patch)` as `(u64, u64, u64)`.
/// Returns `None` if the string is not valid semver.
pub fn parse_semver(version: &str) -> Option<(u64, u64, u64)> {
    let parts: Vec<&str> = version.trim().split('.').collect();
    if parts.len() < 2 {
        return None;
    }

    let major = parts[0].parse::<u64>().ok()?;
    let minor = parts[1].parse::<u64>().ok()?;

    // Patch is optional, default to 0
    let patch = if parts.len() >= 3 {
        // Handle pre-release identifiers (e.g., "1.0.0-rc.1")
        let patch_str = parts[2].split('-').next().unwrap_or("0");
        patch_str.parse::<u64>().ok()
    } else {
        Some(0)
    }?;

    Some((major, minor, patch))
}

/// Check version compatibility between receipt and binary.
///
/// The verifier MUST use the same extraction_version as the receipt.
/// If MAJOR or MINOR differ, the binary is incompatible.
/// Patch version differences are allowed (semver compatibility).
///
/// # Returns
///
/// `Ok(())` if compatible, `Err(message)` if not.
pub fn check_version_compatibility(
    receipt_version: &str,
    binary_version: &str,
) -> Result<(), String> {
    let receipt_ver = parse_semver(receipt_version)
        .ok_or_else(|| format!("Invalid receipt version: {}", receipt_version))?;
    let binary_ver = parse_semver(binary_version)
        .ok_or_else(|| format!("Invalid binary version: {}", binary_version))?;

    // MAJOR must match exactly
    if receipt_ver.0 != binary_ver.0 {
        return Err(format!(
            "Major version mismatch: receipt requires v{}.x.x but binary is v{}.{}.{}",
            receipt_ver.0, binary_ver.0, binary_ver.1, binary_ver.2
        ));
    }

    // MINOR must match exactly
    if receipt_ver.1 != binary_ver.1 {
        return Err(format!(
            "Minor version mismatch: receipt requires v{}.{}.x but binary is v{}.{}.{}",
            receipt_ver.0, receipt_ver.1, binary_ver.0, binary_ver.1, binary_ver.2
        ));
    }

    // Patch can differ (compatible by semver)
    Ok(())
}

/// Span data for verification.
///
/// This represents a single text span extracted from a PDF page,
/// with enough information to compute IoU and content hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanData {
    /// The extracted text content.
    pub text: String,
    /// Bounding box in PDF user-space points [x0, y0, x1, y1].
    pub bbox: [f64; 4],
}

/// Verify a receipt against extracted spans from a PDF page.
///
/// # Arguments
///
/// * `receipt` - The receipt to verify
/// * `spans` - Spans extracted from the receipt's page_index
/// * `actual_fingerprint` - The computed fingerprint of the PDF
///
/// # Returns
///
/// A `VerificationResult` indicating success or the specific failure mode.
pub fn verify_receipt(
    receipt: &Receipt,
    spans: &[SpanData],
    actual_fingerprint: &str,
) -> VerificationResult {
    // Step 1: Check fingerprint
    if receipt.pdf_fingerprint != actual_fingerprint {
        return VerificationResult::FingerprintMismatch {
            expected: receipt.pdf_fingerprint.clone(),
            actual: actual_fingerprint.to_string(),
        };
    }

    // Step 2: Find span with maximum IoU
    let mut best_span: Option<&SpanData> = None;
    let mut best_iou = 0.0;

    for span in spans {
        let span_iou = iou(receipt.bbox, span.bbox);
        if span_iou > best_iou {
            best_iou = span_iou;
            best_span = Some(span);
        }
    }

    // Step 3: Check IoU threshold
    if best_iou < IOU_VERIFICATION_THRESHOLD {
        return VerificationResult::BboxMismatch {
            best_iou,
            threshold: IOU_VERIFICATION_THRESHOLD,
        };
    }

    // Step 4: Verify content hash
    let best_span = best_span.expect("best_span is Some when best_iou >= threshold");
    let actual_hash = compute_content_hash(&best_span.text);

    if receipt.content_hash != actual_hash {
        return VerificationResult::ContentMismatch {
            best_iou,
            expected_hash: receipt.content_hash.clone(),
            actual_hash,
        };
    }

    VerificationResult::Ok {
        best_iou,
        actual_content_hash: actual_hash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iou_identical_boxes() {
        let a = [100.0, 200.0, 300.0, 400.0];
        let b = [100.0, 200.0, 300.0, 400.0];
        assert!((iou(a, b) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_iou_no_overlap() {
        let a = [0.0, 0.0, 100.0, 100.0];
        let b = [200.0, 200.0, 300.0, 300.0];
        assert_eq!(iou(a, b), 0.0);
    }

    #[test]
    fn test_iou_partial_overlap() {
        // 50% overlap
        let a = [0.0, 0.0, 200.0, 200.0];
        let b = [100.0, 0.0, 300.0, 200.0];

        // Intersection: 100 * 200 = 20000
        // Area a: 200 * 200 = 40000
        // Area b: 200 * 200 = 40000
        // Union: 40000 + 40000 - 20000 = 60000
        // IoU: 20000 / 60000 = 1/3
        let expected = 20000.0 / 60000.0;
        assert!((iou(a, b) - expected).abs() < 0.001);
    }

    #[test]
    fn test_iou_one_inside_another() {
        // b is completely inside a
        let a = [0.0, 0.0, 200.0, 200.0];
        let b = [50.0, 50.0, 150.0, 150.0];

        // Intersection = area of b = 100 * 100 = 10000
        // Union = area of a = 200 * 200 = 40000
        // IoU = 10000 / 40000 = 0.25
        let expected = 10000.0 / 40000.0;
        assert!((iou(a, b) - expected).abs() < 0.001);
    }

    #[test]
    fn test_iou_touching_edges() {
        // Boxes touch at edge but don't overlap
        let a = [0.0, 0.0, 100.0, 100.0];
        let b = [100.0, 0.0, 200.0, 100.0];
        assert_eq!(iou(a, b), 0.0);
    }

    #[test]
    fn test_iou_degenerate_boxes() {
        // Zero-area box
        let a = [0.0, 0.0, 0.0, 0.0];
        let b = [0.0, 0.0, 100.0, 100.0];
        assert_eq!(iou(a, b), 0.0);
    }

    #[test]
    fn test_compute_content_hash_format() {
        let hash = compute_content_hash("test");
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), "sha256:".len() + 64);
    }

    #[test]
    fn test_compute_content_hash_nfc_normalization() {
        // NFC and NFD forms should produce the same hash
        let nfc_text = "café"; // U+00E9 (composed)
        let nfd_text: String = "cafe\u{0301}".nfd().collect(); // decomposed

        let hash_nfc = compute_content_hash(nfc_text);
        let hash_nfd = compute_content_hash(&nfd_text);

        assert_eq!(hash_nfc, hash_nfd);
    }

    #[test]
    fn test_parse_semver_valid() {
        assert_eq!(parse_semver("1.0.0"), Some((1, 0, 0)));
        assert_eq!(parse_semver("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_semver("0.1.0"), Some((0, 1, 0)));
        assert_eq!(parse_semver("1.0"), Some((1, 0, 0))); // patch defaults to 0
    }

    #[test]
    fn test_parse_semver_with_prerelease() {
        assert_eq!(parse_semver("1.0.0-rc.1"), Some((1, 0, 0)));
        assert_eq!(parse_semver("1.0.0-beta"), Some((1, 0, 0)));
        assert_eq!(parse_semver("2.1.3-alpha.1"), Some((2, 1, 3)));
    }

    #[test]
    fn test_parse_semver_invalid() {
        assert_eq!(parse_semver("invalid"), None);
        assert_eq!(parse_semver("1"), None);
        assert_eq!(parse_semver(""), None);
        assert_eq!(parse_semver("a.b.c"), None);
    }

    #[test]
    fn test_check_version_compatibility_same() {
        assert!(check_version_compatibility("1.0.0", "1.0.0").is_ok());
        assert!(check_version_compatibility("1.2.3", "1.2.3").is_ok());
    }

    #[test]
    fn test_check_version_compatibility_patch_diff() {
        // Patch differences are allowed
        assert!(check_version_compatibility("1.0.0", "1.0.1").is_ok());
        assert!(check_version_compatibility("1.0.1", "1.0.0").is_ok());
        assert!(check_version_compatibility("1.2.3", "1.2.4").is_ok());
    }

    #[test]
    fn test_check_version_compatibility_minor_diff() {
        // Minor differences are NOT allowed
        assert!(check_version_compatibility("1.0.0", "1.1.0").is_err());
        assert!(check_version_compatibility("1.1.0", "1.0.0").is_err());
        assert!(check_version_compatibility("2.1.0", "2.2.0").is_err());
    }

    #[test]
    fn test_check_version_compatibility_major_diff() {
        // Major differences are NOT allowed
        assert!(check_version_compatibility("1.0.0", "2.0.0").is_err());
        assert!(check_version_compatibility("2.0.0", "1.0.0").is_err());
    }

    #[test]
    fn test_verify_receipt_success() {
        let receipt = Receipt::lite(
            "pdftract-v1:abc123".to_string(),
            0,
            [100.0, 200.0, 300.0, 220.0],
            "Hello, world!",
        );

        let spans = vec![SpanData {
            text: "Hello, world!".to_string(),
            bbox: [100.0, 200.0, 300.0, 220.0],
        }];

        let result = verify_receipt(&receipt, &spans, "pdftract-v1:abc123");

        assert!(result.is_ok());
        assert_eq!(result.exit_code(), 0);
    }

    #[test]
    fn test_verify_receipt_fingerprint_mismatch() {
        let receipt = Receipt::lite(
            "pdftract-v1:abc123".to_string(),
            0,
            [100.0, 200.0, 300.0, 220.0],
            "Hello, world!",
        );

        let spans = vec![SpanData {
            text: "Hello, world!".to_string(),
            bbox: [100.0, 200.0, 300.0, 220.0],
        }];

        let result = verify_receipt(&receipt, &spans, "pdftract-v1:different");

        assert!(!result.is_ok());
        assert_eq!(result.exit_code(), 10);
    }

    #[test]
    fn test_verify_receipt_bbox_mismatch() {
        let receipt = Receipt::lite(
            "pdftract-v1:abc123".to_string(),
            0,
            [100.0, 200.0, 300.0, 220.0],
            "Hello, world!",
        );

        // Span with bbox far from receipt bbox
        let spans = vec![SpanData {
            text: "Hello, world!".to_string(),
            bbox: [500.0, 600.0, 700.0, 620.0], // Far away, low IoU
        }];

        let result = verify_receipt(&receipt, &spans, "pdftract-v1:abc123");

        assert!(!result.is_ok());
        assert_eq!(result.exit_code(), 11);
    }

    #[test]
    fn test_verify_receipt_content_mismatch() {
        let receipt = Receipt::lite(
            "pdftract-v1:abc123".to_string(),
            0,
            [100.0, 200.0, 300.0, 220.0],
            "Hello, world!",
        );

        // Span with different text but same bbox
        let spans = vec![SpanData {
            text: "Different text!".to_string(),
            bbox: [100.0, 200.0, 300.0, 220.0],
        }];

        let result = verify_receipt(&receipt, &spans, "pdftract-v1:abc123");

        assert!(!result.is_ok());
        assert_eq!(result.exit_code(), 12);
    }

    #[test]
    fn test_verify_receipt_best_match_selected() {
        let receipt = Receipt::lite(
            "pdftract-v1:abc123".to_string(),
            0,
            [100.0, 200.0, 300.0, 220.0],
            "Hello, world!",
        );

        // Multiple spans, one with high IoU but wrong text, one with lower IoU but correct text
        let spans = vec![
            SpanData {
                text: "Wrong text".to_string(),
                bbox: [100.0, 200.0, 300.0, 220.0], // Perfect bbox match
            },
            SpanData {
                text: "Hello, world!".to_string(),
                bbox: [105.0, 200.0, 295.0, 220.0], // Slightly offset but >90% IoU
            },
        ];

        let result = verify_receipt(&receipt, &spans, "pdftract-v1:abc123");

        // Should succeed because the best-IoU span (first one) is selected
        // Actually wait - this will fail because the best-IoU span has wrong text!
        // Let me reconsider this test...
        assert!(!result.is_ok()); // Best IoU span has wrong content
        assert_eq!(result.exit_code(), 12);
    }

    #[test]
    fn test_iou_threshold_verification() {
        // Test that IoU slightly below threshold fails
        let receipt = Receipt::lite(
            "pdftract-v1:abc123".to_string(),
            0,
            [100.0, 200.0, 300.0, 220.0],
            "Hello, world!",
        );

        // Span with IoU just below 90%
        // Area: 200 * 20 = 4000
        // To get IoU < 0.9, we need minimal overlap
        let spans = vec![SpanData {
            text: "Hello, world!".to_string(),
            bbox: [250.0, 200.0, 350.0, 220.0], // Only 50 pixel overlap (50*20=1000), IoU = 1000/7000 ≈ 0.14
        }];

        let result = verify_receipt(&receipt, &spans, "pdftract-v1:abc123");
        assert_eq!(result.exit_code(), 11);
    }

    #[test]
    fn test_iou_threshold_pass() {
        // Test that IoU at or above 90% passes bbox check
        let receipt = Receipt::lite(
            "pdftract-v1:abc123".to_string(),
            0,
            [100.0, 200.0, 300.0, 220.0],
            "Hello, world!",
        );

        // Span with IoU > 90% (same bbox = 100%)
        let spans = vec![SpanData {
            text: "Hello, world!".to_string(),
            bbox: [100.0, 200.0, 300.0, 220.0],
        }];

        let result = verify_receipt(&receipt, &spans, "pdftract-v1:abc123");
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_receipt_with_unicode_normalization() {
        // Receipt created from NFC text
        let receipt = Receipt::lite(
            "pdftract-v1:abc123".to_string(),
            0,
            [100.0, 200.0, 300.0, 220.0],
            "café", // NFC: U+00E9
        );

        // Span with NFD text should still verify
        let nfd_text: String = "cafe\u{0301}".nfd().collect(); // NFD: e + combining acute
        let spans = vec![SpanData {
            text: nfd_text,
            bbox: [100.0, 200.0, 300.0, 220.0],
        }];

        let result = verify_receipt(&receipt, &spans, "pdftract-v1:abc123");
        assert!(result.is_ok());
    }
}
