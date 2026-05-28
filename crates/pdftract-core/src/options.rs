//! Extraction options for PDF processing.
//!
//! This module defines the options that control how PDFs are extracted,
//! including the receipts mode for cryptographic provenance tracking.

#[cfg(feature = "schemars")]
use schemars::JsonSchema;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};

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

/// Output filtering options for Phase 4.6.
///
/// Controls which block kinds and span types are included in extraction output.
/// Per INV-1: defaults exclude; flags ADD content. 95% of users want body text only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct OutputOptions {
    /// Include header blocks in output.
    ///
    /// Headers are detected via cross-page deduplication (Phase 4.4).
    /// Default: false (headers excluded from output).
    pub include_headers: bool,

    /// Include footer blocks in output.
    ///
    /// Footers are detected via cross-page deduplication (Phase 4.4).
    /// Default: false (footers excluded from output).
    pub include_footers: bool,

    /// Include watermark blocks in output.
    ///
    /// Watermarks are detected in Phase 7. Prior to Phase 7, this is a no-op
    /// (no watermark blocks are emitted by the pipeline).
    /// Default: false (watermarks excluded from output).
    pub include_watermarks: bool,

    /// Include invisible text spans in output.
    ///
    /// Invisible text has rendering_mode == 3 (invisible to rendering).
    /// Default: false (invisible spans excluded from output).
    pub include_invisible: bool,

    /// Include hidden-layer text spans in output.
    ///
    /// Hidden layers are controlled by Glyph.is_hidden (Phase 3.4 OCG).
    /// Default: false (hidden-layer spans excluded from output).
    pub include_hidden_layers: bool,

    /// Include structural metadata blocks in output.
    ///
    /// Structural metadata comes from tagged PDF StructTree (Phase 7.1).
    /// Prior to Phase 7.1, this is a no-op (no struct metadata blocks emitted).
    /// Default: false (struct metadata excluded from output).
    pub include_struct_metadata: bool,
}

impl Default for OutputOptions {
    fn default() -> Self {
        Self {
            include_headers: false,
            include_footers: false,
            include_watermarks: false,
            include_invisible: false,
            include_hidden_layers: false,
            include_struct_metadata: false,
        }
    }
}

impl OutputOptions {
    /// Check if a block kind should be included in output.
    ///
    /// Returns false if the block kind is filtered out by options.
    ///
    /// # Arguments
    ///
    /// * `kind` - The block kind string (e.g., "header", "footer", "watermark")
    ///
    /// # Returns
    ///
    /// true if the block should be included, false if filtered out.
    pub fn include_block_kind(&self, kind: &str) -> bool {
        match kind {
            "header" => self.include_headers,
            "footer" => self.include_footers,
            "watermark" => self.include_watermarks,
            _ => true, // All other block kinds are included by default
        }
    }

    /// Check if a span should be included in output.
    ///
    /// Returns false if the span is filtered out by options.
    ///
    /// # Arguments
    ///
    /// * `rendering_mode` - Optional rendering mode (Some(3) for invisible text)
    /// * `is_hidden` - Whether the span is in a hidden layer (OCG)
    ///
    /// # Returns
    ///
    /// true if the span should be included, false if filtered out.
    pub fn include_span(&self, rendering_mode: Option<u8>, is_hidden: bool) -> bool {
        // Filter invisible text (rendering_mode == 3)
        if let Some(3) = rendering_mode {
            if !self.include_invisible {
                return false;
            }
        }

        // Filter hidden layers
        if is_hidden && !self.include_hidden_layers {
            return false;
        }

        true
    }

    /// Set both include_headers and include_footers to true.
    ///
    /// Convenience method for --include-headers-footers CLI flag.
    pub fn include_headers_and_footers(&mut self) {
        self.include_headers = true;
        self.include_footers = true;
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

    /// Emit HTML comment anchors before each block in Markdown output (Phase 6.5).
    ///
    /// When enabled, each block in markdown output is preceded by a single-line
    /// HTML comment containing positional metadata:
    ///
    /// ```markdown
    /// <!-- pdftract: page=3 block=12 bbox=[72.0,640.5,540.0,672.0] kind=heading -->
    /// ## Chapter 3
    /// ```
    ///
    /// This allows downstream tools (LLM agents, audit tools, document Q&A systems)
    /// to map a Markdown excerpt back to a precise PDF location. HTML comments
    /// are passthrough in every major Markdown renderer (GitHub, GitLab, Obsidian,
    /// Notion import, pulldown-cmark, marked, markdown-it).
    ///
    /// Default: false (anchors disabled)
    pub markdown_anchors: bool,

    /// Maximum decompressed bytes allowed per document (bomb limit).
    ///
    /// This limit prevents zip-bomb attacks where a small compressed PDF expands
    /// to multi-GB of decompressed data. When the decompressed size exceeds this
    /// limit, a STREAM_BOMB diagnostic is emitted and extraction fails.
    ///
    /// Default: 512 MiB (DEFAULT_MAX_DECOMPRESS_BYTES)
    ///
    /// # Security implications
    ///
    /// - This limit applies to the entire document, not per-stream
    /// - All compressed streams (content streams, embedded files, object streams)
    ///   contribute to this counter
    /// - Exceeding the limit triggers a hard error (STREAM_BOMB diagnostic)
    /// - Set to 0 to disable decompression limits (NOT RECOMMENDED for production)
    pub max_decompress_bytes: u64,

    /// Output filtering options (Phase 4.6).
    ///
    /// Controls which block kinds and span types are included in output.
    pub output: OutputOptions,

    /// Page range specification (1-based, comma-separated).
    ///
    /// When set, only the specified pages are extracted. The format accepts:
    /// - Single pages: "1", "3", "7"
    /// - Closed ranges: "1-5" (pages 1-5 inclusive)
    /// - Open-start ranges: "-5" (equivalent to "1-5")
    /// - Open-end ranges: "12-" (page 12 to end)
    /// - Comma-separated: "1-5,7,12-15"
    ///
    /// Out-of-range page numbers emit PAGE_OUT_OF_RANGE diagnostics but
    /// do not abort extraction. Pages are 1-based (user-facing) but converted
    /// to 0-based indices internally.
    ///
    /// Default: None (all pages extracted)
    pub pages: Option<String>,

    /// PDF password for encrypted documents.
    ///
    /// When set, this password is used to decrypt the PDF before extraction.
    /// The password is kept in a SecretString to prevent accidental exposure
    /// in logs or error messages.
    ///
    /// Default: None (no password; tries empty password first per PDF spec)
    ///
    /// # Password priority
    ///
    /// The extraction flow attempts passwords in this order:
    /// 1. Empty string (for documents with empty owner password)
    /// 2. The password from this field, if set
    ///
    /// If both attempts fail, an ENCRYPTION_UNSUPPORTED diagnostic is emitted
    /// and extraction fails with exit code 3.
    #[serde(skip)]
    pub password: Option<SecretString>,

    /// Custom HTTP headers for remote PDF sources.
    ///
    /// When the input is an HTTP/HTTPS URL, these headers are included in all
    /// HTTP requests (HEAD and Range). This is useful for API keys, authentication
    /// tokens, and other custom headers required by remote PDF hosts.
    ///
    /// Headers are silently ignored for local file extraction.
    ///
    /// Default: None (no custom headers)
    ///
    /// # Header format
    ///
    /// Each header is a tuple of (name, value). Headers are validated before use:
    /// - Name must match [A-Za-z0-9_-]+ (HTTP token format)
    /// - No CRLF characters in name or value (HTTP injection protection)
    /// - Managed headers (Host, Content-Length, etc.) are rejected
    ///
    /// # Example
    ///
    /// ```ignore
    /// let headers = vec![
    ///     ("Authorization".to_string(), "Bearer token123".to_string()),
    ///     ("X-API-Key".to_string(), "secret-key".to_string()),
    /// ];
    /// options.http_headers = Some(headers);
    /// ```
    #[serde(skip)]
    pub http_headers: Option<Vec<(String, String)>>,
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
            markdown_anchors: false,
            max_decompress_bytes: crate::parser::stream::DEFAULT_MAX_DECOMPRESS_BYTES,
            output: OutputOptions::default(),
            pages: None,
            password: None,
            http_headers: None,
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
            markdown_anchors: false,
            output: OutputOptions::default(),
            pages: None,
            password: None,
            http_headers: None,
            ..Default::default()
        }
    }

    /// Create a new ExtractionOptions with receipts mode from a string.
    pub fn with_receipts_str(receipts: &str) -> Result<Self, String> {
        Ok(Self {
            receipts: ReceiptsMode::from_str(receipts)?,
            ocr_dpi_override: None,
            ocr_language: vec!["eng".to_string()],
            markdown_anchors: false,
            output: OutputOptions::default(),
            pages: None,
            password: None,
            http_headers: None,
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
            markdown_anchors: false,
            output: OutputOptions::default(),
            pages: None,
            password: None,
            http_headers: None,
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

    #[test]
    fn test_output_options_default() {
        let opts = OutputOptions::default();
        assert!(!opts.include_headers);
        assert!(!opts.include_footers);
        assert!(!opts.include_watermarks);
        assert!(!opts.include_invisible);
        assert!(!opts.include_hidden_layers);
        assert!(!opts.include_struct_metadata);
    }

    #[test]
    fn test_output_options_include_block_kind() {
        let opts = OutputOptions::default();

        // All filtered out by default
        assert!(!opts.include_block_kind("header"));
        assert!(!opts.include_block_kind("footer"));
        assert!(!opts.include_block_kind("watermark"));

        // Other block kinds are included by default
        assert!(opts.include_block_kind("paragraph"));
        assert!(opts.include_block_kind("heading"));
        assert!(opts.include_block_kind("table"));
        assert!(opts.include_block_kind("list"));
    }

    #[test]
    fn test_output_options_include_block_kind_with_flags() {
        let mut opts = OutputOptions::default();
        opts.include_headers = true;
        opts.include_footers = true;
        opts.include_watermarks = true;

        assert!(opts.include_block_kind("header"));
        assert!(opts.include_block_kind("footer"));
        assert!(opts.include_block_kind("watermark"));
    }

    #[test]
    fn test_output_options_include_span() {
        let opts = OutputOptions::default();

        // Invisible text filtered out by default
        assert!(!opts.include_span(Some(3), false));

        // Hidden layers filtered out by default
        assert!(!opts.include_span(Some(0), true));

        // Normal spans included
        assert!(opts.include_span(Some(0), false));
        assert!(opts.include_span(None, false));
    }

    #[test]
    fn test_output_options_include_span_with_flags() {
        let mut opts = OutputOptions::default();
        opts.include_invisible = true;
        opts.include_hidden_layers = true;

        // Now they're included
        assert!(opts.include_span(Some(3), false));
        assert!(opts.include_span(Some(0), true));
    }

    #[test]
    fn test_output_options_include_headers_and_footers() {
        let mut opts = OutputOptions::default();
        assert!(!opts.include_headers);
        assert!(!opts.include_footers);

        opts.include_headers_and_footers();
        assert!(opts.include_headers);
        assert!(opts.include_footers);
    }

    #[test]
    fn test_output_options_serialize() {
        let opts = OutputOptions::default();
        let json = serde_json::to_string(&opts).unwrap();

        // Verify all fields are present
        assert!(json.contains("\"include_headers\""));
        assert!(json.contains("\"include_footers\""));
        assert!(json.contains("\"include_watermarks\""));
        assert!(json.contains("\"include_invisible\""));
        assert!(json.contains("\"include_hidden_layers\""));
        assert!(json.contains("\"include_struct_metadata\""));
    }

    #[test]
    fn test_output_options_deserialize() {
        let json = r#"{"include_headers":true,"include_footers":true}"#;
        let opts: OutputOptions = serde_json::from_str(json).unwrap();

        assert!(opts.include_headers);
        assert!(opts.include_footers);
        assert!(!opts.include_watermarks);
    }

    #[test]
    fn test_extraction_options_with_output() {
        let json = r#"{"output":{"include_headers":true}}"#;
        let opts: ExtractionOptions = serde_json::from_str(json).unwrap();

        assert!(opts.output.include_headers);
        assert!(!opts.output.include_footers);
    }

    #[test]
    fn test_extraction_options_default_output() {
        let opts = ExtractionOptions::default();
        assert!(!opts.output.include_headers);
        assert!(!opts.output.include_footers);
    }
}
