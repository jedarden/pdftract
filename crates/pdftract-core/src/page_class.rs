//! Page classification enum.
//!
//! This module defines the four canonical page classes used throughout pdftract's
//! extraction pipeline. Per **INV-9 (stable taxonomy)**, these four variants are the
//! complete and stable set; adding new variants requires a schema_version bump and
//! an ADR.
//!
//! The `PageClass` enum drives routing decisions in Phase 5:
//! - `Vector`: Clean text PDF, extract via content-stream parsing
//! - `Scanned`: Image-only pages, require OCR
//! - `Hybrid`: Mixed text and image regions, require hybrid extraction
//! - `BrokenVector`: Text with encoding issues (e.g., invisible text layer over scan),
//!   may escalate to OCR
//!
//! # Serde representation
//!
//! The enum serializes to the variant name verbatim (`Vector`, `Scanned`, `Hybrid`,
//! `BrokenVector`). This internal representation is distinct from the `page_type`
//! strings emitted in JSON output (see Phase 5.1.1 page_type mapping table).

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Classification result for a single page, combining the class with confidence
/// and optional hybrid-cell metadata.
///
/// This struct bundles three pieces of per-page metadata:
/// - `class`: The canonical page class (Vector, Scanned, Hybrid, BrokenVector)
/// - `confidence`: Classifier confidence in `[0.0, 1.0]` (for Phase 5.5 escalation thresholds)
/// - `hybrid_cells`: For Hybrid pages, the set of image-heavy cells on the 8×8 grid
///
/// Per INV-8, the constructor validates confidence range via `debug_assert` in dev
/// builds; production code with out-of-range confidence should clamp silently.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PageClassification {
    /// The canonical page class.
    pub class: PageClass,
    /// Classifier confidence in `[0.0, 1.0]`.
    pub confidence: f32,
    /// For Hybrid pages, the set of image-heavy cells (row, col) on the 8×8 grid.
    /// `None` for non-Hybrid classes per the invariant below.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hybrid_cells: Option<BTreeSet<(u8, u8)>>,
}

impl PageClassification {
    /// Construct a new `PageClassification`.
    ///
    /// # Invariant
    ///
    /// - `confidence` must be in `[0.0, 1.0]`. In dev builds, this is enforced via
    ///   `debug_assert!`; in release builds, out-of-range values should be clamped
    ///   by the caller (per INV-8).
    /// - `hybrid_cells` should be `Some` only when `class == PageClass::Hybrid`.
    ///   The type system permits other combinations, but they represent bugs.
    ///
    /// # Panics
    ///
    /// In debug builds, panics if `confidence` is outside `[0.0, 1.0]`.
    #[must_use]
    pub fn new(
        class: PageClass,
        confidence: f32,
        hybrid_cells: Option<BTreeSet<(u8, u8)>>,
    ) -> Self {
        debug_assert!(
            0.0 <= confidence && confidence <= 1.0,
            "confidence must be in [0.0, 1.0], got {confidence}"
        );
        Self {
            class,
            confidence,
            hybrid_cells,
        }
    }
}

/// The four canonical page classes.
///
/// Per INV-9 (stable taxonomy), this enum is fixed at these four variants.
/// Adding new variants requires a schema_version bump and an ADR.
///
/// # Hash
///
/// This type derives `Hash` so it can be used as a key in `HashMap` and `HashSet`,
/// which is required for Phase 6.9 cache keying and Phase 5 routing tables.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PageClass {
    /// Clean vector PDF with readable text encoding.
    Vector,

    /// Image-only page requiring OCR.
    Scanned,

    /// Mixed page with both vector text and image regions.
    Hybrid,

    /// Text present but encoding is broken (e.g., invisible text over scanned image).
    BrokenVector,
}

impl PageClass {
    /// Returns the JSON output string for this page type.
    ///
    /// Maps internal enum values to the schema's `page_type` field.
    pub fn as_type_str(&self) -> &'static str {
        match self {
            PageClass::Vector => "text",
            PageClass::Scanned => "scanned",
            PageClass::Hybrid => "mixed",
            PageClass::BrokenVector => "broken_vector",
        }
    }

    /// Check if this page class is eligible for BrokenVector escalation.
    ///
    /// Only Vector pages can be escalated to BrokenVector based on readability.
    /// Scanned and Hybrid pages are already handled by other paths.
    pub fn can_escalate_to_broken_vector(&self) -> bool {
        matches!(self, PageClass::Vector)
    }
}

/// Compute the canonical page_type string for the JSON schema output.
///
/// This function implements the stable mapping from (PageClass, ocr_succeeded, has_text, has_images)
/// to the page_type string emitted in the 6.1 JSON schema. The mapping is frozen per INV-9.
///
/// # Mapping Table
///
/// | class           | ocr_succeeded | has_text | has_images | page_type        |
/// |-----------------|---------------|----------|------------|------------------|
/// | Vector          | -             | -        | -          | "text"           |
/// | Scanned         | -             | -        | -          | "scanned"        |
/// | Hybrid          | -             | -        | -          | "mixed"          |
/// | BrokenVector    | false         | -        | -          | "broken_vector"  |
/// | BrokenVector    | true          | -        | -          | "scanned"        | // post-OCR recovery
/// | (any)           | -             | false    | false      | "blank"          | // overrides class
/// | (any)           | -             | false    | true       | "figure_only"    | // overrides class
///
/// # Precedence Rules
///
/// 1. **Override checks first**: If `has_text == false` and `has_images == false`, return "blank".
///    If `has_text == false` and `has_images == true`, return "figure_only".
///    These overrides apply regardless of the PageClass value.
/// 2. **Class-based mapping**: If no override applies, map based on PageClass:
///    - Vector → "text"
///    - Scanned → "scanned"
///    - Hybrid → "mixed"
///    - BrokenVector with `ocr_succeeded == true` → "scanned" (post-OCR recovery)
///    - BrokenVector with `ocr_succeeded == false` → "broken_vector"
///
/// # Arguments
///
/// * `class` - The PageClass from Phase 5.1 classification
/// * `ocr_succeeded` - Whether OCR successfully recovered text (only relevant for BrokenVector)
/// * `has_text` - Whether the page contains any text glyphs
/// * `has_images` - Whether the page contains any images
///
/// # Returns
///
/// The canonical page_type string as a static str. This string is guaranteed to be
/// one of the six values in the 6.1 JSON schema enum: "text", "scanned", "mixed",
/// "broken_vector", "blank", or "figure_only".
///
/// # INV-9 Stable Taxonomy
///
/// The page_type strings are FROZEN by the 6.1 schema version. Any change requires
/// a schema_version bump and a downstream migration plan. Do not modify this function
/// without updating the JSON schema and plan.md.
pub fn page_type_string(
    class: PageClass,
    ocr_succeeded: bool,
    has_text: bool,
    has_images: bool,
) -> &'static str {
    // Override checks take precedence over class-based mapping.
    // These represent the "blank" and "figure_only" page types which are
    // determined solely by content presence, not by classification.
    if !has_text && !has_images {
        return "blank";
    }
    if !has_text && has_images {
        return "figure_only";
    }

    // Class-based mapping (applies when has_text == true or the override didn't match).
    match class {
        PageClass::Vector => "text",
        PageClass::Scanned => "scanned",
        PageClass::Hybrid => "mixed",
        PageClass::BrokenVector => {
            if ocr_succeeded {
                "scanned" // Post-OCR recovery: treated as scanned
            } else {
                "broken_vector"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let variants = [
            PageClass::Vector,
            PageClass::Scanned,
            PageClass::Hybrid,
            PageClass::BrokenVector,
        ];

        for variant in variants {
            // Serialize to JSON
            let json = serde_json::to_string(&variant).expect("serialize failed");
            let expected = match variant {
                PageClass::Vector => "\"Vector\"",
                PageClass::Scanned => "\"Scanned\"",
                PageClass::Hybrid => "\"Hybrid\"",
                PageClass::BrokenVector => "\"BrokenVector\"",
            };
            assert_eq!(json, expected);

            // Deserialize roundtrip
            let deserialized: PageClass = serde_json::from_str(&json).expect("deserialize failed");
            assert_eq!(deserialized, variant);
        }
    }

    #[test]
    fn test_pageclass_hashable() {
        use std::collections::HashMap;
        use std::hash::Hash;

        // Verify Hash trait is implemented and usable
        let mut map: HashMap<PageClass, String> = HashMap::new();
        map.insert(PageClass::Vector, "text".to_string());
        map.insert(PageClass::Scanned, "scanned".to_string());
        map.insert(PageClass::Hybrid, "mixed".to_string());
        map.insert(PageClass::BrokenVector, "broken_vector".to_string());

        assert_eq!(map.len(), 4);
        assert_eq!(map.get(&PageClass::Vector), Some(&"text".to_string()));

        // Verify Hash::hash does not panic
        use std::hash::Hasher;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        PageClass::Vector.hash(&mut hasher);
        PageClass::Scanned.hash(&mut hasher);
    }
}

#[cfg(test)]
mod page_classification_tests {
    use super::*;

    #[test]
    fn test_page_classification_new_vector() {
        // Unit test: PageClassification::new(Vector, 0.85, None) constructs successfully
        let classification = PageClassification::new(PageClass::Vector, 0.85, None);
        assert_eq!(classification.class, PageClass::Vector);
        assert_eq!(classification.confidence, 0.85);
        assert!(classification.hybrid_cells.is_none());
    }

    #[test]
    fn test_page_classification_serialize_hybrid_with_cells() {
        // Unit test: serialize PageClassification { class: Hybrid, confidence: 0.9, hybrid_cells: Some(...) }
        let mut cells = BTreeSet::new();
        cells.insert((0, 0));
        cells.insert((1, 2));
        cells.insert((7, 7));

        let classification = PageClassification::new(PageClass::Hybrid, 0.9, Some(cells.clone()));
        let json = serde_json::to_string(&classification).expect("serialize failed");

        // Verify JSON contains hybrid_cells array
        assert!(json.contains("\"hybrid_cells\""));
        assert!(json.contains("[[0,0],[1,2],[7,7]]"));

        // Deserialize roundtrip → equal
        let deserialized: PageClassification =
            serde_json::from_str(&json).expect("deserialize failed");
        assert_eq!(deserialized.class, PageClass::Hybrid);
        assert_eq!(deserialized.confidence, 0.9);
        assert_eq!(deserialized.hybrid_cells, Some(cells));
    }

    #[test]
    fn test_page_classification_hybrid_cells_none_omitted_from_json() {
        // Unit test: hybrid_cells: None is omitted from JSON output via skip_serializing_if
        let classification = PageClassification::new(PageClass::Vector, 0.85, None);
        let json = serde_json::to_string(&classification).expect("serialize failed");

        // Verify hybrid_cells key is NOT present in JSON
        assert!(!json.contains("hybrid_cells"));

        // Deserialize roundtrip still works (Option defaults to None)
        let deserialized: PageClassification =
            serde_json::from_str(&json).expect("deserialize failed");
        assert_eq!(deserialized, classification);
    }

    #[test]
    #[should_panic(expected = "confidence must be in [0.0, 1.0]")]
    #[cfg(debug_assertions)]
    fn test_page_classification_debug_assert_fires_on_invalid_confidence() {
        // Unit test: debug_assert fires on confidence = 1.5 in dev build
        // This test only runs in debug builds where debug_assert! is active
        let _ = PageClassification::new(PageClass::Vector, 1.5, None);
    }

    #[test]
    fn test_page_classification_btree_set_deterministic_order() {
        // Unit test: BTreeSet provides deterministic iteration order
        let mut cells = BTreeSet::new();
        cells.insert((7, 7));
        cells.insert((0, 0));
        cells.insert((3, 2));
        cells.insert((1, 5));

        let classification = PageClassification::new(PageClass::Hybrid, 0.9, Some(cells));
        let json = serde_json::to_string(&classification).expect("serialize failed");

        // BTreeSet iterates in sorted order, so JSON should have sorted cells
        // Extract the cells array from JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse failed");
        let cells_array = parsed["hybrid_cells"]
            .as_array()
            .expect("hybrid_cells should be array");

        // Verify sorted order: (0,0), (1,5), (3,2), (7,7)
        assert_eq!(cells_array[0], serde_json::json!([0, 0]));
        assert_eq!(cells_array[1], serde_json::json!([1, 5]));
        assert_eq!(cells_array[2], serde_json::json!([3, 2]));
        assert_eq!(cells_array[3], serde_json::json!([7, 7]));
    }

    #[test]
    fn test_page_classification_roundtrip_all_variants() {
        // Roundtrip test: serialize -> deserialize PageClassification == original
        let test_cases = [
            (PageClass::Vector, 0.85, None),
            (PageClass::Scanned, 0.72, None),
            (PageClass::BrokenVector, 0.60, None),
            (
                PageClass::Hybrid,
                0.90,
                Some(BTreeSet::from([(0, 0), (3, 3)])),
            ),
            (PageClass::Hybrid, 0.75, Some(BTreeSet::new())), // Empty cells
        ];

        for (class, confidence, hybrid_cells) in test_cases {
            let original = PageClassification::new(class, confidence, hybrid_cells.clone());
            let json = serde_json::to_string(&original).expect("serialize failed");
            let deserialized: PageClassification =
                serde_json::from_str(&json).expect("deserialize failed");
            assert_eq!(deserialized.class, original.class);
            assert_eq!(deserialized.confidence, original.confidence);
            assert_eq!(deserialized.hybrid_cells, original.hybrid_cells);
        }
    }

    #[test]
    fn test_page_classification_invariant_hybrid_cells_only_for_hybrid() {
        // Verify the invariant: hybrid_cells should only be Some for Hybrid class
        // This test documents the expected invariant; the type system allows
        // violations but they represent bugs.
        let vector_with_cells =
            PageClassification::new(PageClass::Vector, 0.8, Some(BTreeSet::from([(0, 0)])));

        // This is technically allowed by the type system but violates the invariant
        assert_eq!(vector_with_cells.class, PageClass::Vector);
        assert!(vector_with_cells.hybrid_cells.is_some());

        // In production code, callers should enforce: hybrid_cells.is_some() ⇔ class == Hybrid
    }
}

#[cfg(test)]
mod page_type_string_tests {
    use super::*;

    #[test]
    fn test_page_type_string_vector() {
        // AC: Vector → "text"
        assert_eq!(
            page_type_string(PageClass::Vector, false, true, false),
            "text"
        );
        assert_eq!(
            page_type_string(PageClass::Vector, true, true, false),
            "text"
        );
        assert_eq!(
            page_type_string(PageClass::Vector, false, true, true),
            "text"
        );
    }

    #[test]
    fn test_page_type_string_scanned() {
        // AC: Scanned → "scanned"
        assert_eq!(
            page_type_string(PageClass::Scanned, false, true, false),
            "scanned"
        );
        assert_eq!(
            page_type_string(PageClass::Scanned, true, true, false),
            "scanned"
        );
    }

    #[test]
    fn test_page_type_string_hybrid() {
        // AC: Hybrid → "mixed"
        assert_eq!(
            page_type_string(PageClass::Hybrid, false, true, true),
            "mixed"
        );
        assert_eq!(
            page_type_string(PageClass::Hybrid, true, true, true),
            "mixed"
        );
    }

    #[test]
    fn test_page_type_string_broken_vector_ocr_failed() {
        // AC: BrokenVector + ocr_succeeded=false → "broken_vector"
        assert_eq!(
            page_type_string(PageClass::BrokenVector, false, true, false),
            "broken_vector"
        );
        assert_eq!(
            page_type_string(PageClass::BrokenVector, false, true, true),
            "broken_vector"
        );
    }

    #[test]
    fn test_page_type_string_broken_vector_ocr_succeeded() {
        // AC: BrokenVector + ocr_succeeded=true → "scanned" (post-OCR recovery)
        assert_eq!(
            page_type_string(PageClass::BrokenVector, true, true, false),
            "scanned"
        );
        assert_eq!(
            page_type_string(PageClass::BrokenVector, true, true, true),
            "scanned"
        );
    }

    #[test]
    fn test_page_type_string_blank_override() {
        // AC: has_text=false + has_images=false → "blank" (overrides class)
        assert_eq!(
            page_type_string(PageClass::Vector, false, false, false),
            "blank"
        );
        assert_eq!(
            page_type_string(PageClass::Scanned, false, false, false),
            "blank"
        );
        assert_eq!(
            page_type_string(PageClass::Hybrid, false, false, false),
            "blank"
        );
        assert_eq!(
            page_type_string(PageClass::BrokenVector, false, false, false),
            "blank"
        );
        assert_eq!(
            page_type_string(PageClass::BrokenVector, true, false, false),
            "blank"
        );
    }

    #[test]
    fn test_page_type_string_figure_only_override() {
        // AC: has_text=false + has_images=true → "figure_only" (overrides class)
        assert_eq!(
            page_type_string(PageClass::Vector, false, false, true),
            "figure_only"
        );
        assert_eq!(
            page_type_string(PageClass::Scanned, false, false, true),
            "figure_only"
        );
        assert_eq!(
            page_type_string(PageClass::Hybrid, false, false, true),
            "figure_only"
        );
        assert_eq!(
            page_type_string(PageClass::BrokenVector, false, false, true),
            "figure_only"
        );
        assert_eq!(
            page_type_string(PageClass::BrokenVector, true, false, true),
            "figure_only"
        );
    }

    #[test]
    fn test_page_type_string_exhaustive_combinations() {
        // AC: Every combination from the mapping table produces the documented string
        // 4 classes × 2 ocr_succeeded × 2 has_text × 2 has_images = 32 cases

        let all_classes = [
            PageClass::Vector,
            PageClass::Scanned,
            PageClass::Hybrid,
            PageClass::BrokenVector,
        ];

        for &class in &all_classes {
            for &ocr_succeeded in &[false, true] {
                for &has_text in &[false, true] {
                    for &has_images in &[false, true] {
                        let result = page_type_string(class, ocr_succeeded, has_text, has_images);

                        // Verify result is one of the six valid enum values
                        assert!(
                            matches!(
                                result,
                                "text" | "scanned" | "mixed" | "broken_vector" | "blank" | "figure_only"
                            ),
                            "Invalid page_type: '{}' for class={:?}, ocr={}, has_text={}, has_images={}",
                            result,
                            class,
                            ocr_succeeded,
                            has_text,
                            has_images
                        );

                        // Verify override rules
                        if !has_text && !has_images {
                            assert_eq!(result, "blank");
                        } else if !has_text && has_images {
                            assert_eq!(result, "figure_only");
                        } else {
                            // Class-based mapping
                            match class {
                                PageClass::Vector => assert_eq!(result, "text"),
                                PageClass::Scanned => assert_eq!(result, "scanned"),
                                PageClass::Hybrid => assert_eq!(result, "mixed"),
                                PageClass::BrokenVector => {
                                    if ocr_succeeded {
                                        assert_eq!(result, "scanned");
                                    } else {
                                        assert_eq!(result, "broken_vector");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_page_type_enum_schema_set() {
        // Schema list test asserting page_type enum exactly equals
        // { text, scanned, mixed, broken_vector, blank, figure_only }
        let expected = [
            "text",
            "scanned",
            "mixed",
            "broken_vector",
            "blank",
            "figure_only",
        ];

        // Verify all expected values are produced by page_type_string
        let mut found = std::collections::HashSet::new();
        let all_classes = [
            PageClass::Vector,
            PageClass::Scanned,
            PageClass::Hybrid,
            PageClass::BrokenVector,
        ];

        for &class in &all_classes {
            for &ocr_succeeded in &[false, true] {
                for &has_text in &[false, true] {
                    for &has_images in &[false, true] {
                        let result = page_type_string(class, ocr_succeeded, has_text, has_images);
                        found.insert(result);
                    }
                }
            }
        }

        // Verify all expected values are present
        for &expected_value in &expected {
            assert!(
                found.contains(expected_value),
                "Expected page_type '{}' not found in output set",
                expected_value
            );
        }

        // Verify no unexpected values are present
        assert_eq!(
            found.len(),
            expected.len(),
            "page_type set has unexpected values: {:?}",
            found
        );
    }

    #[test]
    fn test_page_class_as_type_str() {
        assert_eq!(PageClass::Vector.as_type_str(), "text");
        assert_eq!(PageClass::Scanned.as_type_str(), "scanned");
        assert_eq!(PageClass::Hybrid.as_type_str(), "mixed");
        assert_eq!(PageClass::BrokenVector.as_type_str(), "broken_vector");
    }

    #[test]
    fn test_page_class_can_escalate_to_broken_vector() {
        // AC: Vector pages can escalate to BrokenVector
        assert!(PageClass::Vector.can_escalate_to_broken_vector());
        // AC: Scanned pages cannot escalate
        assert!(!PageClass::Scanned.can_escalate_to_broken_vector());
        // AC: Hybrid pages cannot escalate
        assert!(!PageClass::Hybrid.can_escalate_to_broken_vector());
        // AC: BrokenVector pages cannot escalate (already there)
        assert!(!PageClass::BrokenVector.can_escalate_to_broken_vector());
    }
}
