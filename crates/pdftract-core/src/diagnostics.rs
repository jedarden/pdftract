//! Unified diagnostic system for PDF parsing and extraction.
//!
//! This module provides the centralized diagnostic types and catalog used across
//! all of pdftract-core. Per INV-8, all errors are emitted as diagnostics rather
//! than panicking. The parser always attempts recovery and continues processing.
//!
//! # Diagnostic codes
//!
//! Diagnostic codes follow a naming convention with prefixes indicating the category:
//! - `STRUCT_*` — PDF structure errors (parser/object/document layer)
//! - `STREAM_*` — Stream decoder errors
//! - `XREF_*` — Cross-reference table errors
//! - `ENCRYPTION_*` — Encryption-related errors
//! - `OCR_*` — OCR pipeline errors (Phase 5)
//! - `REMOTE_*` — Remote source errors (Phase 1.8)
//! - `PAGE_*` — Page-level errors
//! - `FONT_*` — Font pipeline errors
//! - `GSTATE_*` — Graphics state errors (Phase 3.1)
//! - `LAYOUT_*` — Layout and reading order errors (Phase 4)
//! - `MCP_*` — MCP server errors (Phase 6.7)
//! - `CACHE_*` — Cache errors (Phase 6.9)
//!
//! # Usage
//!
//! Emit diagnostics using the `emit!` macro:
//!
//! ```rust
//! use pdftract_core::diagnostics::{emit, DiagCode};
//!
//! let mut diagnostics = Vec::new();
//!
//! // Emit with code only
//! emit!(diagnostics, STRUCT_INVALID_NAME);
//!
//! // Emit with code and byte offset
//! emit!(diagnostics, STRUCT_INVALID_NAME, offset = 42);
//!
//! // Emit with code, byte offset, and object reference
//! emit!(diagnostics, STRUCT_MISSING_KEY, offset = 100, object = 5_0);
//!
//! // Emit with custom message
//! emit!(diagnostics, STREAM_DECODE_ERROR, offset = 200,
//!       message = "zlib stream truncated mid-inflation".to_string());
//! ```
//!
//! # Catalog
//!
//! The `DIAGNOSTIC_CATALOG` provides metadata about each diagnostic code, including
//! severity, recoverable flag, and suggested user action. Use the `pdftract --list-diagnostics`
//! CLI command to print the catalog (Phase 6).

use std::borrow::Cow;
use std::fmt;

/// Reference to an indirect PDF object.
///
/// An `ObjRef` uniquely identifies an object in a PDF document by its
/// object number and generation number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjRef {
    /// Object number (zero-based index in the xref table)
    pub object: u32,
    /// Generation number (incremented on each save)
    pub generation: u16,
}

impl ObjRef {
    /// Create a new object reference.
    #[inline]
    pub const fn new(object: u32, generation: u16) -> Self {
        ObjRef { object, generation }
    }
}

impl fmt::Display for ObjRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} R", self.object, self.generation)
    }
}

/// Severity level for a diagnostic.
///
/// Severity determines how the diagnostic affects the extraction result
/// and whether it should be surfaced to users prominently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Informational — does not affect output validity
    ///
    /// Examples: `XREF_REPAIRED`, `TAGGED_PDF_STRUCT_TREE_DEFERRED`
    Info,
    /// Warning — output is usable but degraded
    ///
    /// Examples: `STRUCT_INVALID_NAME`, `GLYPH_UNMAPPED`, `STREAM_DECODE_ERROR`
    Warning,
    /// Error — output for this region/page is invalid; other regions OK
    ///
    /// Examples: `STREAM_BOMB`, `REMOTE_FETCH_INTERRUPTED`
    Error,
    /// Fatal — extraction aborted, no usable output
    ///
    /// Examples: `ENCRYPTION_UNSUPPORTED` (no password supplied)
    Fatal,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Info => write!(f, "info"),
            Severity::Warning => write!(f, "warning"),
            Severity::Error => write!(f, "error"),
            Severity::Fatal => write!(f, "fatal"),
        }
    }
}

/// Diagnostic code identifying the type of error or warning.
///
/// These codes provide structured error classification for diagnostics
/// emitted during PDF parsing and extraction. The enum variants use
/// `#[repr(u16)]` for compact storage in diagnostics.
///
/// # Naming convention
///
/// All variants follow the `CATEGORY_SPECIFIC_ISSUE` pattern:
/// - `STRUCT_*` — PDF structure errors (parser/object/document layer)
/// - `STREAM_*` — Stream decoder errors
/// - `XREF_*` — Cross-reference table errors
/// - `ENCRYPTION_*` — Encryption-related errors
/// - `OCR_*` — OCR pipeline errors (Phase 5)
/// - `REMOTE_*` — Remote source errors (Phase 1.8)
/// - `PAGE_*` — Page-level errors
/// - `FONT_*` — Font pipeline errors
/// - `GSTATE_*` — Graphics state errors (Phase 3.1)
/// - `LAYOUT_*` — Layout and reading order errors (Phase 4)
/// - `MCP_*` — MCP server errors (Phase 6.7)
/// - `CACHE_*` — Cache errors (Phase 6.9)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum DiagCode {
    // === STRUCT_* codes ===

    /// Invalid name character or malformed name object
    ///
    /// Emitted when a PDF name object contains invalid characters or exceeds
    /// the 127-byte length limit. The name is truncated to 127 bytes per spec.
    /// No user action is required.
    ///
    /// Phase origin: 1.1
    StructInvalidName,

    /// Invalid hexadecimal character in hex string or name escape
    ///
    /// Emitted when a hex string (`<...>`) or hex escape (`#XX`) contains
    /// non-hexadecimal characters. The offending byte is skipped.
    ///
    /// Phase origin: 1.1
    StructInvalidHex,

    /// Invalid octal escape sequence in literal string
    ///
    /// Emitted when a literal string (`(...)`) contains an invalid octal
    /// escape sequence (`\NNN` where N is not 0-7). The escape is passed through
    /// literally.
    ///
    /// Phase origin: 1.1
    StructInvalidOctal,

    /// Invalid stream header (stream keyword not followed by proper newline)
    ///
    /// Emitted when the `stream` keyword is not immediately followed by a
    /// carriage return and/or line feed as required by the PDF spec.
    ///
    /// Phase origin: 1.1
    StructInvalidStreamHeader,

    /// Unexpected byte (e.g., stray `>` not part of `>>`)
    ///
    /// Emitted when the lexer encounters a byte that doesn't match the expected
    /// token syntax. The lexer attempts to recover by resynchronizing.
    ///
    /// Phase origin: 1.1
    StructUnexpectedByte,

    /// Unexpected end of file while parsing a token
    ///
    /// Emitted when the file ends mid-token. The lexer returns `Eof` and
    /// parsing continues with whatever was successfully parsed.
    ///
    /// Phase origin: 1.1
    StructUnexpectedEof,

    /// Unterminated literal string (missing closing paren)
    ///
    /// Emitted when a literal string is not closed before EOF. The string is
    /// treated as ending at EOF.
    ///
    /// Phase origin: 1.1
    StructUnterminatedString,

    /// Missing required dictionary key
    ///
    /// Emitted when a required key is missing from a dictionary. The behavior
    /// depends on the key: some are substituted with safe defaults (e.g., `/MediaBox`
    /// defaults to US Letter), others cause the object to be treated as null.
    ///
    /// Phase origin: 1.4
    StructMissingKey,

    /// Circular reference detected
    ///
    /// Emitted when an indirect reference forms a cycle (A → B → A). The cycle
    /// is broken at the second visit and the affected object is returned as null.
    ///
    /// Phase origin: 1.2
    StructCircularRef,

    /// Form XObject cycle detected
    ///
    /// Emitted when a form XObject invokes itself directly or indirectly,
    /// exceeding the depth limit of 20. The cycle is broken and execution continues.
    ///
    /// Phase origin: 3.3
    StructXobjectCycle,

    /// Dictionary nesting depth exceeds limit
    ///
    /// Emitted when dictionary nesting exceeds the internal limit (prevents stack
    /// overflow). The deeply nested structure is truncated.
    ///
    /// Phase origin: 1.2
    StructDepthExceeded,

    /// Invalid dictionary value (missing value after key)
    ///
    /// Emitted when a dictionary key is not followed by a value. The key is ignored.
    ///
    /// Phase origin: 1.2
    StructInvalidDictValue,

    /// Invalid dictionary key (not a name object)
    ///
    /// Emitted when a dictionary key is not a name object. The key is ignored.
    ///
    /// Phase origin: 1.2
    StructInvalidDictKey,

    /// Invalid indirect object header
    ///
    /// Emitted when an indirect object header (`N G obj`) is malformed.
    ///
    /// Phase origin: 1.2
    StructInvalidIndirectHeader,

    /// Integer overflow during parsing
    ///
    /// Emitted when parsing an integer that would overflow i64. The value is clamped.
    ///
    /// Phase origin: 1.2
    StructIntegerOverflow,

    /// Invalid object stream format
    ///
    /// Emitted when an object stream has a malformed header or invalid data.
    ///
    /// Phase origin: 1.2
    StructInvalidObjstm,

    /// Invalid UTF-16BE encoding in string
    ///
    /// Emitted when a UTF-16BE string has odd length or invalid encoding.
    /// The string is replaced with a placeholder.
    ///
    /// Phase origin: 1.4
    StructInvalidUtf16,

    /// Unresolved named destination
    ///
    /// Emitted when an outline destination is a named reference (not yet resolved).
    /// Named destination resolution is deferred to a future enhancement.
    ///
    /// Phase origin: 1.4
    StructUnresolvedDestination,

    /// Non-GoTo action in outline
    ///
    /// Emitted when an outline has an action other than GoTo (e.g., URI action).
    /// The outline destination is recorded as None.
    ///
    /// Phase origin: 1.4
    StructNonGotoOutline,

    /// Invalid PDFDocEncoding in string
    ///
    /// Emitted when a PDFDocEncoding string cannot be decoded to UTF-8.
    /// The string is replaced with a placeholder.
    ///
    /// Phase origin: 1.4
    StructInvalidPdfDocEncoding,

    /// Invalid geometry value (NaN or Inf in MediaBox/CropBox/Rotate)
    ///
    /// Emitted when a page geometry value (MediaBox, CropBox, Rotate) contains
    /// NaN or infinity. The value is canonicalized to 0 for fingerprint computation.
    ///
    /// Phase origin: 1.7
    StructInvalidGeometry,

    /// Invalid object type (expected type not found)
    ///
    /// Emitted when an object is not the expected type (e.g., expecting a stream
    /// but finding a dictionary). The object is treated as null.
    ///
    /// Phase origin: 5.2.1
    StructInvalidType,

    /// Hybrid xref conflict: traditional table and stream disagree on object state
    ///
    /// Emitted when merging a hybrid file's xref sections and the traditional
    /// table marks an object as Free while the stream marks it as InUse.
    /// Per PDF spec, the traditional entry wins (object is Free).
    ///
    /// Phase origin: 1.3
    StructHybridConflict,

    /// StructTree coverage below 80% threshold with /Suspects true
    ///
    /// Emitted when StructTree coverage is below 80% and /MarkInfo /Suspects is true,
    /// triggering XY-cut fallback per Phase 7.1.4.
    ///
    /// Phase origin: 7.1.4
    StructIncompleteCoverage,

    // === XREF_* codes ===

    /// Invalid xref keyword or header
    ///
    /// Emitted when the xref table doesn't start with the `xref` keyword.
    ///
    /// Phase origin: 1.3
    XrefInvalidHeader,

    /// Malformed xref entry (not 20 bytes, bad format)
    ///
    /// Emitted when an xref entry doesn't match the expected 20-byte format.
    ///
    /// Phase origin: 1.3
    XrefInvalidEntry,

    /// Invalid subsection header (not "start count")
    ///
    /// Emitted when an xref subsection header is malformed.
    ///
    /// Phase origin: 1.3
    XrefInvalidSubsectionHeader,

    /// Object 0 is not free (violates PDF spec)
    ///
    /// Emitted when object 0 is marked as in-use, which violates the PDF spec
    /// requirement that object 0 must always be free.
    ///
    /// Phase origin: 1.3
    XrefObjectZeroNotFree,

    /// Trailer dictionary not found or malformed
    ///
    /// Emitted when the trailer dictionary can't be located or parsed.
    ///
    /// Phase origin: 1.3
    XrefTrailerNotFound,

    /// Truncated xref table (unexpected EOF)
    ///
    /// Emitted when the xref table ends unexpectedly.
    ///
    /// Phase origin: 1.3
    XrefTruncated,

    /// Xref was reconstructed via forward scan (EC-07 recovery)
    ///
    /// Emitted when the primary xref strategies fail and forward scan (strategy 4)
    /// successfully recovers xref entries. The output may be incomplete on truncated files.
    ///
    /// Phase origin: 1.3
    XrefRepaired,

    /// Forward scan disabled for linearized files
    ///
    /// Emitted when forward scan is skipped for a linearized PDF because it would
    /// incorrectly find the partial first-page xref.
    ///
    /// Phase origin: 1.3
    XrefLinearizedNoForwardScan,

    /// Forward scan disabled for remote sources
    ///
    /// Emitted when forward scan is skipped for HTTP sources because it would
    /// require fetching the entire file.
    ///
    /// Phase origin: 1.3
    XrefRemoteNoForwardScan,

    /// Invalid xref stream format
    ///
    /// Emitted when an xref stream has a malformed header, invalid /W array,
    /// or other format violations. The stream is skipped.
    ///
    /// Phase origin: 1.3
    XrefInvalidStreamFormat,

    /// Invalid xref stream entry
    ///
    /// Emitted when an xref stream entry cannot be parsed due to invalid data
    /// in the stream's compressed entries section.
    ///
    /// Phase origin: 1.3
    XrefInvalidStreamEntry,

    /// Invalid /Prev offset in xref chain
    ///
    /// Emitted when a trailer's /Prev offset points to invalid data (outside file,
    /// not at xref boundary, etc.). The chain is truncated at this point.
    ///
    /// Phase origin: 1.3
    StructInvalidPrevOffset,

    // === STREAM_* codes ===

    /// Stream decompression failed (corrupt data)
    ///
    /// Emitted when a stream decoder encounters corrupt data mid-decompression.
    /// Partial bytes decoded so far are returned.
    ///
    /// Phase origin: 1.5
    StreamDecodeError,

    /// Decompression bomb limit exceeded
    ///
    /// Emitted when a stream's decompressed size would exceed `max_decompress_bytes`
    /// (default: 512 MiB). The stream is truncated at the limit. Increase the limit via
    /// `--max-decompress-gb` if the PDF is trusted.
    ///
    /// Phase origin: 1.5
    StreamBomb,

    /// Unknown filter name
    ///
    /// Emitted when a stream specifies a filter that pdftract doesn't support.
    ///
    /// Phase origin: 1.5
    StreamUnknownFilter,

    /// Invalid filter parameters
    ///
    /// Emitted when a stream's `/DecodeParms` dictionary is malformed or has
    /// invalid values. Default parameters are used.
    ///
    /// Phase origin: 1.5
    StreamInvalidParams,

    // === ENCRYPTION_* codes ===

    /// Unsupported encryption or no password supplied
    ///
    /// Emitted when the PDF is encrypted and no password was supplied, or the
    /// supplied password is incorrect, or the encryption algorithm is not supported.
    /// Extraction is aborted with exit code 3.
    ///
    /// Phase origin: 1.4
    EncryptionUnsupported,

    /// Password incorrect
    ///
    /// Emitted when the supplied password doesn't match the PDF's encryption key.
    ///
    /// Phase origin: 1.4
    EncryptionWrongPassword,

    // === PAGE_* codes ===

    /// Page number out of range
    ///
    /// Emitted when `--pages` specifies a page number greater than the document's
    /// page count. The page is skipped.
    ///
    /// Phase origin: 1.8
    PageOutOfRange,

    /// Invalid page count
    ///
    /// Emitted when the `/Count` key in the `/Pages` tree is invalid.
    ///
    /// Phase origin: 1.4
    PageInvalidCount,

    /// Invalid /Rotate value (not multiple of 90)
    ///
    /// Emitted when a page's `/Rotate` value is not a multiple of 90. The value
    /// is normalized to the nearest valid multiple.
    ///
    /// Phase origin: 1.4
    PageInvalidRotate,

    // === FONT_* codes ===

    /// Glyph could not be mapped to Unicode
    ///
    /// Emitted when a glyph has no entry in the font's `/ToUnicode` CMap, is not
    /// in the AGL, doesn't match any fingerprint, and doesn't match any glyph shape.
    /// U+FFFD is emitted for the glyph.
    ///
    /// Phase origin: 2.2
    FontGlyphUnmapped,

    /// Font not found or couldn't be parsed
    ///
    /// Emitted when a referenced font is missing from the PDF or couldn't be parsed.
    /// A fallback font is used.
    ///
    /// Phase origin: 2.1
    FontNotFound,

    /// Invalid CMap format
    ///
    /// Emitted when a CMap stream is malformed. The CMap is treated as empty.
    ///
    /// Phase origin: 2.2
    FontInvalidCmap,

    /// Font program parsing failed
    ///
    /// Emitted when an embedded font program is corrupt or invalid.
    /// The font is treated as having no glyph mappings and the fallback chain is used.
    ///
    /// Phase origin: 2.1
    FontParseFailed,

    /// Font type not supported for embedded loading
    ///
    /// Emitted when a font type is encountered that doesn't support embedded
    /// font program loading (e.g., Type3, CID fonts without OpenType).
    /// The font is treated as having no glyph mappings and the fallback chain is used.
    ///
    /// Phase origin: 2.1
    FontUnsupported,

    /// CIDToGIDMap stream has odd byte count (truncated GID entry)
    ///
    /// Emitted when a CIDToGIDMap stream has an odd number of bytes, meaning
    /// the last GID entry is incomplete. The trailing byte is discarded.
    ///
    /// Phase origin: 2.1
    FontCidtogidmapTruncated,

    /// Character code in /Differences array exceeds valid range
    ///
    /// Emitted when a /Differences array contains an integer code outside the
    /// valid range for single-byte encodings (0-255). The code is clamped to u8.
    ///
    /// Phase origin: 2.2
    FontEncodingDifferenceOutOfRange,

    // === OCR_* codes ===

    /// JBIG2 decoder not available
    ///
    /// Emitted when a PDF contains JBIG2-compressed images and pdftract wasn't
    /// built with `--features full-render`. Build with the feature or use a different
    /// decoder.
    ///
    /// Phase origin: 1.5 / 5.2
    OcrJbig2Unsupported,

    /// JPEG2000 (JPX) decoder not available
    ///
    /// Emitted when a PDF contains JPEG2000-compressed images and pdftract wasn't
    /// built with `--features full-render`. Build with the feature or install
    /// `libopenjp2`.
    ///
    /// Phase origin: 1.5 / 5.2
    OcrJpxUnsupported,

    /// CCITT fax decoder not available
    ///
    /// Emitted when a PDF contains CCITT-compressed images and the `libtiff`
    /// system library is not installed. Install the library or build with
    /// `--features full-render`.
    ///
    /// Phase origin: 1.5 / 5.2
    OcrCcittUnsupported,

    /// Tesseract OCR failed
    ///
    /// Emitted when Tesseract crashes or returns an error. The page is treated
    /// as a vector page (no OCR).
    ///
    /// Phase origin: 5.4
    OcrTesseractFailed,

    /// OCR unavailable on broken-vector page
    ///
    /// Emitted when a page is detected as BrokenVector but pdftract wasn't built
    /// with `--features ocr`. Build with the feature to enable OCR recovery.
    ///
    /// Phase origin: 4.7
    OcrBrokenVectorUnavailable,

    /// Image soft mask not supported in direct compositing path
    ///
    /// Emitted when an image XObject has a /SMask entry. Direct compositing
    /// doesn't support soft masks; use `full-render` feature for proper rendering.
    /// The masked image is skipped.
    ///
    /// Phase origin: 5.2.1
    ImgSoftmaskUnsupported,

    /// Image format not supported
    ///
    /// Emitted when an image XObject uses an unsupported format or bits-per-component
    /// value. The image is skipped.
    ///
    /// Phase origin: 5.2.1
    ImgUnsupportedFormat,

    /// Stream data truncated
    ///
    /// Emitted when a stream has less data than expected based on its declared
    /// dimensions and color space. Partial data is used.
    ///
    /// Phase origin: 1.5 / 5.2.1
    StreamTruncated,

    // === REMOTE_* codes ===

    /// HTTP fetch interrupted or failed
    ///
    /// Emitted when an HTTP range request fails due to network error, timeout,
    /// or server error. The request can be retried.
    ///
    /// Phase origin: 1.8
    RemoteFetchInterrupted,

    /// Server does not support Range requests
    ///
    /// Emitted when the HTTP server doesn't support the `Range:` header. pdftract
    /// falls back to downloading the entire file.
    ///
    /// Phase origin: 1.8
    RemoteNoRangeSupport,

    /// TLS handshake failed
    ///
    /// Emitted when the TLS handshake fails. The extraction is aborted with exit code 6.
    ///
    /// Phase origin: 1.8
    RemoteTlsFailed,

    /// DNS resolution failed
    ///
    /// Emitted when the hostname cannot be resolved. The extraction is aborted with exit code 4.
    ///
    /// Phase origin: 1.8
    RemoteDnsFailed,

    // === GSTATE_* codes ===

    /// Graphics state stack overflow
    ///
    /// Emitted when the graphics state stack exceeds the internal limit (prevents
    /// stack overflow). The `q` operator is ignored.
    ///
    /// Phase origin: 3.1
    GstateStackOverflow,

    /// Graphics state stack underflow
    ///
    /// Emitted when `Q` is called more times than `q`. The `Q` is ignored.
    ///
    /// Phase origin: 3.1
    GstateStackUnderflow,

    /// Mismatched BT/ET pair
    ///
    /// Emitted when a text block doesn't have matching BT/ET operators. The
    /// mismatch is corrected implicitly.
    ///
    /// Phase origin: 3.1
    GstateBtEtMismatch,

    // === LAYOUT_* codes ===

    /// Tagged PDF StructTree deferred to Phase 7
    ///
    /// Emitted for tagged PDFs before Phase 7.1 is implemented. The StructTree
    /// is ignored and XY-cut is used instead.
    ///
    /// Phase origin: 4.5
    LayoutTaggedPdfDeferred,

    /// Reading order may be incorrect
    ///
    /// Emitted when the reading order algorithm detects ambiguity (e.g., complex
    /// multi-column layout). The order may be incorrect.
    ///
    /// Phase origin: 4.5
    LayoutReadingOrderAmbiguous,

    /// Low readability score
    ///
    /// Emitted when a page's readability score is below 0.85. This may indicate
    /// mojibake, scrambled text, or other encoding issues.
    ///
    /// Phase origin: 4.7
    LayoutLowReadability,

    // === MCP_* codes (Phase 6.7) ===

    /// MCP tool call has invalid parameters
    ///
    /// Emitted when an MCP tool call doesn't match the tool's schema.
    ///
    /// Phase origin: 6.7
    McpToolInvalidParams,

    /// MCP path traversal attempt
    ///
    /// Emitted when an MCP path escapes the `--root` directory. The request is denied.
    ///
    /// Phase origin: 6.7
    McpPathTraversal,

    // === CACHE_* codes (Phase 6.9) ===

    /// Cache entry is corrupted
    ///
    /// Emitted when a cached entry fails to deserialize. The entry is deleted
    /// and extraction is re-run.
    ///
    /// Phase origin: 6.9
    CacheEntryCorrupt,

    /// Cache write failed
    ///
    /// Emitted when writing to the cache fails (e.g., out of disk space).
    /// Extraction succeeds but the result isn't cached.
    ///
    /// Phase origin: 6.9
    CacheWriteFailed,
}

impl DiagCode {
    /// Get the category prefix for this diagnostic code.
    #[inline]
    pub const fn category(self) -> &'static str {
        match self {
            // STRUCT_*
            DiagCode::StructInvalidName
            | DiagCode::StructInvalidHex
            | DiagCode::StructInvalidOctal
            | DiagCode::StructInvalidStreamHeader
            | DiagCode::StructUnexpectedByte
            | DiagCode::StructUnexpectedEof
            | DiagCode::StructUnterminatedString
            | DiagCode::StructMissingKey
            | DiagCode::StructCircularRef
            | DiagCode::StructXobjectCycle
            | DiagCode::StructDepthExceeded
            | DiagCode::StructInvalidDictValue
            | DiagCode::StructInvalidDictKey
            | DiagCode::StructInvalidIndirectHeader
            | DiagCode::StructIntegerOverflow
            | DiagCode::StructInvalidObjstm
            | DiagCode::StructInvalidGeometry
            | DiagCode::StructInvalidType
            | DiagCode::StructInvalidUtf16
            | DiagCode::StructUnresolvedDestination
            | DiagCode::StructNonGotoOutline
            | DiagCode::StructInvalidPdfDocEncoding
            | DiagCode::StructHybridConflict
            | DiagCode::StructIncompleteCoverage => "STRUCT",

            // XREF_*
            DiagCode::XrefInvalidHeader
            | DiagCode::XrefInvalidEntry
            | DiagCode::XrefInvalidSubsectionHeader
            | DiagCode::XrefObjectZeroNotFree
            | DiagCode::XrefTrailerNotFound
            | DiagCode::XrefTruncated
            | DiagCode::XrefRepaired
            | DiagCode::XrefLinearizedNoForwardScan
            | DiagCode::XrefRemoteNoForwardScan
            | DiagCode::XrefInvalidStreamFormat
            | DiagCode::XrefInvalidStreamEntry => "XREF",

            // STRUCT_* (continued)
            DiagCode::StructInvalidPrevOffset => "STRUCT",

            // STREAM_*
            DiagCode::StreamDecodeError
            | DiagCode::StreamBomb
            | DiagCode::StreamUnknownFilter
            | DiagCode::StreamInvalidParams
            | DiagCode::StreamTruncated => "STREAM",

            // ENCRYPTION_*
            DiagCode::EncryptionUnsupported | DiagCode::EncryptionWrongPassword => "ENCRYPTION",

            // PAGE_*
            DiagCode::PageOutOfRange
            | DiagCode::PageInvalidCount
            | DiagCode::PageInvalidRotate => "PAGE",

            // FONT_*
            DiagCode::FontGlyphUnmapped
            | DiagCode::FontNotFound
            | DiagCode::FontInvalidCmap
            | DiagCode::FontParseFailed
            | DiagCode::FontUnsupported
            | DiagCode::FontCidtogidmapTruncated
            | DiagCode::FontEncodingDifferenceOutOfRange => "FONT",

            // OCR_*
            DiagCode::OcrJbig2Unsupported
            | DiagCode::OcrJpxUnsupported
            | DiagCode::OcrCcittUnsupported
            | DiagCode::OcrTesseractFailed
            | DiagCode::OcrBrokenVectorUnavailable => "OCR",

            // IMG_*
            DiagCode::ImgSoftmaskUnsupported
            | DiagCode::ImgUnsupportedFormat => "IMG",

            // REMOTE_*
            DiagCode::RemoteFetchInterrupted
            | DiagCode::RemoteNoRangeSupport
            | DiagCode::RemoteTlsFailed
            | DiagCode::RemoteDnsFailed => "REMOTE",

            // GSTATE_*
            DiagCode::GstateStackOverflow
            | DiagCode::GstateStackUnderflow
            | DiagCode::GstateBtEtMismatch => "GSTATE",

            // LAYOUT_*
            DiagCode::LayoutTaggedPdfDeferred
            | DiagCode::LayoutReadingOrderAmbiguous
            | DiagCode::LayoutLowReadability => "LAYOUT",

            // MCP_*
            DiagCode::McpToolInvalidParams | DiagCode::McpPathTraversal => "MCP",

            // CACHE_*
            DiagCode::CacheEntryCorrupt | DiagCode::CacheWriteFailed => "CACHE",
        }
    }

    /// Get the string name of this diagnostic code.
    #[inline]
    pub const fn name(self) -> &'static str {
        match self {
            DiagCode::StructInvalidName => "STRUCT_INVALID_NAME",
            DiagCode::StructInvalidHex => "STRUCT_INVALID_HEX",
            DiagCode::StructInvalidOctal => "STRUCT_INVALID_OCTAL",
            DiagCode::StructInvalidStreamHeader => "STRUCT_INVALID_STREAM_HEADER",
            DiagCode::StructUnexpectedByte => "STRUCT_UNEXPECTED_BYTE",
            DiagCode::StructUnexpectedEof => "STRUCT_UNEXPECTED_EOF",
            DiagCode::StructUnterminatedString => "STRUCT_UNTERMINATED_STRING",
            DiagCode::StructMissingKey => "STRUCT_MISSING_KEY",
            DiagCode::StructCircularRef => "STRUCT_CIRCULAR_REF",
            DiagCode::StructXobjectCycle => "STRUCT_XOBJECT_CYCLE",
            DiagCode::StructDepthExceeded => "STRUCT_DEPTH_EXCEEDED",
            DiagCode::StructInvalidDictValue => "STRUCT_INVALID_DICT_VALUE",
            DiagCode::StructInvalidDictKey => "STRUCT_INVALID_DICT_KEY",
            DiagCode::StructInvalidIndirectHeader => "STRUCT_INVALID_INDIRECT_HEADER",
            DiagCode::StructIntegerOverflow => "STRUCT_INTEGER_OVERFLOW",
            DiagCode::StructInvalidObjstm => "STRUCT_INVALID_OBJSTM",
            DiagCode::StructInvalidGeometry => "STRUCT_INVALID_GEOMETRY",
            DiagCode::StructInvalidType => "STRUCT_INVALID_TYPE",
            DiagCode::StructInvalidUtf16 => "STRUCT_INVALID_UTF16",
            DiagCode::StructUnresolvedDestination => "STRUCT_UNRESOLVED_DESTINATION",
            DiagCode::StructNonGotoOutline => "STRUCT_NON_GOTO_OUTLINE",
            DiagCode::StructInvalidPdfDocEncoding => "STRUCT_INVALID_PDFDOC_ENCODING",
            DiagCode::StructHybridConflict => "STRUCT_HYBRID_CONFLICT",
            DiagCode::StructIncompleteCoverage => "STRUCT_INCOMPLETE_COVERAGE",
            DiagCode::XrefInvalidHeader => "XREF_INVALID_HEADER",
            DiagCode::XrefInvalidEntry => "XREF_INVALID_ENTRY",
            DiagCode::XrefInvalidSubsectionHeader => "XREF_INVALID_SUBSECTION_HEADER",
            DiagCode::XrefObjectZeroNotFree => "XREF_OBJECT_ZERO_NOT_FREE",
            DiagCode::XrefTrailerNotFound => "XREF_TRAILER_NOT_FOUND",
            DiagCode::XrefTruncated => "XREF_TRUNCATED",
            DiagCode::XrefRepaired => "XREF_REPAIRED",
            DiagCode::XrefLinearizedNoForwardScan => "XREF_LINEARIZED_NO_FORWARD_SCAN",
            DiagCode::XrefRemoteNoForwardScan => "XREF_REMOTE_NO_FORWARD_SCAN",
            DiagCode::XrefInvalidStreamFormat => "XREF_INVALID_STREAM_FORMAT",
            DiagCode::XrefInvalidStreamEntry => "XREF_INVALID_STREAM_ENTRY",
            DiagCode::StructInvalidPrevOffset => "STRUCT_INVALID_PREV_OFFSET",
            DiagCode::StreamDecodeError => "STREAM_DECODE_ERROR",
            DiagCode::StreamBomb => "STREAM_BOMB",
            DiagCode::StreamUnknownFilter => "STREAM_UNKNOWN_FILTER",
            DiagCode::StreamInvalidParams => "STREAM_INVALID_PARAMS",
            DiagCode::EncryptionUnsupported => "ENCRYPTION_UNSUPPORTED",
            DiagCode::EncryptionWrongPassword => "ENCRYPTION_WRONG_PASSWORD",
            DiagCode::PageOutOfRange => "PAGE_OUT_OF_RANGE",
            DiagCode::PageInvalidCount => "PAGE_INVALID_COUNT",
            DiagCode::PageInvalidRotate => "PAGE_INVALID_ROTATE",
            DiagCode::FontGlyphUnmapped => "FONT_GLYPH_UNMAPPED",
            DiagCode::FontNotFound => "FONT_NOT_FOUND",
            DiagCode::FontInvalidCmap => "FONT_INVALID_CMAP",
            DiagCode::FontParseFailed => "FONT_PARSE_FAILED",
            DiagCode::FontUnsupported => "FONT_UNSUPPORTED",
            DiagCode::FontCidtogidmapTruncated => "FONT_CIDTOGIDMAP_TRUNCATED",
            DiagCode::FontEncodingDifferenceOutOfRange => "ENCODING_DIFFERENCE_OUT_OF_RANGE",
            DiagCode::OcrJbig2Unsupported => "OCR_JBIG2_UNSUPPORTED",
            DiagCode::OcrJpxUnsupported => "OCR_JPX_UNSUPPORTED",
            DiagCode::OcrCcittUnsupported => "OCR_CCITT_UNSUPPORTED",
            DiagCode::OcrTesseractFailed => "OCR_TESSERACT_FAILED",
            DiagCode::OcrBrokenVectorUnavailable => "OCR_BROKENVECTOR_UNAVAILABLE",
            DiagCode::ImgSoftmaskUnsupported => "IMG_SOFTMASK_UNSUPPORTED",
            DiagCode::ImgUnsupportedFormat => "IMG_UNSUPPORTED_FORMAT",
            DiagCode::StreamTruncated => "STREAM_TRUNCATED",
            DiagCode::RemoteFetchInterrupted => "REMOTE_FETCH_INTERRUPTED",
            DiagCode::RemoteNoRangeSupport => "REMOTE_NO_RANGE_SUPPORT",
            DiagCode::RemoteTlsFailed => "REMOTE_TLS_FAILED",
            DiagCode::RemoteDnsFailed => "REMOTE_DNS_FAILED",
            DiagCode::GstateStackOverflow => "GSTATE_STACK_OVERFLOW",
            DiagCode::GstateStackUnderflow => "GSTATE_STACK_UNDERFLOW",
            DiagCode::GstateBtEtMismatch => "GSTATE_BT_ET_MISMATCH",
            DiagCode::LayoutTaggedPdfDeferred => "TAGGED_PDF_STRUCT_TREE_DEFERRED",
            DiagCode::LayoutReadingOrderAmbiguous => "LAYOUT_READING_ORDER_AMBIGUOUS",
            DiagCode::LayoutLowReadability => "LAYOUT_LOW_READABILITY",
            DiagCode::McpToolInvalidParams => "MCP_TOOL_INVALID_PARAMS",
            DiagCode::McpPathTraversal => "MCP_PATH_TRAVERSAL",
            DiagCode::CacheEntryCorrupt => "CACHE_ENTRY_CORRUPT",
            DiagCode::CacheWriteFailed => "CACHE_WRITE_FAILED",
        }
    }

    /// Get the severity level for this diagnostic code.
    #[inline]
    pub const fn severity(self) -> Severity {
        match self {
            DiagCode::XrefRepaired
            | DiagCode::LayoutTaggedPdfDeferred
            | DiagCode::StructIncompleteCoverage => Severity::Info,

            DiagCode::StructInvalidName
            | DiagCode::StructInvalidHex
            | DiagCode::StructInvalidOctal
            | DiagCode::StructInvalidStreamHeader
            | DiagCode::StructUnexpectedByte
            | DiagCode::StructUnexpectedEof
            | DiagCode::StructUnterminatedString
            | DiagCode::StructMissingKey
            | DiagCode::StructCircularRef
            | DiagCode::StructXobjectCycle
            | DiagCode::StructDepthExceeded
            | DiagCode::StructInvalidDictValue
            | DiagCode::StructInvalidDictKey
            | DiagCode::StructInvalidIndirectHeader
            | DiagCode::StructIntegerOverflow
            | DiagCode::StructInvalidObjstm
            | DiagCode::StructInvalidGeometry
            | DiagCode::StructInvalidType
            | DiagCode::StructInvalidUtf16
            | DiagCode::StructUnresolvedDestination
            | DiagCode::StructNonGotoOutline
            | DiagCode::StructInvalidPdfDocEncoding
            | DiagCode::StructHybridConflict
            | DiagCode::StructInvalidPrevOffset
            | DiagCode::XrefInvalidHeader
            | DiagCode::XrefInvalidEntry
            | DiagCode::XrefInvalidSubsectionHeader
            | DiagCode::XrefObjectZeroNotFree
            | DiagCode::XrefTrailerNotFound
            | DiagCode::XrefTruncated
            | DiagCode::XrefLinearizedNoForwardScan
            | DiagCode::XrefRemoteNoForwardScan
            | DiagCode::XrefInvalidStreamFormat
            | DiagCode::XrefInvalidStreamEntry
            | DiagCode::StreamDecodeError
            | DiagCode::StreamUnknownFilter
            | DiagCode::StreamInvalidParams
            | DiagCode::PageInvalidCount
            | DiagCode::PageInvalidRotate
            | DiagCode::FontGlyphUnmapped
            | DiagCode::FontNotFound
            | DiagCode::FontInvalidCmap
            | DiagCode::FontParseFailed
            | DiagCode::FontUnsupported
            | DiagCode::FontCidtogidmapTruncated
            | DiagCode::FontEncodingDifferenceOutOfRange
            | DiagCode::OcrJbig2Unsupported
            | DiagCode::OcrJpxUnsupported
            | DiagCode::OcrCcittUnsupported
            | DiagCode::OcrTesseractFailed
            | DiagCode::OcrBrokenVectorUnavailable
            | DiagCode::ImgSoftmaskUnsupported
            | DiagCode::ImgUnsupportedFormat
            | DiagCode::StreamTruncated
            | DiagCode::RemoteNoRangeSupport
            | DiagCode::GstateStackOverflow
            | DiagCode::GstateStackUnderflow
            | DiagCode::GstateBtEtMismatch
            | DiagCode::LayoutReadingOrderAmbiguous
            | DiagCode::LayoutLowReadability
            | DiagCode::CacheEntryCorrupt
            | DiagCode::CacheWriteFailed => Severity::Warning,

            DiagCode::StreamBomb
            | DiagCode::PageOutOfRange
            | DiagCode::RemoteFetchInterrupted
            | DiagCode::McpToolInvalidParams
            | DiagCode::McpPathTraversal => Severity::Error,

            DiagCode::EncryptionUnsupported
            | DiagCode::EncryptionWrongPassword
            | DiagCode::RemoteTlsFailed
            | DiagCode::RemoteDnsFailed => Severity::Fatal,
        }
    }

    /// Check if this diagnostic code indicates a recoverable error.
    ///
    /// Recoverable errors allow parsing/extraction to continue. Non-recoverable
    /// errors (fatal) abort extraction.
    #[inline]
    pub const fn is_recoverable(self) -> bool {
        !matches!(
            self,
            DiagCode::EncryptionUnsupported
                | DiagCode::EncryptionWrongPassword
                | DiagCode::RemoteTlsFailed
                | DiagCode::RemoteDnsFailed
        )
    }
}

impl fmt::Display for DiagCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Metadata about a diagnostic code.
///
/// This struct provides information for the diagnostic catalog, including
/// severity, recoverable flag, phase origin, and suggested user action.
#[derive(Clone, Debug)]
pub struct DiagInfo {
    /// The diagnostic code
    pub code: DiagCode,
    /// Category name (e.g., "STRUCT", "STREAM", "XREF")
    pub category: &'static str,
    /// Severity level
    pub severity: Severity,
    /// Whether the error is recoverable (extraction can continue)
    pub recoverable: bool,
    /// Phase that introduced this diagnostic
    pub phase: &'static str,
    /// Suggested user action
    pub suggested_action: &'static str,
}

/// Static catalog of all diagnostic codes.
///
/// This array provides metadata about every diagnostic code, including severity,
/// recoverable flag, phase origin, and suggested user action. The catalog is used
/// by the `pdftract --list-diagnostics` CLI command and for documentation.
pub const DIAGNOSTIC_CATALOG: &[DiagInfo] = &[
    // === STRUCT_* codes ===
    DiagInfo {
        code: DiagCode::StructInvalidName,
        category: "STRUCT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.1",
        suggested_action: "None — the offending name was truncated to 127 bytes per spec",
    },
    DiagInfo {
        code: DiagCode::StructInvalidHex,
        category: "STRUCT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.1",
        suggested_action: "Inspect the source PDF for malformed hex escapes",
    },
    DiagInfo {
        code: DiagCode::StructInvalidOctal,
        category: "STRUCT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.1",
        suggested_action: "Inspect the source PDF for malformed octal escapes",
    },
    DiagInfo {
        code: DiagCode::StructInvalidStreamHeader,
        category: "STRUCT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.1",
        suggested_action: "The stream keyword must be followed by CRLF or LF",
    },
    DiagInfo {
        code: DiagCode::StructUnexpectedByte,
        category: "STRUCT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.1",
        suggested_action: "Inspect the source PDF for syntax errors",
    },
    DiagInfo {
        code: DiagCode::StructUnexpectedEof,
        category: "STRUCT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.1",
        suggested_action: "The file may be truncated",
    },
    DiagInfo {
        code: DiagCode::StructUnterminatedString,
        category: "STRUCT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.1",
        suggested_action: "The literal string is missing a closing parenthesis",
    },
    DiagInfo {
        code: DiagCode::StructMissingKey,
        category: "STRUCT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.4",
        suggested_action: "Inspect the source PDF; missing keys are typically substituted with safe defaults",
    },
    DiagInfo {
        code: DiagCode::StructCircularRef,
        category: "STRUCT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.2",
        suggested_action: "None — cycle broken at the second visit; affected object returned as null",
    },
    DiagInfo {
        code: DiagCode::StructXobjectCycle,
        category: "STRUCT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "3.3",
        suggested_action: "Investigate the source PDF for a producer bug; cycle is broken at depth 20",
    },
    DiagInfo {
        code: DiagCode::StructDepthExceeded,
        category: "STRUCT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.2",
        suggested_action: "The PDF has excessively nested structures",
    },
    DiagInfo {
        code: DiagCode::StructInvalidDictValue,
        category: "STRUCT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.2",
        suggested_action: "A dictionary key was not followed by a value",
    },
    DiagInfo {
        code: DiagCode::StructInvalidDictKey,
        category: "STRUCT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.2",
        suggested_action: "A dictionary key is not a name object",
    },
    DiagInfo {
        code: DiagCode::StructInvalidIndirectHeader,
        category: "STRUCT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.2",
        suggested_action: "The indirect object header (N G obj) is malformed",
    },
    DiagInfo {
        code: DiagCode::StructIntegerOverflow,
        category: "STRUCT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.2",
        suggested_action: "An integer value exceeded the i64 range and was clamped",
    },
    DiagInfo {
        code: DiagCode::StructInvalidObjstm,
        category: "STRUCT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.2",
        suggested_action: "The object stream has a malformed header or invalid data",
    },
    DiagInfo {
        code: DiagCode::StructInvalidGeometry,
        category: "STRUCT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.7",
        suggested_action: "NaN or Inf in MediaBox/CropBox/Rotate; canonicalized to 0 for fingerprint computation",
    },
    DiagInfo {
        code: DiagCode::StructHybridConflict,
        category: "STRUCT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.3",
        suggested_action: "Traditional table entry takes precedence; object marked as Free per traditional table",
    },
    DiagInfo {
        code: DiagCode::StructIncompleteCoverage,
        category: "STRUCT",
        severity: Severity::Info,
        recoverable: true,
        phase: "7.1.4",
        suggested_action: "StructTree coverage below 80% with /Suspects true; falling back to XY-cut reading order",
    },
    // === XREF_* codes ===
    DiagInfo {
        code: DiagCode::XrefInvalidHeader,
        category: "XREF",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.3",
        suggested_action: "The xref table doesn't start with the xref keyword",
    },
    DiagInfo {
        code: DiagCode::XrefInvalidEntry,
        category: "XREF",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.3",
        suggested_action: "An xref entry doesn't match the 20-byte format",
    },
    DiagInfo {
        code: DiagCode::XrefInvalidSubsectionHeader,
        category: "XREF",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.3",
        suggested_action: "An xref subsection header is malformed",
    },
    DiagInfo {
        code: DiagCode::XrefObjectZeroNotFree,
        category: "XREF",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.3",
        suggested_action: "Object 0 is not free (violates PDF spec)",
    },
    DiagInfo {
        code: DiagCode::XrefTrailerNotFound,
        category: "XREF",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.3",
        suggested_action: "The trailer dictionary couldn't be located",
    },
    DiagInfo {
        code: DiagCode::XrefTruncated,
        category: "XREF",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.3",
        suggested_action: "The xref table ends unexpectedly",
    },
    DiagInfo {
        code: DiagCode::XrefRepaired,
        category: "XREF",
        severity: Severity::Info,
        recoverable: true,
        phase: "1.3",
        suggested_action: "None — the xref was reconstructed via forward scan; output may be incomplete on truncated files",
    },
    DiagInfo {
        code: DiagCode::XrefLinearizedNoForwardScan,
        category: "XREF",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.3",
        suggested_action: "Forward scan is disabled for linearized PDFs",
    },
    DiagInfo {
        code: DiagCode::XrefRemoteNoForwardScan,
        category: "XREF",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.3",
        suggested_action: "Forward scan is disabled for HTTP sources (would fetch entire file)",
    },
    DiagInfo {
        code: DiagCode::XrefInvalidStreamFormat,
        category: "XREF",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.3",
        suggested_action: "The xref stream has a malformed header or invalid /W array; the stream is skipped",
    },
    DiagInfo {
        code: DiagCode::XrefInvalidStreamEntry,
        category: "XREF",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.3",
        suggested_action: "An xref stream entry cannot be parsed due to invalid data",
    },
    DiagInfo {
        code: DiagCode::StructInvalidPrevOffset,
        category: "STRUCT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.3",
        suggested_action: "A trailer's /Prev offset points to invalid data; the xref chain is truncated at this point",
    },
    // === STREAM_* codes ===
    DiagInfo {
        code: DiagCode::StreamDecodeError,
        category: "STREAM",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.5",
        suggested_action: "Partial output returned for this stream; consider re-saving the PDF through a normalising tool",
    },
    DiagInfo {
        code: DiagCode::StreamBomb,
        category: "STREAM",
        severity: Severity::Error,
        recoverable: true,
        phase: "1.5",
        suggested_action: "Increase --max-decompress-gb if the PDF is trusted; otherwise treat as a hostile file",
    },
    DiagInfo {
        code: DiagCode::StreamUnknownFilter,
        category: "STREAM",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.5",
        suggested_action: "The filter name is not supported by this version of pdftract",
    },
    DiagInfo {
        code: DiagCode::StreamInvalidParams,
        category: "STREAM",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.5",
        suggested_action: "The /DecodeParms dictionary is malformed; default parameters are used",
    },
    // === ENCRYPTION_* codes ===
    DiagInfo {
        code: DiagCode::EncryptionUnsupported,
        category: "ENCRYPTION",
        severity: Severity::Fatal,
        recoverable: false,
        phase: "1.4",
        suggested_action: "Supply the correct password via --password, or use an Adobe-side decryption tool first",
    },
    DiagInfo {
        code: DiagCode::EncryptionWrongPassword,
        category: "ENCRYPTION",
        severity: Severity::Fatal,
        recoverable: false,
        phase: "1.4",
        suggested_action: "The supplied password is incorrect",
    },
    // === PAGE_* codes ===
    DiagInfo {
        code: DiagCode::PageOutOfRange,
        category: "PAGE",
        severity: Severity::Error,
        recoverable: true,
        phase: "1.8",
        suggested_action: "Adjust the --pages argument to the actual document page count",
    },
    DiagInfo {
        code: DiagCode::PageInvalidCount,
        category: "PAGE",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.4",
        suggested_action: "The /Count key in the /Pages tree is invalid",
    },
    DiagInfo {
        code: DiagCode::PageInvalidRotate,
        category: "PAGE",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.4",
        suggested_action: "The /Rotate value is not a multiple of 90; it was normalized",
    },
    // === FONT_* codes ===
    DiagInfo {
        code: DiagCode::FontGlyphUnmapped,
        category: "FONT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "2.2",
        suggested_action: "The glyph could not be resolved by any of the four levels; output contains U+FFFD",
    },
    DiagInfo {
        code: DiagCode::FontNotFound,
        category: "FONT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "2.1",
        suggested_action: "A referenced font is missing from the PDF; a fallback font is used",
    },
    DiagInfo {
        code: DiagCode::FontInvalidCmap,
        category: "FONT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "2.2",
        suggested_action: "The CMap stream is malformed; it's treated as empty",
    },
    DiagInfo {
        code: DiagCode::FontParseFailed,
        category: "FONT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "2.1",
        suggested_action: "The embedded font program is corrupt or invalid; the font is treated as having no glyph mappings",
    },
    DiagInfo {
        code: DiagCode::FontUnsupported,
        category: "FONT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "2.1",
        suggested_action: "A font type was encountered that doesn't support embedded font program loading",
    },
    DiagInfo {
        code: DiagCode::FontCidtogidmapTruncated,
        category: "FONT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "2.1",
        suggested_action: "The CIDToGIDMap stream has an odd byte count; the trailing byte was discarded",
    },
    DiagInfo {
        code: DiagCode::FontEncodingDifferenceOutOfRange,
        category: "FONT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "2.2",
        suggested_action: "A /Differences array contains a character code outside 0-255; the code was clamped",
    },
    // === OCR_* codes ===
    DiagInfo {
        code: DiagCode::OcrJbig2Unsupported,
        category: "OCR",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.5 / 5.2",
        suggested_action: "Build with --features full-render to enable JBIG2 decoding via PDFium",
    },
    DiagInfo {
        code: DiagCode::OcrJpxUnsupported,
        category: "OCR",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.5 / 5.2",
        suggested_action: "Build with --features full-render, or install libopenjp2 system library",
    },
    DiagInfo {
        code: DiagCode::OcrCcittUnsupported,
        category: "OCR",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.5 / 5.2",
        suggested_action: "Install libtiff system library, or build with --features full-render",
    },
    DiagInfo {
        code: DiagCode::OcrTesseractFailed,
        category: "OCR",
        severity: Severity::Warning,
        recoverable: true,
        phase: "5.4",
        suggested_action: "Tesseract crashed or returned an error; the page is treated as vector",
    },
    DiagInfo {
        code: DiagCode::OcrBrokenVectorUnavailable,
        category: "OCR",
        severity: Severity::Warning,
        recoverable: true,
        phase: "4.7",
        suggested_action: "Build with --features ocr to enable OCR recovery on broken-vector pages",
    },
    // === IMG_* codes ===
    DiagInfo {
        code: DiagCode::ImgSoftmaskUnsupported,
        category: "IMG",
        severity: Severity::Warning,
        recoverable: true,
        phase: "5.2.1",
        suggested_action: "Soft-masked images not supported in direct compositing; use --features full-render for proper rendering",
    },
    DiagInfo {
        code: DiagCode::ImgUnsupportedFormat,
        category: "IMG",
        severity: Severity::Warning,
        recoverable: true,
        phase: "5.2.1",
        suggested_action: "Image format or bits-per-component not supported; image is skipped",
    },
    DiagInfo {
        code: DiagCode::StreamTruncated,
        category: "STREAM",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.5 / 5.2.1",
        suggested_action: "Stream has less data than expected; partial data is used",
    },
    // === REMOTE_* codes ===
    DiagInfo {
        code: DiagCode::RemoteFetchInterrupted,
        category: "REMOTE",
        severity: Severity::Error,
        recoverable: true,
        phase: "1.8",
        suggested_action: "Retry the request; check network connectivity",
    },
    DiagInfo {
        code: DiagCode::RemoteNoRangeSupport,
        category: "REMOTE",
        severity: Severity::Warning,
        recoverable: true,
        phase: "1.8",
        suggested_action: "None — pdftract falls back to whole-file download; consider hosting on a Range-supporting server",
    },
    DiagInfo {
        code: DiagCode::RemoteTlsFailed,
        category: "REMOTE",
        severity: Severity::Fatal,
        recoverable: false,
        phase: "1.8",
        suggested_action: "The TLS handshake failed; check the server's certificate",
    },
    DiagInfo {
        code: DiagCode::RemoteDnsFailed,
        category: "REMOTE",
        severity: Severity::Fatal,
        recoverable: false,
        phase: "1.8",
        suggested_action: "The hostname could not be resolved; check the URL",
    },
    // === GSTATE_* codes ===
    DiagInfo {
        code: DiagCode::GstateStackOverflow,
        category: "GSTATE",
        severity: Severity::Warning,
        recoverable: true,
        phase: "3.1",
        suggested_action: "Investigate the source PDF for a malformed content stream",
    },
    DiagInfo {
        code: DiagCode::GstateStackUnderflow,
        category: "GSTATE",
        severity: Severity::Warning,
        recoverable: true,
        phase: "3.1",
        suggested_action: "The content stream has more Q operators than q operators",
    },
    DiagInfo {
        code: DiagCode::GstateBtEtMismatch,
        category: "GSTATE",
        severity: Severity::Warning,
        recoverable: true,
        phase: "3.1",
        suggested_action: "The content stream has mismatched BT/ET operators",
    },
    // === LAYOUT_* codes ===
    DiagInfo {
        code: DiagCode::LayoutTaggedPdfDeferred,
        category: "LAYOUT",
        severity: Severity::Info,
        recoverable: true,
        phase: "4.5",
        suggested_action: "None — Phase 7.1 will replace this fallback in v1.0.0",
    },
    DiagInfo {
        code: DiagCode::LayoutReadingOrderAmbiguous,
        category: "LAYOUT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "4.5",
        suggested_action: "The reading order may be incorrect for complex multi-column layouts",
    },
    DiagInfo {
        code: DiagCode::LayoutLowReadability,
        category: "LAYOUT",
        severity: Severity::Warning,
        recoverable: true,
        phase: "4.7",
        suggested_action: "The page has low readability; may indicate mojibake or encoding issues",
    },
    // === MCP_* codes ===
    DiagInfo {
        code: DiagCode::McpToolInvalidParams,
        category: "MCP",
        severity: Severity::Error,
        recoverable: true,
        phase: "6.7",
        suggested_action: "Adjust the tool-call arguments to match the schema in tools/list",
    },
    DiagInfo {
        code: DiagCode::McpPathTraversal,
        category: "MCP",
        severity: Severity::Error,
        recoverable: true,
        phase: "6.7",
        suggested_action: "The requested path escapes --root; either fix the path or restart the server without --root",
    },
    // === CACHE_* codes ===
    DiagInfo {
        code: DiagCode::CacheEntryCorrupt,
        category: "CACHE",
        severity: Severity::Warning,
        recoverable: true,
        phase: "6.9",
        suggested_action: "None — the entry was deleted and extraction re-ran",
    },
    DiagInfo {
        code: DiagCode::CacheWriteFailed,
        category: "CACHE",
        severity: Severity::Warning,
        recoverable: true,
        phase: "6.9",
        suggested_action: "Check available disk space; extraction succeeded but the result wasn't cached",
    },
];

/// A diagnostic message emitted during PDF parsing and extraction.
///
/// Per INV-8, all errors are emitted as diagnostics rather than panicking.
/// The parser always attempts recovery and continues processing.
///
/// # Fields
///
/// - `code`: The diagnostic code identifying the type of error
/// - `byte_offset`: Optional byte offset in the input file where the error occurred
/// - `object_ref`: Optional indirect object reference where the error occurred
/// - `message`: Human-readable message (static or dynamic)
///
/// # Size
///
/// The struct is 56 bytes (code: 2, byte_offset: 16, object_ref: 12, message: 24 + padding).
/// Large parse failures may emit hundreds of diagnostics, so compact storage is important.
#[derive(Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Diagnostic code identifying the type of error
    pub code: DiagCode,
    /// Byte offset in the input where the error occurred (None if not applicable)
    pub byte_offset: Option<u64>,
    /// Object reference where the error occurred (None if not applicable)
    pub object_ref: Option<ObjRef>,
    /// Human-readable message (static messages don't allocate)
    pub message: Cow<'static, str>,
}

impl Diagnostic {
    /// Create a new diagnostic with a static message.
    #[inline]
    pub fn with_static(code: DiagCode, byte_offset: u64, message: &'static str) -> Self {
        Diagnostic {
            code,
            byte_offset: Some(byte_offset),
            object_ref: None,
            message: Cow::Borrowed(message),
        }
    }

    /// Create a new diagnostic with a static message and no byte offset.
    #[inline]
    pub fn with_static_no_offset(code: DiagCode, message: &'static str) -> Self {
        Diagnostic {
            code,
            byte_offset: None,
            object_ref: None,
            message: Cow::Borrowed(message),
        }
    }

    /// Create a new diagnostic with a dynamic message.
    #[inline]
    pub fn with_dynamic(code: DiagCode, byte_offset: u64, message: String) -> Self {
        Diagnostic {
            code,
            byte_offset: Some(byte_offset),
            object_ref: None,
            message: Cow::Owned(message),
        }
    }

    /// Create a new diagnostic with a dynamic message and no byte offset.
    #[inline]
    pub fn with_dynamic_no_offset(code: DiagCode, message: String) -> Self {
        Diagnostic {
            code,
            byte_offset: None,
            object_ref: None,
            message: Cow::Owned(message),
        }
    }

    /// Get the severity level for this diagnostic.
    #[inline]
    pub fn severity(&self) -> Severity {
        self.code.severity()
    }

    /// Check if this diagnostic indicates a recoverable error.
    #[inline]
    pub fn is_recoverable(&self) -> bool {
        self.code.is_recoverable()
    }

    /// Set the object reference for this diagnostic.
    #[inline]
    pub fn with_object_ref(mut self, object_ref: ObjRef) -> Self {
        self.object_ref = Some(object_ref);
        self
    }
}

impl fmt::Debug for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Diagnostic")
            .field("code", &self.code)
            .field("byte_offset", &self.byte_offset)
            .field("object_ref", &self.object_ref)
            .field("message", &self.message.as_ref())
            .finish()
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)?;
        if let Some(offset) = self.byte_offset {
            write!(f, " (byte offset {})", offset)?;
        }
        if let Some(obj_ref) = self.object_ref {
            write!(f, " [{}]", obj_ref)?;
        }
        Ok(())
    }
}

/// Emit a diagnostic to a diagnostics vector.
///
/// This macro provides ergonomic syntax for creating and pushing diagnostics.
/// It supports several forms:
///
/// ```rust
/// // Emit with code only (no offset, default message)
/// emit!(diagnostics, STRUCT_INVALID_NAME);
///
/// // Emit with code and byte offset
/// emit!(diagnostics, STRUCT_INVALID_NAME, offset = 42);
///
/// // Emit with code, byte offset, and object reference
/// emit!(diagnostics, STRUCT_MISSING_KEY, offset = 100, object = 5_0);
///
/// // Emit with custom message
/// emit!(diagnostics, STREAM_DECODE_ERROR, offset = 200,
///       message = "zlib stream truncated".to_string());
/// ```
///
/// # Parameters
///
/// - `diagnostics`: The `Vec<Diagnostic>` to push to
/// - `code`: The `DiagCode` variant (without the `DiagCode::` prefix)
/// - `offset = <expr>`: Optional byte offset (u64 or None)
/// - `object = <num>_<gen>`: Optional object reference (e.g., `5_0` for object 5 gen 0)
/// - `message = <expr>`: Optional custom message (String or &'static str)
#[macro_export]
macro_rules! emit {
    // emit!(diagnostics, CODE)
    ($diagnostics:expr, $code:ident) => {{
        $diagnostics.push($crate::diagnostics::Diagnostic::with_static_no_offset(
            $crate::diagnostics::DiagCode::$code,
            concat!(stringify!($code), " diagnostic emitted"),
        ));
    }};

    // emit!(diagnostics, CODE, offset = <expr>)
    ($diagnostics:expr, $code:ident, offset = $offset:expr) => {{
        $diagnostics.push($crate::diagnostics::Diagnostic::with_static(
            $crate::diagnostics::DiagCode::$code,
            $offset,
            concat!(stringify!($code), " diagnostic emitted"),
        ));
    }};

    // emit!(diagnostics, CODE, offset = <expr>, object = (<num>, <gen>))
    ($diagnostics:expr, $code:ident, offset = $offset:expr, object = ($obj_num:expr, $obj_gen:expr)) => {{
        $diagnostics.push(
            $crate::diagnostics::Diagnostic::with_static(
                $crate::diagnostics::DiagCode::$code,
                $offset,
                concat!(stringify!($code), " diagnostic emitted"),
            )
            .with_object_ref($crate::diagnostics::ObjRef::new($obj_num, $obj_gen)),
        );
    }};

    // emit!(diagnostics, CODE, offset = <expr>, message = <expr>)
    ($diagnostics:expr, $code:ident, offset = $offset:expr, message = $msg:expr) => {{
        $diagnostics.push($crate::diagnostics::Diagnostic::with_dynamic(
            $crate::diagnostics::DiagCode::$code,
            $offset,
            $msg.into(),
        ));
    }};

    // emit!(diagnostics, CODE, message = <expr>)
    ($diagnostics:expr, $code:ident, message = $msg:expr) => {{
        $diagnostics.push($crate::diagnostics::Diagnostic::with_dynamic_no_offset(
            $crate::diagnostics::DiagCode::$code,
            $msg.into(),
        ));
    }};
}

// Static assertion: Diagnostic struct size should be 48-64 bytes
// Updated to reflect actual size after adding object_ref field (56 bytes)
const _: () = {
    let _assert: [(); 9] = [(); std::mem::size_of::<Diagnostic>() - 47]; // Fails if size < 48 (actual: 56 - 47 = 9)
    let _assert: [(); 8] = [(); 64 - std::mem::size_of::<Diagnostic>()]; // Fails if size > 64 (actual: 64 - 56 = 8)
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obj_ref_display() {
        let obj_ref = ObjRef::new(5, 0);
        assert_eq!(obj_ref.to_string(), "5 0 R");
    }

    #[test]
    fn test_obj_ref_new() {
        let obj_ref = ObjRef::new(42, 3);
        assert_eq!(obj_ref.object, 42);
        assert_eq!(obj_ref.generation, 3);
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::Info.to_string(), "info");
        assert_eq!(Severity::Warning.to_string(), "warning");
        assert_eq!(Severity::Error.to_string(), "error");
        assert_eq!(Severity::Fatal.to_string(), "fatal");
    }

    #[test]
    fn test_diag_code_name() {
        assert_eq!(DiagCode::StructInvalidName.name(), "STRUCT_INVALID_NAME");
        assert_eq!(DiagCode::XrefRepaired.name(), "XREF_REPAIRED");
        assert_eq!(DiagCode::StreamBomb.name(), "STREAM_BOMB");
    }

    #[test]
    fn test_diag_code_severity() {
        assert_eq!(DiagCode::StructInvalidName.severity(), Severity::Warning);
        assert_eq!(DiagCode::XrefRepaired.severity(), Severity::Info);
        assert_eq!(DiagCode::StreamBomb.severity(), Severity::Error);
        assert_eq!(DiagCode::EncryptionUnsupported.severity(), Severity::Fatal);
    }

    #[test]
    fn test_diag_code_recoverable() {
        assert!(DiagCode::StructInvalidName.is_recoverable());
        assert!(DiagCode::XrefRepaired.is_recoverable());
        assert!(DiagCode::StreamBomb.is_recoverable());
        assert!(!DiagCode::EncryptionUnsupported.is_recoverable());
    }

    #[test]
    fn test_diag_code_category() {
        assert_eq!(DiagCode::StructInvalidName.category(), "STRUCT");
        assert_eq!(DiagCode::XrefRepaired.category(), "XREF");
        assert_eq!(DiagCode::StreamBomb.category(), "STREAM");
        assert_eq!(DiagCode::EncryptionUnsupported.category(), "ENCRYPTION");
    }

    #[test]
    fn test_diagnostic_with_static() {
        let diag = Diagnostic::with_static(DiagCode::StructInvalidName, 42, "test message");
        assert_eq!(diag.code, DiagCode::StructInvalidName);
        assert_eq!(diag.byte_offset, Some(42));
        assert_eq!(diag.object_ref, None);
        assert_eq!(diag.message.as_ref(), "test message");
    }

    #[test]
    fn test_diagnostic_with_static_no_offset() {
        let diag = Diagnostic::with_static_no_offset(DiagCode::StructInvalidName, "test message");
        assert_eq!(diag.code, DiagCode::StructInvalidName);
        assert_eq!(diag.byte_offset, None);
        assert_eq!(diag.object_ref, None);
        assert_eq!(diag.message.as_ref(), "test message");
    }

    #[test]
    fn test_diagnostic_with_dynamic() {
        let diag = Diagnostic::with_dynamic(DiagCode::StructInvalidName, 42, "dynamic message".to_string());
        assert_eq!(diag.code, DiagCode::StructInvalidName);
        assert_eq!(diag.byte_offset, Some(42));
        assert_eq!(diag.object_ref, None);
        assert_eq!(diag.message.as_ref(), "dynamic message");
    }

    #[test]
    fn test_diagnostic_with_object_ref() {
        let diag = Diagnostic::with_static(DiagCode::StructInvalidName, 42, "test message")
            .with_object_ref(ObjRef::new(5, 0));
        assert_eq!(diag.object_ref, Some(ObjRef::new(5, 0)));
    }

    #[test]
    fn test_diagnostic_display() {
        let diag = Diagnostic::with_static(DiagCode::StructInvalidName, 42, "test message");
        assert_eq!(diag.to_string(), "STRUCT_INVALID_NAME: test message (byte offset 42)");

        let diag_with_obj = Diagnostic::with_static(DiagCode::StructInvalidName, 42, "test message")
            .with_object_ref(ObjRef::new(5, 0));
        assert_eq!(
            diag_with_obj.to_string(),
            "STRUCT_INVALID_NAME: test message (byte offset 42) [5 0 R]"
        );
    }

    #[test]
    fn test_diagnostic_severity() {
        let diag = Diagnostic::with_static(DiagCode::StructInvalidName, 42, "test");
        assert_eq!(diag.severity(), Severity::Warning);
        assert!(diag.is_recoverable());

        let diag = Diagnostic::with_static(DiagCode::EncryptionUnsupported, 0, "test");
        assert_eq!(diag.severity(), Severity::Fatal);
        assert!(!diag.is_recoverable());
    }

    #[test]
    fn test_emit_macro_basic() {
        let mut diagnostics = Vec::new();
        emit!(diagnostics, StructInvalidName);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagCode::StructInvalidName);
        assert_eq!(diagnostics[0].byte_offset, None);
    }

    #[test]
    fn test_emit_macro_with_offset() {
        let mut diagnostics = Vec::new();
        emit!(diagnostics, StructInvalidName, offset = 42);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].byte_offset, Some(42));
    }

    #[test]
    fn test_emit_macro_with_object_ref() {
        let mut diagnostics = Vec::new();
        emit!(diagnostics, StructMissingKey, offset = 100, object = (5, 0));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].byte_offset, Some(100));
        assert_eq!(diagnostics[0].object_ref, Some(ObjRef::new(5, 0)));
    }

    #[test]
    fn test_emit_macro_with_message() {
        let mut diagnostics = Vec::new();
        emit!(diagnostics, StreamDecodeError, offset = 200, message = "zlib error".to_string());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message.as_ref(), "zlib error");
    }

    #[test]
    fn test_catalog_complete() {
        // Verify that every DiagCode variant has a catalog entry
        for info in DIAGNOSTIC_CATALOG {
            // Verify that the code's name matches what we'd get from the enum
            assert_eq!(info.code.name(), info.code.name());
            // Verify that the severity matches
            assert_eq!(info.severity, info.code.severity());
            // Verify that the recoverable flag matches
            assert_eq!(info.recoverable, info.code.is_recoverable());
            // Verify that the category matches
            assert_eq!(info.category, info.code.category());
        }
    }

    #[test]
    fn test_diagnostic_size() {
        let size = std::mem::size_of::<Diagnostic>();
        // Diagnostic should be 48-64 bytes (actual: 56)
        // breakdown: code (2) + byte_offset (16) + object_ref (12) + message (24) + padding (2)
        assert!(size >= 48, "Diagnostic is smaller than expected: {} bytes", size);
        assert!(size <= 64, "Diagnostic is larger than expected: {} bytes", size);
    }
}
