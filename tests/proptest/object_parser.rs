//! Property-based tests for the PDF object parser.
//!
//! These tests verify that the object parser maintains its core invariants
//! across all possible inputs, following INV-8 (no panic at public boundary).

use pdftract_core::parser::object::ObjectParser;

/// Property: The object parser never panics on any input.
///
/// This is the most fundamental property of the object parser: it must be total
/// over its input domain. Any panic here is a violation of INV-8.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_never_panics_on_random_bytes(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..10_000)
    ) {
        // This should never panic - if it does, INV-8 is violated
        let mut parser = ObjectParser::new(&bytes);
        let _ = parser.parse_direct_object();
    }
}

/// Property: parse_indirect_object never panics on any input.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_parse_indirect_object_never_panics(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..10_000)
    ) {
        // This should never panic - if it does, INV-8 is violated
        let mut parser = ObjectParser::new(&bytes);
        let _ = parser.parse_indirect_object();
    }
}

/// Property: Diagnostics are never None/null for any input.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_always_returns_some_result_or_eof(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..1000)
    ) {
        let mut parser = ObjectParser::new(&bytes);
        // parse_direct_object always returns Some(obj) or None (EOF), never panics
        match parser.parse_direct_object() {
            Some(_) => {}, // Valid object
            None => {}, // EOF
        }
    }
}

/// Property: Nested structures don't cause stack overflow.
///
/// This test generates deeply nested structures and verifies that
/// the depth limit (256) prevents stack overflow while still
/// producing valid partial results.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_deeply_nested_structures_safe(
        depth in 0usize..500
    ) {
        // Create a deeply nested structure
        let mut input = String::new();
        for _ in 0..depth {
            input.push_str("<< /A ");
        }
        input.push_str("1");
        for _ in 0..depth {
            input.push_str(" >>");
        }

        let mut parser = ObjectParser::new(input.as_bytes());
        // Should not panic even at depth 500 (returns partial result at 256)
        let _ = parser.parse_direct_object();
    }
}

/// Property: Arrays with random elements don't panic.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_array_with_random_elements_no_panic(
        elements in proptest::collection::vec(
            proptest::collection::vec(proptest::num::u8::ANY, 0..50),
            0..100
        )
    ) {
        // Create an array with random byte sequences as elements
        let mut input = String::from("[");
        for (i, elem) in elements.iter().enumerate() {
            if i > 0 {
                input.push_str(" ");
            }
            // Try to interpret as integer, fall back to treating as keyword
            let s = String::from_utf8_lossy(elem);
            input.push_str(&s);
        }
        input.push_str("]");

        let mut parser = ObjectParser::new(input.as_bytes());
        // Should not panic
        let _ = parser.parse_direct_object();
    }
}

/// Property: Dictionaries with random key-value pairs don't panic.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_dict_with_random_kv_no_panic(
        kv_pairs in proptest::collection::vec(
            (proptest::collection::vec(proptest::num::u8::ANY, 0..20),
             proptest::collection::vec(proptest::num::u8::ANY, 0..20)),
            0..50
        )
    ) {
        // Create a dict with random key-value byte sequences
        let mut input = String::from("<<");
        for (key, value) in kv_pairs.iter() {
            let key_str = String::from_utf8_lossy(key);
            let value_str = String::from_utf8_lossy(value);
            input.push_str(&format!(" /{} {} ", key_str, value_str));
        }
        input.push_str(">>");

        let mut parser = ObjectParser::new(input.as_bytes());
        // Should not panic
        let _ = parser.parse_direct_object();
    }
}

/// Property: Position tracking is monotonic.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_position_monotonically_increases(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..1000)
    ) {
        let mut parser = ObjectParser::new(&bytes);
        let mut last_pos = parser.position();

        loop {
            match parser.parse_direct_object() {
                Some(_) => {
                    let current_pos = parser.position();
                    prop_assert!(current_pos >= last_pos,
                        "Position decreased from {} to {}", last_pos, current_pos);
                    last_pos = current_pos;
                }
                None => break,
            }
        }
    }
}

/// Property: Indirect object pattern (N G obj ... endobj) doesn't panic.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_indirect_object_pattern_no_panic(
        obj_num in 0u32..1000u32,
        gen_num in 0u16..100u16,
        body in proptest::collection::vec(proptest::num::u8::ANY, 0..500)
    ) {
        let body_str = String::from_utf8_lossy(&body);
        let input = format!("{} {} obj {} endobj", obj_num, gen_num, body_str);

        let mut parser = ObjectParser::new(input.as_bytes());
        // Should not panic for any valid header
        let _ = parser.parse_indirect_object();
    }
}

/// Property: Malformed indirect object headers don't panic.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_malformed_indirect_headers_no_panic(
        header in proptest::collection::vec(proptest::num::u8::ANY, 0..100)
    ) {
        let header_str = String::from_utf8_lossy(&header);
        let input = format!("{} obj null endobj", header_str);

        let mut parser = ObjectParser::new(input.as_bytes());
        // Should not panic even with completely invalid headers
        let _ = parser.parse_indirect_object();
    }
}

/// Property: Stream parsing doesn't panic on random data.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_stream_parsing_no_panic(
        dict_content in proptest::collection::vec(proptest::num::u8::ANY, 0..200),
        stream_data in proptest::collection::vec(proptest::num::u8::ANY, 0..1000)
    ) {
        let dict_str = String::from_utf8_lossy(&dict_content);
        let input = format!("<< {} >> stream\n{}endstream", dict_str,
            String::from_utf8_lossy(&stream_data));

        let mut parser = ObjectParser::new(input.as_bytes());
        // Should not panic even with malformed streams
        let _ = parser.parse_direct_object();
    }
}

/// Property: Missing endobj doesn't cause infinite loop.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_missing_endobj_no_infinite_loop(
        obj_num in 0u32..100u32,
        gen_num in 0u16..10u16,
        body in proptest::collection::vec(proptest::num::u8::ANY, 0..200)
    ) {
        let body_str = String::from_utf8_lossy(&body);
        // Missing endobj - should recover and return
        let input = format!("{} {} obj {}", obj_num, gen_num, body_str);

        let mut parser = ObjectParser::new(input.as_bytes());
        // Should not infinite loop or panic
        let result = parser.parse_indirect_object();
        // Should either parse something or return None
        match result {
            Some(_) | None => {},
        }
    }
}

/// Property: take_diagnostics is idempotent.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_take_diagnostics_idempotent(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..1000)
    ) {
        let mut parser = ObjectParser::new(&bytes);
        // Parse something
        let _ = parser.parse_direct_object();

        let _diags1 = parser.take_diagnostics();
        let diags2 = parser.take_diagnostics();

        prop_assert!(diags2.is_empty(),
            "Second take_diagnostics() should return empty, got {} diagnostics",
            diags2.len());
    }
}
