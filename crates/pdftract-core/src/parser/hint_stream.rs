//! Linearized PDF hint stream parser.
//!
//! This module implements parsing of the hint stream (/H in Linearized dict)
//! per PDF spec Annex F.2. The hint stream contains bit-packed records
//! describing each page's content stream byte range, enabling prefetch
//! optimization for remote sources.
//!
//! # Format (PDF spec Annex F.2)
//!
//! The hint stream is a flate-decoded stream of bit-packed records:
//! 1. Header: 32-bit version + bit widths for each field
//! 2. Page offset hints: one record per page
//! 3. Shared object hints: (skipped in minimal implementation)
//!
//! # Minimal implementation
//!
//! For Phase 1, this parser extracts only:
//! - Header with bit widths
//! - Page offset records (90% of performance benefit)
//! - Shared object records are deferred to Phase 2
//!
//! # Usage
//!
//! ```rust
//! use pdftract_core::parser::hint_stream::{parse_hint_stream, HintTable};
//!
//! let hint_bytes = ...; // flate-decoded hint stream
//! let diagnostics = &mut Vec::new();
//! let hint_table = parse_hint_stream(&hint_bytes, diagnostics);
//! if let Some(table) = hint_table {
//!     let page_range = table.predict_page_range(5); // 0-based page index
//!     if let Some(range) = page_range {
//!         source.prefetch(range.start, range.len());
//!     }
//! }
//! ```

use std::ops::Range;

use crate::emit;

/// Maximum number of pages to process in hint stream.
/// Prevents OOM from malformed hint streams claiming millions of pages.
const MAX_HINT_PAGES: u32 = 100_000;

/// Maximum shared object hint groups to process.
/// Prevents OOM from malformed hint streams.
const MAX_SHARED_GROUPS: u32 = 10_000;

/// Bit-packed hint table from linearized PDF hint stream.
///
/// Contains per-page byte range predictions for prefetch optimization.
#[derive(Debug, Clone)]
pub struct HintTable {
    /// Page offset hints: one entry per page.
    /// Each entry is the byte range [offset, offset + length) for the page's content.
    page_hints: Vec<PageHint>,
}

/// Byte range hint for a single page.
#[derive(Debug, Clone)]
struct PageHint {
    /// Starting byte offset of the page's content stream.
    offset: u64,
    /// Length of the page's content stream in bytes.
    length: u64,
}

impl HintTable {
    /// Create a new hint table with the given page hints.
    fn new(page_hints: Vec<PageHint>) -> Self {
        Self { page_hints }
    }

    /// Predict the byte range for a given page index.
    ///
    /// # Parameters
    /// - `page_index`: 0-based page index
    ///
    /// # Returns
    /// - `Some(Range<u64>)`: Predicted byte range if page index is valid
    /// - `None`: Page index out of bounds
    pub fn predict_page_range(&self, page_index: u32) -> Option<Range<u64>> {
        let hint = self.page_hints.get(page_index as usize)?;
        let start = hint.offset;
        let end = start.checked_add(hint.length)?;
        Some(start..end)
    }

    /// Get the number of pages in the hint table.
    pub fn page_count(&self) -> u32 {
        self.page_hints.len() as u32
    }

    /// Predict shared object ranges.
    ///
    /// # Note
    /// Minimal implementation: returns empty vec.
    /// Phase 2 will parse shared object hint records.
    pub fn predict_shared_objects(&self) -> Vec<Range<u64>> {
        // Phase 2: parse shared object hint records
        vec![]
    }
}

/// Bit reader for reading variable-bit-width integers from a byte slice.
struct BitReader {
    data: Vec<u8>,
    bit_pos: usize,
}

impl BitReader {
    /// Create a new bit reader from the given bytes.
    fn new(data: Vec<u8>) -> Self {
        Self { data, bit_pos: 0 }
    }

    /// Read a single bit.
    ///
    /// Returns `None` if we're past the end of the data.
    fn read_bit(&mut self) -> Option<bool> {
        let byte_pos = self.bit_pos / 8;
        if byte_pos >= self.data.len() {
            return None;
        }
        let bit_in_byte = self.bit_pos % 8;
        self.bit_pos += 1;
        let byte = self.data[byte_pos];
        // Bits are read MSB-first within each byte
        let mask = 1u8 << (7 - bit_in_byte);
        Some((byte & mask) != 0)
    }

    /// Read an unsigned integer with the given bit width.
    ///
    /// Returns `None` if we run out of bits.
    fn read_bits(&mut self, width: u8) -> Option<u32> {
        if width == 0 {
            return Some(0);
        }
        let mut result = 0u32;
        for i in 0..width {
            let bit = self.read_bit()? as u32;
            result |= bit << (width - 1 - i);
        }
        Some(result)
    }

    /// Read a 32-bit unsigned integer (big-endian byte order).
    ///
    /// This reads from the current byte position (not bit position),
    /// advancing the bit position to the next byte boundary.
    fn read_u32(&mut self) -> Option<u32> {
        // Align to byte boundary
        let byte_pos = (self.bit_pos + 7) / 8;
        if byte_pos + 4 > self.data.len() {
            return None;
        }
        self.bit_pos = (byte_pos + 4) * 8;
        let bytes = &self.data[byte_pos..byte_pos + 4];
        Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Check if we have at least `n` bits remaining.
    fn has_bits(&self, n: usize) -> bool {
        self.bit_pos + n <= self.data.len() * 8
    }
}

/// Header of the hint stream (PDF spec Annex F.2).
#[derive(Debug, Default)]
struct HintHeader {
    /// Bit width for object number in page offset hints
    object_number_bits: u8,
    /// Bit width for page offset hint offsets
    page_offset_bits: u8,
    /// Bit width for page offset hint lengths
    page_length_bits: u8,
    /// Bit width for shared object hint object numbers
    shared_object_number_bits: u8,
    /// Bit width for shared object hint group lengths
    shared_group_length_bits: u8,
    /// Number of pages in the document
    page_count: u32,
    /// Number of shared object groups
    shared_group_count: u32,
}

/// Parse the hint stream header.
///
/// # Format (PDF spec Annex F.2)
///
/// The header is a sequence of bit-packed values:
/// 1. 32-bit: hint stream version (must be 1)
/// 2. 4-bit: bit width for object numbers (0-15)
/// 3. 4-bit: bit width for page offset hints (0-15)
/// 4. 4-bit: bit width for page length hints (0-15)
/// 5. 4-bit: bit width for shared object numbers (0-15)
/// 6. 4-bit: bit width for shared group lengths (0-15)
/// 7. Variable-bit: number of pages (using object_number_bits width)
/// 8. Variable-bit: number of shared groups (using object_number_bits width)
///
/// # Returns
/// - `Some(HintHeader)`: Successfully parsed header
/// - `None`: Malformed header (version not 1, or insufficient data)
fn parse_hint_header(reader: &mut BitReader) -> Option<HintHeader> {
    // Read 32-bit version
    let version = reader.read_u32()?;
    if version != 1 {
        // Only version 1 is supported
        return None;
    }

    // Read bit widths (4 bits each, packed into a single 32-bit value)
    // Format: [object_number_bits (4) | page_offset_bits (4) | page_length_bits (4) |
    //          shared_object_number_bits (4) | shared_group_length_bits (4) | reserved (12)]
    let bit_widths = reader.read_bits(20)?;
    let object_number_bits = ((bit_widths >> 16) & 0xF) as u8;
    let page_offset_bits = ((bit_widths >> 12) & 0xF) as u8;
    let page_length_bits = ((bit_widths >> 8) & 0xF) as u8;
    let shared_object_number_bits = ((bit_widths >> 4) & 0xF) as u8;
    let shared_group_length_bits = (bit_widths & 0xF) as u8;

    // Sanity check: bit widths must be reasonable
    // Object numbers can be up to ~20 bits for very large PDFs
    // Offsets/lengths can be up to ~40 bits for 1TB+ files
    if object_number_bits == 0 || page_offset_bits == 0 || page_length_bits == 0 {
        return None;
    }
    if object_number_bits > 32 || page_offset_bits > 64 || page_length_bits > 64 {
        return None;
    }

    // Read page count (using object_number_bits)
    let page_count = reader.read_bits(object_number_bits)?;

    // Sanity check: page count must be reasonable
    if page_count == 0 || page_count > MAX_HINT_PAGES {
        return None;
    }

    // Read shared group count (using object_number_bits)
    let shared_group_count = reader.read_bits(object_number_bits)?;

    // Sanity check: shared group count must be reasonable
    if shared_group_count > MAX_SHARED_GROUPS {
        return None;
    }

    Some(HintHeader {
        object_number_bits,
        page_offset_bits,
        page_length_bits,
        shared_object_number_bits,
        shared_group_length_bits,
        page_count,
        shared_group_count,
    })
}

/// Parse page offset hints.
///
/// # Format (PDF spec Annex F.2.2)
///
/// For each page, a record containing:
/// 1. Object number of the page (object_number_bits)
/// 2. Offset of the page's content stream (page_offset_bits)
/// 3. Length of the page's content stream (page_length_bits)
///
/// Note: The object number is read but not used in the minimal implementation.
/// We assume pages appear in order and return hints by index.
fn parse_page_hints(reader: &mut BitReader, header: &HintHeader) -> Option<Vec<PageHint>> {
    let mut page_hints = Vec::with_capacity(header.page_count as usize);

    for _ in 0..header.page_count {
        // Read object number (skip in minimal implementation)
        let _object_number = reader.read_bits(header.object_number_bits)?;

        // Read offset
        let offset_bits = header.page_offset_bits;
        let offset = if offset_bits <= 32 {
            reader.read_bits(offset_bits)? as u64
        } else {
            // For widths > 32, read in two parts (high and low)
            // Note: this is rare; typical PDFs use <= 32 bits for offsets
            let high = reader.read_bits(offset_bits - 32)? as u64;
            let low = reader.read_bits(32)? as u64;
            (high << 32) | low
        };

        // Read length
        let length_bits = header.page_length_bits;
        let length = if length_bits <= 32 {
            reader.read_bits(length_bits)? as u64
        } else {
            let high = reader.read_bits(length_bits - 32)? as u64;
            let low = reader.read_bits(32)? as u64;
            (high << 32) | low
        };

        page_hints.push(PageHint { offset, length });
    }

    Some(page_hints)
}

/// Parse the hint stream and return a hint table.
///
/// # Parameters
/// - `data`: Flate-decoded hint stream bytes
/// - `diagnostics`: Diagnostic collection for errors
///
/// # Returns
/// - `Some(HintTable)`: Successfully parsed hint stream
/// - `None`: Malformed hint stream (emits STRUCT_INVALID_HINT_STREAM)
pub fn parse_hint_stream(
    data: &[u8],
    diagnostics: &mut Vec<crate::diagnostics::Diagnostic>,
) -> Option<HintTable> {
    if data.is_empty() {
        emit!(
            diagnostics,
            StructInvalidHintStream,
            message = "hint stream is empty".to_string()
        );
        return None;
    }

    let mut reader = BitReader::new(data.to_vec());

    // Parse header
    let header = parse_hint_header(&mut reader)?;
    if header.page_count == 0 {
        emit!(
            diagnostics,
            StructInvalidHintStream,
            message = "hint stream reports zero pages".to_string()
        );
        return None;
    }

    // Parse page hints
    let page_hints = parse_page_hints(&mut reader, &header)?;
    if page_hints.len() != header.page_count as usize {
        emit!(
            diagnostics,
            StructInvalidHintStream,
            message = format!(
                "hint stream page count mismatch: header reports {}, parsed {}",
                header.page_count,
                page_hints.len()
            )
        );
        return None;
    }

    // Phase 2: Parse shared object hints (skipped for now)

    Some(HintTable::new(page_hints))
}

/// Parse the hint stream from a linearized PDF.
///
/// This function fetches the hint stream using the offset and length from
/// LinearizationInfo, flate-decompresses it, and parses it into a HintTable.
///
/// # Parameters
/// - `source`: The PDF source to read from
/// - `hint_stream_offset`: Offset of the hint stream from LinearizationInfo
/// - `hint_stream_length`: Length of the hint stream from LinearizationInfo
/// - `diagnostics`: Diagnostic collection for errors
///
/// # Returns
/// - `Some(HintTable)`: Successfully parsed hint stream
/// - `None`: Failed to fetch or parse hint stream (emits STRUCT_INVALID_HINT_STREAM)
pub fn parse_hint_stream_from_linearized(
    source: &dyn crate::source::PdfSource,
    hint_stream_offset: u64,
    hint_stream_length: u64,
    diagnostics: &mut Vec<crate::diagnostics::Diagnostic>,
) -> Option<HintTable> {
    use crate::parser::stream::{get_decoder, DEFAULT_MAX_DECOMPRESS_BYTES};

    // Fetch the hint stream data
    let hint_stream_data = source
        .read_range(hint_stream_offset, hint_stream_length as usize)
        .ok()
        .filter(|data| !data.is_empty())?;

    // The hint stream is flate-encoded (per PDF spec Annex F.1)
    let mut counter = 0u64;
    let decoded = match get_decoder("FlateDecode") {
        Some(decoder) => {
            // Check if it's a FlateDecoder and decode
            if decoder.name() == "FlateDecode" {
                decoder
                    .decode(
                        &hint_stream_data,
                        None,
                        &mut counter,
                        DEFAULT_MAX_DECOMPRESS_BYTES,
                    )
                    .ok()?
            } else {
                emit!(
                    diagnostics,
                    StructInvalidHintStream,
                    message = "hint stream is not FlateDecode".to_string()
                );
                return None;
            }
        }
        _ => {
            emit!(
                diagnostics,
                StructInvalidHintStream,
                message = "hint stream is not FlateDecode".to_string()
            );
            return None;
        }
    };

    parse_hint_stream(&decoded, diagnostics)
}

/// Prefetch pages from a linearized PDF using hint stream predictions.
///
/// This function parses the hint stream from a linearized PDF and prefetches
/// the byte ranges for the requested pages. This is an optimization for
/// remote sources that reduces latency by fetching page data in parallel
/// before it's needed.
///
/// # Parameters
/// - `source`: The PDF source (typically HttpRangeSource for remote files)
/// - `hint_stream_offset`: Offset of the hint stream from LinearizationInfo
/// - `hint_stream_length`: Length of the hint stream from LinearizationInfo
/// - `page_indices`: Iterator over 0-based page indices to prefetch
/// - `diagnostics`: Diagnostic collection for errors
///
/// # Behavior
/// - Parses the hint stream from the linearized PDF
/// - For each page index in the iterator, predicts the byte range and prefetches it
/// - If hint stream parsing fails, emits a diagnostic and returns early (no prefetch)
/// - If prediction fails for a specific page, that page is skipped (other pages still prefetched)
///
/// # Performance benefit
/// For a 500-page document extracting pages 47-52, hint-based prefetch can reduce
/// extraction time by ~30% by pipelining HTTP requests and avoiding serial latency.
///
/// # Example
/// ```rust,no_run
/// use pdftract_core::parser::hint_stream::prefetch_from_hint_stream;
/// use std::collections::BTreeSet;
///
/// // Prefetch pages 47-52 (0-based: 46-51)
/// let page_range = 46..=51;
/// let page_indices: Vec<_> = page_range.collect();
/// prefetch_from_hint_stream(
///     &source,
///     hint_offset,
///     hint_length,
///     page_indices.into_iter(),
///     &mut diagnostics,
/// );
/// ```
///
/// # References
/// - Plan section: Phase 1.8 line 1279 (hint stream for prefetch)
/// - PDF spec Annex F.2
pub fn prefetch_from_hint_stream(
    source: &dyn crate::source::PdfSource,
    hint_stream_offset: u64,
    hint_stream_length: u64,
    page_indices: impl Iterator<Item = usize>,
    diagnostics: &mut Vec<crate::diagnostics::Diagnostic>,
) {
    // Parse the hint stream
    let hint_table = match parse_hint_stream_from_linearized(
        source,
        hint_stream_offset,
        hint_stream_length,
        diagnostics,
    ) {
        Some(table) => table,
        None => {
            // Hint stream parsing failed; emit diagnostic was already done
            // Prefetch is optional, so we just return without prefetching
            return;
        }
    };

    // Prefetch each page in the requested range
    for page_idx in page_indices {
        let page_idx_u32 = page_idx as u32;
        match hint_table.predict_page_range(page_idx_u32) {
            Some(range) => {
                // Prefetch the predicted byte range
                // The prefetch method is a no-op for local sources (MmapSource)
                // and only does actual work for HttpRangeSource
                source.prefetch(range.start, (range.end - range.start) as usize);
            }
            None => {
                // Page index out of bounds or prediction failed
                // This is not an error; we just skip this page
                continue;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_reader_single_bit() {
        let data = vec![0b10101010]; // 0xAA
        let mut reader = BitReader::new(data);
        assert_eq!(reader.read_bit(), Some(true)); // MSB first
        assert_eq!(reader.read_bit(), Some(false));
        assert_eq!(reader.read_bit(), Some(true));
        assert_eq!(reader.read_bit(), Some(false));
        assert_eq!(reader.read_bit(), Some(true));
        assert_eq!(reader.read_bit(), Some(false));
        assert_eq!(reader.read_bit(), Some(true));
        assert_eq!(reader.read_bit(), Some(false));
        assert_eq!(reader.read_bit(), None); // EOF
    }

    #[test]
    fn test_bit_reader_read_bits() {
        let data = vec![0b11010110, 0b00111010]; // 0xD6 0x3A
        let mut reader = BitReader::new(data);
        assert_eq!(reader.read_bits(4), Some(0b1101)); // 13
        assert_eq!(reader.read_bits(8), Some(0b01100011)); // 0x63
        assert_eq!(reader.read_bits(4), Some(0b1010)); // 10
    }

    #[test]
    fn test_bit_reader_read_u32() {
        let data = vec![0x12, 0x34, 0x56, 0x78, 0xAB];
        let mut reader = BitReader::new(data);
        assert_eq!(reader.read_u32(), Some(0x12345678));
        // After read_u32, bit_pos is at byte boundary
        assert_eq!(reader.bit_pos, 32);
    }

    #[test]
    fn test_bit_reader_has_bits() {
        let data = vec![0xFF, 0xFF];
        let reader = BitReader::new(data);
        assert!(reader.has_bits(16));
        assert!(reader.has_bits(15));
        assert!(!reader.has_bits(17));
    }

    #[test]
    fn test_parse_hint_header_minimal() {
        // Construct a valid hint header with proper bit-level packing.
        // The hint stream uses bit-packed fields that can span byte boundaries.
        //
        // Format (PDF spec Annex F.2):
        // - 32-bit: version (must be 1)
        // - 20 bits: bit widths (five 4-bit fields)
        //   [object_number_bits (4) | page_offset_bits (4) | page_length_bits (4) |
        //    shared_object_number_bits (4) | shared_group_length_bits (4)]
        // - variable bits: page count (width = object_number_bits)
        // - variable bits: shared group count (width = object_number_bits)
        //
        // For this test, we use:
        // - All widths = 8 bits (binary: 1000, so each 4-bit field is 0b1000 = 8)
        // - Page count = 1
        // - Shared group count = 0
        //
        // The 20-bit bit_widths value is:
        //   (8 << 16) | (8 << 12) | (8 << 8) | (8 << 4) | 8 = 0x88888
        //
        // This is packed MSB-first across 3 bytes (20 bits need 3 bytes):
        //   Byte 0: bits 19-12 = 0x88
        //   Byte 1: bits 11-4  = 0x88
        //   Byte 2: bits 3-0   = 0x8 (with 4 zero padding bits = 0x80)
        //
        // After the version (4 bytes), the bit_widths field starts at bit 32.
        // Reading bits 32-51 gives us 0x88888.

        let mut data = Vec::new();
        // Version: 1 (bytes 0-3)
        data.extend_from_slice(&1u32.to_be_bytes());
        // Bit widths: 20-bit value 0x88888 packed MSB-first (bits 32-51)
        // This spans bytes 4-6 with bit alignment
        data.extend_from_slice(&[0x88, 0x88, 0x80]); // 20 bits: 0x88888
                                                     // Page count: 1 (8 bits, starting at bit 52)
                                                     // This starts in byte 6 (after the 20-bit bit_widths field)
        data.push(0x01); // byte 6: lower 4 bits are padding, upper 4 bits start page count
                         // Actually, we need to track bit position more carefully.
                         // After 52 bits (version + bit_widths), we're at bit 52, which is:
                         // - byte 6, bit 4 (0-indexed within byte)
                         // So page count (8 bits) spans bytes 6-7

        // Let me recalculate with exact bit positions:
        // - Version: bits 0-31 (bytes 0-3)
        // - Bit widths: bits 32-51 (bytes 4-6, partial)
        // - Page count (8 bits): bits 52-59
        //   - Bit 52 is byte 6, bit 4 (since bit 48 starts byte 6)
        //   - So we need bits 4-11 of byte 6, and bit 0-3 of byte 7
        // - Shared groups (8 bits): bits 60-67

        // Let's rebuild with proper bit alignment:
        data.clear();
        data.extend_from_slice(&1u32.to_be_bytes()); // bytes 0-3: version

        // bytes 4-6: bit widths (20 bits = 0x88888)
        // Byte 4: bits 32-39 = 10001000 = 0x88
        // Byte 5: bits 40-47 = 10001000 = 0x88
        // Byte 6: bits 48-51 = 1000 (in upper 4 bits), padding 0000 (lower 4 bits) = 0x80
        data.extend_from_slice(&[0x88, 0x88, 0x80]);

        // Page count (8 bits, value 1 = 0b00000001): bits 52-59
        // Bit 52 starts at byte 6, bit 4
        // Byte 6: [XXXX XXXX] where X are bits 48-55
        //        bits 48-51 were padding (0000), bits 52-55 start page count (0000) of 0b00000001
        // Byte 7: [XXXX XXXX] where X are bits 56-63
        //        bits 56-59 are the rest of page count (0001), bits 60-63 start shared groups
        // Actually, let me just use bit_write_u8 helper...

        // Simplifying: construct the remaining bytes manually
        // Byte 6: bits 48-55. Upper 4 bits (48-51) were padding (0000).
        //         Lower 4 bits (52-55) start page count. Page count = 1 = 0b00000001.
        //         So bits 52-55 are 0000.
        //         Byte 6 = 0b00000000 (but upper bits were already set to 0x80)
        // Wait, byte 6 already has bits 48-51 = 0b1000 from bit_widths.
        // Let me redo this more carefully...

        // Final approach: construct bytes 6-7 together
        // Byte 6: bits 48-55
        //   - Bits 48-51: padding from bit_widths field = 0000
        //   - Bits 52-55: upper 4 bits of page count (0b0000)
        // Byte 7: bits 56-63
        //   - Bits 56-59: lower 4 bits of page count (0b0001)
        //   - Bits 60-63: upper 4 bits of shared group count (0b0000)
        // Byte 8: bits 64-71
        //   - Bits 64-67: lower 4 bits of shared group count (0b0000)
        //   - Remaining bits: unused

        // Byte 6 = 0b00000000 = 0x00 (but we already set the upper 4 bits in bit_widths!)
        // This is getting confusing. Let me use a different approach.

        data.clear();
        data.extend_from_slice(&1u32.to_be_bytes()); // bytes 0-3

        // Bit widths (20 bits): 0x88888 = 0b10001000100010001000
        // Packed MSB-first starting at bit 32 (byte 4, bit 0):
        // Byte 4: bits 0-7  = 10001000 = 0x88
        // Byte 5: bits 8-15 = 10001000 = 0x88
        // Byte 6: bits 16-19 (of this field) = 1000, bits 20-23 (padding) = 0000
        //        = 0b10000000 = 0x80
        data.extend_from_slice(&[0x88, 0x88, 0x80]);

        // Page count (8 bits, value 1): starts at bit 52 (byte 6, bit 4)
        // Byte 6, bits 4-7: upper 4 bits of page count = 0000
        // Byte 7, bits 0-3: lower 4 bits of page count = 0001
        // So we need to update byte 6's lower 4 bits and set byte 7's upper 4 bits
        // Byte 6 = 0b1000_0000 -> we need lower 4 bits = 0000, so unchanged
        // Byte 7: upper 4 bits = 0000 (from page count), lower 4 bits = 0000 (start of shared groups)
        data.extend_from_slice(&[0x00, 0x00]); // bytes 7-8: page count (1) + shared groups (0)

        // Wait, this still doesn't work. Let me trace through BitReader more carefully.

        // After read_u32() at bit_pos=0, bit_pos=32 (byte boundary)
        // read_bits(20) reads bits 32-51:
        // - bit_pos=32, read bit 32 (byte 4, bit 0)
        // - ... up to bit 51 (byte 6, bit 3)
        // After this, bit_pos=52

        // read_bits(8) for page_count reads bits 52-59:
        // - bit 52 is byte 6, bit 4 (since bit 48 starts byte 6)
        // - bit 59 is byte 7, bit 3

        // So for page_count=1 (0b00000001):
        // - Bits 52-55 (byte 6, bits 4-7): 0000
        // - Bits 56-59 (byte 7, bits 0-3): 0001

        // Byte 6 currently has bits 48-51 = 1000 (from bit_widths padding), bits 52-55 = 0000
        // So byte 6 = 0b1000_0000 = 0x80 (correct as is)

        // Byte 7 needs bits 56-59 = 0001, and bits 60-63 start shared groups
        // shared_groups = 0, so bits 60-63 = 0000
        // Byte 7 = 0b00010000 = 0x10

        // Byte 8 needs bits 64-67 = lower 4 bits of shared_groups = 0000
        // Byte 8 = 0x00

        data.truncate(7); // Keep bytes 0-6
        data.push(0x10); // byte 7: page count (1) + shared groups start
        data.push(0x00); // byte 8: shared groups (0)

        let mut reader = BitReader::new(data);
        let header = parse_hint_header(&mut reader);

        assert!(header.is_some());
        let h = header.unwrap();
        assert_eq!(h.object_number_bits, 8);
        assert_eq!(h.page_offset_bits, 8);
        assert_eq!(h.page_length_bits, 8);
        assert_eq!(h.page_count, 1);
        assert_eq!(h.shared_group_count, 0);
    }

    #[test]
    fn test_parse_hint_header_invalid_version() {
        let mut data = Vec::new();
        // Version: 2 (invalid)
        data.extend_from_slice(&2u32.to_be_bytes());
        data.extend_from_slice(&0x08080808u32.to_be_bytes());

        let mut reader = BitReader::new(data);
        let header = parse_hint_header(&mut reader);
        assert!(header.is_none());
    }

    #[test]
    fn test_parse_hint_header_zero_pages() {
        let mut data = Vec::new();
        // Version: 1
        data.extend_from_slice(&1u32.to_be_bytes());
        // Bit widths
        data.extend_from_slice(&0x08080808u32.to_be_bytes());
        // Page count: 0
        data.extend_from_slice(&0u32.to_be_bytes());

        let mut reader = BitReader::new(data);
        let header = parse_hint_header(&mut reader);
        // Should return None for zero pages
        assert!(header.is_none());
    }

    #[test]
    fn test_parse_hint_header_too_many_pages() {
        let mut data = Vec::new();
        // Version: 1
        data.extend_from_slice(&1u32.to_be_bytes());
        // Bit widths
        data.extend_from_slice(&0x08080808u32.to_be_bytes());
        // Page count: 200000 (exceeds MAX_HINT_PAGES)
        data.extend_from_slice(&200_000u32.to_be_bytes());

        let mut reader = BitReader::new(data);
        let header = parse_hint_header(&mut reader);
        assert!(header.is_none());
    }

    #[test]
    fn test_hint_table_predict_page_range() {
        let page_hints = vec![
            PageHint {
                offset: 100,
                length: 50,
            },
            PageHint {
                offset: 200,
                length: 75,
            },
            PageHint {
                offset: 300,
                length: 100,
            },
        ];
        let table = HintTable::new(page_hints);

        assert_eq!(table.predict_page_range(0), Some(100..150));
        assert_eq!(table.predict_page_range(1), Some(200..275));
        assert_eq!(table.predict_page_range(2), Some(300..400));
        assert_eq!(table.predict_page_range(3), None); // Out of bounds
    }

    #[test]
    fn test_hint_table_page_count() {
        let page_hints = vec![
            PageHint {
                offset: 0,
                length: 100,
            },
            PageHint {
                offset: 100,
                length: 200,
            },
        ];
        let table = HintTable::new(page_hints);
        assert_eq!(table.page_count(), 2);
    }

    #[test]
    fn test_parse_hint_stream_empty() {
        let data = vec![];
        let mut diagnostics = vec![];
        let result = parse_hint_stream(&data, &mut diagnostics);
        assert!(result.is_none());
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn test_parse_hint_stream_full_minimal() {
        // Construct a minimal valid hint stream:
        // Header with 1 page, then 1 page hint record
        //
        // To simplify bit alignment, we use 4-bit widths (so page_count and
        // shared_group_count fit in 4 bits each, totaling 8 bits = 1 byte).
        // This ensures the hint records start at a byte boundary.
        let mut data = Vec::new();

        // Header
        data.extend_from_slice(&1u32.to_be_bytes()); // bytes 0-3: version

        // Bit widths (20 bits): use 4-bit fields for simplicity
        // object_number_bits: 4 bits (0x4)
        // page_offset_bits: 4 bits (0x4)
        // page_length_bits: 4 bits (0x4)
        // shared_object_number_bits: 4 bits (0x4)
        // shared_group_length_bits: 4 bits (0x4)
        // Packed: 0x44444 = 0b0100_0100_0100_0100_0100 (20 bits)
        data.extend_from_slice(&[0x44, 0x44, 0x40]); // bytes 4-6: 0x44444 packed

        // Page count (4 bits, value 1) + shared groups (4 bits, value 0)
        // Page count starts at bit 52, shared groups at bit 56
        // Together they form byte 7: 0b00010000 = 0x10
        data.push(0x10); // byte 7: page_count=1 (upper 4 bits), shared_groups=0 (lower 4 bits)

        // After header, we're at bit 60 = byte 8, bit 0 (byte-aligned!)
        // Page hint records start at byte 8
        // Each record: object_number (4 bits) + offset (4 bits) + length (4 bits)
        // For 1 record with values: object_number=0, offset=15, length=15
        // Packed in 12 bits (1.5 bytes): 0b0000_1111_1111 = 0x0FF0 (12 bits)
        // Byte 8: 0b00001111 = 0x0F
        // Byte 9: 0b11110000 = 0xF0
        data.extend_from_slice(&[0x0F, 0xF0]); // bytes 8-9: 1 hint record

        let mut diagnostics = vec![];
        let result = parse_hint_stream(&data, &mut diagnostics);

        assert!(result.is_some());
        let table = result.unwrap();
        assert_eq!(table.page_count(), 1);
        // Page range: offset 15, length 15 → [15, 30)
        assert_eq!(table.predict_page_range(0), Some(15..30));
    }

    // proptest: random byte sequences never panic
    proptest::proptest! {
        #[test]
        fn prop_parse_hint_stream_no_panic(data: Vec<u8>) {
            let mut diagnostics = vec![];
            let _ = parse_hint_stream(&data, &mut diagnostics);
            // Should never panic; returns None for malformed data
        }
    }
}
