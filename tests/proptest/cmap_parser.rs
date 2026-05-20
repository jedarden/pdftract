//! Property-based tests for the PDF CMap parser.
//!
//! These tests verify that CMap parsing foundations (name and string handling)
//! maintain their core invariants across all possible inputs, following INV-8
//! (no panic at public boundary).
//!
//! Note: Full CMap parser is not yet implemented. These tests focus on the
//! lexer's name and string handling which are foundational to CMap parsing.

use pdftract_core::parser::lexer::{Lexer, Token};

/// Property: Name tokens never panic on any input.
///
/// CMap files contain many name tokens (e.g., /CIDInit, /CMapName).
/// The lexer must handle these without panicking.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_name_tokens_never_panic(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..10_000)
    ) {
        let mut lexer = Lexer::new(&bytes);

        loop {
            match lexer.next_token() {
                Some(Token::Eof) | None => break,
                Some(_) => {
                    // Any token is fine, we're checking for panics
                }
            }
        }
    }
}

/// Property: Hex string parsing never panics.
///
/// CMap uses hex strings extensively for character mappings.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_hex_string_never_panics(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..10_000)
    ) {
        let mut lexer = Lexer::new(&bytes);

        loop {
            match lexer.next_token() {
                Some(Token::Eof) | None => break,
                Some(Token::HexString(_)) => {
                    // Hex string parsed successfully
                }
                Some(_) => {
                    // Other tokens are fine
                }
            }
        }
    }
}

/// Property: Literal string parsing never panics.
///
/// CMap also uses literal strings for certain mappings.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_literal_string_never_panics(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..10_000)
    ) {
        let mut lexer = Lexer::new(&bytes);

        loop {
            match lexer.next_token() {
                Some(Token::Eof) | None => break,
                Some(Token::String(_)) => {
                    // String parsed successfully
                }
                Some(_) => {
                    // Other tokens are fine
                }
            }
        }
    }
}

/// Property: CMap-specific keywords don't cause panics.
///
/// CMap files have specific keywords like /CMapType, /WMode, etc.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_cmap_keywords_no_panic(
        prefix in proptest::collection::vec(proptest::num::u8::ANY, 0..100),
        keyword in prop_oneof![
            Just(b"/CMapName"),
            Just(b"/CMapType"),
            Just(b"/WMode"),
            Just(b"/CIDInit"),
            Just(b"/CIDSystemInfo"),
        ],
        suffix in proptest::collection::vec(proptest::num::u8::ANY, 0..100)
    ) {
        let mut input = prefix;
        input.extend_from_slice(keyword);
        input.extend_from_slice(&suffix);

        let mut lexer = Lexer::new(&input);
        let _ = lexer.next_token();
    }
}

/// Property: Mixed token types in CMap-like input don't panic.
///
/// CMap files mix dictionaries, arrays, integers, and names.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_mixed_cmap_tokens_no_panic(
        tokens in proptest::collection::vec(
            proptest::prop_oneof![
                proptest::collection::vec(proptest::num::u8::ANY, 0..20).prop_map(|b| format!("/{}", String::from_utf8_lossy(&b))),
                proptest::collection::vec(proptest::num::u8::ANY, 0..20).prop_map(|b| format!("({})", String::from_utf8_lossy(&b))),
                proptest::num::i32::ANY.prop_map(|n| n.to_string()),
                Just("<<".to_string()),
                Just(">>".to_string()),
                Just("[".to_string()),
                Just("]".to_string()),
            ],
            0..100
        )
    ) {
        let mut input = String::new();
        for token in tokens {
            input.push_str(&token);
            input.push(' ');
        }

        let mut lexer = Lexer::new(input.as_bytes());
        loop {
            match lexer.next_token() {
                Some(Token::Eof) | None => break,
                Some(_) => {}
            }
        }
    }
}

/// Property: Very long name tokens don't cause panics.
///
/// CMap can have long registry names, but names are limited to 127 bytes.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_long_name_tokens_no_panic(
        name_bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..500)
    ) {
        let mut input = vec![b'/'];
        input.extend_from_slice(&name_bytes);

        let mut lexer = Lexer::new(&input);
        let token = lexer.next_token();

        // Should either parse a truncated name or emit diagnostics, never panic
        match token {
            Some(Token::Name(_)) => {
                // Name parsed (possibly truncated to 127 bytes)
            }
            Some(_) => {
                // Other token type (diagnostic emitted)
            }
            None => {
                // EOF or error
            }
        }
    }
}

/// Property: Bracket nesting in arrays doesn't cause infinite loops.
///
/// CMap uses arrays for code ranges; ensure we handle nesting correctly.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_array_bracket_nesting_no_infinite_loop(
        open_brackets in 0usize..100,
        content in proptest::collection::vec(proptest::num::u8::ANY, 0..50)
    ) {
        let mut input = String::new();
        for _ in 0..open_brackets {
            input.push('[');
        }
        input.push_str(&String::from_utf8_lossy(&content));

        let mut lexer = Lexer::new(input.as_bytes());
        let mut iterations = 0;
        let max_iterations = 10000;

        loop {
            match lexer.next_token() {
                Some(Token::Eof) | None => break,
                Some(_) => {
                    iterations += 1;
                    if iterations > max_iterations {
                        panic!("Lexer appears to be in an infinite loop");
                    }
                }
            }
        }
    }
}

/// Property: Dictionary nesting in CMap doesn't cause panics.
///
/// CMap has nested dictionaries for CIDSystemInfo, etc.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_dict_nesting_no_panic(
        depth in 0usize..50
    ) {
        let mut input = String::new();
        for _ in 0..depth {
            input.push_str("<< /A ");
        }
        input.push_str("1");
        for _ in 0..depth {
            input.push_str(" >>");
        }

        let mut lexer = Lexer::new(input.as_bytes());
        loop {
            match lexer.next_token() {
                Some(Token::Eof) | None => break,
                Some(_) => {}
            }
        }
    }
}

/// Property: Special CMap characters in names are handled.
///
/// CMap names can contain # escapes for special characters.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_name_hex_escapes_no_panic(
        prefix in proptest::collection::vec(proptest::num::u8::ANY, 0..20),
        hex_bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..100),
        suffix in proptest::collection::vec(proptest::num::u8::ANY, 0..20)
    ) {
        let mut input = vec![b'/'];
        input.extend_from_slice(&prefix);

        // Add some # hex escapes
        for chunk in hex_bytes.chunks(2) {
            input.push(b'#');
            for &b in chunk.iter().take(2) {
                input.push(b);
            }
        }

        input.extend_from_slice(&suffix);

        let mut lexer = Lexer::new(&input);
        let _ = lexer.next_token();
    }
}

/// Property: take_diagnostics is idempotent for CMap-like inputs.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_take_diagnostics_idempotent(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..1000)
    ) {
        let mut lexer = Lexer::new(&bytes);

        while lexer.next_token().is_some() {}

        let _diags1 = lexer.take_diagnostics();
        let diags2 = lexer.take_diagnostics();

        prop_assert!(diags2.is_empty(),
            "Second take_diagnostics() should return empty, got {} diagnostics",
            diags2.len());
    }
}
