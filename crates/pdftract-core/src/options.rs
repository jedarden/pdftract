//! Extraction options for PDF processing.
//!
//! This module defines the options that control how PDFs are extracted,
//! including the receipts mode for cryptographic provenance tracking.

use serde::{Deserialize, Serialize};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;

/// Receipt generation mode.
///
/// Controls whether visual citation receipts are generated during extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ReceiptsMode {
    /// No receipts generated (default).
    Off,
    /// Lite mode: minimal receipts (~120 bytes each) with fingerprint, page index, bbox, and content hash.
    Lite,
    /// SVG mode: extended receipts that include an SVG clip rendering the glyphs.
    #[serde(rename = "svg")]
    SvgClip,
}

impl Default for ReceiptsMode {
    fn default() -> Self {
        ReceiptsMode::Off
    }
}

impl ReceiptsMode {
    /// Parse a string value into a ReceiptsMode.
    ///
    /// Accepts: "off", "lite", "svg"
    ///
    /// # Examples
    ///
    /// ```
    /// use pdftract_core::options::ReceiptsMode;
    ///
    /// assert_eq!(ReceiptsMode::from_str("off"), Ok(ReceiptsMode::Off));
    /// assert_eq!(ReceiptsMode::from_str("lite"), Ok(ReceiptsMode::Lite));
    /// assert_eq!(ReceiptsMode::from_str("svg"), Ok(ReceiptsMode::SvgClip));
    /// assert!(ReceiptsMode::from_str("bogus").is_err());
    /// ```
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "off" => Ok(ReceiptsMode::Off),
            "lite" => Ok(ReceiptsMode::Lite),
            "svg" => Ok(ReceiptsMode::SvgClip),
            _ => Err(format!(
                "invalid receipts mode: '{}', expected 'off', 'lite', or 'svg'",
                s
            )),
        }
    }

    /// Convert to a lowercase string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            ReceiptsMode::Off => "off",
            ReceiptsMode::Lite => "lite",
            ReceiptsMode::SvgClip => "svg",
        }
    }
}

/// Options that control PDF extraction behavior.
///
/// This struct is passed through the extraction pipeline and controls
/// optional features like receipt generation and parallelism limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExtractionOptions {
    /// Receipt generation mode.
    pub receipts: ReceiptsMode,
    /// Maximum number of pages to process in parallel.
    ///
    /// This caps the number of simultaneously-resident pages to keep memory
    /// bounded regardless of core count. The per-page memory budget is:
    /// `memory_budget_mb / max_parallel_pages`.
    ///
    /// Default: 4 (conservative for memory-constrained environments)
    pub max_parallel_pages: usize,
    /// Memory budget in MB for the entire document extraction.
    ///
    /// This is the target peak RSS for processing the entire document.
    /// The per-page budget is derived from this divided by max_parallel_pages.
    ///
    /// Default: 512 MB (matches the plan's Tier 1 target for 100-page PDFs)
    pub memory_budget_mb: usize,
    /// Enable full-render path using PDFium for complex page rendering.
    ///
    /// When true, pages are rendered using PDFium which correctly handles
    /// overlapping images, soft masks, blend modes, and other complex geometry.
    /// When false or when the `full-render` feature is not compiled in,
    /// the direct compositing path is used (which handles >90% of scanned PDFs).
    ///
    /// Default: false (direct compositing path)
    ///
    /// # Feature Gate
    ///
    /// This option has no effect unless the `full-render` feature is enabled.
    /// When the feature is absent, this field is silently ignored and the
    /// direct compositing path is always used.
    pub full_render: bool,
    /// Override DPI for OCR rendering (Phase 5.2).
    ///
    /// When set, this value overrides the automatic DPI selection algorithm.
    /// Useful for debugging or for documents with known DPI requirements.
    ///
    /// Default: None (automatic selection based on font size and image filters)
    ///
    /// # DPI Selection Algorithm
    ///
    /// When not overridden, DPI is selected as follows:
    /// - JBIG2 images present: 200 DPI (already binary)
    /// - Median font size < 7.0 pt: 400 DPI (fine print)
    /// - Otherwise: 300 DPI (standard body text)
    pub ocr_dpi_override: Option<u32>,
    /// OCR language codes to load for Tesseract (Phase 5.4).
    ///
    /// Each language code corresponds to a `<code>.traineddata` file in the
    /// tessdata directory. Multiple languages can be specified for multi-language
    /// documents; Tesseract will attempt recognition with all loaded languages.
    ///
    /// Default: vec!["eng"] (English)
    ///
    /// # Language codes
    ///
    /// ISO 639-2/3 codes are used: "eng" (English), "fra" (French), "deu" (German),
    /// "spa" (Spanish), "jpn" (Japanese), "chi_sim" (Simplified Chinese), etc.
    ///
    /// # Missing language handling
    ///
    /// If a requested language pack is not installed, extraction proceeds with
    /// an OCR_LANGUAGE_UNAVAILABLE diagnostic and falls back to eng if available.
    /// Run `pdftract doctor tesseract-langs` to verify installed languages.
    ///
    /// # Docker image variants
    ///
    /// - `pdftract:default`: No language packs bundled (OCR not available)
    /// - `pdftract:ocr`: Bundles eng + common languages (~150 MB)
    /// - `pdftract:full`: Bundles all 100+ languages (~600 MB)
    ///
    /// See docs/notes/ocr-language-packs.md for the full distribution strategy.
    pub ocr_language: Vec<String>,
}

impl Default for ExtractionOptions {
    fn default() -> Self {
        Self {
            receipts: ReceiptsMode::default(),
            max_parallel_pages: Self::default_max_parallel_pages(),
            memory_budget_mb: Self::default_memory_budget_mb(),
            full_render: false,
            ocr_dpi_override: None,
            ocr_language: vec!["eng".to_string()],
        }
    }
}

impl ExtractionOptions {
    /// Get the default max_parallel_pages from environment or use conservative default.
    ///
    /// Reads from PDFTRACT_MAX_PARALLEL_PAGES env var, or defaults to 4.
    fn default_max_parallel_pages() -> usize {
        std::env::var("PDFTRACT_MAX_PARALLEL_PAGES")
            .ok()
            .and_then(|s| s.parse().ok())
            .filter(|&n| n > 0)
            .unwrap_or(4)
    }

    /// Get the default memory_budget_mb from environment or use plan target.
    ///
    /// Reads from PDFTRACT_MEMORY_BUDGET_MB env var, or defaults to 512 MB.
    fn default_memory_budget_mb() -> usize {
        std::env::var("PDFTRACT_MEMORY_BUDGET_MB")
            .ok()
            .and_then(|s| s.parse().ok())
            .filter(|&n| n >= 64) // Minimum 64 MB
            .unwrap_or(512)
    }

    /// Create a new ExtractionOptions with the specified receipts mode.
    pub fn with_receipts(receipts: ReceiptsMode) -> Self {
        Self {
            receipts,
            ocr_dpi_override: None,
            ocr_language: vec!["eng".to_string()],
            ..Default::default()
        }
    }

    /// Create a new ExtractionOptions with receipts mode from a string.
    pub fn with_receipts_str(receipts: &str) -> Result<Self, String> {
        Ok(Self {
            receipts: ReceiptsMode::from_str(receipts)?,
            ocr_dpi_override: None,
            ocr_language: vec!["eng".to_string()],
            ..Default::default()
        })
    }

    /// Calculate the per-page memory budget in bytes.
    ///
    /// This is the memory ceiling divided by max_parallel_pages, representing
    /// the maximum memory each page extraction should use.
    pub fn per_page_budget_bytes(&self) -> usize {
        (self.memory_budget_mb * 1024 * 1024) / self.max_parallel_pages
    }

    /// Create a new ExtractionOptions with custom parallelism settings.
    pub fn with_parallelism(max_parallel_pages: usize, memory_budget_mb: usize) -> Self {
        Self {
            max_parallel_pages: max_parallel_pages.max(1),
            memory_budget_mb: memory_budget_mb.max(64),
            ocr_dpi_override: None,
            ocr_language: vec!["eng".to_string()],
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receipts_mode_from_str() {
        assert_eq!(ReceiptsMode::from_str("off"), Ok(ReceiptsMode::Off));
        assert_eq!(ReceiptsMode::from_str("lite"), Ok(ReceiptsMode::Lite));
        assert_eq!(ReceiptsMode::from_str("svg"), Ok(ReceiptsMode::SvgClip));
        assert_eq!(ReceiptsMode::from_str("OFF"), Ok(ReceiptsMode::Off));
        assert_eq!(ReceiptsMode::from_str("LITE"), Ok(ReceiptsMode::Lite));
        assert_eq!(ReceiptsMode::from_str("SVG"), Ok(ReceiptsMode::SvgClip));
    }

    #[test]
    fn test_receipts_mode_from_str_invalid() {
        assert!(ReceiptsMode::from_str("bogus").is_err());
        assert!(ReceiptsMode::from_str("").is_err());
        assert!(ReceiptsMode::from_str("on").is_err());
    }

    #[test]
    fn test_receipts_mode_as_str() {
        assert_eq!(ReceiptsMode::Off.as_str(), "off");
        assert_eq!(ReceiptsMode::Lite.as_str(), "lite");
        assert_eq!(ReceiptsMode::SvgClip.as_str(), "svg");
    }

    #[test]
    fn test_receipts_mode_default() {
        assert_eq!(ReceiptsMode::default(), ReceiptsMode::Off);
    }

    #[test]
    fn test_extraction_options_default() {
        let opts = ExtractionOptions::default();
        assert_eq!(opts.receipts, ReceiptsMode::Off);
    }

    #[test]
    fn test_extraction_options_with_receipts() {
        let opts = ExtractionOptions::with_receipts(ReceiptsMode::Lite);
        assert_eq!(opts.receipts, ReceiptsMode::Lite);
    }

    #[test]
    fn test_extraction_options_with_receipts_str() {
        let opts = ExtractionOptions::with_receipts_str("lite").unwrap();
        assert_eq!(opts.receipts, ReceiptsMode::Lite);

        let opts = ExtractionOptions::with_receipts_str("svg").unwrap();
        assert_eq!(opts.receipts, ReceiptsMode::SvgClip);

        assert!(ExtractionOptions::with_receipts_str("bogus").is_err());
    }

    #[test]
    fn test_receipts_mode_serialize() {
        let mode = ReceiptsMode::Lite;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"lite\"");

        let mode = ReceiptsMode::SvgClip;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"svg\"");

        let mode = ReceiptsMode::Off;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"off\"");
    }

    #[test]
    fn test_receipts_mode_deserialize() {
        let mode: ReceiptsMode = serde_json::from_str("\"lite\"").unwrap();
        assert_eq!(mode, ReceiptsMode::Lite);

        let mode: ReceiptsMode = serde_json::from_str("\"svg\"").unwrap();
        assert_eq!(mode, ReceiptsMode::SvgClip);

        let mode: ReceiptsMode = serde_json::from_str("\"off\"").unwrap();
        assert_eq!(mode, ReceiptsMode::Off);
    }

    #[test]
    fn test_extraction_options_serialize() {
        let opts = ExtractionOptions::with_receipts(ReceiptsMode::Lite);
        let json = serde_json::to_string(&opts).unwrap();
        assert!(json.contains("\"receipts\""));
        assert!(json.contains("\"lite\""));
    }

    #[test]
    fn test_extraction_options_deserialize() {
        let json = "{\"receipts\":\"lite\"}";
        let opts: ExtractionOptions = serde_json::from_str(json).unwrap();
        assert_eq!(opts.receipts, ReceiptsMode::Lite);

        let json = "{}";
        let opts: ExtractionOptions = serde_json::from_str(json).unwrap();
        assert_eq!(opts.receipts, ReceiptsMode::Off);
    }

    #[test]
    fn test_extraction_options_default_parallelism() {
        let opts = ExtractionOptions::default();
        assert_eq!(opts.max_parallel_pages, 4);
        assert_eq!(opts.memory_budget_mb, 512);
    }

    #[test]
    fn test_per_page_budget_calculation() {
        // 512 MB / 4 pages = 128 MB per page
        let opts = ExtractionOptions::with_parallelism(4, 512);
        assert_eq!(opts.per_page_budget_bytes(), 128 * 1024 * 1024);

        // 256 MB / 2 pages = 128 MB per page
        let opts = ExtractionOptions::with_parallelism(2, 256);
        assert_eq!(opts.per_page_budget_bytes(), 128 * 1024 * 1024);

        // 1024 MB / 8 pages = 128 MB per page
        let opts = ExtractionOptions::with_parallelism(8, 1024);
        assert_eq!(opts.per_page_budget_bytes(), 128 * 1024 * 1024);
    }

    #[test]
    fn test_with_parallelism_clamps_minimums() {
        // max_parallel_pages should be at least 1
        let opts = ExtractionOptions::with_parallelism(0, 512);
        assert_eq!(opts.max_parallel_pages, 1);

        // memory_budget_mb should be at least 64
        let opts = ExtractionOptions::with_parallelism(4, 0);
        assert_eq!(opts.memory_budget_mb, 64);
    }

    #[test]
    fn test_extraction_options_default_ocr_language() {
        let opts = ExtractionOptions::default();
        assert_eq!(opts.ocr_language, vec!["eng"]);
    }

    #[test]
    fn test_extraction_options_serialize_ocr_language() {
        let json = "{\"ocr_language\":[\"eng\",\"fra\"]}";
        let opts: ExtractionOptions = serde_json::from_str(json).unwrap();
        assert_eq!(opts.ocr_language, vec!["eng", "fra"]);
    }

    #[test]
    fn test_extraction_options_deserialize_ocr_language_default() {
        let json = "{}";
        let opts: ExtractionOptions = serde_json::from_str(json).unwrap();
        assert_eq!(opts.ocr_language, vec!["eng"]);
    }
}
