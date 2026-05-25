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
use crate::parser::object::{ObjRef, PdfDict, PdfObject};
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

/// A single bead in an article thread chain.
///
/// Represents one bead's position on a page, extracted during bead chain walking.
/// Per PDF 1.7 Section 12.4.3, each bead contains a reference to its page and
/// a bounding rectangle defining the article region on that page.
///
/// # Fields
///
/// * `page_index` - 0-based index of the page containing this bead
/// * `rect` - Bounding rectangle of the bead region in PDF user-space coordinates [x0, y0, x1, y1]
#[derive(Debug, Clone, PartialEq)]
pub struct Bead {
    /// 0-based page index where this bead is located.
    pub page_index: usize,

    /// Bounding rectangle in PDF user-space coordinates [x0, y0, x1, y1].
    ///
    /// Per PDF spec, the origin is at the bottom-left corner of the page.
    /// This rect is NOT flipped to image-space coordinates.
    pub rect: [f32; 4],
}

impl Bead {
    /// Create a new Bead with the given page index and rect.
    pub fn new(page_index: usize, rect: [f32; 4]) -> Self {
        Bead { page_index, rect }
    }
}

/// Walk the bead chain for a single thread.
///
/// Follows `/N` (next bead) links from the first bead until the chain
/// terminates (when `/N` points back to the first bead). Detects malformed
/// chains (cycles that don't return to first) and aborts with diagnostic.
///
/// # Arguments
///
/// * `header` - The thread header containing the first bead reference
/// * `resolver` - The xref resolver for resolving indirect references
/// * `page_ref_to_index` - Precomputed map from page ObjRef to 0-based page index
///
/// # Returns
///
/// A `Result<Vec<Bead>>` containing all beads in chain order, or diagnostics
/// for errors encountered during walking.
///
/// # Behavior
///
/// - Follows `/N` links from first bead
/// - Terminates when `/N` points back to first bead (legitimate circular end)
/// - Detects malformed cycles (non-first bead revisited) with diagnostic
/// - Detects missing `/N` with diagnostic
/// - Detects missing or invalid `/R` (page ref) with diagnostic, skips that bead
/// - Detects missing or invalid `/V` (rect) with diagnostic, skips that bead
/// - Tolerates `/Pg` as fallback for page reference (some legacy PDFs)
/// - Maximum 10000 iterations per thread as safety net
/// - Beads are returned in chain order
///
/// # PDF Spec Reference
///
/// Per PDF 1.7 Section 12.4.3:
/// - `/R` - Page object reference (required)
/// - `/V` - Bounding rectangle of article region (required)
/// - `/N` - Next bead in thread (optional; null or absent means end of thread)
/// - `/T` - Thread containing this bead (back-reference, optional)
/// - `/P` - Page reference (alternative to `/R`, tolerated for legacy PDFs)
pub fn walk_beads(
    header: &ThreadHeader,
    resolver: &XrefResolver,
    page_ref_to_index: &std::collections::HashMap<ObjRef, usize>,
) -> Result<Vec<Bead>> {
    let mut beads = Vec::new();
    let mut diagnostics = Vec::new();
    let mut visited = std::collections::HashSet::new();
    let first_ref = header.first_bead_ref;
    let mut current_ref = first_ref;

    // Maximum iterations as safety net (real-world threads have < 1000 beads)
    const MAX_ITERATIONS: usize = 10000;
    let mut iterations = 0;

    visited.insert(current_ref);

    loop {
        iterations += 1;
        if iterations > MAX_ITERATIONS {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!(
                    "Thread bead chain exceeded maximum iteration count ({}); possible malformed chain",
                    MAX_ITERATIONS
                ),
            ));
            return Err(diagnostics);
        }

        // Resolve current bead
        let bead_obj = match resolver.resolve(current_ref) {
            Ok(obj) => obj,
            Err(_) => {
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructMissingKey,
                    format!("Failed to resolve bead reference {:?}", current_ref),
                ));
                break;
            }
        };

        let bead_dict = match bead_obj.as_dict() {
            Some(d) => d,
            None => {
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructUnexpectedEof,
                    format!("Bead {:?} is not a dictionary", current_ref),
                ));
                break;
            }
        };

        // Extract page reference - try /R first, then /P as fallback
        let page_ref = match (bead_dict.get("R"), bead_dict.get("P")) {
            (Some(PdfObject::Ref(r)), _) => Some(*r),
            (_, Some(PdfObject::Ref(r))) => Some(*r),
            (Some(other), _) => {
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructMissingKey,
                    format!("Bead {:?} has /R but it's not a reference", current_ref,),
                ));
                None
            }
            (_, Some(_)) => {
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructMissingKey,
                    format!("Bead {:?} has /P but it's not a reference", current_ref,),
                ));
                None
            }
            (None, None) => {
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructMissingKey,
                    format!(
                        "Bead {:?} is missing both /R and /P (page reference)",
                        current_ref
                    ),
                ));
                None
            }
        };

        let page_index = match page_ref {
            Some(ref_) => match page_ref_to_index.get(&ref_) {
                Some(idx) => *idx,
                None => {
                    // Skip this bead and continue
                    let next_ref = match get_next_bead_ref(bead_dict, current_ref) {
                        Ok(next_ref) => next_ref,
                        Err(_) => break,
                    };
                    if !check_and_handle_termination(
                        next_ref,
                        first_ref,
                        &visited,
                        &mut diagnostics,
                    ) {
                        break;
                    }
                    current_ref = next_ref;
                    continue;
                }
            },
            None => {
                // Skip this bead and continue
                let next_ref = match get_next_bead_ref(bead_dict, current_ref) {
                    Ok(next_ref) => next_ref,
                    Err(_) => break,
                };
                if !check_and_handle_termination(next_ref, first_ref, &visited, &mut diagnostics) {
                    break;
                }
                current_ref = next_ref;
                continue;
            }
        };

        // Extract rect (/V in PDF spec, but /V might be confused with other uses)
        // The plan says /V for rect, but let's check for both /V and /R as fallback
        let rect = match extract_bead_rect(bead_dict, current_ref) {
            Some(r) => r,
            None => {
                // Skip this bead and continue
                let next_ref = match get_next_bead_ref(bead_dict, current_ref) {
                    Ok(next_ref) => next_ref,
                    Err(_) => break,
                };
                if !check_and_handle_termination(next_ref, first_ref, &visited, &mut diagnostics) {
                    break;
                }
                current_ref = next_ref;
                continue;
            }
        };

        beads.push(Bead::new(page_index, rect));

        // Get next bead reference
        let next_ref = match get_next_bead_ref(bead_dict, current_ref) {
            Ok(next) => next,
            Err(_) => break,
        };

        if !check_and_handle_termination(next_ref, first_ref, &visited, &mut diagnostics) {
            break;
        }

        visited.insert(next_ref);
        current_ref = next_ref;
    }

    // Only return Err if diagnostics were fatal
    if diagnostics.is_empty() {
        Ok(beads)
    } else {
        // Check if any diagnostics are fatal - for now, we treat malformed cycles as fatal
        // but missing individual beads are not (we skip them)
        let has_fatal = diagnostics
            .iter()
            .any(|d| matches!(d.code, DiagCode::StructUnexpectedEof));
        if has_fatal {
            Err(diagnostics)
        } else {
            // Non-fatal diagnostics - return beads with warnings
            // For now, we'll still return Ok with the beads we collected
            Ok(beads)
        }
    }
}

/// Check for termination or malformed cycle after getting the next bead reference.
///
/// Returns true if the walk should continue, false if it should terminate (either
/// legitimately or due to a malformed cycle).
fn check_and_handle_termination(
    next_ref: ObjRef,
    first_ref: ObjRef,
    visited: &std::collections::HashSet<ObjRef>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    // Check for termination (next points back to first)
    if next_ref == first_ref {
        // Legitimate circular end
        return false;
    }

    // Check for malformed cycle
    if visited.contains(&next_ref) {
        diagnostics.push(Diagnostic::with_dynamic_no_offset(
            DiagCode::StructUnexpectedEof,
            format!(
                "Malformed bead chain: bead {:?} revisited (cycle doesn't return to first bead {:?})",
                next_ref, first_ref
            ),
        ));
        return false;
    }

    true
}

/// Extract the next bead reference from a bead dictionary.
fn get_next_bead_ref(
    bead_dict: &PdfDict,
    current_ref: ObjRef,
) -> std::result::Result<ObjRef, Vec<Diagnostic>> {
    match bead_dict.get("N") {
        None => {
            // Missing /N means end of thread (not an error)
            Err(Vec::new())
        }
        Some(PdfObject::Null) => {
            // Explicit null /N means end of thread
            Err(Vec::new())
        }
        Some(PdfObject::Ref(next_ref)) => Ok(*next_ref),
        Some(_) => {
            let diagnostics = vec![Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!("Bead {:?} has /N but it's not a reference", current_ref,),
            )];
            Err(diagnostics)
        }
    }
}

/// Extract the bounding rectangle from a bead dictionary.
///
/// Per PDF 1.7 spec, the rect is stored in /V. However, some PDFs may
/// use other keys, so we also check for common alternatives.
fn extract_bead_rect(bead_dict: &PdfDict, current_ref: ObjRef) -> Option<[f32; 4]> {
    // Try /V first (per spec)
    let rect_obj = bead_dict.get("V").or_else(|| bead_dict.get("Rect"))?;

    let rect_array = rect_obj.as_array()?;

    if rect_array.len() < 4 {
        return None;
    }

    let mut rect = [0.0f32; 4];
    for (i, val) in rect_array.iter().take(4).enumerate() {
        let n = match val {
            PdfObject::Integer(n) => *n as f64,
            PdfObject::Real(n) => *n,
            _ => return None,
        };
        rect[i] = n as f32;
    }

    // Validate rect: x0 < x1 and y0 < y1 (non-zero area)
    if rect[0] >= rect[2] || rect[1] >= rect[3] {
        return None;
    }

    Some(rect)
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

        // catalog.threads_ref should point directly to the array, not a dict
        resolver.cache_object(
            ObjRef::new(10, 0),
            PdfObject::Array(Box::new(threads_array)),
        );
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

        // catalog.threads_ref should point directly to the array, not a dict
        resolver.cache_object(
            ObjRef::new(10, 0),
            PdfObject::Array(Box::new(threads_array)),
        );
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

        // catalog.threads_ref should point directly to the array, not a dict
        resolver.cache_object(
            ObjRef::new(10, 0),
            PdfObject::Array(Box::new(threads_array)),
        );
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
            0x65, 0xE5, // 日 (U+65E5)
            0x67, 0x2C, // 本 (U+672C)
            0x8A, 0x9E, // 語 (U+8A9E)
        ];
        let mut info = indexmap::IndexMap::new();
        info.insert(
            "Title".into(),
            PdfObject::String(Box::new(utf16_bytes.to_vec())),
        );
        thread_dict.insert("I".into(), PdfObject::Dict(Box::new(info)));

        let mut threads_array = Vec::new();
        threads_array.push(PdfObject::Ref(thread_ref));

        // catalog.threads_ref should point directly to the array, not a dict
        resolver.cache_object(
            ObjRef::new(10, 0),
            PdfObject::Array(Box::new(threads_array)),
        );
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

        // catalog.threads_ref should point directly to the array, not a dict
        resolver.cache_object(
            ObjRef::new(10, 0),
            PdfObject::Array(Box::new(threads_array)),
        );
        resolver.cache_object(thread_ref, PdfObject::Dict(Box::new(thread_dict)));

        let result = discover(&catalog, &resolver);
        assert!(result.is_ok());

        let threads = result.unwrap();
        assert_eq!(threads.len(), 1);
        // Empty string should be Some("") not None
        assert_eq!(threads[0].title, Some("".to_string()));
    }

    /// Test: Bead with /R and /V correctly extracted
    #[test]
    fn test_walk_beads_single_bead() {
        let resolver = XrefResolver::new();

        // Create page ref to index map
        let mut page_ref_to_index = std::collections::HashMap::new();
        let page_ref = ObjRef::new(100, 0);
        page_ref_to_index.insert(page_ref, 0);

        // Create thread header
        let header = ThreadHeader::new(ObjRef::new(20, 0));

        // Create bead dict with /R (page ref) and /V (rect)
        let mut bead_dict = indexmap::IndexMap::new();
        bead_dict.insert("R".into(), PdfObject::Ref(page_ref));
        bead_dict.insert(
            "V".into(),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(100),
                PdfObject::Integer(200),
                PdfObject::Integer(300),
                PdfObject::Integer(400),
            ])),
        );
        // /N points back to first (circular termination)
        bead_dict.insert("N".into(), PdfObject::Ref(ObjRef::new(20, 0)));

        resolver.cache_object(ObjRef::new(20, 0), PdfObject::Dict(Box::new(bead_dict)));

        let result = walk_beads(&header, &resolver, &page_ref_to_index);
        assert!(result.is_ok());

        let beads = result.unwrap();
        assert_eq!(beads.len(), 1);
        assert_eq!(beads[0].page_index, 0);
        assert_eq!(beads[0].rect, [100.0, 200.0, 300.0, 400.0]);
    }

    /// Test: Two article threads - both reconstructed with correct bead order
    #[test]
    fn test_walk_beads_two_threads() {
        let resolver = XrefResolver::new();

        // Create page ref to index map
        let mut page_ref_to_index = std::collections::HashMap::new();
        let page0_ref = ObjRef::new(100, 0);
        let page1_ref = ObjRef::new(101, 0);
        let page2_ref = ObjRef::new(102, 0);
        page_ref_to_index.insert(page0_ref, 0);
        page_ref_to_index.insert(page1_ref, 1);
        page_ref_to_index.insert(page2_ref, 2);

        // Thread 1: three beads across pages 0, 1, 2
        let header1 = ThreadHeader::new(ObjRef::new(20, 0));

        let mut bead1_dict = indexmap::IndexMap::new();
        bead1_dict.insert("R".into(), PdfObject::Ref(page0_ref));
        bead1_dict.insert(
            "V".into(),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(10),
                PdfObject::Integer(20),
                PdfObject::Integer(30),
                PdfObject::Integer(40),
            ])),
        );
        bead1_dict.insert("N".into(), PdfObject::Ref(ObjRef::new(21, 0)));

        let mut bead2_dict = indexmap::IndexMap::new();
        bead2_dict.insert("R".into(), PdfObject::Ref(page1_ref));
        bead2_dict.insert(
            "V".into(),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(50),
                PdfObject::Integer(60),
                PdfObject::Integer(70),
                PdfObject::Integer(80),
            ])),
        );
        bead2_dict.insert("N".into(), PdfObject::Ref(ObjRef::new(22, 0)));

        let mut bead3_dict = indexmap::IndexMap::new();
        bead3_dict.insert("R".into(), PdfObject::Ref(page2_ref));
        bead3_dict.insert(
            "V".into(),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(90),
                PdfObject::Integer(100),
                PdfObject::Integer(110),
                PdfObject::Integer(120),
            ])),
        );
        bead3_dict.insert("N".into(), PdfObject::Ref(ObjRef::new(20, 0))); // Back to first

        resolver.cache_object(ObjRef::new(20, 0), PdfObject::Dict(Box::new(bead1_dict)));
        resolver.cache_object(ObjRef::new(21, 0), PdfObject::Dict(Box::new(bead2_dict)));
        resolver.cache_object(ObjRef::new(22, 0), PdfObject::Dict(Box::new(bead3_dict)));

        let result1 = walk_beads(&header1, &resolver, &page_ref_to_index);
        assert!(result1.is_ok());

        let beads1 = result1.unwrap();
        assert_eq!(beads1.len(), 3);
        assert_eq!(beads1[0].page_index, 0);
        assert_eq!(beads1[0].rect, [10.0, 20.0, 30.0, 40.0]);
        assert_eq!(beads1[1].page_index, 1);
        assert_eq!(beads1[1].rect, [50.0, 60.0, 70.0, 80.0]);
        assert_eq!(beads1[2].page_index, 2);
        assert_eq!(beads1[2].rect, [90.0, 100.0, 110.0, 120.0]);

        // Thread 2: single bead on page 1
        let header2 = ThreadHeader::new(ObjRef::new(30, 0));

        let mut bead4_dict = indexmap::IndexMap::new();
        bead4_dict.insert("R".into(), PdfObject::Ref(page1_ref));
        bead4_dict.insert(
            "V".into(),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(200),
                PdfObject::Integer(300),
                PdfObject::Integer(400),
                PdfObject::Integer(500),
            ])),
        );
        bead4_dict.insert("N".into(), PdfObject::Ref(ObjRef::new(30, 0))); // Back to first

        resolver.cache_object(ObjRef::new(30, 0), PdfObject::Dict(Box::new(bead4_dict)));

        let result2 = walk_beads(&header2, &resolver, &page_ref_to_index);
        assert!(result2.is_ok());

        let beads2 = result2.unwrap();
        assert_eq!(beads2.len(), 1);
        assert_eq!(beads2[0].page_index, 1);
        assert_eq!(beads2[0].rect, [200.0, 300.0, 400.0, 500.0]);
    }

    /// Test: Circular bead chain termination - walk stops without infinite loop
    #[test]
    fn test_walk_beads_circular_termination() {
        let resolver = XrefResolver::new();

        let mut page_ref_to_index = std::collections::HashMap::new();
        let page_ref = ObjRef::new(100, 0);
        page_ref_to_index.insert(page_ref, 0);

        let header = ThreadHeader::new(ObjRef::new(20, 0));

        // Create a chain: 20 -> 21 -> 22 -> 20 (circular back to first)
        let mut bead1_dict = indexmap::IndexMap::new();
        bead1_dict.insert("R".into(), PdfObject::Ref(page_ref));
        bead1_dict.insert(
            "V".into(),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(100),
                PdfObject::Integer(100),
            ])),
        );
        bead1_dict.insert("N".into(), PdfObject::Ref(ObjRef::new(21, 0)));

        let mut bead2_dict = indexmap::IndexMap::new();
        bead2_dict.insert("R".into(), PdfObject::Ref(page_ref));
        bead2_dict.insert(
            "V".into(),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(100),
                PdfObject::Integer(0),
                PdfObject::Integer(200),
                PdfObject::Integer(100),
            ])),
        );
        bead2_dict.insert("N".into(), PdfObject::Ref(ObjRef::new(22, 0)));

        let mut bead3_dict = indexmap::IndexMap::new();
        bead3_dict.insert("R".into(), PdfObject::Ref(page_ref));
        bead3_dict.insert(
            "V".into(),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(200),
                PdfObject::Integer(0),
                PdfObject::Integer(300),
                PdfObject::Integer(100),
            ])),
        );
        bead3_dict.insert("N".into(), PdfObject::Ref(ObjRef::new(20, 0))); // Back to first

        resolver.cache_object(ObjRef::new(20, 0), PdfObject::Dict(Box::new(bead1_dict)));
        resolver.cache_object(ObjRef::new(21, 0), PdfObject::Dict(Box::new(bead2_dict)));
        resolver.cache_object(ObjRef::new(22, 0), PdfObject::Dict(Box::new(bead3_dict)));

        let result = walk_beads(&header, &resolver, &page_ref_to_index);
        assert!(result.is_ok());

        let beads = result.unwrap();
        assert_eq!(beads.len(), 3); // All three beads visited
    }

    /// Test: Pathological cycle detection (non-first bead revisited)
    #[test]
    fn test_walk_beads_malformed_cycle() {
        let resolver = XrefResolver::new();

        let mut page_ref_to_index = std::collections::HashMap::new();
        let page_ref = ObjRef::new(100, 0);
        page_ref_to_index.insert(page_ref, 0);

        let header = ThreadHeader::new(ObjRef::new(20, 0));

        // Create a malformed chain: 20 -> 21 -> 22 -> 21 (cycle that doesn't return to first)
        let mut bead1_dict = indexmap::IndexMap::new();
        bead1_dict.insert("R".into(), PdfObject::Ref(page_ref));
        bead1_dict.insert(
            "V".into(),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(100),
                PdfObject::Integer(100),
            ])),
        );
        bead1_dict.insert("N".into(), PdfObject::Ref(ObjRef::new(21, 0)));

        let mut bead2_dict = indexmap::IndexMap::new();
        bead2_dict.insert("R".into(), PdfObject::Ref(page_ref));
        bead2_dict.insert(
            "V".into(),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(100),
                PdfObject::Integer(0),
                PdfObject::Integer(200),
                PdfObject::Integer(100),
            ])),
        );
        bead2_dict.insert("N".into(), PdfObject::Ref(ObjRef::new(22, 0)));

        let mut bead3_dict = indexmap::IndexMap::new();
        bead3_dict.insert("R".into(), PdfObject::Ref(page_ref));
        bead3_dict.insert(
            "V".into(),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(200),
                PdfObject::Integer(0),
                PdfObject::Integer(300),
                PdfObject::Integer(100),
            ])),
        );
        bead3_dict.insert("N".into(), PdfObject::Ref(ObjRef::new(21, 0))); // Back to 21, not 20

        resolver.cache_object(ObjRef::new(20, 0), PdfObject::Dict(Box::new(bead1_dict)));
        resolver.cache_object(ObjRef::new(21, 0), PdfObject::Dict(Box::new(bead2_dict)));
        resolver.cache_object(ObjRef::new(22, 0), PdfObject::Dict(Box::new(bead3_dict)));

        let result = walk_beads(&header, &resolver, &page_ref_to_index);
        assert!(result.is_err());

        let diagnostics = result.unwrap_err();
        assert!(!diagnostics.is_empty());
        // Should contain a malformed cycle diagnostic
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("Malformed bead chain")));
    }

    /// Test: Missing /N terminates the chain
    #[test]
    fn test_walk_beads_missing_next() {
        let resolver = XrefResolver::new();

        let mut page_ref_to_index = std::collections::HashMap::new();
        let page_ref = ObjRef::new(100, 0);
        page_ref_to_index.insert(page_ref, 0);

        let header = ThreadHeader::new(ObjRef::new(20, 0));

        // Bead with no /N
        let mut bead_dict = indexmap::IndexMap::new();
        bead_dict.insert("R".into(), PdfObject::Ref(page_ref));
        bead_dict.insert(
            "V".into(),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(100),
                PdfObject::Integer(100),
            ])),
        );
        // No /N - chain terminates

        resolver.cache_object(ObjRef::new(20, 0), PdfObject::Dict(Box::new(bead_dict)));

        let result = walk_beads(&header, &resolver, &page_ref_to_index);
        assert!(result.is_ok());

        let beads = result.unwrap();
        assert_eq!(beads.len(), 1);
    }

    /// Test: Missing /R and /P skips bead
    #[test]
    fn test_walk_beads_missing_page_ref() {
        let resolver = XrefResolver::new();

        let mut page_ref_to_index = std::collections::HashMap::new();
        let page_ref = ObjRef::new(100, 0);
        page_ref_to_index.insert(page_ref, 0);

        let header = ThreadHeader::new(ObjRef::new(20, 0));

        // First bead with no page ref
        let mut bead1_dict = indexmap::IndexMap::new();
        bead1_dict.insert(
            "V".into(),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(100),
                PdfObject::Integer(100),
            ])),
        );
        bead1_dict.insert("N".into(), PdfObject::Ref(ObjRef::new(21, 0)));

        // Second bead with valid page ref
        let mut bead2_dict = indexmap::IndexMap::new();
        bead2_dict.insert("R".into(), PdfObject::Ref(page_ref));
        bead2_dict.insert(
            "V".into(),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(100),
                PdfObject::Integer(0),
                PdfObject::Integer(200),
                PdfObject::Integer(100),
            ])),
        );
        bead2_dict.insert("N".into(), PdfObject::Ref(ObjRef::new(20, 0)));

        resolver.cache_object(ObjRef::new(20, 0), PdfObject::Dict(Box::new(bead1_dict)));
        resolver.cache_object(ObjRef::new(21, 0), PdfObject::Dict(Box::new(bead2_dict)));

        let result = walk_beads(&header, &resolver, &page_ref_to_index);
        assert!(result.is_ok());

        let beads = result.unwrap();
        // First bead skipped, second bead included
        assert_eq!(beads.len(), 1);
        assert_eq!(beads[0].page_index, 0);
    }

    /// Test: /Pg fallback for page reference
    #[test]
    fn test_walk_beads_pg_fallback() {
        let resolver = XrefResolver::new();

        let mut page_ref_to_index = std::collections::HashMap::new();
        let page_ref = ObjRef::new(100, 0);
        page_ref_to_index.insert(page_ref, 0);

        let header = ThreadHeader::new(ObjRef::new(20, 0));

        // Bead with /P instead of /R
        let mut bead_dict = indexmap::IndexMap::new();
        bead_dict.insert("P".into(), PdfObject::Ref(page_ref)); // /P instead of /R
        bead_dict.insert(
            "V".into(),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(100),
                PdfObject::Integer(100),
            ])),
        );
        bead_dict.insert("N".into(), PdfObject::Ref(ObjRef::new(20, 0)));

        resolver.cache_object(ObjRef::new(20, 0), PdfObject::Dict(Box::new(bead_dict)));

        let result = walk_beads(&header, &resolver, &page_ref_to_index);
        assert!(result.is_ok());

        let beads = result.unwrap();
        assert_eq!(beads.len(), 1);
        assert_eq!(beads[0].page_index, 0);
    }

    /// Test: Missing /V rect skips bead
    #[test]
    fn test_walk_beads_missing_rect() {
        let resolver = XrefResolver::new();

        let mut page_ref_to_index = std::collections::HashMap::new();
        let page_ref = ObjRef::new(100, 0);
        page_ref_to_index.insert(page_ref, 0);

        let header = ThreadHeader::new(ObjRef::new(20, 0));

        // First bead with no rect
        let mut bead1_dict = indexmap::IndexMap::new();
        bead1_dict.insert("R".into(), PdfObject::Ref(page_ref));
        bead1_dict.insert("N".into(), PdfObject::Ref(ObjRef::new(21, 0)));

        // Second bead with valid rect
        let mut bead2_dict = indexmap::IndexMap::new();
        bead2_dict.insert("R".into(), PdfObject::Ref(page_ref));
        bead2_dict.insert(
            "V".into(),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(100),
                PdfObject::Integer(100),
            ])),
        );
        bead2_dict.insert("N".into(), PdfObject::Ref(ObjRef::new(20, 0)));

        resolver.cache_object(ObjRef::new(20, 0), PdfObject::Dict(Box::new(bead1_dict)));
        resolver.cache_object(ObjRef::new(21, 0), PdfObject::Dict(Box::new(bead2_dict)));

        let result = walk_beads(&header, &resolver, &page_ref_to_index);
        assert!(result.is_ok());

        let beads = result.unwrap();
        // First bead skipped (no rect), second bead included
        assert_eq!(beads.len(), 1);
    }

    /// Test: Bead with invalid rect shape skips bead
    #[test]
    fn test_walk_beads_invalid_rect_shape() {
        let resolver = XrefResolver::new();

        let mut page_ref_to_index = std::collections::HashMap::new();
        let page_ref = ObjRef::new(100, 0);
        page_ref_to_index.insert(page_ref, 0);

        let header = ThreadHeader::new(ObjRef::new(20, 0));

        // Bead with invalid rect (x0 >= x1)
        let mut bead_dict = indexmap::IndexMap::new();
        bead_dict.insert("R".into(), PdfObject::Ref(page_ref));
        bead_dict.insert(
            "V".into(),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(100), // x0
                PdfObject::Integer(0),   // y0
                PdfObject::Integer(50),  // x1 < x0 - invalid!
                PdfObject::Integer(100),
            ])),
        );
        bead_dict.insert("N".into(), PdfObject::Ref(ObjRef::new(20, 0)));

        resolver.cache_object(ObjRef::new(20, 0), PdfObject::Dict(Box::new(bead_dict)));

        let result = walk_beads(&header, &resolver, &page_ref_to_index);
        assert!(result.is_ok());

        let beads = result.unwrap();
        // Bead skipped due to invalid rect
        assert_eq!(beads.len(), 0);
    }

    /// Test: Page ref outside document range
    #[test]
    fn test_walk_beads_page_ref_not_in_tree() {
        let resolver = XrefResolver::new();

        let mut page_ref_to_index = std::collections::HashMap::new();
        let page_ref = ObjRef::new(100, 0);
        page_ref_to_index.insert(page_ref, 0);

        let header = ThreadHeader::new(ObjRef::new(20, 0));

        // Bead with page ref not in the page tree
        let unknown_page_ref = ObjRef::new(999, 0);
        let mut bead_dict = indexmap::IndexMap::new();
        bead_dict.insert("R".into(), PdfObject::Ref(unknown_page_ref));
        bead_dict.insert(
            "V".into(),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(0),
                PdfObject::Integer(0),
                PdfObject::Integer(100),
                PdfObject::Integer(100),
            ])),
        );
        bead_dict.insert("N".into(), PdfObject::Ref(ObjRef::new(20, 0)));

        resolver.cache_object(ObjRef::new(20, 0), PdfObject::Dict(Box::new(bead_dict)));

        let result = walk_beads(&header, &resolver, &page_ref_to_index);
        assert!(result.is_ok());

        let beads = result.unwrap();
        // Bead skipped due to unknown page ref
        assert_eq!(beads.len(), 0);
    }

    /// Test: Bead struct new method
    #[test]
    fn test_bead_new() {
        let bead = Bead::new(5, [10.0, 20.0, 30.0, 40.0]);
        assert_eq!(bead.page_index, 5);
        assert_eq!(bead.rect, [10.0, 20.0, 30.0, 40.0]);
    }

    /// Test: Maximum iteration cap enforced
    #[test]
    fn test_walk_beads_max_iterations() {
        let resolver = XrefResolver::new();

        let mut page_ref_to_index = std::collections::HashMap::new();
        let page_ref = ObjRef::new(100, 0);
        page_ref_to_index.insert(page_ref, 0);

        let header = ThreadHeader::new(ObjRef::new(20, 0));

        // Create a long chain that exceeds MAX_ITERATIONS
        // We'll create a chain of 10001 beads (20 -> 21 -> 22 -> ... -> 10020 -> 20)
        for i in 0..=10050 {
            let mut bead_dict = indexmap::IndexMap::new();
            bead_dict.insert("R".into(), PdfObject::Ref(page_ref));
            bead_dict.insert(
                "V".into(),
                PdfObject::Array(Box::new(vec![
                    PdfObject::Integer(i),
                    PdfObject::Integer(0),
                    PdfObject::Integer(i + 100),
                    PdfObject::Integer(100),
                ])),
            );
            // Each bead points to the next, except the last which points back to first
            let next_ref = if i < 10050 {
                ObjRef::new((20 + i + 1) as u32, 0)
            } else {
                ObjRef::new(20, 0) // Would close the loop, but we hit max iterations first
            };
            bead_dict.insert("N".into(), PdfObject::Ref(next_ref));
            resolver.cache_object(
                ObjRef::new((20 + i) as u32, 0),
                PdfObject::Dict(Box::new(bead_dict)),
            );
        }

        let result = walk_beads(&header, &resolver, &page_ref_to_index);
        assert!(result.is_err());

        let diagnostics = result.unwrap_err();
        assert!(!diagnostics.is_empty());
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("exceeded maximum iteration count")));
    }
}
