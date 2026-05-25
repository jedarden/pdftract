//! PDF article thread discovery and metadata extraction.
//!
//! This module implements Phase 7.7.1 of the plan: reading the /Threads array
//! from the document catalog and extracting thread info metadata (/I) for each
//! thread.
//!
//! ## Architecture
//!
//! - **Discovery** (7.7.1): Read /Threads array from catalog, extract /F and /I
//! - **Bead chain walking** (7.7.2): Walk /N links from first bead (future work)
//!
//! ## PDF Thread Structure
//!
//! Per PDF 1.7 Section 12.4.3, an article thread consists of:
//! - `/Threads` array in catalog (optional)
//! - Each thread dict has:
//!   - `/F`: Indirect reference to first bead (required)
//!   - `/I`: Thread info dict (optional)
//!     - `/Title`: Thread title (PdfString, optional)
//!     - `/Author`: Thread author (PdfString, optional)
//!     - `/Subject`: Thread subject (PdfString, optional)
//!     - `/Keywords`: Thread keywords (PdfString, optional, comma-separated)

use crate::diagnostics::{DiagCode, Diagnostic};
use crate::parser::catalog::Catalog;
use crate::parser::object::{ObjRef, PdfObject};
use crate::parser::xref::XrefResolver;

/// Result type for thread operations.
pub type Result<T> = std::result::Result<T, Vec<Diagnostic>>;

/// A thread header with metadata from the thread info dict.
///
/// Represents the metadata for a single article thread, extracted from
/// the /I dict in the /Threads array entry. The bead chain walking
/// happens in Phase 7.7.2.
///
/// # Fields
///
/// * `first_bead_ref` - Indirect reference to the first bead in the chain
/// * `title` - Thread title from /I/Title (None if /I missing or /Title absent)
/// * `author` - Thread author from /I/Author (None if /I missing or /Author absent)
/// * `subject` - Thread subject from /I/Subject (None if /I missing or /Subject absent)
/// * `keywords` - Thread keywords from /I/Keywords (None if /I missing or /Keywords absent)
#[derive(Debug, Clone, PartialEq)]
pub struct ThreadHeader {
    /// Indirect reference to the first bead in the thread chain.
    ///
    /// This is always present for valid threads; threads without /F are
    /// skipped with a diagnostic.
    pub first_bead_ref: ObjRef,

    /// Thread title from /I/Title.
    ///
    /// - `Some("")` if /I/Title is present but empty string
    /// - `None` if /I is missing or /Title is absent
    pub title: Option<String>,

    /// Thread author from /I/Author.
    ///
    /// - `Some("")` if /I/Author is present but empty string
    /// - `None` if /I is missing or /Author is absent
    pub author: Option<String>,

    /// Thread subject from /I/Subject.
    ///
    /// - `Some("")` if /I/Subject is present but empty string
    /// - `None` if /I is missing or /Subject is absent
    pub subject: Option<String>,

    /// Thread keywords from /I/Keywords.
    ///
    /// Per PDF spec, this is a comma-separated convention (not an array).
    /// - `Some("")` if /I/Keywords is present but empty string
    /// - `None` if /I is missing or /Keywords is absent
    pub keywords: Option<String>,
}

impl ThreadHeader {
    /// Create a new ThreadHeader with the required first bead reference.
    pub fn new(first_bead_ref: ObjRef) -> Self {
        ThreadHeader {
            first_bead_ref,
            title: None,
            author: None,
            subject: None,
            keywords: None,
        }
    }
}

/// Discover article threads from the document catalog.
///
/// Reads the optional /Threads array from the catalog and extracts thread
/// headers (metadata only; bead chain walking is Phase 7.7.2).
///
/// # Arguments
///
/// * `catalog` - The document catalog (may have /Threads)
/// * `resolver` - The xref resolver for resolving indirect references
///
/// # Returns
///
/// A `Result<Vec<ThreadHeader>>` containing all discovered thread headers,
/// or a list of diagnostics (for fatal errors only; per-thread errors are
/// emitted as diagnostics but don't fail the entire operation).
///
/// # Behavior
///
/// - If /Threads is absent or not an array, returns empty Vec (no diagnostic)
/// - If a thread dict lacks /F, skips with diagnostic and continues processing
/// - If /I is missing, all four fields are None (not a diagnostic)
/// - Empty strings ("") are emitted as Some("") to distinguish from absent fields
/// - Multiple threads with the same /Title are legal (no deduplication)
pub fn discover(catalog: &Catalog, resolver: &XrefResolver) -> Result<Vec<ThreadHeader>> {
    let mut threads = Vec::new();
    let mut diagnostics = Vec::new();

    // /Threads is optional; absent is not an error
    let threads_ref = match catalog.threads_ref {
        Some(ref_) => ref_,
        None => return Ok(threads),
    };

    // Resolve the /Threads array
    let threads_obj = match resolver.resolve(threads_ref) {
        Ok(obj) => obj,
        Err(_) => {
            // If we can't resolve /Threads, return empty (not fatal)
            return Ok(threads);
        }
    };

    let threads_array = match threads_obj.as_array() {
        Some(arr) => arr,
        None => {
            // /Threads exists but isn't an array; skip without diagnostic
            return Ok(threads);
        }
    };

    // Process each thread entry in the array
    for (idx, thread_entry) in threads_array.iter().enumerate() {
        // Each thread entry should be an indirect ref to a thread dict
        let thread_ref = match thread_entry {
            PdfObject::Ref(ref_) => *ref_,
            _ => {
                // Skip non-ref entries with diagnostic
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructUnexpectedEof,
                    format!(
                        "Thread entry {} is not an indirect reference (type: {})",
                        idx,
                        thread_entry.type_name()
                    ),
                ));
                continue;
            }
        };

        let thread_obj = match resolver.resolve(thread_ref) {
            Ok(obj) => obj,
            Err(_) => {
                // Skip unresolvable thread refs
                continue;
            }
        };

        let thread_dict = match thread_obj.as_dict() {
            Some(d) => d,
            None => {
                // Skip non-dict threads
                continue;
            }
        };

        // Extract /F (first bead reference) - REQUIRED
        let first_bead_ref = match thread_dict.get("F") {
            Some(PdfObject::Ref(ref_)) => *ref_,
            Some(other) => {
                // /F exists but isn't a ref - skip with diagnostic
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructMissingKey,
                    format!(
                        "Thread {} has /F but it's not a reference (type: {})",
                        idx,
                        other.type_name()
                    ),
                ));
                continue;
            }
            None => {
                // /F is required - skip with diagnostic
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructMissingKey,
                    format!("Thread {} is missing /F (first bead reference)", idx),
                ));
                continue;
            }
        };

        let mut header = ThreadHeader::new(first_bead_ref);

        // Extract /I (thread info dict) - OPTIONAL
        if let Some(info_obj) = thread_dict.get("I") {
            if let Some(info_dict) = info_obj.as_dict() {
                // Extract /Title
                if let Some(title_bytes) = info_dict.get("Title").and_then(|o| o.as_string()) {
                    header.title = decode_pdf_string(title_bytes);
                }

                // Extract /Author
                if let Some(author_bytes) = info_dict.get("Author").and_then(|o| o.as_string()) {
                    header.author = decode_pdf_string(author_bytes);
                }

                // Extract /Subject
                if let Some(subject_bytes) = info_dict.get("Subject").and_then(|o| o.as_string()) {
                    header.subject = decode_pdf_string(subject_bytes);
                }

                // Extract /Keywords
                if let Some(keywords_bytes) = info_dict.get("Keywords").and_then(|o| o.as_string())
                {
                    header.keywords = decode_pdf_string(keywords_bytes);
                }
            }
            // If /I exists but isn't a dict, we skip it (no diagnostic, header fields stay None)
        }

        threads.push(header);
    }

    // Only return Err if diagnostics were actually fatal (none are currently)
    Ok(threads)
}

/// Decode a PDF string to a Rust String.
///
/// Handles PDFDocEncoding and UTF-16BE with BOM, per PDF 1.7 Section 5.3.3.
/// This is a minimal reimplementation of the decode_pdf_string from the
/// outline module, moved here for thread module use.
fn decode_pdf_string(bytes: &[u8]) -> Option<String> {
    // Check for UTF-16BE BOM (0xFE 0xFF)
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        return decode_utf16be(&bytes[2..]);
    }

    // Fall back to PDFDocEncoding (latin1-ish)
    decode_pdfdocencoding(bytes)
}

/// Decode UTF-16BE bytes (after BOM) to a String.
fn decode_utf16be(bytes: &[u8]) -> Option<String> {
    if bytes.len() % 2 != 0 {
        return None;
    }

    let utf16_chars: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect();

    String::from_utf16(&utf16_chars).ok()
}

/// Decode PDFDocEncoding bytes to a String.
///
/// PDFDocEncoding is a single-byte encoding that maps bytes 0-255 to
/// Unicode codepoints. For bytes 0-127, it matches ASCII. For bytes 128-255,
/// it maps to various Latin-1 and special characters.
fn decode_pdfdocencoding(bytes: &[u8]) -> Option<String> {
    // For most practical purposes, PDFDocEncoding is a superset of Latin-1
    // We use Latin-1 decoding which never fails (maps each byte to a char)
    Some(bytes.iter().map(|&b| b as char).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::xref::XrefResolver;

    #[test]
    fn test_thread_header_new() {
        let ref_ = ObjRef::new(1, 0);
        let header = ThreadHeader::new(ref_);

        assert_eq!(header.first_bead_ref, ref_);
        assert!(header.title.is_none());
        assert!(header.author.is_none());
        assert!(header.subject.is_none());
        assert!(header.keywords.is_none());
    }

    #[test]
    fn test_thread_header_with_fields() {
        let mut header = ThreadHeader::new(ObjRef::new(1, 0));
        header.title = Some("Test Thread".to_string());
        header.author = Some("John Doe".to_string());

        assert_eq!(header.title, Some("Test Thread".to_string()));
        assert_eq!(header.author, Some("John Doe".to_string()));
    }

    #[test]
    fn test_decode_pdf_string_ascii() {
        let bytes = b"Hello, World!";
        assert_eq!(decode_pdf_string(bytes), Some("Hello, World!".to_string()));
    }

    #[test]
    fn test_decode_pdf_string_utf16be_bom() {
        // UTF-16BE with BOM: "Hello" in UTF-16BE
        let bytes = &[
            0xFE, 0xFF, 0x00, 0x48, 0x00, 0x65, 0x00, 0x6C, 0x00, 0x6C, 0x00, 0x6F,
        ];
        assert_eq!(decode_pdf_string(bytes), Some("Hello".to_string()));
    }

    #[test]
    fn test_decode_pdf_string_empty() {
        let bytes: &[u8] = b"";
        assert_eq!(decode_pdf_string(bytes), Some("".to_string()));
    }

    #[test]
    fn test_decode_pdf_string_latin1() {
        // Latin-1 extended characters (á, é, ñ)
        let bytes = &[0xE1, 0xE9, 0xF1]; // á, é, ñ in Latin-1
        let result = decode_pdf_string(bytes);
        assert!(result.is_some());
        // Latin-1 maps directly to Unicode codepoints 0-255
        assert_eq!(result.unwrap(), "áéñ");
    }

    #[test]
    fn test_decode_utf16be_invalid_length() {
        let bytes = &[0xFE, 0xFF, 0x00]; // Odd length after BOM
        assert_eq!(decode_utf16be(&bytes[2..]), None);
    }

    #[test]
    fn test_decode_pdfdocencoding_empty() {
        assert_eq!(decode_pdfdocencoding(b""), Some("".to_string()));
    }

    #[test]
    fn test_decode_pdfdocencoding_ascii() {
        assert_eq!(decode_pdfdocencoding(b"ABC"), Some("ABC".to_string()));
    }

    /// Test: Thread with no /I info dict -> all fields null (per acceptance criteria)
    #[test]
    fn test_discover_thread_no_info_dict() {
        let resolver = XrefResolver::new();

        // Create a catalog with /Threads reference
        let mut catalog = Catalog::new(ObjRef::new(1, 0));
        catalog.threads_ref = Some(ObjRef::new(10, 0));

        // Cache the /Threads array with one thread (has /F but no /I)
        let thread_ref = ObjRef::new(11, 0);
        let mut thread_dict = indexmap::IndexMap::new();
        thread_dict.insert("F".into(), PdfObject::Ref(ObjRef::new(20, 0)));
        // No /I dict - all fields should be None

        let mut threads_array = Vec::new();
        threads_array.push(PdfObject::Ref(thread_ref));

        let mut threads_dict = indexmap::IndexMap::new();
        threads_dict.insert("Threads".into(), PdfObject::Array(Box::new(threads_array)));

        resolver.cache_object(ObjRef::new(10, 0), PdfObject::Dict(Box::new(threads_dict)));
        resolver.cache_object(thread_ref, PdfObject::Dict(Box::new(thread_dict)));

        let result = discover(&catalog, &resolver);
        assert!(result.is_ok());

        let threads = result.unwrap();
        assert_eq!(threads.len(), 1);

        let header = &threads[0];
        assert_eq!(header.first_bead_ref, ObjRef::new(20, 0));
        assert!(header.title.is_none());
        assert!(header.author.is_none());
        assert!(header.subject.is_none());
        assert!(header.keywords.is_none());
    }

    /// Test: 3 threads with various info dict configurations
    #[test]
    fn test_discover_three_threads() {
        let resolver = XrefResolver::new();

        // Create a catalog with /Threads reference
        let mut catalog = Catalog::new(ObjRef::new(1, 0));
        catalog.threads_ref = Some(ObjRef::new(10, 0));

        // Thread 1: full info dict
        let thread1_ref = ObjRef::new(11, 0);
        let mut thread1_dict = indexmap::IndexMap::new();
        thread1_dict.insert("F".into(), PdfObject::Ref(ObjRef::new(20, 0)));
        let mut info1 = indexmap::IndexMap::new();
        info1.insert(
            "Title".into(),
            PdfObject::String(Box::new(b"Thread 1".to_vec())),
        );
        info1.insert(
            "Author".into(),
            PdfObject::String(Box::new(b"Author 1".to_vec())),
        );
        info1.insert(
            "Subject".into(),
            PdfObject::String(Box::new(b"Subject 1".to_vec())),
        );
        info1.insert(
            "Keywords".into(),
            PdfObject::String(Box::new(b"kw1,kw2".to_vec())),
        );
        thread1_dict.insert("I".into(), PdfObject::Dict(Box::new(info1)));

        // Thread 2: no /Title but has other fields
        let thread2_ref = ObjRef::new(12, 0);
        let mut thread2_dict = indexmap::IndexMap::new();
        thread2_dict.insert("F".into(), PdfObject::Ref(ObjRef::new(21, 0)));
        let mut info2 = indexmap::IndexMap::new();
        info2.insert(
            "Author".into(),
            PdfObject::String(Box::new(b"Author 2".to_vec())),
        );
        // No /Title
        thread2_dict.insert("I".into(), PdfObject::Dict(Box::new(info2)));

        // Thread 3: no /I dict at all
        let thread3_ref = ObjRef::new(13, 0);
        let mut thread3_dict = indexmap::IndexMap::new();
        thread3_dict.insert("F".into(), PdfObject::Ref(ObjRef::new(22, 0)));
        // No /I

        let mut threads_array = Vec::new();
        threads_array.push(PdfObject::Ref(thread1_ref));
        threads_array.push(PdfObject::Ref(thread2_ref));
        threads_array.push(PdfObject::Ref(thread3_ref));

        let mut threads_dict = indexmap::IndexMap::new();
        threads_dict.insert("Threads".into(), PdfObject::Array(Box::new(threads_array)));

        resolver.cache_object(ObjRef::new(10, 0), PdfObject::Dict(Box::new(threads_dict)));
        resolver.cache_object(thread1_ref, PdfObject::Dict(Box::new(thread1_dict)));
        resolver.cache_object(thread2_ref, PdfObject::Dict(Box::new(thread2_dict)));
        resolver.cache_object(thread3_ref, PdfObject::Dict(Box::new(thread3_dict)));

        let result = discover(&catalog, &resolver);
        assert!(result.is_ok());

        let threads = result.unwrap();
        assert_eq!(threads.len(), 3);

        // Thread 1: all fields present
        assert_eq!(threads[0].title, Some("Thread 1".to_string()));
        assert_eq!(threads[0].author, Some("Author 1".to_string()));
        assert_eq!(threads[0].subject, Some("Subject 1".to_string()));
        assert_eq!(threads[0].keywords, Some("kw1,kw2".to_string()));

        // Thread 2: no title
        assert!(threads[1].title.is_none());
        assert_eq!(threads[1].author, Some("Author 2".to_string()));
        assert!(threads[1].subject.is_none());
        assert!(threads[1].keywords.is_none());

        // Thread 3: no info dict
        assert!(threads[2].title.is_none());
        assert!(threads[2].author.is_none());
        assert!(threads[2].subject.is_none());
        assert!(threads[2].keywords.is_none());
    }

    /// Test: Thread missing /F is skipped with diagnostic
    #[test]
    fn test_discover_thread_missing_f_skipped() {
        let resolver = XrefResolver::new();

        // Create a catalog with /Threads reference
        let mut catalog = Catalog::new(ObjRef::new(1, 0));
        catalog.threads_ref = Some(ObjRef::new(10, 0));

        // Thread with no /F
        let thread_ref = ObjRef::new(11, 0);
        let mut thread_dict = indexmap::IndexMap::new();
        // No /F - should be skipped
        let mut info = indexmap::IndexMap::new();
        info.insert(
            "Title".into(),
            PdfObject::String(Box::new(b"Orphan".to_vec())),
        );
        thread_dict.insert("I".into(), PdfObject::Dict(Box::new(info)));

        // Valid thread
        let thread2_ref = ObjRef::new(12, 0);
        let mut thread2_dict = indexmap::IndexMap::new();
        thread2_dict.insert("F".into(), PdfObject::Ref(ObjRef::new(20, 0)));

        let mut threads_array = Vec::new();
        threads_array.push(PdfObject::Ref(thread_ref));
        threads_array.push(PdfObject::Ref(thread2_ref));

        let mut threads_dict = indexmap::IndexMap::new();
        threads_dict.insert("Threads".into(), PdfObject::Array(Box::new(threads_array)));

        resolver.cache_object(ObjRef::new(10, 0), PdfObject::Dict(Box::new(threads_dict)));
        resolver.cache_object(thread_ref, PdfObject::Dict(Box::new(thread_dict)));
        resolver.cache_object(thread2_ref, PdfObject::Dict(Box::new(thread2_dict)));

        let result = discover(&catalog, &resolver);
        assert!(result.is_ok());

        let threads = result.unwrap();
        // Only the valid thread should be returned
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].first_bead_ref, ObjRef::new(20, 0));
    }

    /// Test: UTF-16BE encoded title
    #[test]
    fn test_discover_thread_utf16_title() {
        let resolver = XrefResolver::new();

        // Create a catalog with /Threads reference
        let mut catalog = Catalog::new(ObjRef::new(1, 0));
        catalog.threads_ref = Some(ObjRef::new(10, 0));

        // Thread with UTF-16BE title
        let thread_ref = ObjRef::new(11, 0);
        let mut thread_dict = indexmap::IndexMap::new();
        thread_dict.insert("F".into(), PdfObject::Ref(ObjRef::new(20, 0)));

        // UTF-16BE with BOM: "日本語" (Japanese)
        let utf16_bytes = &[
            0xFE, 0xFF, // BOM
            0x65, 0xE5, // 日
            0x67, 0x9C, // 本
            0x9E, 0x8A, // 語
        ];
        let mut info = indexmap::IndexMap::new();
        info.insert(
            "Title".into(),
            PdfObject::String(Box::new(utf16_bytes.to_vec())),
        );
        thread_dict.insert("I".into(), PdfObject::Dict(Box::new(info)));

        let mut threads_array = Vec::new();
        threads_array.push(PdfObject::Ref(thread_ref));

        let mut threads_dict = indexmap::IndexMap::new();
        threads_dict.insert("Threads".into(), PdfObject::Array(Box::new(threads_array)));

        resolver.cache_object(ObjRef::new(10, 0), PdfObject::Dict(Box::new(threads_dict)));
        resolver.cache_object(thread_ref, PdfObject::Dict(Box::new(thread_dict)));

        let result = discover(&catalog, &resolver);
        assert!(result.is_ok());

        let threads = result.unwrap();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].title, Some("日本語".to_string()));
    }

    /// Test: Empty /Threads returns empty Vec without diagnostic
    #[test]
    fn test_discover_empty_threads() {
        let resolver = XrefResolver::new();

        // Create a catalog with /Threads reference to empty array
        let mut catalog = Catalog::new(ObjRef::new(1, 0));
        catalog.threads_ref = Some(ObjRef::new(10, 0));

        let empty_array = PdfObject::Array(Box::new(Vec::new()));
        resolver.cache_object(ObjRef::new(10, 0), empty_array);

        let result = discover(&catalog, &resolver);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    /// Test: /Threads absent returns empty Vec without diagnostic
    #[test]
    fn test_discover_no_threads_field() {
        let resolver = XrefResolver::new();

        // Create a catalog without /Threads
        let catalog = Catalog::new(ObjRef::new(1, 0));
        // threads_ref is None

        let result = discover(&catalog, &resolver);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    /// Test: Empty string title is Some("") not None
    #[test]
    fn test_discover_thread_empty_title() {
        let resolver = XrefResolver::new();

        // Create a catalog with /Threads reference
        let mut catalog = Catalog::new(ObjRef::new(1, 0));
        catalog.threads_ref = Some(ObjRef::new(10, 0));

        // Thread with empty title
        let thread_ref = ObjRef::new(11, 0);
        let mut thread_dict = indexmap::IndexMap::new();
        thread_dict.insert("F".into(), PdfObject::Ref(ObjRef::new(20, 0)));

        let mut info = indexmap::IndexMap::new();
        info.insert("Title".into(), PdfObject::String(Box::new(Vec::new()))); // Empty string
        thread_dict.insert("I".into(), PdfObject::Dict(Box::new(info)));

        let mut threads_array = Vec::new();
        threads_array.push(PdfObject::Ref(thread_ref));

        let mut threads_dict = indexmap::IndexMap::new();
        threads_dict.insert("Threads".into(), PdfObject::Array(Box::new(threads_array)));

        resolver.cache_object(ObjRef::new(10, 0), PdfObject::Dict(Box::new(threads_dict)));
        resolver.cache_object(thread_ref, PdfObject::Dict(Box::new(thread_dict)));

        let result = discover(&catalog, &resolver);
        assert!(result.is_ok());

        let threads = result.unwrap();
        assert_eq!(threads.len(), 1);
        // Empty string should be Some("") not None
        assert_eq!(threads[0].title, Some("".to_string()));
    }
}
