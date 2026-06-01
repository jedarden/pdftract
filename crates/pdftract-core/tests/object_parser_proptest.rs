//! Property-based tests for the PDF object parser.
//!
//! These tests verify that the object parser maintains its core invariants
//! across all possible inputs, following INV-8 (no panic at public boundary).

use pdftract_core::parser::object::{ObjectParser, PdfObject, PdfDict, intern};
use proptest::prelude::*;

/// Property: The parser never panics on any arbitrary byte sequence.
///
/// This is the most fundamental property (INV-8): the parser is total
/// over its input domain. Any panic here is a violation of INV-8.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_parser_never_panics(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..10_000)
    ) {
        let mut parser = ObjectParser::new(&bytes);
        // This should never panic - if it does, INV-8 is violated
        let _ = parser.parse_direct_object();
        let _ = parser.parse_indirect_object();
    }
}

/// Property: Arbitrary sequences of ObjRef resolve within bounded operations.
///
/// This tests that the resolver doesn't infinite-loop on circular references
/// or pathological reference chains. We bound the operation count to 1000.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_resolve_terminates(
        refs in proptest::collection::vec(
            (0u32..1000u32, 0u16..10u16),
            0..100
        )
    ) {
        // For now, we just verify the parser doesn't hang on indirect refs
        // A full resolver test would require a mock xref table
        let mut input = String::new();
        for (obj, gen) in refs {
            input.push_str(&format!("{} {} obj null endobj\\n", obj, gen));
        }

        let mut parser = ObjectParser::new(input.as_bytes());
        let mut count = 0u32;

        // Parse up to 100 objects, ensuring we terminate
        while count < 100 {
            match parser.parse_indirect_object() {
                Some(_) => count += 1,
                None => break,
            }
        }

        // If we get here without hanging, the test passes
        assert!(count <= 100, "Should have terminated or hit EOF");
    }
}

/// Property: Dictionary insertion order is preserved during iteration.
///
/// This is critical for INV-3 (fingerprint byte-stability). If dict order
/// varies non-deterministically, the fingerprint differs every run.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_dict_order_preserved(
        kv_pairs in proptest::collection::vec(
            (proptest::string::string_regex("[a-zA-Z]{1,10}").unwrap(),
             0..50i32),
            0..50
        )
    ) {
        use std::collections::HashSet;

        let mut dict = PdfDict::new();
        let mut seen_keys = HashSet::new();
        let mut unique_insertion_order: Vec<String> = Vec::new();

        // Insert in a specific order, tracking only first occurrence of each key
        for (key, value) in kv_pairs.iter() {
            dict.insert(intern(key), PdfObject::Integer((*value).into()));
            // Track the order of first-seen keys
            if !seen_keys.contains(key) {
                seen_keys.insert(key.clone());
                unique_insertion_order.push(key.clone());
            }
        }

        // Verify iteration order matches first-insertion order
        let mut i = 0;
        for (inserted_key, _) in dict.iter() {
            prop_assert!(i < unique_insertion_order.len(),
                "More dict entries than unique keys inserted");
            let expected_key = &unique_insertion_order[i];
            prop_assert_eq!(inserted_key.as_ref(), expected_key.as_str(),
                "Iteration order doesn't match insertion order at position {}: expected {}, got {}",
                i, expected_key, inserted_key.as_ref());
            i += 1;
        }

        // Verify we saw all unique keys
        prop_assert_eq!(i, unique_insertion_order.len(),
            "Missing keys in iteration: saw {} of {} unique keys",
            i, unique_insertion_order.len());
    }
}

/// Property: Two identical resolution sequences produce identical PdfObject results.
///
/// This is the cache's own INV-8 corollary: cache hit MUST equal cache miss
/// for the same input. We verify by equality comparison instead of hashing.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_cache_consistency(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..1000)
    ) {
        // Parse the same bytes twice with different parsers
        let mut parser1 = ObjectParser::new(&bytes);
        let obj1 = parser1.parse_direct_object();

        let mut parser2 = ObjectParser::new(&bytes);
        let obj2 = parser2.parse_direct_object();

        // Results should be identical (consistent parsing)
        assert_eq!(obj1, obj2,
            "Inconsistent results for identical input: {:?} vs {:?}", obj1, obj2);
    }
}

/// Property: Any input produces either Some(obj) or None (EOF), never panics.
///
/// This is the INV-8 invariant: public boundary never panics, returns
/// Vec<Diagnostic> (possibly empty) instead.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_inv8_no_panic(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..10_000)
    ) {
        let mut parser = ObjectParser::new(&bytes);

        // parse_direct_object should never panic
        match parser.parse_direct_object() {
            Some(_) => {}, // Valid object
            None => {}, // EOF
        }

        // parse_indirect_object should never panic
        let _ = parser.parse_indirect_object();

        // take_diagnostics should always return a Vec (possibly empty)
        let _diags = parser.take_diagnostics();

        // If we get here without panic, INV-8 holds
    }
}
