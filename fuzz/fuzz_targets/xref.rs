//! Fuzz target for the PDF xref parser.
//!
//! This target tests INV-8 (no panic at public boundary) for the xref parser.
//! Any panic indicates an xref parser bug that must be fixed.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    use pdftract_core::parser::xref::{parse_traditional_xref, forward_scan_xref};
    use pdftract_core::parser::stream::MemorySource;

    let source = MemorySource::new(data.to_vec());

    // Test parse_traditional_xref - must never panic
    let _ = parse_traditional_xref(&source, 0);

    // Test forward_scan_xref - must never panic
    let _ = forward_scan_xref(&source, false);

    // Test with linearized flag
    let _ = forward_scan_xref(&source, true);
});
