//! Fuzz target for the PDF content stream interpreter.
//!
//! This target tests INV-8 (no panic at public boundary) for the content stream interpreter.
//! Any panic indicates a content stream bug that must be fixed.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    use pdftract_core::content_stream::{process_with_mode, ProcessingMode};
    use pdftract_core::parser::resources::ResourceDict;

    // The content stream interpreter must never panic on any input
    // Create a minimal empty resource dict for testing
    let resources = ResourceDict::new();

    // Test in Normal mode
    let _ = process_with_mode(data, &resources, ProcessingMode::Normal, None);

    // Test in PositionHint mode
    let _ = process_with_mode(data, &resources, ProcessingMode::PositionHint, None);
});
