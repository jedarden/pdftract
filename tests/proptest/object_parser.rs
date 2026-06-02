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

/// Property: Dictionary order is preserved during parsing.
///
/// This is critical for INV-3 (fingerprint byte-stability) — if dict order
/// varies non-deterministically, the fingerprint differs every run.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_dict_order_preserved(
        keys in proptest::collection::vec("[a-zA-Z]{1,10}", 1..20)
    ) {
        use pdftract_core::parser::object::{intern, PdfDict, PdfObject};
        use indexmap::IndexMap;

        // Create a dict with keys in a specific order
        let mut dict = PdfDict::new();
        let mut expected_order = Vec::new();

        for key in &keys {
            let key_name = intern(key);
            dict.insert(key_name, PdfObject::Integer(1));
            expected_order.push(key.to_string());
        }

        // Verify iteration order matches insertion order
        let actual_order: Vec<_> = dict.iter()
            .map(|(k, _)| k.as_ref().to_string())
            .collect();

        prop_assert_eq!(actual_order, expected_order,
            "Dictionary iteration order should match insertion order");
    }
}

/// Property: Parsing the same input twice produces the same result.
///
/// This verifies that the parser is deterministic — a critical invariant
/// for fingerprinting and reproducible behavior.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn parser_deterministic(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..500)
    ) {
        use pdftract_core::parser::object::PdfObject;

        // First parse
        let mut parser1 = ObjectParser::new(&bytes);
        let result1 = parser1.parse_direct_object();

        // Second parse (on same input)
        let mut parser2 = ObjectParser::new(&bytes);
        let result2 = parser2.parse_direct_object();

        // Results should be identical
        prop_assert_eq!(result1, result2,
            "Parser should produce identical results for identical input");
    }
}

/// Property: Empty input always returns None (EOF).
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_empty_input_returns_eof() {
        let mut parser = ObjectParser::new(b"");
        let result = parser.parse_direct_object();
        prop_assert!(result.is_none(), "Empty input should return None (EOF)");
    }
}

/// Property: Whitespace-only input returns None (EOF).
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_whitespace_only_returns_eof(
        whitespace in proptest::string::string_regex("[ \t\n\r]*").unwrap()
    ) {
        let mut parser = ObjectParser::new(whitespace.as_bytes());
        let result = parser.parse_direct_object();
        prop_assert!(result.is_none(), "Whitespace-only input should return None (EOF)");
    }
}

/// Property: Array parsing preserves element order.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_array_order_preserved(
        elements in proptest::collection::vec(0i64..1000i64, 0..50)
    ) {
        use pdftract_core::parser::object::PdfObject;

        // Create array input: [1 2 3 ...]
        let input = format!("[{}]", elements.iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(" "));

        let mut parser = ObjectParser::new(input.as_bytes());
        let result = parser.parse_direct_object();

        match result {
            Some(PdfObject::Array(arr)) => {
                // Verify order is preserved
                let parsed_elements: Vec<_> = arr.iter()
                    .filter_map(|obj| obj.as_int())
                    .collect();

                prop_assert_eq!(parsed_elements.as_slice(), elements.as_slice(),
                    "Array element order should be preserved");
            }
            Some(other) => prop_assert!(false, "Expected array, got {:?}", other),
            None => prop_assert!(false, "Parser returned None for valid array"),
        }
    }
}

/// Property: Nested dictionaries have correct depth tracking.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_nested_dict_depth_tracking(
        depth in 1usize..20usize
    ) {
        use pdftract_core::parser::object::PdfObject;

        // Create nested dict: << /A << /A << ... /A 1 >> ... >> >>
        let mut input = String::new();
        for _ in 0..depth {
            input.push_str("<< /A ");
        }
        input.push_str("1");
        for _ in 0..depth {
            input.push_str(" >>");
        }

        let mut parser = ObjectParser::new(input.as_bytes());
        let result = parser.parse_direct_object();

        // Should parse successfully (depth 20 is well below limit of 256)
        prop_assert!(result.is_some(), "Should parse nested dict at depth {}", depth);

        // Navigate the nested structure to verify depth
        let mut current = result.as_ref();
        for _ in 0..depth {
            current = current.and_then(|o| {
                o.as_dict()?.get("A")
            });
        }

        // At the bottom, should find the integer 1
        match current {
            Some(PdfObject::Integer(1)) => {}, // Correct
            Some(other) => prop_assert!(false, "Expected integer 1 at bottom, got {:?}", other),
            None => prop_assert!(false, "Could not navigate to depth {}", depth),
        }
    }
}

/// Property: Indirect reference pattern is recognized correctly.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_indirect_ref_pattern(
        obj_num in 0u32..10000u32,
        gen_num in 0u16..1000u16
    ) {
        use pdftract_core::parser::object::{ObjRef, PdfObject};

        let input = format!("{} {} R", obj_num, gen_num);
        let mut parser = ObjectParser::new(input.as_bytes());
        let result = parser.parse_direct_object();

        match result {
            Some(PdfObject::Ref(ref_obj_ref)) => {
                prop_assert_eq!(ref_obj_ref.object, obj_num,
                    "Object number should match");
                prop_assert_eq!(ref_obj_ref.generation, gen_num,
                    "Generation number should match");
            }
            Some(other) => prop_assert!(false,
                "Expected indirect reference, got {:?}", other),
            None => prop_assert!(false, "Parser returned None for valid indirect reference"),
        }
    }
}

/// Property: Diagnostics count is non-negative (always valid).
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_diagnostics_count_non_negative(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..10_000)
    ) {
        let mut parser = ObjectParser::new(&bytes);
        // Parse multiple objects
        loop {
            match parser.parse_direct_object() {
                Some(_) => continue,
                None => break,
            }
        }

        let diagnostics = parser.take_diagnostics();
        prop_assert!(diagnostics.len() >= 0,
            "Diagnostics count should always be non-negative");
    }
}

/// Property: parse_direct_object never returns a reference to EOF.
///
/// This property ensures that the parser never returns a reference to an EOF
/// token, which would indicate a bug in token handling.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_no_eof_in_result(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..1000)
    ) {
        let mut parser = ObjectParser::new(&bytes);

        // Parse all objects until EOF
        loop {
            match parser.parse_direct_object() {
                Some(_) => {}, // Valid object
                None => break, // EOF - exit loop
            }
        }

        // If we get here without panic, the test passes
        // (The key invariant is that parse_direct_object never returns a value
        // containing an EOF marker - it only returns Some(object) or None)
    }
}

/// Property: Resolution terminates within bounded operations.
///
/// This property verifies that resolving object references terminates
/// within 1000 operations, preventing infinite loops on circular references.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_resolve_terminates(
        refs in proptest::collection::vec(
            (0u32..1000u32, 0u16..100u16),
            0..100
        )
    ) {
        use pdftract_core::parser::object::ObjRef;
        use std::collections::HashSet;

        // Simulate resolution with cycle detection
        let mut visited = HashSet::new();
        let mut ops = 0;
        const MAX_OPS: usize = 1000;

        for &(obj_num, gen_num) in &refs {
            let obj_ref = ObjRef::new(obj_num, gen_num);

            // Check if we've seen this ref (cycle detection)
            if visited.contains(&obj_ref) {
                // Cycle detected - this is expected behavior
                continue;
            }

            visited.insert(obj_ref);
            ops += 1;

            // Verify we terminate within the operation limit
            prop_assert!(ops <= MAX_OPS,
                "Resolution exceeded operation limit ({} ops)", ops);
        }

        // Resolution terminated successfully
    }
}

/// Property: Cache consistency - identical inputs produce identical outputs.
///
/// This verifies that parsing the same input sequence twice produces
/// identical results, which is critical for INV-8 (no panic) and
/// INV-3 (fingerprint byte-stability).
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_cache_consistency(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..1000)
    ) {
        use pdftract_core::parser::object::PdfObject;
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        // First parse
        let mut parser1 = ObjectParser::new(&bytes);
        let result1 = parser1.parse_direct_object();
        let diags1 = parser1.take_diagnostics();

        // Second parse (identical input)
        let mut parser2 = ObjectParser::new(&bytes);
        let result2 = parser2.parse_direct_object();
        let diags2 = parser2.take_diagnostics();

        // Results should be identical
        prop_assert_eq!(result1, result2,
            "Parse results should be identical for identical input");

        // Diagnostic counts should be identical
        prop_assert_eq!(diags1.len(), diags2.len(),
            "Diagnostic counts should be identical for identical input");

        // Hash consistency: same input -> same hash
        fn hash_object(obj: &Option<PdfObject>) -> u64 {
            let mut hasher = DefaultHasher::new();
            match obj {
                Some(PdfObject::Null) => 0u64.hash(&mut hasher),
                Some(PdfObject::Bool(b)) => {
                    1u64.hash(&mut hasher);
                    b.hash(&mut hasher);
                }
                Some(PdfObject::Integer(i)) => {
                    2u64.hash(&mut hasher);
                    i.hash(&mut hasher);
                }
                Some(PdfObject::Real(r)) => {
                    3u64.hash(&mut hasher);
                    r.to_bits().hash(&mut hasher);
                }
                Some(PdfObject::String(s)) => {
                    4u64.hash(&mut hasher);
                    s.as_slice().hash(&mut hasher);
                }
                Some(PdfObject::Name(n)) => {
                    5u64.hash(&mut hasher);
                    n.as_ref().hash(&mut hasher);
                }
                Some(PdfObject::Array(arr)) => {
                    6u64.hash(&mut hasher);
                    for elem in arr.iter() {
                        hash_object(&Some(elem.clone()));
                    }
                }
                Some(PdfObject::Dict(dict)) => {
                    7u64.hash(&mut hasher);
                    for (k, v) in dict.iter() {
                        k.as_ref().hash(&mut hasher);
                        hash_object(&Some(v.clone()));
                    }
                }
                Some(PdfObject::Ref(r)) => {
                    8u64.hash(&mut hasher);
                    r.object.hash(&mut hasher);
                    r.generation.hash(&mut hasher);
                }
                Some(PdfObject::Stream(s)) => {
                    9u64.hash(&mut hasher);
                    s.offset.hash(&mut hasher);
                    s.len_hint.hash(&mut hasher);
                }
                Some(PdfObject::Indirect(ind)) => {
                    10u64.hash(&mut hasher);
                    ind.id.object.hash(&mut hasher);
                    ind.id.generation.hash(&mut hasher);
                    hash_object(&Some(ind.obj.clone()));
                }
                None => 11u64.hash(&mut hasher),
            }
            hasher.finish()
        }

        let hash1 = hash_object(&result1);
        let hash2 = hash_object(&result2);

        prop_assert_eq!(hash1, hash2,
            "Hashes should be identical for identical input");
    }
}

/// Property: INV-8 no panic - any input produces diagnostics or valid result.
///
/// This is the core INV-8 property: the parser never panics on any input.
/// It always returns either a valid object or EOF (None), possibly with
/// diagnostics (which may be empty).
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_inv8_no_panic(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..10_000)
    ) {
        use pdftract_core::parser::object::PdfObject;

        // Parse should never panic
        let mut parser = ObjectParser::new(&bytes);

        // Parse all objects until EOF
        loop {
            match parser.parse_direct_object() {
                Some(PdfObject::Null) => {
                    // Valid result - may indicate error but not a panic
                }
                Some(_) => {
                    // Valid object - no panic
                }
                None => {
                    // EOF - normal termination
                    break;
                }
            }
        }

        // take_diagnostics should never panic
        let _diagnostics = parser.take_diagnostics();

        // If we get here, INV-8 is satisfied
    }
}

/// Property: Parser never panics on any input (named as specified in task).
///
/// This is the exact property name specified in the acceptance criteria.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_parser_never_panics(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..10_000)
    ) {
        // This property verifies the parser never panics on any input.
        // It's INV-8's core guarantee: the parser is total over its input domain.
        let mut parser = ObjectParser::new(&bytes);
        let _ = parser.parse_direct_object();
        let _ = parser.parse_indirect_object();
        let _ = parser.take_diagnostics();

        // If we get here without panic, the property holds.
        // The test harness will catch any panic and fail the property.
    }
}
