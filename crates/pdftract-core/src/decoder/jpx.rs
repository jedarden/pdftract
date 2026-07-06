//! JPXDecode filter handler.
//!
//! This module provides JPEG2000-specific stream decoding with:
//! - Passthrough of raw JPX bytes (pdftract-core does not decode JPEG2000)
//! - JP2 box magic validation (12-byte signature at start)
//! - OCR_JPX_UNSUPPORTED diagnostic emission when full-render and libopenjp2 are unavailable
//!
//! Per PDF spec 7.4.9:
//! - JPXDecode is the JPEG2000 compression format (ISO/IEC 15444-1)
//! - Data may be JP2-wrapped (with box headers) or raw J2K codestream
//! - JP2 wrapper starts with 12-byte signature: 00 00 00 0C 6A 50 20 20 0D 0A 87 0A
//!
//! # Phase origin
//!
//! - 1.5: Stream passthrough and JP2 validation
//! - 5.2: OCR pipeline consumes JPX via pdfium-render (full-render feature)
//!
//! # EC-12 compliance
//!
//! When full-render is NOT compiled AND libopenjp2 is not available at runtime,
//! this module emits OCR_JPX_UNSUPPORTED once per JPX stream. The downstream
//! consumer (Phase 5.2) raises a clearer user-facing error.

use crate::diagnostics::{DiagCode, Diagnostic};

/// JP2 signature box magic bytes (12 bytes).
///
/// Per ISO/IEC 15444-1, every JP2 file starts with a 12-byte signature:
/// - 4 bytes: box length (0x0000000C = 12)
/// - 4 bytes: box type (0x6A502020 = "jP  " with trailing space)
/// - 4 bytes: brand signature (0x0D0A870A =\r\n\x87\n)
const JP2_SIGNATURE: [u8; 12] = [
    0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20, 0x0D, 0x0A, 0x87, 0x0A,
];

/// JPXDecode filter decoder with metadata extraction.
///
/// This decoder handles JPX streams by:
/// 1. Passing through raw bytes unchanged (pdftract-core does not decode JPEG2000)
/// 2. Validating JP2 box magic if present
/// 3. Emitting STREAM_INVALID_JPX if magic doesn't match (raw J2K or corrupt)
/// 4. Emitting OCR_JPX_UNSUPPORTED when full-render and libopenjp2 are unavailable
///
/// # Per-plan behavior (EC-12)
///
/// - **With full-render**: Passthrough only, no diagnostic
/// - **Without full-render but with libopenjp2**: Passthrough only, no diagnostic
/// - **Without full-render AND without libopenjp2**: Emit OCR_JPX_UNSUPPORTED, still passthrough
///
/// The diagnostic alerts downstream consumers (Phase 5.2) that the page
/// cannot be processed via OCR without pdfium-render.
#[derive(Debug, Clone, Copy)]
pub struct JpxDecoder;

impl JpxDecoder {
    /// Create a new JPX decoder.
    #[inline]
    pub const fn new() -> Self {
        Self
    }

    /// Check if full-render feature is enabled at compile time.
    ///
    /// Returns `true` if pdftract was built with `--features full-render`,
    /// enabling PDFium-based JPX decoding in the OCR pipeline.
    #[inline]
    pub const fn has_full_render() -> bool {
        cfg!(feature = "full-render")
    }

    /// Check if libopenjp2 is available at runtime.
    ///
    /// Returns `true` if pkg-config reports libopenjp2 exists or if libopenjp2
    /// is found in ldconfig. This provides a runtime fallback when full-render
    /// is not compiled.
    ///
    /// Per EC-12, this check mirrors the Phase 6.10 doctor approach.
    pub fn has_libopenjp2() -> bool {
        // Try pkg-config first (preferred, more precise)
        if let Ok(output) = std::process::Command::new("pkg-config")
            .args(["--exists", "libopenjp2"])
            .output()
        {
            if output.status.success() {
                return true;
            }
        }

        // Fallback to ldconfig -p grep
        if let Ok(output) = std::process::Command::new("ldconfig").arg("-p").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("libopenjp2") {
                return true;
            }
        }

        false
    }

    /// Check if JPX decoding is available (full-render OR libopenjp2).
    ///
    /// Returns `true` if either full-render is compiled or libopenjp2 is
    /// available at runtime.
    pub fn has_jpx_support() -> bool {
        Self::has_full_render() || Self::has_libopenjp2()
    }

    /// Validate JP2 box magic at the start of data.
    ///
    /// Returns `true` if the first 12 bytes match the JP2 signature.
    /// Returns `false` if the data is too short or magic doesn't match.
    ///
    /// # Arguments
    ///
    /// * `data` - The JPX stream data to validate
    ///
    /// # Returns
    ///
    /// - `true` if JP2 signature is present
    /// - `false` if raw J2K codestream (no wrapper) or corrupt
    pub fn validate_jp2_magic(data: &[u8]) -> bool {
        data.len() >= 12 && &data[0..12] == JP2_SIGNATURE
    }

    /// Emit diagnostic if JPX support is not available.
    ///
    /// Per EC-12, this emits OCR_JPX_UNSUPPORTED once per JPX stream
    /// when neither full-render nor libopenjp2 is available. The diagnostic
    /// alerts downstream consumers that OCR processing will fail for this page.
    ///
    /// # Arguments
    ///
    /// * `diagnostics` - Buffer to receive emitted diagnostics
    ///
    /// # Returns
    ///
    /// - `true` if diagnostic was emitted (no JPX support available)
    /// - `false` if no diagnostic needed (full-render or libopenjp2 available)
    pub fn emit_unsupported_diagnostic(&self, diagnostics: &mut Vec<Diagnostic>) -> bool {
        if !Self::has_jpx_support() {
            let message = if Self::has_full_render() {
                // This case shouldn't happen with the has_jpx_support check,
                // but is kept for clarity
                "JPXDecode filter encountered with full-render feature (should not emit)"
                    .to_string()
            } else if Self::has_libopenjp2() {
                // This case shouldn't happen with the has_jpx_support check,
                // but is kept for clarity
                "JPXDecode filter encountered with libopenjp2 available (should not emit)"
                    .to_string()
            } else {
                format!(
                    "JPXDecode filter encountered; build with --features full-render or install libopenjp2 ({})",
                    if Self::has_libopenjp2() { "libopenjp2 found" } else { "libopenjp2 not found" }
                )
            };

            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::OcrJpxUnsupported,
                message,
            ));
            return true;
        }
        false
    }

    /// Emit diagnostic for invalid JP2 magic.
    ///
    /// Emits STREAM_INVALID_JPX when the JP2 box magic signature is not found.
    /// This indicates raw J2K codestream (no JP2 wrapper) or corrupted data.
    /// The data is still passed through unchanged.
    ///
    /// # Arguments
    ///
    /// * `diagnostics` - Buffer to receive emitted diagnostics
    pub fn emit_invalid_magic_diagnostic(&self, diagnostics: &mut Vec<Diagnostic>) {
        diagnostics.push(Diagnostic::with_static_no_offset(
            DiagCode::StreamInvalidJpx,
            "JP2 box magic signature not found; raw J2K codestream (no JP2 wrapper) or corrupted data; data is passed through anyway",
        ));
    }
}

/// Default implementation for Read trait passthrough.
///
/// This provides compatibility with code that expects a Read-style
/// decoder, though JPX passthrough is typically handled at the
/// stream pipeline level via PassthroughDecoder in stream.rs.
impl std::io::Read for &JpxDecoder {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        // Passthrough decoder returns no data via Read interface.
        // Actual passthrough happens in the stream pipeline.
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_jp2_signature_constant() {
        // Verify the JP2 signature matches the spec
        assert_eq!(
            JP2_SIGNATURE,
            [0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20, 0x0D, 0x0A, 0x87, 0x0A]
        );
    }

    #[test]
    fn test_validate_jp2_magic_with_valid_jp2() {
        // Valid JP2 signature at start
        let mut data = JP2_SIGNATURE.to_vec();
        data.extend_from_slice(&[0xFF, 0x4F, 0xFF, 0x51]); // Some J2K codestream markers

        assert!(JpxDecoder::validate_jp2_magic(&data));
    }

    #[test]
    fn test_validate_jp2_magic_with_raw_j2k() {
        // Raw J2K codestream starts with SOC (0xFF 0x4F), not JP2 signature
        let data = [0xFF, 0x4F, 0x51, 0x00]; // SOC marker + some data

        assert!(!JpxDecoder::validate_jp2_magic(&data));
    }

    #[test]
    fn test_validate_jp2_magic_with_truncated_data() {
        // Data too short for JP2 signature
        let data = [
            0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20, 0x0D, 0x0A, 0x87,
        ]; // Only 11 bytes

        assert!(!JpxDecoder::validate_jp2_magic(&data));
    }

    #[test]
    fn test_validate_jp2_magic_with_empty_data() {
        let data: [u8; 0] = [];

        assert!(!JpxDecoder::validate_jp2_magic(&data));
    }

    #[test]
    fn test_validate_jp2_magic_with_corrupt_signature() {
        // Almost JP2 signature but last byte wrong
        let mut data = JP2_SIGNATURE.to_vec();
        data[11] = 0x00; // Corrupt last byte

        assert!(!JpxDecoder::validate_jp2_magic(&data));
    }

    #[test]
    fn test_has_full_render() {
        // Result depends on whether full-render feature is enabled
        let has_full_render = JpxDecoder::has_full_render();
        assert_eq!(has_full_render, cfg!(feature = "full-render"));
    }

    #[test]
    fn test_has_jpx_support_with_full_render() {
        // When full-render is enabled, has_jpx_support should always return true
        if cfg!(feature = "full-render") {
            assert!(JpxDecoder::has_jpx_support());
        }
    }

    #[test]
    fn test_emit_invalid_magic_diagnostic() {
        let decoder = JpxDecoder::new();
        let mut diagnostics = Vec::new();

        decoder.emit_invalid_magic_diagnostic(&mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagCode::StreamInvalidJpx);
        assert!(diagnostics[0]
            .message
            .contains("JP2 box magic signature not found"));
    }

    #[test]
    fn test_emit_unsupported_diagnostic_when_no_support() {
        let decoder = JpxDecoder::new();
        let mut diagnostics = Vec::new();

        // This test only validates behavior when support is missing
        // The actual emission depends on compile-time and runtime state
        if !JpxDecoder::has_jpx_support() {
            let emitted = decoder.emit_unsupported_diagnostic(&mut diagnostics);
            assert!(emitted);
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].code, DiagCode::OcrJpxUnsupported);
        } else {
            let emitted = decoder.emit_unsupported_diagnostic(&mut diagnostics);
            assert!(!emitted);
            assert!(diagnostics.is_empty());
        }
    }

    #[test]
    fn test_jpx_decoder_const() {
        // Test that JpxDecoder can be created at compile time
        const DECODER: JpxDecoder = JpxDecoder::new();
        assert!(JpxDecoder::has_full_render() == cfg!(feature = "full-render"));
        let _ = DECODER;
    }

    #[test]
    fn test_jp2_signature_roundtrip() {
        // Create a realistic JP2 header and verify it validates
        let mut jp2_data = Vec::new();

        // JP2 signature box (12 bytes)
        jp2_data.extend_from_slice(&JP2_SIGNATURE);

        // File Type box (20 bytes)
        // Length: 0x00000014 (20)
        jp2_data.extend_from_slice(&0x00_00_00_14_u32.to_be_bytes());
        // Type: 0x66747970 ("ftyp")
        jp2_data.extend_from_slice(b"ftyp");
        // Brand: 0x6A703220 ("jp2 ")
        jp2_data.extend_from_slice(b"jp2 ");
        // Minor version: 0
        jp2_data.extend_from_slice(&0u32.to_be_bytes());
        // Compatibility: 0x6A703220 ("jp2 ")
        jp2_data.extend_from_slice(b"jp2 ");

        // Some codestream data
        jp2_data.extend_from_slice(&[0xFF, 0x4F, 0xFF, 0x51]);

        assert!(JpxDecoder::validate_jp2_magic(&jp2_data));
    }

    #[test]
    fn test_raw_j2k_codestream_not_valid_jp2() {
        // Raw J2K codestream starts with SOC marker (0xFF 0x4F)
        let j2k_data = [
            0xFF, 0x4F, // SOC (Start of Codestream)
            0xFF, 0x51, // SIZ (Image and tile size)
            0x00, 0x29, 0x00,
            0x01, // Lsiz (length), Rsiz (capabilities)
                  // ... rest of SIZ segment
        ];

        assert!(!JpxDecoder::validate_jp2_magic(&j2k_data));
    }

    #[test]
    fn test_jpx_decoder_is_send_sync() {
        // Verify JpxDecoder implements Send + Sync (required for StreamDecoder)
        fn is_send_sync<T: Send + Sync>() {}
        is_send_sync::<JpxDecoder>();
    }

    #[test]
    fn test_jpx_decoder_read_trait() {
        // Test that &JpxDecoder implements Read
        let decoder = JpxDecoder::new();
        let mut buf = [0u8; 10];

        // Read should return 0 bytes (passthrough handled at stream level)
        let mut decoder_ref = &decoder;
        let result = decoder_ref.read(&mut buf);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_emit_unsupported_diagnostic_message_content() {
        let decoder = JpxDecoder::new();
        let mut diagnostics = Vec::new();

        // Only test emission when support is missing
        if !JpxDecoder::has_jpx_support() {
            decoder.emit_unsupported_diagnostic(&mut diagnostics);

            let message = &diagnostics[0].message;
            // Message should mention the feature or libopenjp2
            assert!(message.contains("full-render") || message.contains("libopenjp2"));
        }
    }

    #[test]
    fn test_has_libopenjp2_runtime_check() {
        // This test validates that the runtime check runs without panicking
        // The result depends on the system state
        let _has_libopenjp2 = JpxDecoder::has_libopenjp2();

        // When full-render is enabled, this should not cause any issues
        if cfg!(feature = "full-render") {
            // The runtime check is irrelevant when full-render is compiled,
            // but should still execute without error
            let _ = JpxDecoder::has_libopenjp2();
        }
    }

    #[cfg(feature = "full-render")]
    #[test]
    fn test_full_render_always_has_support() {
        // When full-render is compiled, has_jpx_support should always return true
        assert!(JpxDecoder::has_jpx_support());
        assert!(!JpxDecoder::new().emit_unsupported_diagnostic(&mut Vec::new()));
    }
}
