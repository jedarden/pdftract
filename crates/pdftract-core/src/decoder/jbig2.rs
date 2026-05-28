//! JBIG2Decode filter handler.
//!
//! This module provides JBIG2-specific stream decoding with:
//! - Passthrough of raw JBIG2 bytes (pdftract-core does not decode JBIG2)
//! - /JBIG2Globals reference recording for downstream consumers
//! - OCR_JBIG2_UNSUPPORTED diagnostic emission when full-render is disabled
//!
//! Per PDF spec 7.4.7:
//! - JBIG2Decode is a lossless compression format for bitonal images
//! - /JBIG2Globals is an indirect reference to a globally-shared symbol dictionary
//! - Without globals, the stream is self-contained (still decodable)
//!
//! # Phase origin
//!
//! - 1.5: Stream passthrough and globals recording
//! - 5.4: OCR pipeline consumes globals via pdfium-render (full-render feature)
//!
//! # EC-11 compliance
//!
//! When full-render is NOT compiled, this module emits OCR_JBIG2_UNSUPPORTED
//! once per JBIG2 stream. The downstream consumer (Phase 5.4 OCR pipeline)
//! raises a clearer user-facing error.

use crate::diagnostics::{DiagCode, Diagnostic};
use crate::parser::object::{ObjRef, PdfObject};
use std::io::Read;

/// Reference to a JBIG2Globals stream.
///
/// This struct captures the indirect reference to a globally-shared symbol
/// dictionary that must be prepended to JBIG2 data before decoding.
/// The OCR pipeline (Phase 5.4) resolves this reference and fetches the
/// global symbols stream before sending to pdfium-render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Jbig2GlobalsRef {
    /// Indirect reference to the globals stream object.
    pub obj_ref: ObjRef,
}

impl Jbig2GlobalsRef {
    /// Create a new JBIG2 globals reference from an object reference.
    #[inline]
    pub const fn new(obj_ref: ObjRef) -> Self {
        Self { obj_ref }
    }
}

/// JBIG2Decode filter decoder with metadata extraction.
///
/// This decoder handles JBIG2 streams by:
/// 1. Passing through raw bytes unchanged (pdftract-core does not decode JBIG2)
/// 2. Extracting /JBIG2Globals reference if present
/// 3. Emitting OCR_JBIG2_UNSUPPORTED diagnostic when full-render is disabled
///
/// # Per-plan behavior (EC-11)
///
/// - **With full-render**: Passthrough only, no diagnostic
/// - **Without full-render**: Emit OCR_JBIG2_UNSUPPORTED, still passthrough
///
/// The diagnostic alerts downstream consumers (Phase 5.4) that the page
/// cannot be processed via OCR without pdfium-render.
#[derive(Debug, Clone, Copy)]
pub struct Jbig2Decoder;

impl Jbig2Decoder {
    /// Create a new JBIG2 decoder.
    #[inline]
    pub const fn new() -> Self {
        Self
    }

    /// Extract /JBIG2Globals reference from stream dictionary.
    ///
    /// Per PDF spec 7.4.7, /JBIG2Globals is an indirect reference to a
    /// globally-shared symbol dictionary stream. This reference is recorded
    /// for the OCR pipeline (Phase 5.4), which fetches and prepends the
    /// globals before sending to pdfium-render.
    ///
    /// # Returns
    ///
    /// - `Some(Jbig2GlobalsRef)` if /JBIG2Globals is present
    /// - `None` if the stream is self-contained (no globals)
    ///
    /// # Arguments
    ///
    /// * `stream_dict` - The stream dictionary (from PdfStream.dict)
    pub fn extract_globals_ref(stream_dict: &crate::parser::object::PdfDict) -> Option<Jbig2GlobalsRef> {
        let globals_obj = stream_dict.get("/JBIG2Globals")?;

        match globals_obj {
            PdfObject::Ref(ref_obj) => {
                Some(Jbig2GlobalsRef::new(*ref_obj))
            }
            _ => {
                // /JBIG2Globals must be an indirect reference per PDF spec.
                // Inline or invalid types are treated as missing (self-contained stream).
                None
            }
        }
    }

    /// Check if full-render feature is enabled at compile time.
    ///
    /// Returns `true` if pdftract was built with `--features full-render`,
    /// enabling PDFium-based JBIG2 decoding in the OCR pipeline.
    #[inline]
    pub const fn has_full_render() -> bool {
        cfg!(feature = "full-render")
    }

    /// Emit diagnostic if full-render is not available.
    ///
    /// Per EC-11, this emits OCR_JBIG2_UNSUPPORTED once per JBIG2 stream
    /// when the full-render feature is not compiled. The diagnostic alerts
    /// downstream consumers that OCR processing will fail for this page.
    ///
    /// # Arguments
    ///
    /// * `diagnostics` - Buffer to receive emitted diagnostics
    ///
    /// # Returns
    ///
    /// - `true` if diagnostic was emitted (full-render not available)
    /// - `false` if no diagnostic needed (full-render available)
    pub fn emit_unsupported_diagnostic(&self, diagnostics: &mut Vec<Diagnostic>) -> bool {
        if !Self::has_full_render() {
            diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::OcrJbig2Unsupported,
                "JBIG2Decode filter encountered; build with --features full-render to enable JBIG2 decoding via PDFium",
            ));
            return true;
        }
        false
    }
}

/// Default implementation for Read trait passthrough.
///
/// This provides compatibility with code that expects a Read-style
/// decoder, though JBIG2 passthrough is typically handled at the
/// stream pipeline level via PassthroughDecoder in stream.rs.
impl Read for &Jbig2Decoder {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        // Passthrough decoder returns no data via Read interface.
        // Actual passthrough happens in the stream pipeline.
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::PdfDict;
    use indexmap::indexmap;

    #[test]
    fn test_extract_globals_ref_with_valid_ref() {
        let mut dict = PdfDict::new();
        dict.insert(
            crate::parser::object::intern("/JBIG2Globals"),
            PdfObject::Ref(ObjRef::new(10, 0)),
        );

        let globals_ref = Jbig2Decoder::extract_globals_ref(&dict);
        assert!(globals_ref.is_some());
        assert_eq!(globals_ref.unwrap().obj_ref.object, 10);
    }

    #[test]
    fn test_extract_globals_ref_without_globals() {
        let dict = PdfDict::new();

        let globals_ref = Jbig2Decoder::extract_globals_ref(&dict);
        assert!(globals_ref.is_none());
    }

    #[test]
    fn test_extract_globals_ref_with_invalid_type() {
        let mut dict = PdfDict::new();
        // /JBIG2Globals must be a Ref, not a Name or other type
        dict.insert(
            crate::parser::object::intern("/JBIG2Globals"),
            PdfObject::Name(crate::parser::object::intern("InvalidGlobals")),
        );

        let globals_ref = Jbig2Decoder::extract_globals_ref(&dict);
        assert!(globals_ref.is_none());
    }

    #[test]
    fn test_emit_unsupported_diagnostic_when_feature_disabled() {
        // This test verifies the diagnostic is emitted when full-render is disabled.
        // The actual cfg check happens at compile time, so we test the logic path.
        let decoder = Jbig2Decoder::new();
        let mut diagnostics = Vec::new();

        let emitted = decoder.emit_unsupported_diagnostic(&mut diagnostics);

        // Result depends on whether full-render feature is enabled
        if cfg!(feature = "full-render") {
            assert!(!emitted);
            assert!(diagnostics.is_empty());
        } else {
            assert!(emitted);
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].code, DiagCode::OcrJbig2Unsupported);
        }
    }

    #[test]
    fn test_jbig2_globals_ref_const() {
        // Test that Jbig2GlobalsRef can be created at compile time
        const GLOBALS_REF: Jbig2GlobalsRef = Jbig2GlobalsRef::new(ObjRef::new(42, 0));
        assert_eq!(GLOBALS_REF.obj_ref.object, 42);
        assert_eq!(GLOBALS_REF.obj_ref.generation, 0);
    }

    #[test]
    fn test_jbig2_decoder_const() {
        // Test that Jbig2Decoder can be created at compile time
        const DECODER: Jbig2Decoder = Jbig2Decoder::new();
        assert!(Jbig2Decoder::has_full_render() == cfg!(feature = "full-render"));
    }
}
