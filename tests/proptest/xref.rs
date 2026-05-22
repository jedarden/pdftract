//! Property-based tests for the PDF xref parser and resolver.
//!
//! These tests verify that the xref parser and resolver maintain their core
//! invariants across all possible inputs, following INV-8 (no panic at public boundary).

use pdftract_core::parser::xref::{XrefResolver, XrefEntry, XrefSection, parse_traditional_xref, forward_scan_xref, merge_hybrid};
use pdftract_core::parser::stream::MemorySource;

/// Property: XrefResolver never panics on any entry.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_xref_resolver_never_panics_on_entry(
        obj_num in 0u32..10000u32,
        offset in 0u64..1_000_000u64,
        gen_nr in 0u16..65536u16
    ) {
        let mut resolver = XrefResolver::new();
        // Adding any valid entry should not panic
        resolver.add_entry(obj_num, XrefEntry::InUse { offset, gen_nr });
    }
}

/// Property: parse_traditional_xref never panics on random input.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_parse_traditional_xref_never_panics(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..50_000)
    ) {
        let source = MemorySource::new(bytes.clone());
        // Any random input should not panic xref parsing
        let _ = parse_traditional_xref(&source, 0);
    }
}

/// Property: parse_traditional_xref with random offset never panics.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_parse_traditional_xref_random_offset_never_panics(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..50_000),
        offset in 0u64..10_000u64
    ) {
        let source = MemorySource::new(bytes);
        // Any random input and offset should not panic
        let _ = parse_traditional_xref(&source, offset);
    }
}

/// Property: forward_scan_xref never panics on random input.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_forward_scan_xref_never_panics(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..100_000)
    ) {
        let source = MemorySource::new(bytes);
        // Forward scan should never panic, even on garbage input
        let _ = forward_scan_xref(&source, false);
    }
}

/// Property: forward_scan_xref with linearized flag never panics.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_forward_scan_xref_linearized_never_panics(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..100_000),
        is_linearized in proptest::bool::ANY
    ) {
        let source = MemorySource::new(bytes);
        // Should never panic regardless of linearized flag
        let _ = forward_scan_xref(&source, is_linearized);
    }
}

/// Property: XrefEntry round-trips through add_entry and get_entry.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_xref_entry_roundtrip(
        obj_num in 0u32..10000u32,
        offset in 0u64..1_000_000u64,
        gen_nr in 0u16..65536u16
    ) {
        let mut resolver = XrefResolver::new();
        let entry = XrefEntry::InUse { offset, gen_nr };

        resolver.add_entry(obj_num, entry.clone());
        let retrieved = resolver.get_entry(obj_num);

        prop_assert_eq!(retrieved, Some(&entry));
    }
}

/// Property: is_resolving tracks correctly across resolve attempts.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_is_resolving_tracking(
        obj_num in 1u32..10000u32,
        gen_num in 0u16..65536u16
    ) {
        use pdftract_core::parser::object::ObjRef;

        let resolver = XrefResolver::new();
        let obj_ref = ObjRef::new(obj_num, gen_num);

        // Initially not resolving
        prop_assert!(!resolver.is_resolving(obj_ref));

        // Start resolving
        let started = resolver.start_resolving(obj_ref);
        prop_assert!(started);
        prop_assert!(resolver.is_resolving(obj_ref));

        // Second start fails (already resolving)
        let started_again = resolver.start_resolving(obj_ref);
        prop_assert!(!started_again);

        // Finish resolving
        resolver.finish_resolving(obj_ref);
        prop_assert!(!resolver.is_resolving(obj_ref));
    }
}

/// Property: Circular reference detection works.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_circular_ref_detection(
        obj_num in 1u32..10000u32,
        gen_num in 0u16..65536u16
    ) {
        use pdftract_core::parser::object::ObjRef;

        let resolver = XrefResolver::new();
        let obj_ref = ObjRef::new(obj_num, gen_num);

        // Start resolving
        resolver.start_resolving(obj_ref);

        // Try to resolve while already resolving -> circular ref error
        let result = resolver.resolve(obj_ref);
        prop_assert!(matches!(result, Err(_)));
    }
}

/// Property: XrefResolver handles non-existent objects gracefully.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_resolve_nonexistent_object(
        obj_num in 0u32..10000u32,
        gen_num in 0u16..65536u16
    ) {
        use pdftract_core::parser::object::ObjRef;

        let resolver = XrefResolver::new();
        let obj_ref = ObjRef::new(obj_num, gen_num);

        // Non-existent object should return NotFound error
        let result = resolver.resolve(obj_ref);
        prop_assert!(matches!(result, Err(_)));
    }
}

/// Property: XrefEntry::Free entries are handled correctly.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_free_entry_handling(
        obj_num in 0u32..10000u32,
        next_free in 0u32..10000u32,
        gen_nr in 0u16..65536u16
    ) {
        let mut resolver = XrefResolver::new();
        let entry = XrefEntry::Free { next_free, gen_nr };

        resolver.add_entry(obj_num, entry);
        let retrieved = resolver.get_entry(obj_num);

        prop_assert_eq!(retrieved, Some(&XrefEntry::Free { next_free, gen_nr }));
    }
}

/// Property: XrefEntry::Compressed entries are handled correctly.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_compressed_entry_handling(
        obj_num in 0u32..10000u32,
        obj_stm_nr in 0u32..10000u32,
        index in 0u32..10000u32
    ) {
        let mut resolver = XrefResolver::new();
        let entry = XrefEntry::Compressed { obj_stm_nr, index };

        resolver.add_entry(obj_num, entry);
        let retrieved = resolver.get_entry(obj_num);

        prop_assert_eq!(retrieved, Some(&XrefEntry::Compressed { obj_stm_nr, index }));
    }
}

/// Property: XrefResolver len() and is_empty() are consistent.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_len_empty_consistency(
        entries in proptest::collection::vec(
            (0u32..1000u32, 0u64..1_000_000u64, 0u16..1000u16),
            0..100
        )
    ) {
        let mut resolver = XrefResolver::new();

        for (obj_num, offset, gen_nr) in entries {
            resolver.add_entry(obj_num, XrefEntry::InUse { offset, gen_nr });
        }

        let is_empty = resolver.is_empty();
        let len = resolver.len();

        prop_assert_eq!(is_empty, len == 0);
    }
}

/// Property: XrefSection handles malformed xref entries gracefully.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_malformed_xref_entry_no_panic(
        prefix in proptest::collection::vec(proptest::num::u8::ANY, 0..50),
        entry_bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..50),
        suffix in proptest::collection::vec(proptest::num::u8::ANY, 0..50)
    ) {
        let mut xref_data = String::from("xref\n0 1\n");
        xref_data.push_str(&String::from_utf8_lossy(&prefix));
        xref_data.push_str(&String::from_utf8_lossy(&entry_bytes));
        xref_data.push_str(&String::from_utf8_lossy(&suffix));
        xref_data.push_str("\ntrailer\n<<>>\n");

        let source = MemorySource::new(xref_data.into_bytes());
        // Should not panic even with completely malformed entry
        let result = parse_traditional_xref(&source, 0);
        // Result should be valid (possibly empty with diagnostics)
        prop_assert!(result.entries.len() >= 0);
    }
}

/// Property: parse_traditional_xref with various xref keyword positions.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_xref_keyword_position_variations(
        leading_bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..100),
        obj_count in 0usize..10usize
    ) {
        let mut xref_data = String::from_utf8_lossy(&leading_bytes).to_string();
        xref_data.push_str("xref\n0 ");
        xref_data.push_str(&obj_count.to_string());
        xref_data.push_str("\n");

        for i in 0..obj_count {
            xref_data.push_str(&format!("000000000{:04x} 00000 n \n", i));
        }

        xref_data.push_str("trailer\n<<>>\n");

        let source = MemorySource::new(xref_data.into_bytes());
        // Should not panic regardless of leading bytes
        let _ = parse_traditional_xref(&source, 0);
    }
}

/// Property: Xref with multiple subsections doesn't panic.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_multiple_subsections_no_panic(
        subsections in proptest::collection::vec(
            (0u32..100u32, 0usize..20usize),
            0..10
        )
    ) {
        let mut xref_data = String::from("xref\n");

        for (start, count) in subsections {
            xref_data.push_str(&format!("{} {}\n", start, count));
            for _ in 0..count {
                xref_data.push_str("0000000000 00000 n \n");
            }
        }

        xref_data.push_str("trailer\n<<>>\n");

        let source = MemorySource::new(xref_data.into_bytes());
        // Should not panic with any number of subsections
        let _ = parse_traditional_xref(&source, 0);
    }

    /// Property: merge_hybrid never panics on random xref sections.
    #[test]
    fn prop_merge_hybrid_never_panics(
        trad_entries in proptest::collection::vec(
            (0u32..1000u32, 0u64..1_000_000u64, 0u16..1000u16),
            0..50
        ),
        stream_entries in proptest::collection::vec(
            (0u32..1000u32, 0u32..1000u32, 0u32..1000u32),
            0..50
        ),
    ) {
        use pdftract_core::parser::xref::{XrefSection, XrefEntry, merge_hybrid};

        let mut traditional = XrefSection::new();
        for (obj_num, offset, gen_nr) in trad_entries {
            traditional.add_entry(obj_num, XrefEntry::InUse { offset, gen_nr });
        }

        let mut stream = XrefSection::new();
        for (obj_num, obj_stm_nr, index) in stream_entries {
            stream.add_entry(obj_num, XrefEntry::Compressed { obj_stm_nr, index });
        }

        // Should never panic on any combination of sections
        let merged = merge_hybrid(traditional, stream);
        prop_assert!(merged.is_hybrid);
    }
}
