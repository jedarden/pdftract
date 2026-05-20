//! Fuzz target for the PDF lexer.
//!
//! This target tests INV-8 (no panic at public boundary) for the lexer.
//! Any panic indicates a lexer bug that must be fixed.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    use pdftract_core::parser::lexer::Lexer;

    // The lexer must never panic on any input
    let mut lexer = Lexer::new(data);

    // Consume all tokens
    loop {
        match lexer.next_token() {
            Some(_) => continue,
            None => break,
        }
    }

    // Also test peek operations
    let _ = Lexer::new(data).peek_token();

    // Test take_diagnostics
    let mut lexer = Lexer::new(data);
    while lexer.next_token().is_some() {}
    let _ = lexer.take_diagnostics();
});
