//! Extraction options for PDF processing.
//!
//! This module defines the options that control how PDFs are extracted,
//! including the receipts mode for cryptographic provenance tracking.

use serde::{Deserialize, Serialize};

/// Receipt generation mode.
///
/// Controls whether visual citation receipts are generated during extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
/// optional features like receipt generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExtractionOptions {
    /// Receipt generation mode.
    pub receipts: ReceiptsMode,
}

impl Default for ExtractionOptions {
    fn default() -> Self {
        Self {
            receipts: ReceiptsMode::default(),
        }
    }
}

impl ExtractionOptions {
    /// Create a new ExtractionOptions with the specified receipts mode.
    pub fn with_receipts(receipts: ReceiptsMode) -> Self {
        Self {
            receipts,
            ..Default::default()
        }
    }

    /// Create a new ExtractionOptions with receipts mode from a string.
    pub fn with_receipts_str(receipts: &str) -> Result<Self, String> {
        Ok(Self {
            receipts: ReceiptsMode::from_str(receipts)?,
            ..Default::default()
        })
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
}
