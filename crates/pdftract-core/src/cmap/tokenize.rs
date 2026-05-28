//! Multi-byte content-stream tokenizer for CJK text.
//!
//! This module implements tokenization of byte strings from TJ/Tj operators
//! in PDF content streams. Multi-byte encodings (CJK PDFs, ToUnicode CMaps,
//! custom encodings) require parsing variable-length byte sequences (1-4 bytes)
//! according to the codespace ranges defined by the font's CMap.
//!
//! # Algorithm
//!
//! Per ISO 32000-1 9.10.3.1, the tokenizer uses widest-first matching:
//! > "To determine the length of an encoded byte sequence, the lengths of the
//! > codespace ranges are examined, and the byte sequence is determined to have
//! > the same length as the longest matching range."
//!
//! This resolves ambiguities when a byte prefix could start either a single-byte
//! or multi-byte sequence (e.g., 0x80 in both a 1-byte range and a 2-byte lead range).
//!
//! # Empty codespace
//!
//! If the codespace is empty (no ranges defined), the tokenizer defaults to
//! single-byte coverage for all byte values 0x00-0xFF. This matches the behavior
//! of many PDF readers when no codespace is explicitly declared.
//!
//! # Unrecognized bytes
//!
//! Bytes that do not match any codespace range emit U+FFFD (REPLACEMENT CHARACTER)
//! and produce a `CJK_TOKENIZE_UNKNOWN_BYTE` diagnostic. To prevent diagnostic spam,
//! each unique byte value emits at most one diagnostic per tokenization call.

use std::collections::HashSet;

use crate::diagnostics::DiagCode;
use crate::{emit, diagnostics::Diagnostic};

use super::{CodespaceRange, CodespaceRanges};

/// Tokenize a byte string into character codes using codespace ranges.
///
/// Walks a TJ/Tj byte string and emits a sequence of character codes
/// (each up to 4 bytes wide, packed big-endian into a u32).
///
/// # Arguments
///
/// * `codespace` - The codespace ranges defining valid byte sequences
/// * `bytes` - The byte string to tokenize (from a TJ/Tj operand)
/// * `diagnostics` - Output buffer for diagnostics
///
/// # Returns
///
/// A vector of packed character codes. Each code is a big-endian packing
/// of 1-4 bytes into a u32. Unrecognized bytes produce 0xFFFD (U+FFFD).
///
/// # Examples
///
/// ```
/// use pdftract_core::cmap::{CodespaceRange, CodespaceRanges, tokenize_cjk_bytes};
///
/// // ASCII-only codespace
/// let mut codespace = CodespaceRanges::new();
/// codespace.push(CodespaceRange::new([0x00, 0, 0, 0], [0x7F, 0, 0, 0], 1));
///
/// let bytes = &[0x48, 0x65, 0x6C, 0x6C, 0x6F]; // "Hello"
/// let mut diagnostics = Vec::new();
/// let codes = tokenize_cjk_bytes(&codespace, bytes, &mut diagnostics);
///
/// assert_eq!(codes, &[0x48, 0x65, 0x6C, 0x6C, 0x6F]);
/// ```
///
/// # Widest-first matching
///
/// When ranges overlap, the widest matching range is chosen:
///
/// ```
/// use pdftract_core::cmap::{CodespaceRange, CodespaceRanges, tokenize_cjk_bytes};
///
/// // Overlapping: 0x80-0xFF as both 1-byte and 2-byte lead
/// let mut codespace = CodespaceRanges::new();
/// codespace.push(CodespaceRange::new([0x80, 0, 0, 0], [0xFF, 0, 0, 0], 1));
/// codespace.push(CodespaceRange::new([0x80, 0x00, 0, 0], [0xFF, 0xFF, 0, 0], 2));
///
/// let bytes = &[0x80, 0xA0]; // Should tokenize as single 2-byte code 0x80A0
/// let mut diagnostics = Vec::new();
/// let codes = tokenize_cjk_bytes(&codespace, bytes, &mut diagnostics);
///
/// assert_eq!(codes, &[0x80A0]);
/// ```
pub fn tokenize_cjk_bytes(
    codespace: &CodespaceRanges,
    bytes: &[u8],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<u32> {
    // Preallocate: upper bound is bytes.len() for 1-byte codes
    let mut codes = Vec::with_capacity(bytes.len());

    // Track which byte values we've already emitted diagnostics for (flood prevention)
    let mut emitted_unknown: HashSet<u8> = HashSet::new();

    let mut cursor = 0;

    // Handle empty codespace: default to single-byte 0x00-0xFF coverage
    let use_default_fallback = codespace.is_empty();

    while cursor < bytes.len() {
        let mut matched = false;

        // Try widest first (per ISO 32000-1 9.10.3.1)
        for width in [4u8, 3, 2, 1] {
            let width_usize = width as usize;

            // Check if we have enough bytes remaining
            if cursor + width_usize > bytes.len() {
                continue;
            }

            let candidate = &bytes[cursor..cursor + width_usize];

            // Check against all ranges of this width
            for range in &codespace.ranges {
                if range.width != width {
                    continue;
                }

                // Per-byte range check: candidate[i] must be in [range.lo[i], range.hi[i]]
                let in_range = (0..width_usize).all(|i| {
                    let b = candidate[i];
                    b >= range.lo[i] && b <= range.hi[i]
                });

                if in_range {
                    // Pack big-endian into u32
                    let mut code = 0u32;
                    for &b in candidate {
                        code = (code << 8) | b as u32;
                    }
                    codes.push(code);
                    cursor += width_usize;
                    matched = true;
                    break;
                }
            }

            if matched {
                break;
            }
        }

        if !matched {
            // Handle unrecognized byte
            let b = bytes[cursor];

            if use_default_fallback {
                // Empty codespace: default to single-byte coverage
                codes.push(b as u32);
            } else {
                // Emit U+FFFD and diagnostic once per unique byte value
                codes.push(0xFFFD);
                if emitted_unknown.insert(b) {
                    emit!(diagnostics, CjkTokenizeUnknownByte, offset = cursor as u64);
                }
            }

            cursor += 1;
        }
    }

    codes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_hello() {
        // Acceptance criterion: Input ASCII bytes 0x48 0x65 0x6C 0x6C 0x6F with codespace <00><7F> → codes [0x48, 0x65, 0x6C, 0x6C, 0x6F]
        let mut codespace = CodespaceRanges::new();
        codespace.push(CodespaceRange::new([0x00, 0, 0, 0], [0x7F, 0, 0, 0], 1));

        let bytes = &[0x48, 0x65, 0x6C, 0x6C, 0x6F]; // "Hello"
        let mut diagnostics = Vec::new();
        let codes = tokenize_cjk_bytes(&codespace, bytes, &mut diagnostics);

        assert_eq!(codes, &[0x48, 0x65, 0x6C, 0x6C, 0x6F]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_2_byte_cjk() {
        // Acceptance criterion: Input 2-byte CJK 0x82 0xA0 with codespace <8000><FFFF> → codes [0x82A0]
        let mut codespace = CodespaceRanges::new();
        codespace.push(CodespaceRange::new([0x80, 0x00, 0, 0], [0xFF, 0xFF, 0, 0], 2));

        let bytes = &[0x82, 0xA0];
        let mut diagnostics = Vec::new();
        let codes = tokenize_cjk_bytes(&codespace, bytes, &mut diagnostics);

        assert_eq!(codes, &[0x82A0]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_mixed_1_and_2_byte() {
        // Acceptance criterion: Mixed 1+2 byte input: 0x48 0x82 0xA0 with codespace <00><7F><8000><FFFF> → [0x48, 0x82A0]
        let mut codespace = CodespaceRanges::new();
        codespace.push(CodespaceRange::new([0x00, 0, 0, 0], [0x7F, 0, 0, 0], 1));
        codespace.push(CodespaceRange::new([0x80, 0x00, 0, 0], [0xFF, 0xFF, 0, 0], 2));

        let bytes = &[0x48, 0x82, 0xA0];
        let mut diagnostics = Vec::new();
        let codes = tokenize_cjk_bytes(&codespace, bytes, &mut diagnostics);

        assert_eq!(codes, &[0x48, 0x82A0]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_unrecognized_byte_emits_replacement_and_diagnostic() {
        // Acceptance criterion: Unrecognized byte (no matching range): emit U+FFFD code + CJK_TOKENIZE_UNKNOWN_BYTE diagnostic once
        let mut codespace = CodespaceRanges::new();
        codespace.push(CodespaceRange::new([0x00, 0, 0, 0], [0x7F, 0, 0, 0], 1));

        // 0x80 is outside the 0x00-0x7F range
        let bytes = &[0x48, 0x80, 0x6C];
        let mut diagnostics = Vec::new();
        let codes = tokenize_cjk_bytes(&codespace, bytes, &mut diagnostics);

        assert_eq!(codes, &[0x48, 0xFFFD, 0x6C]);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagCode::CjkTokenizeUnknownByte);
    }

    #[test]
    fn test_unrecognized_byte_diagnostic_emitted_once_per_unique_byte() {
        // Multiple occurrences of the same unrecognized byte should emit only one diagnostic
        let mut codespace = CodespaceRanges::new();
        codespace.push(CodespaceRange::new([0x00, 0, 0, 0], [0x7F, 0, 0, 0], 1));

        // 0x80 appears three times, 0x90 once
        let bytes = &[0x48, 0x80, 0x80, 0x90, 0x80];
        let mut diagnostics = Vec::new();
        let codes = tokenize_cjk_bytes(&codespace, bytes, &mut diagnostics);

        assert_eq!(codes, &[0x48, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD]);
        // Two diagnostics: one for 0x80, one for 0x90
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics.iter().all(|d| d.code == DiagCode::CjkTokenizeUnknownByte));
    }

    #[test]
    fn test_empty_codespace_defaults_to_single_byte() {
        // Acceptance criterion: Empty codespace defaults to 1-byte 0x00-0xFF coverage
        let codespace = CodespaceRanges::new();

        let bytes = &[0x00, 0x48, 0x80, 0xFF];
        let mut diagnostics = Vec::new();
        let codes = tokenize_cjk_bytes(&codespace, bytes, &mut diagnostics);

        // All bytes should be passed through as-is
        assert_eq!(codes, &[0x00, 0x48, 0x80, 0xFF]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_widest_first_matching() {
        // Acceptance criterion: Regression test for widest-first vs shortest-first
        // 0x80 in both 1-byte and 2-byte lead range should match 2-byte
        let mut codespace = CodespaceRanges::new();
        codespace.push(CodespaceRange::new([0x80, 0, 0, 0], [0xFF, 0, 0, 0], 1));
        codespace.push(CodespaceRange::new([0x80, 0x00, 0, 0], [0xFF, 0xFF, 0, 0], 2));

        let bytes = &[0x80, 0xA0];
        let mut diagnostics = Vec::new();
        let codes = tokenize_cjk_bytes(&codespace, bytes, &mut diagnostics);

        // Should tokenize as a single 2-byte code, not two 1-byte codes
        assert_eq!(codes, &[0x80A0]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_3_byte_range() {
        let mut codespace = CodespaceRanges::new();
        codespace.push(CodespaceRange::new([0x80, 0x00, 0x00, 0], [0xFF, 0xFF, 0xFF, 0], 3));

        let bytes = &[0x81, 0x40, 0xA0];
        let mut diagnostics = Vec::new();
        let codes = tokenize_cjk_bytes(&codespace, bytes, &mut diagnostics);

        assert_eq!(codes, &[0x8140A0]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_4_byte_range() {
        let mut codespace = CodespaceRanges::new();
        codespace.push(CodespaceRange::new(
            [0x80, 0x00, 0x00, 0x00],
            [0xFF, 0xFF, 0xFF, 0xFF],
            4,
        ));

        let bytes = &[0x81, 0x40, 0xA0, 0xB0];
        let mut diagnostics = Vec::new();
        let codes = tokenize_cjk_bytes(&codespace, bytes, &mut diagnostics);

        assert_eq!(codes, &[0x8140A0B0]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_mixed_widths_jis_cmap() {
        // Realistic JIS CMap: 1-byte ASCII + 2-byte CJK
        let mut codespace = CodespaceRanges::new();
        codespace.push(CodespaceRange::new([0x00, 0, 0, 0], [0x7F, 0, 0, 0], 1));
        codespace.push(CodespaceRange::new([0x81, 0x40, 0, 0], [0xFE, 0xFE, 0, 0], 2));

        // "Hello" followed by two CJK characters
        let bytes = &[0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x81, 0x40, 0x82, 0xA0];
        let mut diagnostics = Vec::new();
        let codes = tokenize_cjk_bytes(&codespace, bytes, &mut diagnostics);

        assert_eq!(codes, &[0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x8140, 0x82A0]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_partial_match_at_end_of_input() {
        // If we're at the end of input and don't have enough bytes for a multi-byte sequence,
        // we should fall through to unrecognized byte handling
        let mut codespace = CodespaceRanges::new();
        codespace.push(CodespaceRange::new([0x00, 0, 0, 0], [0x7F, 0, 0, 0], 1));
        codespace.push(CodespaceRange::new([0x80, 0x00, 0, 0], [0xFF, 0xFF, 0, 0], 2));

        // 0x81 at end of input (partial 2-byte sequence)
        let bytes = &[0x48, 0x81];
        let mut diagnostics = Vec::new();
        let codes = tokenize_cjk_bytes(&codespace, bytes, &mut diagnostics);

        assert_eq!(codes, &[0x48, 0xFFFD]);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagCode::CjkTokenizeUnknownByte);
    }

    #[test]
    fn test_per_byte_range_check() {
        // Ensure range matching is per-byte, not just comparing packed values
        let mut codespace = CodespaceRanges::new();
        // Range: first byte 0x80-0x9F, second byte 0x40-0x7F
        codespace.push(CodespaceRange::new([0x80, 0x40, 0, 0], [0x9F, 0x7F, 0, 0], 2));

        let bytes = &[0x85, 0x50]; // Both bytes in range
        let mut diagnostics = Vec::new();
        let codes = tokenize_cjk_bytes(&codespace, bytes, &mut diagnostics);

        assert_eq!(codes, &[0x8550]);
        assert!(diagnostics.is_empty());

        // Second byte out of range
        let bytes = &[0x85, 0x80];
        let mut diagnostics = Vec::new();
        let codes = tokenize_cjk_bytes(&codespace, bytes, &mut diagnostics);

        // Should not match the 2-byte range (0x80 > 0x7F for second byte)
        // and should emit U+FFFD for unrecognized bytes
        assert!(codes.len() == 2);
        assert_eq!(codes[0], 0xFFFD);
        assert_eq!(codes[1], 0xFFFD);
    }

    #[test]
    fn test_empty_input() {
        let mut codespace = CodespaceRanges::new();
        codespace.push(CodespaceRange::new([0x00, 0, 0, 0], [0x7F, 0, 0, 0], 1));

        let bytes = &[];
        let mut diagnostics = Vec::new();
        let codes = tokenize_cjk_bytes(&codespace, bytes, &mut diagnostics);

        assert!(codes.is_empty());
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_identity_h_cmap() {
        // Identity-H CMap: <00><FF> for 1-byte, <0100><FFFF> for 2-byte
        let mut codespace = CodespaceRanges::new();
        codespace.push(CodespaceRange::new([0x00, 0, 0, 0], [0xFF, 0, 0, 0], 1));
        codespace.push(CodespaceRange::new([0x01, 0x00, 0, 0], [0xFF, 0xFF, 0, 0], 2));

        // Mix of 1-byte and 2-byte codes
        let bytes = &[0x41, 0x01, 0x00, 0xFF, 0x01, 0x23, 0x45];
        let mut diagnostics = Vec::new();
        let codes = tokenize_cjk_bytes(&codespace, bytes, &mut diagnostics);

        // 0x41 is 1-byte, 0x0100 is 2-byte (at index 1-2), 0xFF is 1-byte, 0x012345 is incomplete at end
        // So: 0x41, 0x0100 (as 2-byte), 0xFF, then 0x01 and 0x23 as unrecognized (since incomplete 2-byte)
        // Actually wait, 0x01 0x23 is a valid 2-byte sequence in range <0100><FFFF>
        // And 0x45 is left over as a 1-byte code
        assert_eq!(codes, &[0x41, 0x0100, 0xFF, 0x0123, 0x45]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_widest_first_three_byte_overlap() {
        // Test that widest-first correctly prefers 3-byte over 2-byte and 1-byte
        let mut codespace = CodespaceRanges::new();
        codespace.push(CodespaceRange::new([0x80, 0, 0, 0], [0xFF, 0, 0, 0], 1));
        codespace.push(CodespaceRange::new([0x80, 0x00, 0, 0], [0xFF, 0xFF, 0, 0], 2));
        codespace.push(CodespaceRange::new([0x80, 0x00, 0x00, 0], [0xFF, 0xFF, 0xFF, 0], 3));

        let bytes = &[0x81, 0x40, 0xA0];
        let mut diagnostics = Vec::new();
        let codes = tokenize_cjk_bytes(&codespace, bytes, &mut diagnostics);

        // Should match as a single 3-byte code
        assert_eq!(codes, &[0x8140A0]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_all_bytes_0x00_to_0xff_empty_codespace() {
        // Comprehensive test: all 256 byte values with empty codespace
        let codespace = CodespaceRanges::new();

        let mut bytes = Vec::with_capacity(256);
        for b in 0u8..=255 {
            bytes.push(b);
        }

        let mut diagnostics = Vec::new();
        let codes = tokenize_cjk_bytes(&codespace, &bytes, &mut diagnostics);

        assert_eq!(codes.len(), 256);
        for (i, &code) in codes.iter().enumerate() {
            assert_eq!(code, i as u32);
        }
        assert!(diagnostics.is_empty());
    }
}
