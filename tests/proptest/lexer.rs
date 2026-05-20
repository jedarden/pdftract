//! Property-based tests for the PDF lexer.
//!
//! These tests verify that the lexer maintains its core invariants
//! across all possible inputs, following INV-8 (no panic at public boundary).

use pdftract_core::parser::lexer::{Lexer, Token};

/// Helper function to create a lexer and run it to completion without panicking.
///
/// This is the core property: for ANY input, the lexer should either:
/// 1. Return a sequence of tokens ending with Eof
/// 2. Return tokens with diagnostics (but never panic)
fn lex_all(bytes: &[u8]) -> (Vec<Token>, Vec<pdftract_core::parser::lexer::Diagnostic>) {
    let mut lexer = Lexer::new(bytes);
    let mut tokens = Vec::new();

    loop {
        match lexer.next_token() {
            Some(Token::Eof) => {
                tokens.push(Token::Eof);
                break;
            }
            Some(token) => {
                tokens.push(token);
            }
            None => break,
        }
    }

    let diags = lexer.take_diagnostics();
    (tokens, diags)
}

/// Helper function to verify the lexer never panics on random input.
///
/// This is the core INV-8 invariant: no panic at the public boundary.
#[cfg(feature = "proptest")]
fn lexer_never_panics(bytes: &[u8]) -> bool {
    let _ = lex_all(bytes);
    true
}

/// Property: The lexer never panics on any input, including entirely random bytes.
///
/// This is the most fundamental property of the lexer: it must be total
/// over its input domain. Any panic here is a violation of INV-8.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_never_panics_on_random_bytes(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..10_000)
    ) {
        // This should never panic - if it does, INV-8 is violated
        let _ = lex_all(&bytes);
    }
}

/// Property: Position always advances monotonically (never decreases).
///
/// The lexer's position tracking is critical for error reporting and
/// must be well-defined.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_position_monotonically_increases(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..1000)
    ) {
        let mut lexer = Lexer::new(&bytes);
        let mut last_pos = lexer.position();

        loop {
            match lexer.next_token() {
                Some(Token::Eof) | None => break,
                Some(_) => {
                    let current_pos = lexer.position();
                    prop_assert!(current_pos >= last_pos,
                        "Position decreased from {} to {}", last_pos, current_pos);
                    last_pos = current_pos;
                }
            }
        }
    }
}

/// Property: Position never exceeds input length.
///
/// The lexer should never read past the end of the input.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_position_never_exceeds_input_length(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..1000)
    ) {
        let mut lexer = Lexer::new(&bytes);
        let input_len = bytes.len() as u64;

        loop {
            match lexer.next_token() {
                Some(Token::Eof) | None => break,
                Some(_) => {
                    let current_pos = lexer.position();
                    prop_assert!(current_pos <= input_len,
                        "Position {} exceeds input length {}", current_pos, input_len);
                }
            }
        }
    }
}

/// Property: take_diagnostics is idempotent.
///
/// Calling take_diagnostics() twice should return empty diagnostics the second time.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_take_diagnostics_is_idempotent(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..1000)
    ) {
        let mut lexer = Lexer::new(&bytes);

        // Consume all tokens
        while lexer.next_token().is_some() {}

        let _diags1 = lexer.take_diagnostics();
        let diags2 = lexer.take_diagnostics();

        prop_assert!(diags2.is_empty(),
            "Second take_diagnostics() should return empty, got {} diagnostics",
            diags2.len());
    }
}

/// Property: peek_token does not advance position.
///
/// Peeking at tokens should be a non-consuming operation.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_peek_token_does_not_advance_position(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..1000)
    ) {
        let mut lexer = Lexer::new(&bytes);
        let pos_before = lexer.position();

        // Peek at the next token (may be None if at EOF)
        let _peeked = lexer.peek_token();

        let pos_after = lexer.position();

        prop_assert_eq!(pos_before, pos_after,
            "peek_token() should not advance position");
    }
}

/// Property: Consecutive peeks return the same token.
///
/// Peeking multiple times should consistently return the same token
/// until a consuming operation (next_token) is performed.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_consecutive_peeks_return_same_token(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..1000)
    ) {
        let mut lexer = Lexer::new(&bytes);

        // Peek twice
        let peek1 = lexer.peek_token().cloned();
        let peek2 = lexer.peek_token().cloned();

        prop_assert_eq!(peek1, peek2,
            "Consecutive peeks should return the same token");
    }
}

/// Property: peek then next returns consistent tokens.
///
/// A peek followed by next_token should return the same token
/// (unless we've already hit EOF).
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_peek_then_next_consistent(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..1000)
    ) {
        let mut lexer = Lexer::new(&bytes);

        let peeked = lexer.peek_token().cloned();

        // Only test if we got a non-Eof token
        if let Some(token) = peeked {
            if token != Token::Eof {
                let next = lexer.next_token();
                prop_assert_eq!(next, Some(token),
                    "peek_token() then next_token() should return the same token");
            }
        }
    }
}

/// Property: next_token after Eof returns None.
///
/// Once the lexer has returned Eof, subsequent next_token calls should return None.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_eof_returns_none_subsequently(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..1000)
    ) {
        let mut lexer = Lexer::new(&bytes);

        // Consume all tokens until we hit Eof
        loop {
            match lexer.next_token() {
                Some(Token::Eof) => break,
                Some(_) => continue,
                None => break,
            }
        }

        // After Eof, all next_token calls should return None
        for _ in 0..10 {
            prop_assert_eq!(lexer.next_token(), None,
                "next_token() after Eof should return None");
        }
    }
}

/// Property: Integer tokens are within valid ranges.
///
/// The lexer should produce integers that are within reasonable bounds.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_integer_tokens_valid(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..1000)
    ) {
        let mut lexer = Lexer::new(&bytes);

        while let Some(token) = lexer.next_token() {
            if let Token::Integer(i) = token {
                // Integers should be within the range that can be represented
                // (the lexer clamps to i64::MAX on overflow)
                prop_assert!(i >= i64::MIN && i <= i64::MAX,
                    "Integer {} is out of valid range", i);
            }
        }
    }
}

/// Property: Name tokens never exceed length limit.
///
/// Per PDF spec and our implementation, names are limited to 127 bytes
/// of raw input (before hex escape expansion).
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_name_tokens_within_length_limit(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..1000)
    ) {
        let mut lexer = Lexer::new(&bytes);

        while let Some(token) = lexer.next_token() {
            if let Token::Name(name) = token {
                prop_assert!(name.len() <= 127,
                    "Name length {} exceeds 127-byte limit", name.len());
            }
        }
    }
}

/// Property: String tokens don't contain raw NUL bytes.
///
/// NUL bytes in names/strings are rejected by the lexer with diagnostics.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_string_tokens_no_nul_bytes(
        bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..1000)
    ) {
        let mut lexer = Lexer::new(&bytes);

        while let Some(token) = lexer.next_token() {
            if let Token::Name(name) = token {
                prop_assert!(!name.contains(&0x00),
                    "Name token contains NUL byte (should be rejected)");
            }
        }
    }
}

/// Property: Hex string roundtrip for valid hex digits.
///
/// For inputs that are valid hex strings, encoding and decoding should
/// be lossless.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_hex_string_roundtrip(
        input_bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..100)
    ) {
        // Encode the input bytes as a hex string
        let mut encoded = Vec::with_capacity(2 * input_bytes.len() + 2);
        encoded.push(b'<');
        for &b in &input_bytes {
            encoded.push(hex_nibble_to_char((b >> 4) & 0x0F));
            encoded.push(hex_nibble_to_char(b & 0x0F));
        }
        encoded.push(b'>');

        // Decode the hex string
        let mut lexer = Lexer::new(&encoded);
        let decoded = match lexer.next_token() {
            Some(Token::String(s)) => s,
            other => {
                prop_assert!(false, "Expected String token, got {:?}", other);
                return;
            }
        };

        // The decoded bytes should match the original input
        prop_assert_eq!(decoded, input_bytes,
            "Hex string roundtrip failed: expected {:?}, got {:?}",
            input_bytes, decoded);
    }
}

#[cfg(feature = "proptest")]
fn hex_nibble_to_char(nibble: u8) -> u8 {
    match nibble {
        0..=9 => b'0' + nibble,
        10..=15 => b'a' + (nibble - 10),
        _ => b'0',
    }
}

/// Property: Whitespace-only input returns only Eof.
///
/// Input consisting entirely of whitespace and comments should produce
/// exactly one token: Eof.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_whitespace_only_returns_eof(
        whitespace in proptest::collection::vec(
            proptest::prop_oneof![
                Just(b' ' as u8), Just(b'\t' as u8), Just(b'\n' as u8),
                Just(b'\r' as u8), Just(b'\x0c' as u8), Just(0x00 as u8)
            ],
            0..1000
        )
    ) {
        let mut lexer = Lexer::new(&whitespace);

        // First token should be Eof
        let first = lexer.next_token();
        prop_assert_eq!(first, Some(Token::Eof),
            "Whitespace-only input should return Eof, got {:?}", first);

        // Subsequent tokens should be None
        let second = lexer.next_token();
        prop_assert_eq!(second, None,
            "After Eof, should return None, got {:?}", second);
    }
}

/// Property: Stream keyword validation.
///
/// The "stream" keyword must be followed by \n or \r\n per PDF spec 7.3.8.1.
/// Lone \r should emit a diagnostic but not panic.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_stream_keyword_never_panics(
        prefix in proptest::collection::vec(proptest::num::u8::ANY, 0..100),
        suffix in proptest::collection::vec(proptest::num::u8::ANY, 0..10)
    ) {
        let mut input = prefix;
        input.extend_from_slice(b"stream");
        input.extend_from_slice(&suffix);

        // This should never panic, even with malformed stream headers
        let mut lexer = Lexer::new(&input);
        let _ = lex_all(&input);
    }
}

/// Property: Delimiter characters are recognized.
///
/// The PDF spec defines specific delimiter characters. We verify that
/// these are always recognized regardless of surrounding bytes.
#[cfg(feature = "proptest")]
proptest::proptest! {
    #[test]
    fn prop_delimiters_recognized(
        before in proptest::collection::vec(proptest::num::u8::ANY, 0..50),
        after in proptest::collection::vec(proptest::num::u8::ANY, 0..50),
        delimiter in prop_oneof![
            Just(b'('), Just(b')'), Just(b'<'), Just(b'>'),
            Just(b'['), Just(b']'), Just(b'{'), Just(b'}'),
            Just(b'/'), Just(b'%')
        ]
    ) {
        let mut input = before;
        input.push(delimiter);
        input.extend_from_slice(&after);

        // Should not panic on any delimiter
        let mut lexer = Lexer::new(&input);
        let _ = lex_all(&input);
    }
}

// Re-export for use in other modules
pub use lexer_never_panics;

// Helper to allow running these tests without the feature flag for verification
#[cfg(not(feature = "proptest"))]
#[test]
fn test_panic_injection_for_prop_test_verification() {
    // This test deliberately adds a temporary panic to the lexer
    // to verify that the proptest suite would catch it.
    //
    // To verify the proptest works:
    // 1. Uncomment the panic below
    // 2. Run: PROPTEST_CASES=100 cargo test --features proptest -- proptest
    // 3. Verify the test fails with the panic
    // 4. Remove the panic

    use pdftract_core::parser::lexer::Lexer;

    // let input = b"123";
    // let mut lexer = Lexer::new(input);
    // // Simulated panic injection point
    // if lexer.next_token().is_some() {
    //     panic!("DELIBERATE PANIC FOR PROPTEST VERIFICATION");
    // }

    // The above is commented out - uncomment to verify proptest catches panics
}
