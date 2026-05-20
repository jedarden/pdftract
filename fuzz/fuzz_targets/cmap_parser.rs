//! Fuzz target for the PDF CMap parser.
//!
//! This target tests INV-8 (no panic at public boundary) for the CMap parser.
//! Any panic indicates a CMap parser bug that must be fixed.
//!
//! Note: Full CMap parser is not yet implemented. This target tests the
//! lexer's name and string handling which are foundational to CMap parsing.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    use pdftract_core::parser::lexer::Lexer;

    // CMap parsing relies heavily on name and string parsing
    // Test that the lexer handles these correctly without panic
    let mut lexer = Lexer::new(data);

    loop {
        match lexer.next_token() {
            Some(token) => {
                // CMap uses many names and strings
                match token {
                    pdftract_core::parser::lexer::Token::Name(_) => {
                        // Name parsing succeeded
                    }
                    pdftract_core::parser::lexer::Token::String(_) => {
                        // String parsing succeeded
                    }
                    _ => {}
                }
            }
            None => break,
        }
    }
});
