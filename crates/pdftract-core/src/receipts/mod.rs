//! Visual citation receipts for PDF extraction verification.
//!
//! This module implements portable receipt objects that bind extracted text
//! to specific regions in a PDF document, enabling downstream verification
//! of provenance.
//!
//! # Receipt modes
//!
//! - **Lite mode** (`--receipts=lite`): Minimal receipts with ~120 bytes each,
//!   containing fingerprint, page index, bbox, content hash, and extraction version.
//! - **SVG mode** (`--receipts=svg`): Extended receipts that include an SVG clip
//!   rendering the glyphs within the bbox for standalone verification.
//!
//! # Receipt schema
//!
//! All receipts contain:
//! - `pdf_fingerprint`: Phase 1.7 fingerprint of the source PDF
//! - `page_index`: 0-based page index matching the extraction schema
//! - `bbox`: [x0, y0, x1, y1] in PDF user-space points
//! - `content_hash`: SHA-256 of NFC-normalized text
//! - `extraction_version`: pdftract semver that produced this receipt
//! - `svg_clip`: Optional SVG rendering (only in SVG mode)

pub mod lite;
pub mod svg;
pub mod verifier;

use serde::{Deserialize, Serialize};

/// A visual citation receipt for extracted text.
///
/// Receipts provide cryptographic proof that a piece of extracted text
/// originated from a specific region in a specific PDF. They can be
/// verified independently by re-running pdftract on the original file.
///
/// # Lite mode
///
/// In lite mode, `svg_clip` is `None` and the JSON output does not
/// include the key at all (via `skip_serializing_if`). This keeps
/// receipts small (~120-180 bytes) for high-volume use cases like
/// RAG citation pipelines.
///
/// # SVG mode
///
/// In SVG mode, `svg_clip` contains a self-contained SVG element
/// that renders only the glyphs whose bboxes fall within the receipt
/// bbox. The SVG is normalized to the bbox coordinate system and
/// can be rendered standalone in any browser.
///
/// # Example
///
/// ```json
/// {
///   "pdf_fingerprint": "pdftract-v1:a7f3...",
///   "page_index": 14,
///   "bbox": [220.0, 412.0, 412.0, 432.0],
///   "content_hash": "sha256:9b21...",
///   "extraction_version": "1.0.0"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Receipt {
    /// Phase 1.7 fingerprint of the source PDF.
    ///
    /// Format: `"pdftract-v1:" + hex(SHA-256)`.
    /// The verifier compares this string literally (not parsed).
    pub pdf_fingerprint: String,

    /// 0-based page index in the source PDF.
    ///
    /// Matches the page_index in the extraction schema.
    pub page_index: usize,

    /// Bounding box in PDF user-space points.
    ///
    /// Format: `[x0, y0, x1, y1]` where:
    /// - x0, y0: bottom-left corner
    /// - x1, y1: top-right corner
    /// - Units: PDF points (1/72 inch)
    ///
    /// This is a copy of the parent span's bbox, included so the
    /// receipt is self-contained.
    pub bbox: [f64; 4],

    /// SHA-256 hash of the NFC-normalized text content.
    ///
    /// Format: `"sha256:" + hex(SHA-256)`.
    ///
    /// The text is normalized to NFC form before hashing to ensure
    /// stability across platforms that may use different Unicode
    /// normalization forms (e.g., macOS HFS+/APFS sometimes round-trips
    /// through NFD).
    pub content_hash: String,

    /// The pdftract version that produced this receipt.
    ///
    /// Format: semver string (e.g., "1.0.0", "1.0.0-rc.1").
    /// Taken from `CARGO_PKG_VERSION` at compile time.
    pub extraction_version: String,

    /// Optional SVG clip rendering the glyphs in this receipt.
    ///
    /// - `None` in lite mode (the key is omitted from JSON entirely)
    /// - `Some(svg)` in SVG mode, where `svg` is a self-contained SVG element
    ///
    /// The SVG coordinate system is normalized to the bbox itself,
    /// so it renders correctly in isolation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub svg_clip: Option<String>,
}

impl Receipt {
    /// Create a lite-mode receipt.
    ///
    /// This constructor computes the `content_hash` internally by
    /// NFC-normalizing the text before hashing. The `svg_clip` field
    /// is set to `None`.
    ///
    /// # Arguments
    ///
    /// * `pdf_fingerprint` - Phase 1.7 fingerprint of the source PDF
    /// * `page_index` - 0-based page index
    /// * `bbox` - Bounding box in PDF points [x0, y0, x1, y1]
    /// * `text` - The text content (will be NFC-normalized before hashing)
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdftract_core::receipts::Receipt;
    ///
    /// let receipt = Receipt::lite(
    ///     "pdftract-v1:a7f3...".to_string(),
    ///     14,
    ///     [220.0, 412.0, 412.0, 432.0],
    ///     "Net Income: $2.4M"
    /// );
    /// assert_eq!(receipt.svg_clip, None);
    /// assert!(receipt.content_hash.starts_with("sha256:"));
    /// ```
    pub fn lite(pdf_fingerprint: String, page_index: usize, bbox: [f64; 4], text: &str) -> Self {
        let content_hash = compute_content_hash(text);
        let extraction_version = env!("CARGO_PKG_VERSION").to_string();

        Self {
            pdf_fingerprint,
            page_index,
            bbox,
            content_hash,
            extraction_version,
            svg_clip: None,
        }
    }

    /// Create a receipt with an SVG clip (SVG mode).
    ///
    /// This is the constructor used by Phase 6.8.2. The lite-mode
    /// constructor above is preferred for most use cases.
    #[doc(hidden)]
    pub fn with_svg(
        pdf_fingerprint: String,
        page_index: usize,
        bbox: [f64; 4],
        text: &str,
        svg_clip: String,
    ) -> Self {
        let content_hash = compute_content_hash(text);
        let extraction_version = env!("CARGO_PKG_VERSION").to_string();

        Self {
            pdf_fingerprint,
            page_index,
            bbox,
            content_hash,
            extraction_version,
            svg_clip: Some(svg_clip),
        }
    }
}

/// Compute the content hash for a piece of text.
///
/// The text is NFC-normalized before hashing to ensure stability
/// across platforms that may use different Unicode normalization forms.
///
/// # Returns
///
/// A string in the format `"sha256:" + hex(SHA-256)`.
fn compute_content_hash(text: &str) -> String {
    use sha2::{Digest, Sha256};
    use unicode_normalization::UnicodeNormalization;

    // NFC normalization is required for cross-platform stability
    let nfc: String = text.nfc().collect();
    let hash = Sha256::digest(nfc.as_bytes());
    format!("sha256:{}", hex::encode(hash))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receipt_lite_creates_valid_receipt() {
        let receipt = Receipt::lite(
            "pdftract-v1:abc123".to_string(),
            5,
            [10.0, 20.0, 100.0, 120.0],
            "Hello, world!",
        );

        assert_eq!(receipt.pdf_fingerprint, "pdftract-v1:abc123");
        assert_eq!(receipt.page_index, 5);
        assert_eq!(receipt.bbox, [10.0, 20.0, 100.0, 120.0]);
        assert!(receipt.content_hash.starts_with("sha256:"));
        assert_eq!(receipt.svg_clip, None);
    }

    #[test]
    fn test_receipt_lite_serializes_without_svg_clip() {
        let receipt = Receipt::lite(
            "pdftract-v1:abc123".to_string(),
            5,
            [10.0, 20.0, 100.0, 120.0],
            "Hello, world!",
        );

        let json = serde_json::to_string(&receipt).unwrap();

        // In lite mode, svg_clip should NOT appear in the JSON
        assert!(!json.contains("svg_clip"));

        // But the other fields should be present
        assert!(json.contains("pdf_fingerprint"));
        assert!(json.contains("page_index"));
        assert!(json.contains("bbox"));
        assert!(json.contains("content_hash"));
        assert!(json.contains("extraction_version"));
    }

    #[test]
    fn test_receipt_with_svg_includes_svg_clip() {
        let receipt = Receipt::with_svg(
            "pdftract-v1:abc123".to_string(),
            5,
            [10.0, 20.0, 100.0, 120.0],
            "Hello, world!",
            "<svg>...</svg>".to_string(),
        );

        let json = serde_json::to_string(&receipt).unwrap();

        // In SVG mode, svg_clip SHOULD appear in the JSON
        assert!(json.contains("svg_clip"));
        assert!(json.contains("<svg>...</svg>"));
    }

    #[test]
    fn test_content_hash_format() {
        let hash = compute_content_hash("test");

        assert!(hash.starts_with("sha256:"));
        // sha256: prefix (7) + 64 hex chars = 71
        assert_eq!(hash.len(), 71);
    }

    #[test]
    fn test_content_hash_roundtrip() {
        let text = "Hello, world!";
        let hash1 = compute_content_hash(text);
        let hash2 = compute_content_hash(text);

        assert_eq!(hash1, hash2, "Hashing the same text should produce the same result");
    }

    #[test]
    fn test_content_hash_nfc_normalization() {
        use unicode_normalization::UnicodeNormalization;

        // U+00E9 is "é" in NFC (composed form)
        let nfc_text = "café";  // U+0063 U+0061 U+0066 U+00E9

        // U+0065 U+0301 is "é" in NFD (decomposed form: e + combining acute)
        let nfd_text: String = "cafe\u{0301}".nfd().collect();  // U+0063 U+0061 U+0066 U+0065 U+0301

        // Both should produce the same hash after NFC normalization
        let hash_nfc = compute_content_hash(nfc_text);
        let hash_nfd = compute_content_hash(&nfd_text);

        assert_eq!(
            hash_nfc, hash_nfd,
            "NFC and NFD forms of the same logical string should produce the same hash"
        );
    }

    #[test]
    fn test_content_hash_different_strings() {
        let hash1 = compute_content_hash("Hello");
        let hash2 = compute_content_hash("World");

        assert_ne!(
            hash1, hash2,
            "Different strings should produce different hashes"
        );
    }

    #[test]
    fn test_content_hash_empty_string() {
        let hash = compute_content_hash("");

        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 71);
    }

    #[test]
    fn test_content_hash_unicode() {
        // Test with various Unicode characters
        let texts = [
            "Hello 世界",  // Chinese
            "Привет мир",  // Cyrillic
            "مرحبا",       // Arabic
            "🎉🎊",        // Emoji
            "café",        // Latin with diacritics (NFC)
        ];

        for text in texts {
            let hash = compute_content_hash(text);
            assert!(hash.starts_with("sha256:"));
            assert_eq!(hash.len(), 71);
        }
    }

    #[test]
    fn test_receipt_size_estimate() {
        // Create a realistic receipt
        let receipt = Receipt::lite(
            // Real fingerprint: 11 + 64 = 75 chars
            "pdftract-v1:a7f3b8c4d2e1f6a9b5c3d8e7f4a2b1c9d6e3f8a7b4c2d9e6f3a8b7c4d1e9f6a3b8".to_string(),
            14,
            [220.0, 412.0, 412.0, 432.0],
            "Net Income: $2.4M",
        );

        let json = serde_json::to_string(&receipt).unwrap();

        // Lite mode receipt should be roughly 150-180 bytes
        // This is a sanity check, not a strict requirement
        assert!(json.len() > 100, "Receipt JSON should be at least 100 bytes");
        assert!(json.len() < 300, "Receipt JSON should be less than 300 bytes in lite mode");
    }
}
