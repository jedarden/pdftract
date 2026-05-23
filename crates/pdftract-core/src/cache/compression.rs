//! Zstandard compression for cache entries.
//!
//! This module implements Phase 6.9.3: zstd compression for cache entries.
//! Compression reduces cache storage by 5-10x for JSON (repeated keys,
//! structural similarity). Decompression is fast (~1 GB/s) to meet the
//! < 20 ms p99 cache-hit target.

use std::io::{self, Read, Write};

/// Default zstd compression level.
///
/// Level 3 is the sweet spot for JSON: good compression ratio at fast speed.
/// - Level 1: ~30% worse ratio, ~2x faster
/// - Level 3: baseline (tuned for JSON)
/// - Level 9: ~10% better ratio, ~3x slower
const DEFAULT_COMPRESSION_LEVEL: i32 = 3;

/// Maximum decompressed size for bomb protection.
///
/// This matches the Phase 6.4 HTTP upload max. A 1 KB zstd file that
/// decompresses to 10 GB must not crash pdftract.
const MAX_DECOMPRESSED_SIZE: usize = 256 * 1024 * 1024; // 256 MB

/// zstd frame magic bytes.
///
/// All valid zstd frames start with these 4 bytes. We check this on decode
/// to reject corrupted or non-zstd inputs early.
const ZSTD_MAGIC: &[u8] = &[0x28, 0xB5, 0x2F, 0xFD];

/// Environment variable for overriding the default compression level.
///
/// This is exposed for benchmarking only. Not surfaced as a CLI flag.
const ENV_COMPRESSION_LEVEL: &str = "PDFTRACT_CACHE_ZSTD_LEVEL";

/// Get the compression level from environment or use the default.
fn compression_level() -> i32 {
    std::env::var(ENV_COMPRESSION_LEVEL)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_COMPRESSION_LEVEL)
}

/// Encode data with zstd compression.
///
/// Uses the default compression level (3) or the value from
/// `PDFTRACT_CACHE_ZSTD_LEVEL` if set.
///
/// # Arguments
///
/// * `data` - Input bytes to compress
///
/// # Returns
///
/// Compressed zstd frame as a `Vec<u8>`.
///
/// # Errors
///
/// Returns `Err` if compression fails (e.g., invalid compression level).
///
/// # Example
///
/// ```ignore
/// use pdftract_core::cache::compression::encode;
///
/// let json = br#"{"key": "value"}"#.to_vec();
/// let compressed = encode(&json).unwrap();
/// assert!(compressed.len() < json.len()); // Compressed is smaller
/// ```
pub fn encode(data: &[u8]) -> io::Result<Vec<u8>> {
    let level = compression_level();
    zstd::encode_all(data, level)
}

/// Decode a zstd-compressed frame.
///
/// This function enforces:
/// - Magic-byte validation (must start with 0x28 0xB5 0x2F 0xFD)
/// - Decompression bomb protection (max 256 MB output)
/// - Frame integrity (zstd's built-in CRC)
///
/// # Arguments
///
/// * `data` - Compressed zstd frame bytes
///
/// # Returns
///
/// Decompressed data as a `Vec<u8>`.
///
/// # Errors
///
/// Returns `Err` if:
/// - Input is empty
/// - Magic bytes don't match (corrupted or non-zstd data)
/// - Decompressed size exceeds `MAX_DECOMPRESSED_SIZE`
/// - Frame is truncated or corrupt (CRC failure)
///
/// # Example
///
/// ```ignore
/// use pdftract_core::cache::compression::{encode, decode};
///
/// let original = br#"{"key": "value"}"#.to_vec();
/// let compressed = encode(&original).unwrap();
/// let decompressed = decode(&compressed).unwrap();
/// assert_eq!(original, decompressed);
/// ```
pub fn decode(data: &[u8]) -> io::Result<Vec<u8>> {
    // Reject empty input early
    if data.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cannot decode empty input",
        ));
    }

    // Check magic bytes for early rejection of corrupted/non-zstd data
    if !data.starts_with(ZSTD_MAGIC) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "invalid zstd magic bytes: expected {:?}, got {:?}",
                &ZSTD_MAGIC[..],
                &data[..ZSTD_MAGIC.len().min(data.len())]
            ),
        ));
    }

    // Decode with bomb protection
    let mut result = Vec::with_capacity(data.len().min(MAX_DECOMPRESSED_SIZE));
    {
        let mut decoder = zstd::Decoder::new(data)?;
        decoder.take(MAX_DECOMPRESSED_SIZE as u64).read_to_end(&mut result)?;
    }

    // Check if we hit the bomb limit
    if result.len() >= MAX_DECOMPRESSED_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "decompression bomb protection: output exceeds {} MB limit",
                MAX_DECOMPRESSED_SIZE / (1024 * 1024)
            ),
        ));
    }

    Ok(result)
}

/// Streaming encode from a reader.
///
/// Reads all data from `reader` and compresses it. Suitable for very large
/// entries where you want to avoid buffering the full uncompressed data
/// in memory.
///
/// # Arguments
///
/// * `reader` - Source of uncompressed data
///
/// # Returns
///
/// Compressed zstd frame as a `Vec<u8>`.
///
/// # Errors
///
/// Returns `Err` if reading from `reader` fails or compression fails.
///
/// # Example
///
/// ```ignore
/// use std::io::Cursor;
/// use pdftract_core::cache::compression::encode_from_reader;
///
/// let json = br#"{"key": "value"}"#;
/// let cursor = Cursor::new(json);
/// let compressed = encode_from_reader(cursor).unwrap();
/// ```
pub fn encode_from_reader<R: Read>(mut reader: R) -> io::Result<Vec<u8>> {
    let level = compression_level();
    let mut compressed = Vec::new();
    {
        let mut encoder = zstd::Encoder::new(&mut compressed, level)?;
        let mut buffer = vec![0u8; 64 * 1024]; // 64 KB buffer
        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            encoder.write_all(&buffer[..n])?;
        }
        encoder.finish()?;
    }
    Ok(compressed)
}

/// Streaming decode into a writer.
///
/// Decompresses `data` and writes the result to `writer`. Enforces the
/// 256 MB decompression bomb limit incrementally during decode.
///
/// # Arguments
///
/// * `data` - Compressed zstd frame bytes
/// * `writer` - Destination for decompressed data
///
/// # Returns
///
/// Number of bytes written to `writer`.
///
/// # Errors
///
/// Returns `Err` if:
/// - Input is empty
/// - Magic bytes don't match
/// - Decompressed size exceeds `MAX_DECOMPRESSED_SIZE`
/// - Frame is truncated or corrupt
/// - Writing to `writer` fails
///
/// # Example
///
/// ```ignore
/// use std::io::Cursor;
/// use pdftract_core::cache::compression::{encode, decode_into_writer};
///
/// let original = br#"{"key": "value"}"#.to_vec();
/// let compressed = encode(&original).unwrap();
/// let mut output = Vec::new();
/// let bytes_written = decode_into_writer(&compressed, &mut output).unwrap();
/// assert_eq!(bytes_written, original.len());
/// assert_eq!(output, original);
/// ```
pub fn decode_into_writer<W: Write>(data: &[u8], mut writer: W) -> io::Result<usize> {
    // Reject empty input early
    if data.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cannot decode empty input",
        ));
    }

    // Check magic bytes
    if !data.starts_with(ZSTD_MAGIC) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "invalid zstd magic bytes: expected {:?}, got {:?}",
                &ZSTD_MAGIC[..],
                &data[..ZSTD_MAGIC.len().min(data.len())]
            ),
        ));
    }

    // Decode with bomb protection
    let mut decoder = zstd::Decoder::new(data)?;
    let mut limited_decoder = decoder.take(MAX_DECOMPRESSED_SIZE as u64);

    // Copy decompressed data with a limited buffer
    let mut buffer = vec![0u8; 64 * 1024]; // 64 KB buffer
    let mut total_written = 0;
    loop {
        let n = limited_decoder.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buffer[..n])?;
        total_written += n;
    }

    // Check if we hit the bomb limit
    if total_written >= MAX_DECOMPRESSED_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "decompression bomb protection: output exceeds {} MB limit",
                MAX_DECOMPRESSED_SIZE / (1024 * 1024)
            ),
        ));
    }

    Ok(total_written)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_JSON: &str = r#"{
        "pages": [
            {
                "index": 0,
                "width": 612,
                "height": 792,
                "spans": [
                    {"text": "Hello, world!", "font": "Helvetica", "size": 12, "x": 100, "y": 100},
                    {"text": "This is a test.", "font": "Helvetica", "size": 12, "x": 100, "y": 120}
                ],
                "blocks": [
                    {"type": "paragraph", "spans": [0, 1], "bounds": {"x": 100, "y": 100, "width": 200, "height": 40}}
                ]
            }
        ],
        "metadata": {
            "page_count": 1,
            "span_count": 2,
            "block_count": 1
        }
    }"#;

    /// Create a 5 MB DocumentJson fixture for testing.
    ///
    /// Generates representative JSON for a ~100-page PDF extraction result.
    fn create_5mb_fixture() -> Vec<u8> {
        let page_json = r#"{
            "index": 0,
            "width": 612,
            "height": 792,
            "spans": [
                {"text": "Lorem ipsum dolor sit amet, consectetur adipiscing elit.", "font": "Times-Roman", "size": 12, "x": 72, "y": 72},
                {"text": "Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.", "font": "Times-Roman", "size": 12, "x": 72, "y": 90},
                {"text": "Ut enim ad minim veniam, quis nostrud exercitation ullamco.", "font": "Times-Roman", "size": 12, "x": 72, "y": 108}
            ],
            "blocks": [
                {"type": "paragraph", "spans": [0, 1, 2], "bounds": {"x": 72, "y": 72, "width": 468, "height": 60}}
            ]
        }"#;

        let metadata = r#"{
            "page_count": 100,
            "span_count": 300,
            "block_count": 100
        }"#;

        // Calculate how many pages we need for ~5 MB
        // Each page is ~500 bytes, so we need ~10000 pages
        let page_bytes = page_json.as_bytes();
        let target_size = 5 * 1024 * 1024;
        let num_pages = (target_size / page_bytes.len()).max(1);

        let mut json = String::from("{\"pages\":[");
        for i in 0..num_pages {
            if i > 0 {
                json.push(',');
            }
            // Replace the page index
            let page_with_index = page_json.replace("\"index\": 0", &format!("\"index\": {}", i));
            json.push_str(&page_with_index);
        }
        json.push_str("],\"metadata\":");
        json.push_str(metadata);
        json.push('}');

        json.into_bytes()
    }

    #[test]
    fn test_round_trip() {
        let original = TEST_JSON.as_bytes().to_vec();
        let compressed = encode(&original).unwrap();
        let decompressed = decode(&compressed).unwrap();
        assert_eq!(original, decompressed);
    }

    #[test]
    fn test_compression_ratio() {
        let fixture = create_5mb_fixture();
        let original_size = fixture.len();
        let compressed = encode(&fixture).unwrap();
        let compressed_size = compressed.len();

        // Should compress to <= 1.5 MB (>= 3.3x ratio)
        assert!(
            compressed_size <= 1_500_000,
            "compressed size {} exceeds 1.5 MB limit",
            compressed_size
        );

        let ratio = original_size as f64 / compressed_size as f64;
        assert!(
            ratio >= 3.3,
            "compression ratio {} is below 3.3x threshold",
            ratio
        );
    }

    #[test]
    fn test_truncated_frame() {
        let fixture = create_5mb_fixture();
        let compressed = encode(&fixture).unwrap();

        // Take only the first 100 bytes (truncated frame)
        let truncated = &compressed[..100.min(compressed.len())];

        let result = decode(truncated);
        assert!(result.is_err(), "truncated frame should return Err");
    }

    #[test]
    fn test_empty_input() {
        let result = decode(&[]);
        assert!(result.is_err(), "empty input should return Err");
    }

    #[test]
    fn test_invalid_magic_bytes() {
        let invalid = b"NOT_ZSTD_DATA_HERE";
        let result = decode(invalid);
        assert!(result.is_err(), "invalid magic bytes should return Err");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("invalid zstd magic bytes"));
    }

    #[test]
    fn test_magic_bytes() {
        let fixture = create_5mb_fixture();
        let compressed = encode(&fixture).unwrap();

        // Verify magic bytes are present
        assert!(compressed.starts_with(ZSTD_MAGIC));
    }

    #[test]
    fn test_encode_from_reader() {
        let original = TEST_JSON.as_bytes();
        let cursor = io::Cursor::new(original);
        let compressed = encode_from_reader(cursor).unwrap();

        let decompressed = decode(&compressed).unwrap();
        assert_eq!(original, decompressed.as_slice());
    }

    #[test]
    fn test_decode_into_writer() {
        let original = TEST_JSON.as_bytes().to_vec();
        let compressed = encode(&original).unwrap();

        let mut output = Vec::new();
        let bytes_written = decode_into_writer(&compressed, &mut output).unwrap();

        assert_eq!(bytes_written, original.len());
        assert_eq!(output, original);
    }

    #[test]
    fn test_decode_into_writer_empty_input() {
        let output = Vec::new();
        let result = decode_into_writer(&[], output);
        assert!(result.is_err(), "empty input should return Err");
    }

    #[test]
    fn test_decode_into_writer_invalid_magic() {
        let invalid = b"NOT_ZSTD";
        let output = Vec::new();
        let result = decode_into_writer(invalid, output);
        assert!(result.is_err(), "invalid magic bytes should return Err");
    }

    #[test]
    #[ignore] // Expensive benchmark test
    fn benchmark_encode_1mb() {
        let fixture = create_5mb_fixture();
        // Use 1 MB for benchmark
        let data = &fixture[..1_000_000];

        let start = std::time::Instant::now();
        let _compressed = encode(data).unwrap();
        let duration = start.elapsed();

        // Should encode 1 MB in < 5 ms on typical x86-64
        assert!(
            duration.as_millis() < 5,
            "encode took {:?}, expected < 5 ms",
            duration
        );
    }

    #[test]
    #[ignore] // Expensive benchmark test
    fn benchmark_decode_1mb() {
        let fixture = create_5mb_fixture();
        let data = &fixture[..1_000_000];
        let compressed = encode(data).unwrap();

        let start = std::time::Instant::now();
        let _decompressed = decode(&compressed).unwrap();
        let duration = start.elapsed();

        // Should decode 1 MB zstd entry in < 2 ms on typical x86-64
        assert!(
            duration.as_millis() < 2,
            "decode took {:?}, expected < 2 ms",
            duration
        );
    }
}
