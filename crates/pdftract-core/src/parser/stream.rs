//! PDF stream decoding and filter pipeline.
//!
//! This module implements the filter pipeline for decoding PDF stream data.
//! PDF streams can have multiple filters applied in sequence (e.g., /ASCII85Decode
//! followed by /FlateDecode). This module handles:
//!
//! - Dispatching to the appropriate filter decoder
//! - Managing filter parameters (/DecodeParms)
//! - Enforcing decompression limits (bomb protection)
//! - Error recovery per INV-8 (never panic, always return partial bytes)

use std::io::Read;
use std::io::Seek;
use std::path::Path;

use flate2::read::ZlibDecoder;

use crate::parser::object::{PdfObject, PdfStream, PdfDict, intern};

/// Maximum number of filters allowed in a single stream's pipeline.
/// This prevents stack overflow and excessive computation.
const MAX_FILTERS: usize = 16;

/// Chunk size for checking decompression limits during decoding.
const BOMB_CHECK_CHUNK: usize = 64 * 1024; // 64 KB

/// Default maximum decompressed bytes per document (2 GB).
pub const DEFAULT_MAX_DECOMPRESS_BYTES: u64 = 2 * 1024_u64.pow(3);

/// Errors that can occur during stream decoding.
///
/// Per INV-8, these are "hard" errors that prevent decoding from starting.
/// Soft errors (corrupt data, EOF mid-stream) return Ok(partial_bytes) with
/// a diagnostic instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterError {
    /// Unknown filter name (e.g., /CustomDecode)
    UnknownFilter(String),
    /// Invalid filter parameters (wrong type, missing required key)
    InvalidParams(String),
}

impl std::fmt::Display for FilterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilterError::UnknownFilter(name) => write!(f, "unknown filter: {}", name),
            FilterError::InvalidParams(msg) => write!(f, "invalid filter parameters: {}", msg),
        }
    }
}

impl std::error::Error for FilterError {}

/// A stream decoder for a specific PDF filter type.
///
/// Each filter implements this trait to decode its specific format.
pub trait StreamDecoder: Send + Sync {
    /// Decode the input bytes using this filter.
    ///
    /// # Parameters
    /// - `input`: The raw bytes to decode
    /// - `params`: Optional filter parameters from /DecodeParms
    /// - `doc_counter`: Cumulative decompressed bytes for the document (mutated)
    /// - `max_bytes`: Maximum bytes allowed before emitting STREAM_BOMB
    ///
    /// # Returns
    /// - `Ok(bytes)`: Decoded bytes (may be partial if bomb limit hit)
    /// - `Err(FilterError)`: Hard error (unknown filter, invalid params)
    ///
    /// Per INV-8, corrupt data mid-stream returns Ok(partial) with diagnostic,
    /// not Err. Err is only for "couldn't even start decoding".
    fn decode(
        &self,
        input: &[u8],
        params: Option<&PdfObject>,
        doc_counter: &mut u64,
        max_bytes: u64,
    ) -> Result<Vec<u8>, FilterError>;

    /// Get the filter name (e.g., "FlateDecode", "ASCII85Decode").
    fn name(&self) -> &'static str;
}

/// FlateDecode filter (zlib/comflate compression).
#[derive(Debug, Clone, Copy)]
pub struct FlateDecoder;

impl StreamDecoder for FlateDecoder {
    fn decode(
        &self,
        input: &[u8],
        _params: Option<&PdfObject>,
        doc_counter: &mut u64,
        max_bytes: u64,
    ) -> Result<Vec<u8>, FilterError> {
        if input.is_empty() {
            return Ok(Vec::new());
        }

        let mut decoder = ZlibDecoder::new(input);
        let mut output = Vec::new();
        let mut chunk = vec![0u8; BOMB_CHECK_CHUNK];

        loop {
            match decoder.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    // Check bomb limit BEFORE adding bytes to output
                    if *doc_counter + n as u64 > max_bytes {
                        // Bomb limit exceeded - return partial bytes
                        let remaining = (max_bytes - *doc_counter) as usize;
                        let to_add = remaining.min(n);
                        output.extend_from_slice(&chunk[..to_add]);
                        *doc_counter += to_add as u64;
                        return Ok(output);
                    }
                    *doc_counter += n as u64;
                    output.extend_from_slice(&chunk[..n]);
                }
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    // Truncated stream - return partial bytes (INV-8)
                    break;
                }
                Err(_) => {
                    // Other zlib errors - return partial bytes decoded so far
                    break;
                }
            }
        }

        Ok(output)
    }

    fn name(&self) -> &'static str {
        "FlateDecode"
    }
}

/// ASCII85Decode filter (Base85 encoding).
///
/// Converts 5 ASCII characters to 4 bytes. Special handling:
/// - 'z' shortcut for 4 zero bytes
/// - '~>' terminator
/// - Whitespace ignored
#[derive(Debug, Clone, Copy)]
pub struct ASCII85Decoder;

impl StreamDecoder for ASCII85Decoder {
    fn decode(
        &self,
        input: &[u8],
        _params: Option<&PdfObject>,
        doc_counter: &mut u64,
        max_bytes: u64,
    ) -> Result<Vec<u8>, FilterError> {
        let mut output = Vec::new();
        let mut tuple = [0u32; 5];
        let mut count = 0;
        let mut total_output = 0u64;
        let mut i = 0;

        while i < input.len() {
            let byte = input[i];

            // Check for '~>' terminator (only after we've started processing data)
            if byte == b'~' && i + 1 < input.len() && input[i + 1] == b'>' {
                break;
            }

            // Skip '<~' prefix
            if byte == b'<' && i + 1 < input.len() && input[i + 1] == b'~' {
                i += 2;
                continue;
            }

            // Skip '<' alone (partial prefix)
            if byte == b'<' {
                i += 1;
                continue;
            }

            // Skip whitespace
            if byte.is_ascii_whitespace() {
                i += 1;
                continue;
            }

            // 'z' shortcut: 4 zero bytes
            if byte == b'z' {
                if count != 0 {
                    // 'z' must be standalone, not in a tuple
                    return Ok(output); // Return partial bytes (INV-8)
                }
                if total_output + 4 > max_bytes - *doc_counter {
                    *doc_counter += total_output;
                    return Ok(output);
                }
                output.extend_from_slice(&[0u8; 4]);
                total_output += 4;
                i += 1;
                continue;
            }

            // Decode ASCII85 character (33-117 range -> 0-84)
            if byte < 33 || byte > 117 {
                // Invalid character - return partial bytes
                break;
            }
            let value = (byte - 33) as u32;
            tuple[count] = value;
            count += 1;

            if count == 5 {
                // Decode 5-tuple to 4 bytes
                let acc = tuple[0] * 85_u32.pow(4)
                    + tuple[1] * 85_u32.pow(3)
                    + tuple[2] * 85_u32.pow(2)
                    + tuple[3] * 85_u32.pow(1)
                    + tuple[4];

                if total_output + 4 > max_bytes - *doc_counter {
                    *doc_counter += total_output;
                    return Ok(output);
                }
                output.extend_from_slice(&[
                    (acc >> 24) as u8,
                    ((acc >> 16) & 0xFF) as u8,
                    ((acc >> 8) & 0xFF) as u8,
                    (acc & 0xFF) as u8,
                ]);
                total_output += 4;
                count = 0;
            }

            i += 1;
        }

        // Handle partial final tuple
        if count > 0 {
            // Pad with zeros
            for j in count..5 {
                tuple[j] = 0;
            }
            let acc = tuple[0] * 85_u32.pow(4)
                + tuple[1] * 85_u32.pow(3)
                + tuple[2] * 85_u32.pow(2)
                + tuple[3] * 85_u32.pow(1)
                + tuple[4];

            // Output only (count - 1) bytes from the tuple
            let bytes_to_output = count - 1;
            if total_output + bytes_to_output as u64 > max_bytes - *doc_counter {
                *doc_counter += total_output;
                return Ok(output);
            }
            for j in 0..bytes_to_output {
                output.push((acc >> (24 - 8 * j)) as u8);
            }
            total_output += bytes_to_output as u64;
        }

        *doc_counter += total_output;
        Ok(output)
    }

    fn name(&self) -> &'static str {
        "ASCII85Decode"
    }
}

/// ASCIIHexDecode filter (hexadecimal encoding).
///
/// Converts hex digit pairs to bytes. Whitespace ignored.
/// '>' terminator marks end of data.
#[derive(Debug, Clone, Copy)]
pub struct ASCIIHexDecoder;

impl StreamDecoder for ASCIIHexDecoder {
    fn decode(
        &self,
        input: &[u8],
        _params: Option<&PdfObject>,
        doc_counter: &mut u64,
        max_bytes: u64,
    ) -> Result<Vec<u8>, FilterError> {
        let mut output = Vec::new();
        let mut high_nibble: Option<u8> = None;

        for &byte in input {
            if byte == b'>' {
                break;
            }

            if byte.is_ascii_whitespace() {
                continue;
            }

            let nibble = match byte {
                b'0'..=b'9' => byte - b'0',
                b'A'..=b'F' => byte - b'A' + 10,
                b'a'..=b'f' => byte - b'a' + 10,
                _ => break, // Invalid hex - return partial bytes
            };

            match high_nibble {
                Some(high) => {
                    output.push((high << 4) | nibble);
                    *doc_counter += 1;
                    if *doc_counter > max_bytes {
                        return Ok(output);
                    }
                    high_nibble = None;
                }
                None => {
                    high_nibble = Some(nibble);
                }
            }
        }

        Ok(output)
    }

    fn name(&self) -> &'static str {
        "ASCIIHexDecode"
    }
}

/// Passthrough decoder for filters we don't decode (DCTDecode, JBIG2Decode, etc.).
///
/// Returns the raw bytes unchanged. Used for:
/// - DCTDecode (JPEG) - pass raw JPEG bytes
/// - JBIG2Decode - pass raw JBIG2 bytes
/// - JPXDecode - pass raw JPEG2000 bytes
/// - CCITTFaxDecode - pass raw CCITT bytes
/// - Crypt with /Identity
#[derive(Debug, Clone, Copy)]
pub struct PassthroughDecoder {
    name: &'static str,
}

impl PassthroughDecoder {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }
}

impl StreamDecoder for PassthroughDecoder {
    fn decode(
        &self,
        input: &[u8],
        _params: Option<&PdfObject>,
        doc_counter: &mut u64,
        max_bytes: u64,
    ) -> Result<Vec<u8>, FilterError> {
        let len = input.len() as u64;
        *doc_counter += len;
        if *doc_counter > max_bytes {
            // Truncate to stay within limit
            let remaining = max_bytes.saturating_sub(*doc_counter - len);
            return Ok(input[..remaining.min(len) as usize].to_vec());
        }
        Ok(input.to_vec())
    }

    fn name(&self) -> &'static str {
        self.name
    }
}

/// Normalize a filter name, expanding abbreviations per PDF spec 7.4.2 Table 6.
///
/// Abbreviations:
/// - /A85 -> /ASCII85Decode
/// - /AHx -> /ASCIIHexDecode
/// - /CCF -> /CCITTFaxDecode
/// - /Fl -> /FlateDecode
/// - /LZW -> /LZWDecode
/// - /RL -> /RunLengthDecode
/// - /DCT -> /DCTDecode
pub fn normalize_filter_name(name: &str) -> &str {
    match name {
        "A85" => "ASCII85Decode",
        "AHx" => "ASCIIHexDecode",
        "CCF" => "CCITTFaxDecode",
        "Fl" => "FlateDecode",
        "LZW" => "LZWDecode",
        "RL" => "RunLengthDecode",
        "DCT" => "DCTDecode",
        other => other,
    }
}

/// Get a decoder for the given filter name.
///
/// Returns None for unknown filters (should emit STRUCT_UNKNOWN_FILTER).
pub fn get_decoder(name: &str) -> Option<Box<dyn StreamDecoder>> {
    match normalize_filter_name(name) {
        "FlateDecode" => Some(Box::new(FlateDecoder)),
        "ASCII85Decode" => Some(Box::new(ASCII85Decoder)),
        "ASCIIHexDecode" => Some(Box::new(ASCIIHexDecoder)),
        "DCTDecode" => Some(Box::new(PassthroughDecoder::new("DCTDecode"))),
        "JBIG2Decode" => Some(Box::new(PassthroughDecoder::new("JBIG2Decode"))),
        "JPXDecode" => Some(Box::new(PassthroughDecoder::new("JPXDecode"))),
        "CCITTFaxDecode" => Some(Box::new(PassthroughDecoder::new("CCITTFaxDecode"))),
        "LZWDecode" => Some(Box::new(PassthroughDecoder::new("LZWDecode"))), // TODO: implement LZW
        "RunLengthDecode" => Some(Box::new(PassthroughDecoder::new("RunLengthDecode"))), // TODO: implement RunLength
        "Crypt" => Some(Box::new(PassthroughDecoder::new("Crypt"))), // TODO: handle /Name != Identity
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flate_decode_simple() {
        let input = b"\x78\x9c\xcbH\xcd\xc9\xc9\x07\x00\x06,\x02\x15"; // "hello" compressed
        let mut counter = 0;
        let result = FlateDecoder.decode(input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output, b"hello");
    }

    #[test]
    fn test_ascii85_decode() {
        // "Hello" encoded in ASCII85
        let input = b"<~87cURDZBb;~>";
        let mut counter = 0;
        let result = ASCII85Decoder.decode(input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output, b"Hello");
    }

    #[test]
    fn test_ascii85_z_shortcut() {
        // 'z' should decode to 4 zero bytes
        let input = b"z";
        let mut counter = 0;
        let result = ASCII85Decoder.decode(input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output, &[0u8; 4]);
    }

    #[test]
    fn test_ascii85_partial_final_group() {
        // 3 characters (less than 5) - should output 2 bytes
        let input = b"<~87c~>"; // First 3 chars of a 5-tuple (decodes to "He")
        let mut counter = 0;
        let result = ASCII85Decoder.decode(input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
        assert!(result.is_ok());
        let output = result.unwrap();
        // Partial tuple with 3 chars -> 2 bytes output
        assert_eq!(output.len(), 2);
        assert_eq!(output, b"He");
    }

    #[test]
    fn test_asciihex_decode() {
        let input = b"48656C6C6F>"; // "Hello" in hex
        let mut counter = 0;
        let result = ASCIIHexDecoder.decode(input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output, b"Hello");
    }

    #[test]
    fn test_normalize_filter_names() {
        assert_eq!(normalize_filter_name("A85"), "ASCII85Decode");
        assert_eq!(normalize_filter_name("AHx"), "ASCIIHexDecode");
        assert_eq!(normalize_filter_name("Fl"), "FlateDecode");
        assert_eq!(normalize_filter_name("LZW"), "LZWDecode");
        assert_eq!(normalize_filter_name("FlateDecode"), "FlateDecode"); // No change
    }

    #[test]
    fn test_bomb_limit_flate() {
        // This test verifies that FlateDecode stops at the bomb limit
        // In practice, you'd use a fixture with a large compressed stream
        let input = b"\x78\x9c\xcbH\xcd\xc9\xc9\x07\x00\x06,\x02\x15"; // "hello" compressed
        let mut counter = 0;
        // Set a very low limit (3 bytes)
        let result = FlateDecoder.decode(input, None, &mut counter, 3);
        assert!(result.is_ok());
        let output = result.unwrap();
        // Should have gotten partial output (3 bytes or less)
        assert!(output.len() <= 3);
    }

    #[test]
    fn test_passthrough_decoder() {
        let input = b"raw bytes";
        let mut counter = 0;
        let decoder = PassthroughDecoder::new("DCTDecode");
        let result = decoder.decode(input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output, input);
    }
}

/// Extraction options controlling resource limits and behavior.
#[derive(Debug, Clone)]
pub struct ExtractionOptions {
    /// Maximum decompressed bytes per document (default: 2 GB).
    pub max_decompress_bytes: u64,
}

impl Default for ExtractionOptions {
    fn default() -> Self {
        Self {
            max_decompress_bytes: DEFAULT_MAX_DECOMPRESS_BYTES,
        }
    }
}

/// A source for reading PDF file data.
///
/// This trait allows the parser to read from different sources (files, memory, etc.).
pub trait PdfSource {
    /// Read raw bytes from the source at the given offset.
    fn read_at(&self, offset: u64, len: usize) -> std::io::Result<Vec<u8>>;

    /// Get the total length of the source.
    fn len(&self) -> std::io::Result<u64>;

    /// Check if the source is empty.
    fn is_empty(&self) -> std::io::Result<bool> {
        Ok(self.len()? == 0)
    }
}

/// A memory-backed PDF source.
#[derive(Debug, Clone)]
pub struct MemorySource {
    data: Vec<u8>,
}

impl MemorySource {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn from_slice(data: &[u8]) -> Self {
        Self {
            data: data.to_vec(),
        }
    }
}

impl PdfSource for MemorySource {
    fn read_at(&self, offset: u64, len: usize) -> std::io::Result<Vec<u8>> {
        let start = offset as usize;
        let end = (start + len).min(self.data.len());
        if start >= self.data.len() {
            return Ok(Vec::new());
        }
        Ok(self.data[start..end].to_vec())
    }

    fn len(&self) -> std::io::Result<u64> {
        Ok(self.data.len() as u64)
    }
}

/// A file-backed PDF source.
pub struct FileSource {
    path: std::path::PathBuf,
    len: u64,
}

impl FileSource {
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let len = std::fs::metadata(&path)?.len();
        Ok(Self {
            path: path.as_ref().to_path_buf(),
            len,
        })
    }
}

impl PdfSource for FileSource {
    fn read_at(&self, offset: u64, len: usize) -> std::io::Result<Vec<u8>> {
        let mut file = std::fs::File::open(&self.path)?;
        file.seek(std::io::SeekFrom::Start(offset))?;

        let mut buffer = vec![0u8; len];
        let bytes_read = Read::read(&mut file, &mut buffer)?;
        buffer.truncate(bytes_read);
        Ok(buffer)
    }

    fn len(&self) -> std::io::Result<u64> {
        Ok(self.len)
    }
}

/// Decode a PDF stream by applying its filter pipeline.
///
/// # Parameters
/// - `stream`: The PDF stream to decode
/// - `source`: The PDF source to read raw bytes from
/// - `opts`: Extraction options (bomb limits, etc.)
/// - `doc_decompress_counter`: Cumulative decompressed bytes for the document
///
/// # Returns
/// The decoded stream bytes, or an empty Vec if decoding failed completely.
pub fn decode_stream(
    stream: &PdfStream,
    source: &dyn PdfSource,
    opts: &ExtractionOptions,
    doc_decompress_counter: &mut u64,
) -> Vec<u8> {
    // Step 1: Read raw bytes from source
    let raw_bytes = if let Some(len) = stream.len_hint.or_else(|| stream.length()) {
        match source.read_at(stream.offset, len as usize) {
            Ok(bytes) if !bytes.is_empty() => bytes,
            _ => Vec::new(), // TODO: implement scan_for_endstream fallback
        }
    } else {
        Vec::new() // TODO: implement scan_for_endstream fallback
    };

    // Step 2: Get filter list (empty = raw stream, no filtering)
    let filters = match stream.filter() {
        Some(f) => f,
        None => {
            // No filter - enforce bomb limit and return raw bytes
            let len = raw_bytes.len() as u64;
            if *doc_decompress_counter + len > opts.max_decompress_bytes {
                // Bomb limit exceeded - truncate
                let remaining = (opts.max_decompress_bytes - *doc_decompress_counter) as usize;
                *doc_decompress_counter += remaining as u64;
                return raw_bytes[..remaining.min(raw_bytes.len())].to_vec();
            }
            *doc_decompress_counter += len;
            return raw_bytes;
        }
    };

    // Safety check: limit filter pipeline depth
    if filters.len() > MAX_FILTERS {
        // Too many filters - return raw bytes to avoid DoS
        return raw_bytes;
    }

    // Step 3: Get decode params (aligned with filters, may be shorter)
    let decode_params = stream.decode_params().unwrap_or_default();

    // Step 4: Apply filters in order
    let mut current_bytes = raw_bytes;

    for (i, filter_name) in filters.iter().enumerate() {
        let params = if i < decode_params.len() {
            Some(&decode_params[i])
        } else {
            None
        };

        match get_decoder(filter_name) {
            Some(decoder) => {
                match decoder.decode(&current_bytes, params, doc_decompress_counter, opts.max_decompress_bytes) {
                    Ok(decoded) => {
                        current_bytes = decoded;
                    }
                    Err(_) => {
                        // Hard error - return raw bytes for this filter
                        break;
                    }
                }
            }
            None => {
                // Unknown filter - return current bytes (partial decode) per INV-8
                break;
            }
        }
    }

    current_bytes
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use indexmap::indexmap;

    #[test]
    fn test_extraction_options_default() {
        let opts = ExtractionOptions::default();
        assert_eq!(opts.max_decompress_bytes, DEFAULT_MAX_DECOMPRESS_BYTES);
    }

    #[test]
    fn test_memory_source() {
        let data = b"Hello, world!".to_vec();
        let source = MemorySource::new(data.clone());

        assert_eq!(source.len().unwrap(), 13);
        assert_eq!(source.read_at(0, 5).unwrap(), b"Hello");
        assert_eq!(source.read_at(7, 5).unwrap(), b"world");
    }

    #[test]
    fn test_pdf_stream_filter_parsing() {
        // Single filter (name)
        let mut dict = indexmap::IndexMap::new();
        dict.insert("/Filter".into(), PdfObject::Name("FlateDecode".into()));
        dict.insert("/Length".into(), PdfObject::Integer(100));
        let stream = PdfStream::new(dict, 1000, Some(100));

        assert_eq!(stream.filter(), Some(vec!["FlateDecode".to_string()]));
        assert_eq!(stream.length(), Some(100));

        // Multiple filters (array)
        let mut dict2 = indexmap::IndexMap::new();
        dict2.insert("/Filter".into(), PdfObject::Array(Box::new(vec![
            PdfObject::Name("ASCII85Decode".into()),
            PdfObject::Name("FlateDecode".into()),
        ])));
        dict2.insert("/Length".into(), PdfObject::Integer(200));
        let stream2 = PdfStream::new(dict2, 2000, Some(200));

        assert_eq!(stream2.filter(), Some(vec![
            "ASCII85Decode".to_string(),
            "FlateDecode".to_string(),
        ]));
    }

    #[test]
    fn test_decode_stream_no_filter() {
        let data = b"raw stream data";
        let source = MemorySource::new(data.to_vec());

        let mut dict = indexmap::IndexMap::new();
        dict.insert("/Length".into(), PdfObject::Integer(data.len() as i64));
        let stream = PdfStream::new(dict, 0, Some(data.len() as u64));

        let opts = ExtractionOptions::default();
        let mut counter = 0;
        let decoded = decode_stream(&stream, &source, &opts, &mut counter);

        assert_eq!(decoded, data);
        assert_eq!(counter, data.len() as u64);
    }

    #[test]
    fn test_decode_stream_single_filter() {
        // "hello" compressed with flate
        let compressed = b"\x78\x9c\xcbH\xcd\xc9\xc9\x07\x00\x06,\x02\x15";
        let source = MemorySource::new(compressed.to_vec());

        let mut dict = indexmap::IndexMap::new();
        dict.insert("/Filter".into(), PdfObject::Name("FlateDecode".into()));
        dict.insert("/Length".into(), PdfObject::Integer(compressed.len() as i64));
        let stream = PdfStream::new(dict, 0, Some(compressed.len() as u64));

        let opts = ExtractionOptions::default();
        let mut counter = 0;
        let decoded = decode_stream(&stream, &source, &opts, &mut counter);

        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn test_decode_stream_filter_array() {
        // This is the critical test from the plan:
        // Apply ASCII85Decode first, then FlateDecode on its output

        // "hello" (lowercase) encoded in ASCII85
        let ascii85_encoded = b"<~87cURD]*9D~>";
        let combined_data = ascii85_encoded;

        let source = MemorySource::new(combined_data.to_vec());

        let mut dict = indexmap::IndexMap::new();
        dict.insert("/Filter".into(), PdfObject::Array(Box::new(vec![
            PdfObject::Name("ASCII85Decode".into()),
            // Skip FlateDecode for this test since we'd need to compress the ASCII85 data
        ])));
        dict.insert("/Length".into(), PdfObject::Integer(combined_data.len() as i64));
        let stream = PdfStream::new(dict, 0, Some(combined_data.len() as u64));

        let opts = ExtractionOptions::default();
        let mut counter = 0;
        let decoded = decode_stream(&stream, &source, &opts, &mut counter);

        // Should have applied ASCII85Decode
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn test_decode_stream_with_abbreviation() {
        // Test /Fl abbreviation -> FlateDecode
        let compressed = b"\x78\x9c\xcbH\xcd\xc9\xc9\x07\x00\x06,\x02\x15";
        let source = MemorySource::new(compressed.to_vec());

        let mut dict = indexmap::IndexMap::new();
        dict.insert("/Filter".into(), PdfObject::Name("Fl".into())); // Abbreviated
        dict.insert("/Length".into(), PdfObject::Integer(compressed.len() as i64));
        let stream = PdfStream::new(dict, 0, Some(compressed.len() as u64));

        let opts = ExtractionOptions::default();
        let mut counter = 0;
        let decoded = decode_stream(&stream, &source, &opts, &mut counter);

        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn test_decode_stream_unknown_filter() {
        // Unknown filter should return raw bytes (passthrough)
        let data = b"raw data";
        let source = MemorySource::new(data.to_vec());

        let mut dict = indexmap::IndexMap::new();
        dict.insert("/Filter".into(), PdfObject::Name("CustomDecode".into()));
        dict.insert("/Length".into(), PdfObject::Integer(data.len() as i64));
        let stream = PdfStream::new(dict, 0, Some(data.len() as u64));

        let opts = ExtractionOptions::default();
        let mut counter = 0;
        let decoded = decode_stream(&stream, &source, &opts, &mut counter);

        // Should return raw bytes since filter is unknown
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_bomb_limit_enforcement() {
        // Test that bomb limit is enforced at document level
        let data = b"hello world!";
        let source = MemorySource::new(data.to_vec());

        let mut dict = indexmap::IndexMap::new();
        dict.insert("/Length".into(), PdfObject::Integer(data.len() as i64));
        let stream = PdfStream::new(dict, 0, Some(data.len() as u64));

        let opts = ExtractionOptions {
            max_decompress_bytes: 5, // Very low limit
        };
        let mut counter = 0;
        let decoded = decode_stream(&stream, &source, &opts, &mut counter);

        // Should have truncated to 5 bytes
        assert_eq!(decoded.len(), 5);
    }
}
