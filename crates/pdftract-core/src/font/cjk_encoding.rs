//! CJK byte encoding adapter for raw byte fallback.
//!
//! This module provides a thin wrapper around `encoding_rs` for decoding
//! the four major CJK byte encodings used in legacy PDFs:
//! - Shift-JIS (Japanese)
//! - GB18030 (Chinese)
//! - Big5 (Traditional Chinese, with Big5-HKSCS extension)
//! - EUC-KR (Korean, covering KS X 1001 + Unified Hangul)
//!
//! These are FALLBACK encodings used when:
//! - A font's encoding indicates a raw byte encoding (e.g., /Encoding /ShiftJIS)
//! - No CMap or ToUnicode is present
//! - The lead byte is in a CJK range
//!
//! The primary text extraction path uses predefined CMaps + ToUnicode; this
//! module is only for legacy PDFs that don't provide proper Unicode mappings.

/// CJK byte encoding identifier.
///
/// Represents the four major legacy CJK encodings used in PDFs. These are
/// raw byte encodings that need to be decoded to Unicode for text extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CjkEncoding {
    /// Shift-JIS (JIS X 0208 + extensions)
    ///
    /// The most common encoding for Japanese PDFs. Variable-width: 1 byte for
    /// ASCII (0x00-0x7F), 2 bytes for JIS X 0208 characters (lead byte 0x81-0x9F,
    /// 0xE0-0xEF).
    ShiftJis,

    /// GB18030 (Chinese national standard)
    ///
    /// The mandatory encoding for PRC PDFs. Variable-width: 1, 2, or 4 bytes.
    /// Covers all Unicode code points assigned to Chinese characters.
    Gb18030,

    /// Big5 (Traditional Chinese, with Big5-HKSCS extension)
    ///
    /// Common encoding for Traditional Chinese PDFs (Taiwan, Hong Kong).
    /// Variable-width: 1 byte for ASCII (0x00-0x7F), 2 bytes for Big5 characters
    /// (lead byte 0x81-0xFE). The encoding_rs implementation includes the
    /// Big5-HKSCS extension for Hong Kong-specific characters.
    Big5,

    /// EUC-KR (KS X 1001 + Unified Hangul)
    ///
    /// The standard encoding for Korean PDFs. Variable-width: 1 byte for ASCII
    /// (0x00-0x7F), 2 bytes for KS X 1001 characters (lead byte 0x81-0xFE).
    /// The encoding_rs implementation covers KS X 1001 + Unified Hangul.
    EucKr,
}

impl CjkEncoding {
    /// Get the encoding_rs singleton for this encoding.
    fn encoding(&self) -> &'static encoding_rs::Encoding {
        match self {
            CjkEncoding::ShiftJis => encoding_rs::SHIFT_JIS,
            CjkEncoding::Gb18030 => encoding_rs::GB18030,
            CjkEncoding::Big5 => encoding_rs::BIG5,
            CjkEncoding::EucKr => encoding_rs::EUC_KR,
        }
    }

    /// Get the name of this encoding for diagnostic messages.
    pub fn name(&self) -> &'static str {
        match self {
            CjkEncoding::ShiftJis => "Shift-JIS",
            CjkEncoding::Gb18030 => "GB18030",
            CjkEncoding::Big5 => "Big5",
            CjkEncoding::EucKr => "EUC-KR",
        }
    }
}

/// Decode CJK-encoded bytes to a String.
///
/// This is a fallback path for legacy PDFs that use raw byte encodings instead
/// of proper CMap/ToUnicode mappings. The function uses `encoding_rs` to decode
/// the byte sequence according to the specified encoding.
///
/// # Arguments
///
/// * `enc` - The CJK encoding to use for decoding
/// * `bytes` - The raw byte sequence to decode
///
/// # Returns
///
/// A tuple `(String, bool)` where:
/// - The `String` is the decoded Unicode text (with U+FFFD for malformed bytes)
/// - The `bool` is `true` if any malformed bytes were encountered, `false` otherwise
///
/// # Behavior
///
/// - Empty input returns an empty string with `malformed = false`
/// - Malformed byte sequences are replaced with U+FFFD (Unicode REPLACEMENT CHARACTER)
/// - No panic occurs on any input
/// - PDF byte streams never have a BOM, so we use `decode_without_bom_handling`
///
/// # Example
///
/// ```
/// use pdftract_core::font::cjk_encoding::{decode_cjk_bytes, CjkEncoding};
///
/// // Round-trip: encode "テスト" as Shift-JIS bytes, decode -> get "テスト" back
/// let test_str = "テスト";
/// let shift_jis_bytes = encoding_rs::SHIFT_JIS.encode(test_str);
/// let (decoded, malformed) = decode_cjk_bytes(CjkEncoding::ShiftJis, &shift_jis_bytes);
/// assert_eq!(decoded, test_str);
/// assert!(!malformed);
/// ```
pub fn decode_cjk_bytes(enc: CjkEncoding, bytes: &[u8]) -> (String, bool) {
    if bytes.is_empty() {
        return (String::new(), false);
    }

    let encoding = enc.encoding();
    let (cow, had_malformed) = encoding.decode_without_bom_handling(bytes);

    // The encoding_rs decoder already replaces malformed sequences with U+FFFD
    // We just need to convert Cow<str> to String and report the malformed status
    (cow.into_owned(), had_malformed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_shift_jis_valid() {
        // "テスト" in Shift-JIS
        // 0x83 0x65 = テ, 0x83 0x58 = ス, 0x83 0x67 = ト
        let bytes = [0x83, 0x65, 0x83, 0x58, 0x83, 0x67];
        let (decoded, malformed) = decode_cjk_bytes(CjkEncoding::ShiftJis, &bytes);
        assert_eq!(decoded, "テスト");
        assert!(!malformed);
    }

    #[test]
    fn test_decode_gb18030_valid() {
        // "中文测试" in GB18030
        // Verify correct encoding by encoding the string first
        let test_str = "中文测试";
        let (bytes, _, _) = encoding_rs::GB18030.encode(test_str);
        let (decoded, malformed) = decode_cjk_bytes(CjkEncoding::Gb18030, &bytes);
        assert_eq!(decoded, test_str);
        assert!(!malformed);
    }

    #[test]
    fn test_decode_big5_valid() {
        // "測試" in Big5 (Traditional Chinese)
        // Verify correct encoding by encoding the string first
        let test_str = "測試";
        let (bytes, _, _) = encoding_rs::BIG5.encode(test_str);
        let (decoded, malformed) = decode_cjk_bytes(CjkEncoding::Big5, &bytes);
        assert_eq!(decoded, test_str);
        assert!(!malformed);
    }

    #[test]
    fn test_decode_euc_kr_valid() {
        // "한글" in EUC-KR (Korean)
        // Verify correct encoding by encoding the string first
        let test_str = "한글";
        let (bytes, _, _) = encoding_rs::EUC_KR.encode(test_str);
        let (decoded, malformed) = decode_cjk_bytes(CjkEncoding::EucKr, &bytes);
        assert_eq!(decoded, test_str);
        assert!(!malformed);
    }

    #[test]
    fn test_decode_empty_input() {
        let (decoded, malformed) = decode_cjk_bytes(CjkEncoding::ShiftJis, &[]);
        assert_eq!(decoded, "");
        assert!(!malformed);
    }

    #[test]
    fn test_decode_ascii_passthrough() {
        // ASCII should pass through unchanged in all encodings
        let bytes = b"Hello, World!";
        for enc in &[
            CjkEncoding::ShiftJis,
            CjkEncoding::Gb18030,
            CjkEncoding::Big5,
            CjkEncoding::EucKr,
        ] {
            let (decoded, malformed) = decode_cjk_bytes(*enc, bytes);
            assert_eq!(decoded, "Hello, World!");
            assert!(!malformed);
        }
    }

    #[test]
    fn test_decode_malformed_shift_jis() {
        // Invalid Shift-JIS: lead byte 0x83 followed by ASCII range byte
        // This is not a valid Shift-JIS sequence
        let bytes = [0x83, 0x20]; // 0x83 is a lead byte, 0x20 is ASCII space
        let (decoded, malformed) = decode_cjk_bytes(CjkEncoding::ShiftJis, &bytes);
        // Should contain replacement character and report malformed
        assert!(malformed);
        assert!(decoded.contains('\u{FFFD}') || decoded.len() < 2);
    }

    #[test]
    fn test_decode_malformed_gb18030() {
        // Invalid GB18030: incomplete multi-byte sequence
        let bytes = [0x81]; // Lead byte without trail byte
        let (decoded, malformed) = decode_cjk_bytes(CjkEncoding::Gb18030, &bytes);
        assert!(malformed);
        // Should contain replacement character
        assert!(decoded.contains('\u{FFFD}') || decoded == "\u{FFFD}");
    }

    #[test]
    fn test_round_trip_shift_jis() {
        let test_str = "テスト";
        let (shift_jis_bytes, _, _) = encoding_rs::SHIFT_JIS.encode(test_str);
        let (decoded, malformed) = decode_cjk_bytes(CjkEncoding::ShiftJis, &shift_jis_bytes);
        assert_eq!(decoded, test_str);
        assert!(!malformed);
    }

    #[test]
    fn test_round_trip_gb18030() {
        let test_str = "中文测试";
        let (gb18030_bytes, _, _) = encoding_rs::GB18030.encode(test_str);
        let (decoded, malformed) = decode_cjk_bytes(CjkEncoding::Gb18030, &gb18030_bytes);
        assert_eq!(decoded, test_str);
        assert!(!malformed);
    }

    #[test]
    fn test_round_trip_big5() {
        let test_str = "測試";
        let (big5_bytes, _, _) = encoding_rs::BIG5.encode(test_str);
        let (decoded, malformed) = decode_cjk_bytes(CjkEncoding::Big5, &big5_bytes);
        assert_eq!(decoded, test_str);
        assert!(!malformed);
    }

    #[test]
    fn test_round_trip_euc_kr() {
        let test_str = "한글";
        let (euc_kr_bytes, _, _) = encoding_rs::EUC_KR.encode(test_str);
        let (decoded, malformed) = decode_cjk_bytes(CjkEncoding::EucKr, &euc_kr_bytes);
        assert_eq!(decoded, test_str);
        assert!(!malformed);
    }

    #[test]
    fn test_encoding_names() {
        assert_eq!(CjkEncoding::ShiftJis.name(), "Shift-JIS");
        assert_eq!(CjkEncoding::Gb18030.name(), "GB18030");
        assert_eq!(CjkEncoding::Big5.name(), "Big5");
        assert_eq!(CjkEncoding::EucKr.name(), "EUC-KR");
    }

    #[test]
    fn test_big5_hkscs_extension() {
        // Big5-HKSCS adds Hong Kong-specific characters
        // The encoding_rs BIG5 implementation includes this extension
        // Test with a character that's more likely to be in the Big5-HKSCS range
        let hkscs_str = "香港"; // "Hong Kong" in Traditional Chinese
        let (big5_bytes, _, _) = encoding_rs::BIG5.encode(hkscs_str);
        let (decoded, malformed) = decode_cjk_bytes(CjkEncoding::Big5, &big5_bytes);
        // The characters should round-trip
        if !big5_bytes.is_empty() {
            assert_eq!(decoded, hkscs_str);
            assert!(!malformed);
        }
    }

    #[test]
    fn test_malformed_no_panic() {
        // Test various malformed inputs that should not panic
        let malformed_inputs: Vec<&[u8]> = vec![
            &[0xFF],       // Invalid lead byte in Shift-JIS
            &[0x80, 0x80], // Invalid sequence in GB18030
            &[0xFE, 0xFF], // Invalid in Big5
            &[0xFF, 0xFF], // Invalid in EUC-KR
        ];

        for (i, bytes) in malformed_inputs.iter().enumerate() {
            for enc in &[
                CjkEncoding::ShiftJis,
                CjkEncoding::Gb18030,
                CjkEncoding::Big5,
                CjkEncoding::EucKr,
            ] {
                let (decoded, had_malformed) = decode_cjk_bytes(*enc, bytes);
                // Should not panic and should return a valid String
                assert!(!decoded.is_empty() || had_malformed || decoded == "\u{FFFD}");
            }
        }
    }
}
