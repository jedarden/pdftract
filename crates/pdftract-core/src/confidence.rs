//! Confidence categorization for extracted text spans.
//!
//! This module defines the [`ConfidenceSource`] enum, which provides a stable,
//! three-variant taxonomy for categorizing the source of confidence values
//! assigned to extracted text spans. This categorization is exposed in the
//! output schema (Phase 6.1) and enables downstream consumers such as
//! dashboards, audit tools, and RAG pipelines to filter or highlight
//! low-confidence text.
//!
//! # Stability
//!
//! The variant set and serialized string representations are **frozen** by
//! the 6.1 JSON schema version. Adding or removing variants constitutes a
//! breaking change to the public API.
//!
//! # Mapping
//!
//! The mapping from internal [`UnicodeSource`](crate::font::UnicodeSource)
//! (6 variants) to [`ConfidenceSource`] (3 variants) is:
//!
//! | `UnicodeSource` | `ConfidenceSource` |
//! |-----------------|-------------------|
//! | `ToUnicode`     | `Native`          |
//! | `Agl`           | `Native`          |
//! | `Fingerprint`   | `Native`          |
//! | `ShapeMatch`    | `Heuristic`       |
//! | `Unknown` (U+FFFD) | `Heuristic`   |
//! | OCR path        | `Ocr`             |

use serde::{Deserialize, Serialize};

/// The source of confidence for an extracted text span.
///
/// This enum provides a stable, three-variant taxonomy for categorizing
/// confidence values. It is exposed in the JSON output schema and enables
/// downstream consumers to make decisions based on confidence provenance.
///
/// # Variants
///
/// - **`Native`**: Confidence derived from the PDF's native encoding
///   mechanisms (ToUnicode CMaps, Adobe Glyph List, font fingerprinting).
///   This represents the highest-confidence extraction path.
///
/// - **`Heuristic`**: Confidence derived from algorithmic recovery methods
///   (shape matching, encoding detection) or fallback to the Unicode
///   replacement character (U+FFFD). These methods have lower reliability
///   than native encoding.
///
/// - **`Ocr`**: Confidence derived from optical character recognition
///   (Tesseract). OCR confidence is generally lower than native text and
///   varies based on scan quality, resolution, and language models.
///
/// # Serialization
///
/// Variants serialize to lowercase strings for JSON output:
///
/// ```json
/// { "confidence_source": "native" }
/// { "confidence_source": "heuristic" }
/// { "confidence_source": "ocr" }
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfidenceSource {
    /// Native PDF encoding: ToUnicode CMap, Adobe Glyph List, or font fingerprinting.
    Native,
    /// Heuristic recovery: shape matching, encoding detection, or U+FFFD fallback.
    Heuristic,
    /// Optical character recognition via Tesseract.
    Ocr,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_lowercase() {
        assert_eq!(
            serde_json::to_string(&ConfidenceSource::Native).unwrap(),
            r#""native""#
        );
        assert_eq!(
            serde_json::to_string(&ConfidenceSource::Heuristic).unwrap(),
            r#""heuristic""#
        );
        assert_eq!(
            serde_json::to_string(&ConfidenceSource::Ocr).unwrap(),
            r#""ocr""#
        );
    }

    #[test]
    fn test_deserialize_lowercase() {
        assert_eq!(
            serde_json::from_str::<ConfidenceSource>(r#""native""#).unwrap(),
            ConfidenceSource::Native
        );
        assert_eq!(
            serde_json::from_str::<ConfidenceSource>(r#""heuristic""#).unwrap(),
            ConfidenceSource::Heuristic
        );
        assert_eq!(
            serde_json::from_str::<ConfidenceSource>(r#""ocr""#).unwrap(),
            ConfidenceSource::Ocr
        );
    }

    #[test]
    fn test_roundtrip() {
        for variant in &[
            ConfidenceSource::Native,
            ConfidenceSource::Heuristic,
            ConfidenceSource::Ocr,
        ] {
            let serialized = serde_json::to_string(variant).unwrap();
            let deserialized: ConfidenceSource = serde_json::from_str(&serialized).unwrap();
            assert_eq!(*variant, deserialized);
        }
    }

    #[test]
    fn test_hash_map_usable() {
        use std::collections::HashMap;

        let mut counts: HashMap<ConfidenceSource, usize> = HashMap::new();
        counts.insert(ConfidenceSource::Native, 10);
        counts.insert(ConfidenceSource::Heuristic, 5);
        counts.insert(ConfidenceSource::Ocr, 2);

        assert_eq!(counts[&ConfidenceSource::Native], 10);
        assert_eq!(counts[&ConfidenceSource::Heuristic], 5);
        assert_eq!(counts[&ConfidenceSource::Ocr], 2);
    }
}
