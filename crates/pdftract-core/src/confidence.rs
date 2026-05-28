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
//! # Mapping (INV-9)
//!
//! The mapping from internal [`UnicodeSource`](crate::font::UnicodeSource)
//! (6 variants) to [`ConfidenceSource`] (3 variants) is:
//!
//! | `UnicodeSource` | `corrected_in_4_7` | `ConfidenceSource` |
//! |-----------------|-------------------|-------------------|
//! | `ToUnicode`     | `false`           | `Native`          |
//! | `ToUnicode`     | `true`            | `Heuristic`       |
//! | `Agl`           | `false`           | `Native`          |
//! | `Agl`           | `true`            | `Heuristic`       |
//! | `Fingerprint`   | `false`           | `Native`          |
//! | `Fingerprint`   | `true`            | `Heuristic`       |
//! | `ShapeMatch`    | `(any)`           | `Heuristic`       |
//! | `Unknown`       | `(any)`           | `Heuristic`       |
//! | `Ocr`           | `(any)`           | `Ocr`             |
//!
//! The `corrected_in_4_7` flag indicates whether the Unicode value was
//! corrected during Phase 4.7. Corrections downgrade the confidence from
//! `Native` to `Heuristic` because the corrected value is no longer the
//! original resolution from the PDF. OCR is never affected by corrections.

use crate::font::resolver::UnicodeSource;
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

/// Map a UnicodeSource to a ConfidenceSource with optional Phase 4.7 correction.
///
/// This function collapses the 6 internal [`UnicodeSource`] variants down to
/// the 3 schema-exposed [`ConfidenceSource`] variants. The mapping is one-way
/// (multiple UnicodeSource variants map to the same ConfidenceSource).
///
/// # Arguments
///
/// * `unicode_source` - The internal Unicode source to map from
/// * `corrected_in_4_7` - Whether the Unicode was corrected during Phase 4.7
///
/// # Returns
///
/// The corresponding [`ConfidenceSource`] variant.
///
/// # Mapping Logic (INV-9)
///
/// - **Ocr** always maps to `Ocr` (not affected by corrections)
/// - **ShapeMatch** and **Unknown** always map to `Heuristic` (already heuristic)
/// - **ToUnicode**, **Agl**, and **Fingerprint**:
///   - Map to `Native` when `corrected_in_4_7` is `false`
///   - Map to `Heuristic` when `corrected_in_4_7` is `true`
///
/// The `corrected_in_4_7` flag downgrades Native sources to Heuristic because
/// a corrected Unicode value is no longer the original resolution from the PDF.
/// OCR is never affected by corrections because corrections only apply to
/// vector text, not raster OCR output.
///
/// # Examples
///
/// ```
/// use pdftract_core::confidence::{map_confidence_source, ConfidenceSource};
/// use pdftract_core::font::resolver::UnicodeSource;
///
/// // Native ToUnicode without correction
/// assert_eq!(
///     map_confidence_source(UnicodeSource::ToUnicode, false),
///     ConfidenceSource::Native
/// );
///
/// // Native ToUnicode with correction -> downgraded to Heuristic
/// assert_eq!(
///     map_confidence_source(UnicodeSource::ToUnicode, true),
///     ConfidenceSource::Heuristic
/// );
///
/// // OCR is never affected by correction
/// assert_eq!(
///     map_confidence_source(UnicodeSource::Ocr, true),
///     ConfidenceSource::Ocr
/// );
/// ```
///
/// # Compiler Exhaustiveness
///
/// This function uses an exhaustive match on [`UnicodeSource`]. If a new
/// variant is added to the enum, this function will fail to compile until
/// a match arm is added, ensuring the mapping is always complete.
pub fn map_confidence_source(unicode_source: UnicodeSource, corrected_in_4_7: bool) -> ConfidenceSource {
    match unicode_source {
        UnicodeSource::Ocr => ConfidenceSource::Ocr,
        UnicodeSource::ShapeMatch | UnicodeSource::Unknown => ConfidenceSource::Heuristic,
        UnicodeSource::ToUnicode | UnicodeSource::Agl | UnicodeSource::Fingerprint => {
            if corrected_in_4_7 {
                ConfidenceSource::Heuristic
            } else {
                ConfidenceSource::Native
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::resolver::UnicodeSource;

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

    // Tests for map_confidence_source

    #[test]
    fn test_map_tounicode_without_correction() {
        assert_eq!(
            map_confidence_source(UnicodeSource::ToUnicode, false),
            ConfidenceSource::Native
        );
    }

    #[test]
    fn test_map_tounicode_with_correction_downgrades_to_heuristic() {
        // Phase 4.7 correction override: Native -> Heuristic
        assert_eq!(
            map_confidence_source(UnicodeSource::ToUnicode, true),
            ConfidenceSource::Heuristic
        );
    }

    #[test]
    fn test_map_agl_without_correction() {
        assert_eq!(
            map_confidence_source(UnicodeSource::Agl, false),
            ConfidenceSource::Native
        );
    }

    #[test]
    fn test_map_agl_with_correction_downgrades_to_heuristic() {
        assert_eq!(
            map_confidence_source(UnicodeSource::Agl, true),
            ConfidenceSource::Heuristic
        );
    }

    #[test]
    fn test_map_fingerprint_without_correction() {
        assert_eq!(
            map_confidence_source(UnicodeSource::Fingerprint, false),
            ConfidenceSource::Native
        );
    }

    #[test]
    fn test_map_fingerprint_with_correction_downgrades_to_heuristic() {
        assert_eq!(
            map_confidence_source(UnicodeSource::Fingerprint, true),
            ConfidenceSource::Heuristic
        );
    }

    #[test]
    fn test_map_shapematch_always_heuristic() {
        // ShapeMatch is always Heuristic, regardless of correction
        assert_eq!(
            map_confidence_source(UnicodeSource::ShapeMatch, false),
            ConfidenceSource::Heuristic
        );
        assert_eq!(
            map_confidence_source(UnicodeSource::ShapeMatch, true),
            ConfidenceSource::Heuristic
        );
    }

    #[test]
    fn test_map_unknown_always_heuristic() {
        // Unknown (U+FFFD) is always Heuristic, regardless of correction
        assert_eq!(
            map_confidence_source(UnicodeSource::Unknown, false),
            ConfidenceSource::Heuristic
        );
        assert_eq!(
            map_confidence_source(UnicodeSource::Unknown, true),
            ConfidenceSource::Heuristic
        );
    }

    #[test]
    fn test_map_ocr_always_cr_unaffected_by_correction() {
        // OCR is always Ocr, corrections do NOT apply to OCR
        assert_eq!(
            map_confidence_source(UnicodeSource::Ocr, false),
            ConfidenceSource::Ocr
        );
        assert_eq!(
            map_confidence_source(UnicodeSource::Ocr, true),
            ConfidenceSource::Ocr
        );
    }

    #[test]
    fn test_map_all_combinations() {
        // Comprehensive test of all (UnicodeSource, corrected) combinations
        let test_cases = &[
            (UnicodeSource::ToUnicode, false, ConfidenceSource::Native),
            (UnicodeSource::ToUnicode, true, ConfidenceSource::Heuristic),
            (UnicodeSource::Agl, false, ConfidenceSource::Native),
            (UnicodeSource::Agl, true, ConfidenceSource::Heuristic),
            (UnicodeSource::Fingerprint, false, ConfidenceSource::Native),
            (UnicodeSource::Fingerprint, true, ConfidenceSource::Heuristic),
            (UnicodeSource::ShapeMatch, false, ConfidenceSource::Heuristic),
            (UnicodeSource::ShapeMatch, true, ConfidenceSource::Heuristic),
            (UnicodeSource::Unknown, false, ConfidenceSource::Heuristic),
            (UnicodeSource::Unknown, true, ConfidenceSource::Heuristic),
            (UnicodeSource::Ocr, false, ConfidenceSource::Ocr),
            (UnicodeSource::Ocr, true, ConfidenceSource::Ocr),
        ];

        for (source, corrected, expected) in test_cases {
            assert_eq!(
                map_confidence_source(*source, *corrected),
                *expected,
                "map_confidence_source({:?}, {}) should be {:?}",
                source, corrected, expected
            );
        }
    }
}
