//! Fuzz target for the PDF object parser.
//!
//! This target tests INV-8 (no panic at public boundary) for the object parser.
//! Any panic indicates an object parser bug that must be fixed.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    use pdftract_core::parser::object::ObjectParser;

    // The object parser must never panic on any input
    let mut parser = ObjectParser::new(data);

    // Test parse_direct_object
    loop {
        match parser.parse_direct_object() {
            Some(_) => continue,
            None => break,
        }
    }

    // Also test parse_indirect_object
    let mut parser2 = ObjectParser::new(data);
    let _ = parser2.parse_indirect_object();

    // Test take_diagnostics
    let _ = parser.take_diagnostics();
});
