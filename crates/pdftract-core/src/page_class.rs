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
        PageClassification::new(PageClass::Vector, 1.5, None);
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
