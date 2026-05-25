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
