//! Filespec dictionary and EF stream decoder (PDF 1.7+ embedded files).
//!
//! This module implements extraction of embedded files from Filespec dictionaries.
//! Per PDF 1.7 spec §7.11, each Filespec contains:
//! - /F or /UF (filename, with /UF preferred for Unicode)
//! - /Desc (optional description)
//! - /EF dictionary → /F stream reference (embedded file data)
//!
//! The embedded file stream dictionary contains:
//! - /Subtype (MIME type hint)
//! - /Params dictionary → /Size, /CreationDate, /ModDate, /CheckSum
//!
//! # Size Limit
//!
//! Per 7.5.3, attachments > 50 MB are truncated (metadata only, content empty).

use crate::diagnostics::{DiagCode, Diagnostic};
use crate::parser::object::ObjRef;
use crate::parser::stream::{ExtractionOptions, PdfSource, DEFAULT_MAX_DECOMPRESS_BYTES};
use crate::parser::xref::XrefResolver;

use base64::engine::Engine;

/// Base64 encoder for attachment content (RFC 4648 standard alphabet with padding).
///
/// Uses the standard base64 alphabet (+ and /) with = padding and no line breaks.
const BASE64_ENGINE: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

/// Maximum attachment size before truncation (50 MB per plan 7.5.3).
const MAX_ATTACHMENT_SIZE: u64 = 50 * 1024 * 1024;

/// Result type for Filespec extraction.
pub type Result<T> = std::result::Result<T, Vec<Diagnostic>>;

/// An extracted attachment with all metadata and decoded content.
///
/// This is the builder/intermediate type returned by `extract_one`.
/// The final JSON schema type is defined in Phase 6.1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentBuilder {
    /// Filename from /UF (preferred) or /F (system-independent)
    pub name: String,
    /// Description from /Desc (None if absent, not empty string)
    pub description: Option<String>,
    /// MIME type from stream /Subtype (None if absent)
    pub mime_type: Option<String>,
    /// Original byte size from /Params /Size (None if absent)
    pub size: Option<u64>,
    /// Creation date from /Params /CreationDate (ISO 8601, None if absent)
    pub created: Option<String>,
    /// Modification date from /Params /ModDate (ISO 8601, None if absent)
    pub modified: Option<String>,
    /// MD5 checksum from /Params /CheckSum as hex string (None if absent)
    pub checksum_md5: Option<String>,
    /// Decoded attachment content (empty if truncated or error)
    pub content: Vec<u8>,
    /// Whether content was truncated due to size limit
    pub truncated: bool,
}

impl AttachmentBuilder {
    /// Create a new attachment with empty content.
    fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            mime_type: None,
            size: None,
            created: None,
            modified: None,
            checksum_md5: None,
            content: Vec::new(),
            truncated: false,
        }
    }

    /// Convert to the JSON schema type with base64 encoding.
    ///
    /// This method converts the intermediate `AttachmentBuilder` to the final
    /// `AttachmentJson` type that is serialized to JSON output. It handles:
    /// - Base64 encoding of content (RFC 4648 standard alphabet)
    /// - Setting `data: None` when truncated
    /// - Populating `size` from the original content length
    ///
    /// Per plan 7.5.3, the 50 MB size limit is enforced during extraction,
    /// so this method only needs to encode the content that's already been
    /// truncated if necessary.
    ///
    /// # Returns
    ///
    /// A `crate::schema::AttachmentJson` ready for JSON serialization.
    pub fn into_json(self) -> crate::schema::AttachmentJson {
        let data = if self.truncated || self.content.is_empty() {
            None
        } else {
            Some(BASE64_ENGINE.encode(&self.content))
        };

        // Use the size from /Params if available, otherwise use the actual content length
        let size = self.size.unwrap_or(self.content.len() as u64);

        crate::schema::AttachmentJson {
            name: self.name,
            description: self.description,
            mime_type: self.mime_type,
            size,
            created: self.created,
            modified: self.modified,
            checksum_md5: self.checksum_md5,
            data,
            truncated: self.truncated,
        }
    }
}

/// Extract a single attachment from a Filespec reference.
///
/// # Arguments
/// * `resolver` - The xref resolver for resolving indirect references
/// * `filespec_ref` - Reference to the Filespec dictionary
/// * `source` - Optional PDF source for reading stream data (None for metadata-only extraction)
///
/// # Returns
///
/// `Ok(AttachmentBuilder)` with extracted metadata and decoded content.
/// Returns `Err` with diagnostics if the Filespec is invalid or resolution fails.
///
/// # Behavior
///
/// - Filename: prefers /UF (Unicode) over /F (system-independent)
/// - Description: None if /Desc absent (not empty string)
/// - MIME type: from EF stream /Subtype, None if absent (no guessing from extension)
/// - Size: from /Params /Size, None if absent
/// - Dates: parsed from PDF date format to ISO 8601, None if parsing fails
/// - Checksum: hex-encoded from /Params /CheckSum (16 bytes), None if absent
/// - Content: decoded through stream filter pipeline, empty if source is None or size > 50 MB
///
/// # Example
///
/// ```ignore
/// use pdftract_core::attachment::filespec::{extract_one, AttachmentBuilder};
///
/// // filespec_ref is from /EmbeddedFiles name tree or /AF array
/// let attachment = extract_one(&resolver, filespec_ref, Some(&source))?;
///
/// println!("File: {} ({} bytes)", attachment.name, attachment.content.len());
/// if let Some(mime) = attachment.mime_type {
///     println!("Type: {}", mime);
/// }
/// ```
pub fn extract_one(
    resolver: &XrefResolver,
    filespec_ref: ObjRef,
    source: Option<&dyn PdfSource>,
) -> Result<AttachmentBuilder> {
    let diagnostics = Vec::new();

    // Resolve the Filespec dictionary
    let filespec_obj = resolver.resolve(filespec_ref).map_err(|e| {
        vec![Diagnostic::with_dynamic_no_offset(
            DiagCode::StructUnexpectedEof,
            format!("Failed to resolve Filespec {}: {}", filespec_ref, e),
        )]
    })?;

    let filespec_dict = filespec_obj.as_dict().ok_or_else(|| {
        vec![Diagnostic::with_dynamic_no_offset(
            DiagCode::StructInvalidType,
            format!(
                "Filespec {} is not a dictionary (type: {})",
                filespec_ref,
                filespec_obj.type_name()
            ),
        )]
    })?;

    // Extract filename: /UF (Unicode, preferred) or /F (system-independent)
    let name = extract_filename(filespec_dict)?;

    // Create attachment builder
    let mut attachment = AttachmentBuilder::new(name);

    // Extract description (optional)
    attachment.description = extract_description(filespec_dict);

    // Extract /EF dictionary → /F stream reference
    let ef_stream_ref = extract_ef_stream_ref(filespec_dict)?;

    // Resolve the EF stream
    let stream_obj = resolver.resolve(ef_stream_ref).map_err(|e| {
        vec![Diagnostic::with_dynamic_no_offset(
            DiagCode::StructUnexpectedEof,
            format!("Failed to resolve EF stream {}: {}", ef_stream_ref, e),
        )]
    })?;

    let stream_dict = stream_obj.as_stream().ok_or_else(|| {
        vec![Diagnostic::with_dynamic_no_offset(
            DiagCode::StructInvalidType,
            format!(
                "EF stream {} is not a stream (type: {})",
                ef_stream_ref,
                stream_obj.type_name()
            ),
        )]
    })?;

    // Extract metadata from stream dictionary
    attachment.mime_type = extract_mime_type(&stream_dict.dict);
    attachment.size = extract_size(&stream_dict.dict);
    attachment.created = extract_date(&stream_dict.dict, "/CreationDate");
    attachment.modified = extract_date(&stream_dict.dict, "/ModDate");
    attachment.checksum_md5 = extract_checksum(&stream_dict.dict);

    // Decode stream content (respecting size limit)
    let (content, truncated) = decode_stream_content(source, ef_stream_ref, stream_dict);
    attachment.content = content;
    attachment.truncated = truncated;

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    Ok(attachment)
}

/// Extract filename from Filespec, preferring /UF over /F.
fn extract_filename(filespec_dict: &crate::parser::object::PdfDict) -> Result<String> {
    // Try /UF (Unicode filename) first
    if let Some(uf_obj) = filespec_dict.get("/UF") {
        if let Some(uf_bytes) = uf_obj.as_string() {
            let decoded = decode_pdf_string(uf_bytes);
            if !decoded.is_empty() {
                return Ok(decoded);
            }
        }
    }

    // Fall back to /F (system-independent filename)
    if let Some(f_obj) = filespec_dict.get("/F") {
        if let Some(f_bytes) = f_obj.as_string() {
            let decoded = decode_pdfdocencoding(f_bytes);
            if !decoded.is_empty() {
                return Ok(decoded);
            }
        }
    }

    // Neither /UF nor /F present or both empty
    Err(vec![Diagnostic::with_static_no_offset(
        DiagCode::StructMissingKey,
        "Filespec missing /UF and /F (no filename)",
    )])
}

/// Extract description from Filespec (/Desc, optional).
fn extract_description(filespec_dict: &crate::parser::object::PdfDict) -> Option<String> {
    filespec_dict
        .get("/Desc")
        .and_then(|obj| obj.as_string())
        .and_then(|bytes| {
            let decoded = decode_pdf_string(bytes);
            if decoded.is_empty() {
                None
            } else {
                Some(decoded)
            }
        })
}

/// Extract /EF /F stream reference from Filespec.
fn extract_ef_stream_ref(filespec_dict: &crate::parser::object::PdfDict) -> Result<ObjRef> {
    let ef_obj = filespec_dict.get("/EF").ok_or_else(|| {
        vec![Diagnostic::with_static_no_offset(
            DiagCode::StructMissingKey,
            "Filespec missing /EF dictionary",
        )]
    })?;

    let ef_dict = ef_obj.as_dict().ok_or_else(|| {
        vec![Diagnostic::with_dynamic_no_offset(
            DiagCode::StructInvalidType,
            format!("/EF is not a dictionary (type: {})", ef_obj.type_name()),
        )]
    })?;

    // Get /F from /EF (the embedded file stream reference)
    // Note: /EF may also have /UF, /DOS, /Mac, /Unix variants, but /F is the canonical
    let stream_ref_obj = ef_dict.get("/F").ok_or_else(|| {
        vec![Diagnostic::with_static_no_offset(
            DiagCode::StructMissingKey,
            "/EF missing /F stream reference",
        )]
    })?;

    stream_ref_obj.as_ref().ok_or_else(|| {
        vec![Diagnostic::with_dynamic_no_offset(
            DiagCode::StructInvalidType,
            format!(
                "/EF /F is not a reference (type: {})",
                stream_ref_obj.type_name()
            ),
        )]
    })
}

/// Extract MIME type from stream dictionary (/Subtype, optional).
fn extract_mime_type(stream_dict: &crate::parser::object::PdfDict) -> Option<String> {
    stream_dict
        .get("/Subtype")
        .and_then(|obj| obj.as_name())
        .map(|s| s.to_string())
}

/// Extract original size from stream params (/Params /Size, optional).
fn extract_size(stream_dict: &crate::parser::object::PdfDict) -> Option<u64> {
    stream_dict
        .get("/Params")
        .and_then(|obj| obj.as_dict())
        .and_then(|params| params.get("/Size"))
        .and_then(|obj| obj.as_int())
        .filter(|&size| size >= 0)
        .map(|size| size as u64)
}

/// Extract and parse a date field from stream params (/CreationDate or /ModDate).
fn extract_date(stream_dict: &crate::parser::object::PdfDict, key: &str) -> Option<String> {
    stream_dict
        .get("/Params")
        .and_then(|obj| obj.as_dict())
        .and_then(|params| params.get(key))
        .and_then(|obj| obj.as_string())
        .and_then(parse_pdf_date)
}

/// Extract and hex-encode checksum from stream params (/Params /CheckSum, optional).
///
/// Per PDF spec, /CheckSum is a 16-byte binary string (MD5). We hex-encode it
/// as 32 lowercase hex characters.
fn extract_checksum(stream_dict: &crate::parser::object::PdfDict) -> Option<String> {
    stream_dict
        .get("/Params")
        .and_then(|obj| obj.as_dict())
        .and_then(|params| params.get("/CheckSum"))
        .and_then(|obj| obj.as_string())
        .map(|bytes| {
            bytes
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
        })
}

/// Decode the stream content, respecting the 50 MB size limit.
///
/// Returns (content, truncated) tuple.
fn decode_stream_content(
    source: Option<&dyn PdfSource>,
    _stream_ref: ObjRef,
    stream: &crate::parser::object::PdfStream,
) -> (Vec<u8>, bool) {
    use crate::parser::stream::decode_stream;

    // If no source provided, return empty content (metadata-only extraction)
    let Some(source) = source else {
        return (Vec::new(), false);
    };

    // Check if we have a /Size hint from /Params
    let size_hint = stream
        .dict
        .get("/Params")
        .and_then(|p| p.as_dict())
        .and_then(|params| params.get("/Size"))
        .and_then(|s| s.as_int())
        .filter(|&s| s > 0)
        .map(|s| s as u64);

    // If size hint exceeds limit, truncate immediately
    if let Some(size) = size_hint {
        if size > MAX_ATTACHMENT_SIZE {
            return (Vec::new(), true);
        }
    }

    // Decode the stream with a budget of min(50MB, DEFAULT_MAX_DECOMPRESS_BYTES)
    let budget = MAX_ATTACHMENT_SIZE.min(DEFAULT_MAX_DECOMPRESS_BYTES);
    let opts = ExtractionOptions {
        max_decompress_bytes: budget,
        password: None,
    };

    let mut counter = 0u64;
    let content = decode_stream(stream, source, &opts, &mut counter);

    // Check if decoded content exceeds limit
    if content.len() as u64 > MAX_ATTACHMENT_SIZE {
        // Truncate to 50 MB
        let truncated_content = content
            .iter()
            .copied()
            .take(MAX_ATTACHMENT_SIZE as usize)
            .collect();
        (truncated_content, true)
    } else {
        (content, false)
    }
}

// ============================================================================
// String decoding utilities (copied from signature/mod.rs)
// ============================================================================

/// Decode a PDF text string to UTF-8.
///
/// Per PDF 1.7 spec section "Text String Type":
/// - If the string starts with UTF-16BE BOM (0xFE 0xFF), decode as UTF-16BE
/// - Otherwise, decode as PDFDocEncoding (Latin-1 with named character overrides)
fn decode_pdf_string(bytes: &[u8]) -> String {
    // Check for UTF-16BE BOM
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        return decode_utf16be_bom(&bytes[2..]);
    }

    // Check for UTF-16BE without BOM (heuristic: every other byte is 0x00 for non-ASCII)
    if looks_like_utf16be(bytes) {
        if let Ok(s) = decode_utf16be_raw(bytes) {
            return s;
        }
    }

    // Fall back to PDFDocEncoding (treat as Latin-1 for basic use)
    decode_pdfdocencoding(bytes)
}

/// Decode UTF-16BE string with BOM (bytes after 0xFE 0xFF).
fn decode_utf16be_bom(bytes: &[u8]) -> String {
    if bytes.len() % 2 != 0 {
        return decode_pdfdocencoding(bytes);
    }

    let utf16_chars: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect();

    String::from_utf16(&utf16_chars).unwrap_or_default()
}

/// Decode raw UTF-16BE (without BOM).
fn decode_utf16be_raw(bytes: &[u8]) -> std::result::Result<String, ()> {
    if bytes.len() % 2 != 0 {
        return Err(());
    }

    let utf16_chars: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect();

    String::from_utf16(&utf16_chars).map_err(|_| ())
}

/// Heuristic check if bytes look like UTF-16BE.
///
/// Returns true if:
/// - Length is even
/// - Most high bytes (first byte of each pair) are 0x00
fn looks_like_utf16be(bytes: &[u8]) -> bool {
    if bytes.len() < 2 || bytes.len() % 2 != 0 {
        return false;
    }

    let mut zero_high_bytes = 0;
    let total_pairs = bytes.len() / 2;

    for chunk in bytes.chunks_exact(2) {
        if chunk[0] == 0x00 {
            zero_high_bytes += 1;
        }
    }

    zero_high_bytes >= total_pairs * 3 / 4
}

/// Decode PDFDocEncoding (treat as Latin-1 for basic use).
///
/// PDFDocEncoding is a superset of ISO-8859-1 (Latin-1) with some characters
/// remapped. For attachment filenames and descriptions, treating as Latin-1
/// is sufficient for most use cases.
fn decode_pdfdocencoding(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

/// Parse a PDF date string to ISO 8601 format.
///
/// PDF date format: `D:YYYYMMDDHHmmSSOHH'mm'`
/// - Truncation is allowed (date only, date+time only)
/// - Timezone can be `Z`, `+HH'mm'`, `-HH'mm'`, or omitted (defaults to UTC)
///
/// Returns ISO 8601 format (RFC 3339) or None if parsing fails.
fn parse_pdf_date(pdf_date: &[u8]) -> Option<String> {
    let date_str = std::str::from_utf8(pdf_date).ok()?;

    // Strip "D:" prefix if present
    let date_str = date_str.strip_prefix("D:").unwrap_or(date_str);

    // Minimum required: YYYYMMDD (8 characters after stripping D:)
    if date_str.len() < 8 {
        return None;
    }

    // Parse date components
    let year = date_str[0..4].parse::<u32>().ok()?;
    let month = date_str[4..6].parse::<u32>().ok()?;
    let day = date_str[6..8].parse::<u32>().ok()?;

    // Validate date ranges
    if month == 0 || month > 12 || day == 0 || day > 31 {
        return None;
    }

    // Parse time components if present
    let (hour, minute, second) = if date_str.len() >= 14 {
        let hour = date_str[8..10].parse::<u32>().ok()?;
        let minute = date_str[10..12].parse::<u32>().ok()?;
        let second = date_str[12..14].parse::<u32>().ok()?;

        // Validate time ranges
        if hour > 23 || minute > 59 || second > 59 {
            return None;
        }
        (hour, minute, second)
    } else {
        // Default to midnight if time not present
        (0, 0, 0)
    };

    // Parse timezone if present
    let tz_str = if date_str.len() > 14 {
        &date_str[14..]
    } else {
        ""
    };

    let timezone = if tz_str.is_empty() || tz_str == "Z" {
        // Default to UTC if no timezone specified
        "Z".to_string()
    } else if tz_str.starts_with('+') || tz_str.starts_with('-') {
        // Parse OHH'mm format (e.g., +05'30' or -08'00')
        let sign = if tz_str.starts_with('+') { "+" } else { "-" };

        // Extract HH and mm from format like +05'30' or +0530
        let tz_digits: String = tz_str[1..].chars().filter(|c| c.is_ascii_digit()).collect();
        if tz_digits.len() >= 4 {
            let tz_hour = &tz_digits[0..2];
            let tz_min = &tz_digits[2..4];
            // Check if this is UTC (+00'00' or +0000)
            if tz_hour == "00" && tz_min == "00" {
                "Z".to_string()
            } else {
                format!("{}{}:{}", sign, tz_hour, tz_min)
            }
        } else {
            // Malformed timezone, default to UTC
            "Z".to_string()
        }
    } else {
        // Unknown format, default to UTC
        "Z".to_string()
    };

    // Format as ISO 8601: YYYY-MM-DDTHH:MM:SS+HH:MM
    Some(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{}",
        year, month, day, hour, minute, second, timezone
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::{intern, PdfDict, PdfObject, PdfStream};
    use indexmap::IndexMap;

    /// Helper to create a test Filespec dictionary.
    fn make_filespec(
        resolver: &XrefResolver,
        obj_ref: ObjRef,
        filename: &str,
        description: Option<&str>,
        stream_ref: ObjRef,
    ) {
        let mut dict = IndexMap::new();
        dict.insert(intern("/Type"), PdfObject::Name(intern("Filespec")));

        // /UF (Unicode filename, preferred)
        let mut uf_bytes = filename
            .encode_utf16()
            .flat_map(|c| c.to_be_bytes())
            .collect::<Vec<u8>>();
        // Add UTF-16BE BOM
        let mut uf_with_bom = vec![0xFE, 0xFF];
        uf_with_bom.extend_from_slice(&uf_bytes);
        dict.insert(intern("/UF"), PdfObject::String(Box::new(uf_with_bom)));

        // /F (system-independent filename, fallback)
        dict.insert(
            intern("/F"),
            PdfObject::String(Box::new(filename.as_bytes().to_vec())),
        );

        if let Some(desc) = description {
            dict.insert(
                intern("/Desc"),
                PdfObject::String(Box::new(desc.as_bytes().to_vec())),
            );
        }

        // /EF dictionary with /F stream reference
        let mut ef_dict = IndexMap::new();
        ef_dict.insert(intern("/F"), PdfObject::Ref(stream_ref));
        dict.insert(intern("/EF"), PdfObject::Dict(Box::new(ef_dict)));

        resolver.cache_object(obj_ref, PdfObject::Dict(Box::new(dict)));
    }

    /// Helper to create a test EF stream.
    fn make_ef_stream(
        resolver: &XrefResolver,
        stream_ref: ObjRef,
        content: &[u8],
        mime_type: Option<&str>,
        size: Option<u64>,
    ) {
        let mut dict = IndexMap::new();
        dict.insert(intern("/Length"), PdfObject::Integer(content.len() as i64));

        if let Some(mime) = mime_type {
            dict.insert(intern("/Subtype"), PdfObject::Name(intern(mime)));
        }

        // /Params dictionary
        let mut params_dict = IndexMap::new();
        if let Some(sz) = size {
            params_dict.insert(intern("/Size"), PdfObject::Integer(sz as i64));
        }
        if !params_dict.is_empty() {
            dict.insert(intern("/Params"), PdfObject::Dict(Box::new(params_dict)));
        }

        let stream = PdfStream::new(dict, 0, Some(content.len() as u64));
        resolver.cache_object(stream_ref, PdfObject::Stream(Box::new(stream)));
    }

    #[test]
    fn test_extract_filename_uf_preferred() {
        // UTF-16BE BOM (0xFE 0xFF) + "Test.txt" in big-endian
        let filespec_bytes = b"\xFE\xFF\x00T\x00e\x00s\x00t\x00.\x00t\x00x\x00t";
        let decoded = decode_pdf_string(filespec_bytes);
        assert_eq!(decoded, "Test.txt");
    }

    #[test]
    fn test_extract_filename_f_fallback() {
        let filespec_bytes = b"Test.txt"; // ASCII
        let decoded = decode_pdfdocencoding(filespec_bytes);
        assert_eq!(decoded, "Test.txt");
    }

    #[test]
    fn test_parse_pdf_date_full() {
        let result = parse_pdf_date(b"D:20230115143045+05'30'");
        assert_eq!(result, Some("2023-01-15T14:30:45+05:30".to_string()));
    }

    #[test]
    fn test_parse_pdf_date_utc() {
        let result = parse_pdf_date(b"D:20230115143045Z");
        assert_eq!(result, Some("2023-01-15T14:30:45Z".to_string()));
    }

    #[test]
    fn test_parse_pdf_date_only() {
        let result = parse_pdf_date(b"D:20230115");
        assert_eq!(result, Some("2023-01-15T00:00:00Z".to_string()));
    }

    #[test]
    fn test_parse_pdf_date_malformed() {
        assert!(parse_pdf_date(b"invalid").is_none());
        assert!(parse_pdf_date(b"D:2023").is_none());
    }

    #[test]
    fn test_decode_pdf_string_utf16be_bom() {
        // UTF-16BE BOM (0xFE 0xFF) + "Hello" in big-endian
        let bytes = b"\xFE\xFF\x00H\x00e\x00l\x00l\x00o";
        let decoded = decode_pdf_string(bytes);
        assert_eq!(decoded, "Hello");
    }

    #[test]
    fn test_decode_pdf_string_ascii() {
        let bytes = b"Hello";
        let decoded = decode_pdf_string(bytes);
        assert_eq!(decoded, "Hello");
    }

    #[test]
    fn test_decode_pdfdocencoding() {
        let bytes = b"Test\xE9\xE0\xEE"; // "Testéàî" in Latin-1
        let decoded = decode_pdfdocencoding(bytes);
        assert_eq!(decoded, "Testéàî");
    }
}
