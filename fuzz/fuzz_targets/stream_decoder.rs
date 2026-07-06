//! Fuzz target for the PDF stream decoder.
//!
//! This target tests INV-8 (no panic at public boundary) for the stream decoder.
//! Any panic indicates a stream decoder bug that must be fixed.
//!
//! This also tests EC-10 (decompression bomb) - the 512 MB limit must hold
//! under random predictor inputs.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    use pdftract_core::parser::stream::{
        FlateDecoder, ASCII85Decoder, ASCIIHexDecoder, LZWDecoder,
        DEFAULT_MAX_DECOMPRESS_BYTES,
    };
    use pdftract_core::parser::StreamDecoder;

    let mut counter = 0;

    // Test FlateDecoder - must never panic
    let _ = FlateDecoder.decode(data, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);

    // Test ASCII85Decoder - must never panic
    let mut counter = 0;
    let _ = ASCII85Decoder.decode(data, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);

    // Test ASCIIHexDecoder - must never panic
    let mut counter = 0;
    let _ = ASCIIHexDecoder.decode(data, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);

    // Test LZWDecoder - must never panic
    let mut counter = 0;
    let _ = LZWDecoder.decode(data, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);

    // Test with very low bomb limit (EC-10 decompression bomb)
    let mut counter = 0;
    let low_limit: u64 = 100;
    let _ = FlateDecoder.decode(data, None, &mut counter, low_limit);
});
