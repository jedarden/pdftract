//! C FFI API for pdftract.
//!
//! This module provides the extern "C" API surface for C/C++ integrations.
//! All functions return owned JSON strings that must be freed with pdftract_free().
//! Panics are caught at the FFI boundary and converted to JSON errors.
//!
//! # Memory management
//!
//! - All functions except pdftract_version() return owned strings
//! - The caller MUST free these strings with pdftract_free()
//! - Do not call libc free() on these pointers (Rust allocator mismatch)
//!
//! # Error handling
//!
//! All errors are returned as JSON objects with the shape:
//! ```json
//! {"error":"CODE","message":"..."}
//! ```

use libc::{c_char, c_void};
use pdftract_core::extract::{extract_pdf, result_to_json};
use pdftract_core::options::ExtractionOptions;
use pdftract_core::document::{parse_pdf_file, compute_pdf_fingerprint, PdfExtractor};
use pdftract_core::receipts::{Receipt, verifier::{verify_receipt, SpanData, VerificationResult, exit_code}};
use std::ffi::{CString, CStr};
use std::panic::catch_unwind;
use std::path::Path;
use std::sync::Mutex;
use std::default::Default;

/// Error codes returned in JSON error responses.
mod error_codes {
    pub const NULL_POINTER: &str = "NULL_POINTER";
    pub const INVALID_UTF8: &str = "INVALID_UTF8";
    pub const INVALID_JSON: &str = "INVALID_JSON";
    pub const EXTRACTION_ERROR: &str = "EXTRACTION_ERROR";
    pub const FILE_NOT_FOUND: &str = "FILE_NOT_FOUND";
    pub const PARSE_ERROR: &str = "PARSE_ERROR";
    pub const PANIC: &str = "PANIC";
    pub const NOT_IMPLEMENTED: &str = "NOT_IMPLEMENTED";
    pub const INVALID_HANDLE: &str = "INVALID_HANDLE";
}

/// Convert an error to a JSON error string.
fn json_error(code: &str, message: &str) -> String {
    format!(r#"{{"error":"{}","message":"{}"}}"#, code, escape_json(message))
}

/// Escape a string for JSON (minimal escaping).
fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Convert an anyhow::Error to a JSON error string.
fn anyhow_to_json_error(err: anyhow::Error) -> String {
    let message = err.to_string();
    // Try to determine a more specific error code
    let code = if err.chain().any(|e| e.to_string().contains("No such file")) {
        error_codes::FILE_NOT_FOUND
    } else if err.chain().any(|e| e.to_string().contains("UTF-8")) {
        error_codes::INVALID_UTF8
    } else {
        error_codes::EXTRACTION_ERROR
    };
    json_error(code, &message)
}

/// Convert a C string pointer to a Rust string, handling null and invalid UTF-8.
unsafe fn cstr_to_string(ptr: *const c_char) -> Result<String, &'static str> {
    if ptr.is_null() {
        return Err("null pointer");
    }
    CStr::from_ptr(ptr)
        .to_str()
        .map(|s| s.to_string())
        .map_err(|_| error_codes::INVALID_UTF8)
}

/// Parse options JSON, returning an error string on failure.
fn parse_options_json(options_json: &str) -> Result<ExtractionOptions, String> {
    serde_json::from_str(options_json)
        .map_err(|e| format!("Invalid options JSON: {}", e))
}

/// Result type for FFI operations that can fail.
enum FfiResult {
    Ok(String),
    Err(String),
}

/// Extract text and structure from a PDF file.
///
/// # Arguments
///
/// * `source` - Path to the PDF file (null-terminated UTF-8 string)
/// * `options_json` - JSON string with extraction options (can be empty object "{}")
///
/// # Returns
///
/// A JSON string representing the extraction result. The caller MUST free this
/// with pdftract_free(). On error, returns a JSON object with "error" and "message" fields.
///
/// # Example
///
/// ```c
/// char *result = pdftract_extract("document.pdf", "{}");
/// // ... use result ...
/// pdftract_free(result);
/// ```
#[no_mangle]
pub extern "C" fn pdftract_extract(
    source: *const c_char,
    options_json: *const c_char,
) -> *mut c_char {
    let result = catch_unwind(|| unsafe {
        // Validate and convert arguments
        let source_path = match cstr_to_string(source) {
            Ok(s) => s,
            Err(_) => return FfiResult::Err(json_error(error_codes::NULL_POINTER, "source pointer is null")),
        };

        let options_str = match cstr_to_string(options_json) {
            Ok(s) => s,
            Err(_) => return FfiResult::Err(json_error(error_codes::NULL_POINTER, "options_json pointer is null")),
        };

        // Parse options
        let options: ExtractionOptions = match parse_options_json(&options_str) {
            Ok(opts) => opts,
            Err(e) => return FfiResult::Err(json_error(error_codes::INVALID_JSON, &e)),
        };

        // Perform extraction
        let pdf_path = Path::new(&source_path);
        let extraction_result = match extract_pdf(pdf_path, &options) {
            Ok(result) => result,
            Err(e) => return FfiResult::Err(anyhow_to_json_error(e)),
        };

        // Convert to JSON
        let json_value = result_to_json(&extraction_result);
        match serde_json::to_string(&json_value) {
            Ok(json) => FfiResult::Ok(json),
            Err(e) => FfiResult::Err(json_error(error_codes::EXTRACTION_ERROR, &format!("JSON serialization failed: {}", e))),
        }
    });

    match result {
        Ok(FfiResult::Ok(json)) => CString::new(json).unwrap().into_raw(),
        Ok(FfiResult::Err(err)) => CString::new(err).unwrap().into_raw(),
        Err(_) => CString::new(json_error(error_codes::PANIC, "panic in pdftract_extract")).unwrap().into_raw(),
    }
}

/// Extract plain text from a PDF file.
///
/// # Arguments
///
/// * `source` - Path to the PDF file (null-terminated UTF-8 string)
/// * `options_json` - JSON string with extraction options (can be empty object "{}")
///
/// # Returns
///
/// A JSON string containing the extracted text. The caller MUST free this
/// with pdftract_free().
#[no_mangle]
pub extern "C" fn pdftract_extract_text(
    source: *const c_char,
    options_json: *const c_char,
) -> *mut c_char {
    let result = catch_unwind(|| unsafe {
        let source_path = match cstr_to_string(source) {
            Ok(s) => s,
            Err(_) => return FfiResult::Err(json_error(error_codes::NULL_POINTER, "source pointer is null")),
        };

        let options_str = match cstr_to_string(options_json) {
            Ok(s) => s,
            Err(_) => return FfiResult::Err(json_error(error_codes::NULL_POINTER, "options_json pointer is null")),
        };

        let options: ExtractionOptions = match parse_options_json(&options_str) {
            Ok(opts) => opts,
            Err(e) => return FfiResult::Err(json_error(error_codes::INVALID_JSON, &e)),
        };

        let pdf_path = Path::new(&source_path);
        let extraction_result = match extract_pdf(pdf_path, &options) {
            Ok(result) => result,
            Err(e) => return FfiResult::Err(anyhow_to_json_error(e)),
        };

        // Extract just the text from all pages
        let text: String = extraction_result.pages
            .iter()
            .flat_map(|page| page.spans.iter().map(|span| span.text.as_str()))
            .collect::<Vec<_>>()
            .join(" ");

        match serde_json::to_string(&text) {
            Ok(json) => FfiResult::Ok(json),
            Err(e) => FfiResult::Err(json_error(error_codes::EXTRACTION_ERROR, &format!("JSON serialization failed: {}", e))),
        }
    });

    match result {
        Ok(FfiResult::Ok(json)) => CString::new(json).unwrap().into_raw(),
        Ok(FfiResult::Err(err)) => CString::new(err).unwrap().into_raw(),
        Err(_) => CString::new(json_error(error_codes::PANIC, "panic in pdftract_extract_text")).unwrap().into_raw(),
    }
}

/// Extract markdown from a PDF file.
///
/// # Arguments
///
/// * `source` - Path to the PDF file (null-terminated UTF-8 string)
/// * `options_json` - JSON string with extraction options (can be empty object "{}")
///
/// # Returns
///
/// A JSON string containing the extracted markdown. The caller MUST free this
/// with pdftract_free().
#[no_mangle]
pub extern "C" fn pdftract_extract_markdown(
    source: *const c_char,
    options_json: *const c_char,
) -> *mut c_char {
    let result = catch_unwind(|| unsafe {
        let source_path = match cstr_to_string(source) {
            Ok(s) => s,
            Err(_) => return FfiResult::Err(json_error(error_codes::NULL_POINTER, "source pointer is null")),
        };

        let options_str = match cstr_to_string(options_json) {
            Ok(s) => s,
            Err(_) => return FfiResult::Err(json_error(error_codes::NULL_POINTER, "options_json pointer is null")),
        };

        let options: ExtractionOptions = match parse_options_json(&options_str) {
            Ok(opts) => opts,
            Err(e) => return FfiResult::Err(json_error(error_codes::INVALID_JSON, &e)),
        };

        let pdf_path = Path::new(&source_path);
        let extraction_result = match extract_pdf(pdf_path, &options) {
            Ok(result) => result,
            Err(e) => return FfiResult::Err(anyhow_to_json_error(e)),
        };

        // Convert blocks to markdown
        let markdown: String = extraction_result.pages
            .iter()
            .flat_map(|page| page.blocks.iter())
            .map(|block| {
                match block.kind.as_str() {
                    "heading" => {
                        let level = block.level.unwrap_or(1);
                        let hashes = "#".repeat(level as usize);
                        format!("{} {}\n\n", hashes, block.text)
                    }
                    "paragraph" => format!("{}\n\n", block.text),
                    "list" => format!("- {}\n", block.text),
                    _ => format!("{}\n\n", block.text),
                }
            })
            .collect();

        match serde_json::to_string(&markdown) {
            Ok(json) => FfiResult::Ok(json),
            Err(e) => FfiResult::Err(json_error(error_codes::EXTRACTION_ERROR, &format!("JSON serialization failed: {}", e))),
        }
    });

    match result {
        Ok(FfiResult::Ok(json)) => CString::new(json).unwrap().into_raw(),
        Ok(FfiResult::Err(err)) => CString::new(err).unwrap().into_raw(),
        Err(_) => CString::new(json_error(error_codes::PANIC, "panic in pdftract_extract_markdown")).unwrap().into_raw(),
    }
}

/// Stream state for iterative page extraction.
///
/// This struct holds a PdfExtractor and extracts pages on-demand,
/// ensuring that we never materialize the entire document in memory.
struct StreamState {
    /// The PDF extractor for lazy page iteration
    extractor: PdfExtractor,
    /// Lazy page iterator (created on first call to next())
    page_iter: Option<pdftract_core::document::PageIter<'static>>,
    /// Current page index (for tracking progress)
    current_index: usize,
    /// Extraction options (cached for reuse)
    options: ExtractionOptions,
}

/// Open a streaming extraction session.
///
/// Returns an opaque handle that can be used with pdftract_stream_next()
/// to iterate through pages one at a time. When done, call pdftract_stream_close().
///
/// # Memory Efficiency
///
/// This function does NOT materialize all pages. It creates a PdfExtractor
/// that will extract each page on-demand when pdftract_stream_next() is called.
/// This ensures memory usage stays bounded regardless of document size.
///
/// # Arguments
///
/// * `source` - Path to the PDF file (null-terminated UTF-8 string)
/// * `options_json` - JSON string with extraction options (can be empty object "{}")
///
/// # Returns
///
/// An opaque handle (*mut c_void) on success, or NULL on error.
/// Check for errors by examining the handle.
#[no_mangle]
pub extern "C" fn pdftract_extract_stream_open(
    source: *const c_char,
    options_json: *const c_char,
) -> *mut c_void {
    clear_last_error();

    let result = catch_unwind(|| unsafe {
        let source_path = match cstr_to_string(source) {
            Ok(s) => s,
            Err(e) => {
                set_last_error(json_error(error_codes::NULL_POINTER, "source pointer is null"));
                return None;
            }
        };

        let options_str = match cstr_to_string(options_json) {
            Ok(s) => s,
            Err(e) => {
                set_last_error(json_error(error_codes::NULL_POINTER, "options_json pointer is null"));
                return None;
            }
        };

        let options: ExtractionOptions = match parse_options_json(&options_str) {
            Ok(opts) => opts,
            Err(e) => {
                set_last_error(json_error(error_codes::INVALID_JSON, &e));
                return None;
            }
        };

        let pdf_path = Path::new(&source_path);

        // Use PdfExtractor for lazy page iteration
        // This does NOT materialize all pages upfront
        let extractor = match PdfExtractor::open(pdf_path) {
            Ok(ex) => ex,
            Err(e) => {
                set_last_error(anyhow_to_json_error(e));
                return None;
            }
        };

        Some(StreamState {
            extractor,
            page_iter: None,
            current_index: 0,
            options,
        })
    });

    match result {
        Ok(Some(state)) => Box::into_raw(Box::new(state)) as *mut c_void,
        Ok(None) => std::ptr::null_mut(),
        Err(_) => {
            set_last_error(json_error(error_codes::PANIC, "panic in pdftract_extract_stream_open"));
            std::ptr::null_mut()
        }
    }
}

/// Get the next page from a streaming extraction session.
///
/// # Memory Efficiency
///
/// This function extracts one page at a time on-demand. The page's
/// content streams are decoded, the result is serialized to JSON,
/// and then all page data is dropped before returning. This ensures
/// memory usage stays bounded.
///
/// # Arguments
///
/// * `handle` - Opaque handle from pdftract_extract_stream_open()
///
/// # Returns
///
/// A JSON string representing one page, or NULL when the stream ends.
/// The caller MUST free non-NULL returns with pdftract_free().
///
/// # Note
///
/// The handle remains valid after this call and must be closed with
/// pdftract_stream_close() when done.
#[no_mangle]
pub extern "C" fn pdftract_stream_next(handle: *mut c_void) -> *mut c_char {
    if handle.is_null() {
        return CString::new(json_error(error_codes::INVALID_HANDLE, "null handle")).unwrap().into_raw();
    }

    let result = catch_unwind(|| -> Option<*mut c_char> {
        unsafe {
            // Get a mutable reference to the state
            let state = &mut *(handle as *mut StreamState);

            // Initialize the lazy iterator on first call
            if state.page_iter.is_none() {
                state.page_iter = Some(state.extractor.pages());
            }

            // Get the next page from the lazy iterator
            // This walks the page tree depth-first, materializing only the current path
            let iter = state.page_iter.as_mut()?;
            let page_extraction = match iter.next() {
                Some(Ok(page)) => page,
                Some(Err(e)) => {
                    // Return an error page instead of failing
                    let error_json = serde_json::json!({
                        "index": state.current_index,
                        "error": e.to_string(),
                        "spans": [],
                        "blocks": [],
                    });
                    state.current_index += 1;
                    return Some(CString::new(serde_json::to_string(&error_json).unwrap()).unwrap().into_raw());
                }
                None => {
                    // Stream ended - return null pointer
                    return None;
                }
            };

            // Convert to JSON
            let page_json = serde_json::json!({
                "index": page_extraction.index,
                "spans": page_extraction.spans,
                "blocks": page_extraction.blocks,
            });

            // Increment the index for the next call
            state.current_index += 1;

            // Serialize and return
            // The page_json is dropped after this call, freeing all page data
            Some(CString::new(serde_json::to_string(&page_json).unwrap()).unwrap().into_raw())
        }
    });

    match result {
        Ok(Some(ptr)) => ptr,
        Ok(None) => std::ptr::null_mut(),
        Err(_) => CString::new(json_error(error_codes::PANIC, "panic in pdftract_stream_next")).unwrap().into_raw(),
    }
}

/// Close a streaming extraction session and free resources.
///
/// # Arguments
///
/// * `handle` - Opaque handle from pdftract_extract_stream_open()
#[no_mangle]
pub extern "C" fn pdftract_stream_close(handle: *mut c_void) {
    if handle.is_null() {
        return;
    }

    let result = catch_unwind(|| unsafe {
        // Drop the Box<StreamState>
        let _ = Box::from_raw(handle as *mut StreamState);
    });

    // We can't report errors from a close function, so we just ignore panics
    let _ = result;
}

/// Search for text patterns in a PDF file.
///
/// # Arguments
///
/// * `source` - Path to the PDF file (null-terminated UTF-8 string)
/// * `pattern` - Search pattern (null-terminated UTF-8 string)
/// * `options_json` - JSON string with extraction options (can be empty object "{}")
///
/// # Returns
///
/// A JSON string containing search results. The caller MUST free this
/// with pdftract_free().
#[no_mangle]
pub extern "C" fn pdftract_search(
    source: *const c_char,
    pattern: *const c_char,
    options_json: *const c_char,
) -> *mut c_char {
    let result = catch_unwind(|| unsafe {
        let source_path = match cstr_to_string(source) {
            Ok(s) => s,
            Err(_) => return FfiResult::Err(json_error(error_codes::NULL_POINTER, "source pointer is null")),
        };

        let search_pattern = match cstr_to_string(pattern) {
            Ok(s) => s,
            Err(_) => return FfiResult::Err(json_error(error_codes::NULL_POINTER, "pattern pointer is null")),
        };

        let options_str = match cstr_to_string(options_json) {
            Ok(s) => s,
            Err(_) => return FfiResult::Err(json_error(error_codes::NULL_POINTER, "options_json pointer is null")),
        };

        let options: ExtractionOptions = match parse_options_json(&options_str) {
            Ok(opts) => opts,
            Err(e) => return FfiResult::Err(json_error(error_codes::INVALID_JSON, &e)),
        };

        let pdf_path = Path::new(&source_path);
        let extraction_result = match extract_pdf(pdf_path, &options) {
            Ok(result) => result,
            Err(e) => return FfiResult::Err(anyhow_to_json_error(e)),
        };

        // Search for the pattern in spans
        let mut matches = Vec::new();
        for page in &extraction_result.pages {
            for (span_idx, span) in page.spans.iter().enumerate() {
                if span.text.contains(&search_pattern) {
                    matches.push(serde_json::json!({
                        "page": page.index,
                        "span": span_idx,
                        "text": span.text,
                        "bbox": span.bbox,
                    }));
                }
            }
        }

        match serde_json::to_string(&serde_json::json!({
            "pattern": search_pattern,
            "match_count": matches.len(),
            "matches": matches,
        })) {
            Ok(json) => FfiResult::Ok(json),
            Err(e) => FfiResult::Err(json_error(error_codes::EXTRACTION_ERROR, &format!("JSON serialization failed: {}", e))),
        }
    });

    match result {
        Ok(FfiResult::Ok(json)) => CString::new(json).unwrap().into_raw(),
        Ok(FfiResult::Err(err)) => CString::new(err).unwrap().into_raw(),
        Err(_) => CString::new(json_error(error_codes::PANIC, "panic in pdftract_search")).unwrap().into_raw(),
    }
}

/// Get metadata about a PDF file.
///
/// # Arguments
///
/// * `source` - Path to the PDF file (null-terminated UTF-8 string)
/// * `options_json` - JSON string with extraction options (can be empty object "{}")
///
/// # Returns
///
/// A JSON string containing PDF metadata. The caller MUST free this
/// with pdftract_free().
#[no_mangle]
pub extern "C" fn pdftract_get_metadata(
    source: *const c_char,
    options_json: *const c_char,
) -> *mut c_char {
    let result = catch_unwind(|| unsafe {
        let source_path = match cstr_to_string(source) {
            Ok(s) => s,
            Err(_) => return FfiResult::Err(json_error(error_codes::NULL_POINTER, "source pointer is null")),
        };

        let options_str = match cstr_to_string(options_json) {
            Ok(s) => s,
            Err(_) => return FfiResult::Err(json_error(error_codes::NULL_POINTER, "options_json pointer is null")),
        };

        let options: ExtractionOptions = match parse_options_json(&options_str) {
            Ok(opts) => opts,
            Err(e) => return FfiResult::Err(json_error(error_codes::INVALID_JSON, &e)),
        };

        let pdf_path = Path::new(&source_path);
        let extraction_result = match extract_pdf(pdf_path, &options) {
            Ok(result) => result,
            Err(e) => return FfiResult::Err(anyhow_to_json_error(e)),
        };

        match serde_json::to_string(&serde_json::json!({
            "fingerprint": extraction_result.fingerprint,
            "page_count": extraction_result.metadata.page_count,
            "span_count": extraction_result.metadata.span_count,
            "block_count": extraction_result.metadata.block_count,
            "receipts_mode": extraction_result.metadata.receipts_mode.as_str(),
        })) {
            Ok(json) => FfiResult::Ok(json),
            Err(e) => FfiResult::Err(json_error(error_codes::EXTRACTION_ERROR, &format!("JSON serialization failed: {}", e))),
        }
    });

    match result {
        Ok(FfiResult::Ok(json)) => CString::new(json).unwrap().into_raw(),
        Ok(FfiResult::Err(err)) => CString::new(err).unwrap().into_raw(),
        Err(_) => CString::new(json_error(error_codes::PANIC, "panic in pdftract_get_metadata")).unwrap().into_raw(),
    }
}

/// Compute the cryptographic fingerprint of a PDF file.
///
/// # Arguments
///
/// * `source` - Path to the PDF file (null-terminated UTF-8 string)
///
/// # Returns
///
/// A JSON string containing the fingerprint. The caller MUST free this
/// with pdftract_free().
#[no_mangle]
pub extern "C" fn pdftract_hash(source: *const c_char) -> *mut c_char {
    let result = catch_unwind(|| unsafe {
        let source_path = match cstr_to_string(source) {
            Ok(s) => s,
            Err(_) => return FfiResult::Err(json_error(error_codes::NULL_POINTER, "source pointer is null")),
        };

        let pdf_path = Path::new(&source_path);
        let fingerprint = match compute_pdf_fingerprint(pdf_path) {
            Ok(fp) => fp,
            Err(e) => return FfiResult::Err(anyhow_to_json_error(e)),
        };

        match serde_json::to_string(&serde_json::json!({
            "fingerprint": fingerprint,
        })) {
            Ok(json) => FfiResult::Ok(json),
            Err(e) => FfiResult::Err(json_error(error_codes::EXTRACTION_ERROR, &format!("JSON serialization failed: {}", e))),
        }
    });

    match result {
        Ok(FfiResult::Ok(json)) => CString::new(json).unwrap().into_raw(),
        Ok(FfiResult::Err(err)) => CString::new(err).unwrap().into_raw(),
        Err(_) => CString::new(json_error(error_codes::PANIC, "panic in pdftract_hash")).unwrap().into_raw(),
    }
}

/// Classify a PDF file by type.
///
/// # Arguments
///
/// * `source` - Path to the PDF file (null-terminated UTF-8 string)
///
/// # Returns
///
/// A JSON string containing classification information. The caller MUST free this
/// with pdftract_free().
///
/// # Note
///
/// This is currently a stub that returns a basic classification.
/// Full implementation requires a trained classifier.
#[no_mangle]
pub extern "C" fn pdftract_classify(source: *const c_char) -> *mut c_char {
    let result = catch_unwind(|| unsafe {
        let source_path = match cstr_to_string(source) {
            Ok(s) => s,
            Err(_) => return FfiResult::Err(json_error(error_codes::NULL_POINTER, "source pointer is null")),
        };

        let pdf_path = Path::new(&source_path);

        // Get basic info
        let (fingerprint, _catalog, pages, _resolver) = match parse_pdf_file(pdf_path) {
            Ok(result) => result,
            Err(e) => return FfiResult::Err(anyhow_to_json_error(e)),
        };

        // Basic classification based on page count
        let doc_type = if pages.len() == 1 {
            "single_page"
        } else if pages.len() <= 5 {
            "short_document"
        } else {
            "long_document"
        };

        match serde_json::to_string(&serde_json::json!({
            "type": doc_type,
            "page_count": pages.len(),
            "fingerprint": fingerprint,
            "confidence": 0.5,
        })) {
            Ok(json) => FfiResult::Ok(json),
            Err(e) => FfiResult::Err(json_error(error_codes::EXTRACTION_ERROR, &format!("JSON serialization failed: {}", e))),
        }
    });

    match result {
        Ok(FfiResult::Ok(json)) => CString::new(json).unwrap().into_raw(),
        Ok(FfiResult::Err(err)) => CString::new(err).unwrap().into_raw(),
        Err(_) => CString::new(json_error(error_codes::PANIC, "panic in pdftract_classify")).unwrap().into_raw(),
    }
}

/// Free a string returned by pdftract_* functions.
///
/// # Arguments
///
/// * `ptr` - Pointer to string returned by any pdftract_* function (except pdftract_version)
///
/// # Safety
///
/// This function MUST be called to free strings returned by the API.
/// Do NOT call libc free() on these pointers.
#[no_mangle]
pub extern "C" fn pdftract_free(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}

/// Get the pdftract library version string.
///
/// # Returns
///
/// A static C string containing the version. Do NOT free this string.
#[no_mangle]
pub extern "C" fn pdftract_version() -> *const c_char {
    // Use a static C string with proper lifetime
    static VERSION: &[u8] = b"0.1.0\0";
    VERSION.as_ptr() as *const c_char
}

/// Thread-local storage for the last error message.
///
/// This allows C callers to retrieve detailed error information after
/// a function returns NULL or an error indicator. Each thread has its
/// own error storage, making the library thread-safe.
thread_local! {
    static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);
    static LAST_ERROR_CSTR: Mutex<Option<CString>> = Mutex::new(None);
}

/// Set the last error message for the current thread.
fn set_last_error(message: String) {
    LAST_ERROR.with(|error| {
        let mut guard = error.lock().unwrap();
        *guard = Some(message);
    });
}

/// Clear the last error message for the current thread.
fn clear_last_error() {
    LAST_ERROR.with(|error| {
        let mut guard = error.lock().unwrap();
        *guard = None;
    });
    LAST_ERROR_CSTR.with(|cstr| {
        let mut guard = cstr.lock().unwrap();
        *guard = None;
    });
}

/// Get the last error message for the current thread.
///
/// # Returns
///
/// A pointer to a null-terminated string containing the last error message,
/// or NULL if no error has been set. The caller MUST NOT free this string.
/// The string remains valid until the next API call on this thread.
///
/// # Note
///
/// This function returns a pointer to thread-local storage that is invalidated
/// by the next API call on the same thread. If you need to retain the error
/// message, make a copy of it immediately.
#[no_mangle]
pub extern "C" fn pdftract_last_error() -> *const c_char {
    LAST_ERROR_CSTR.with(|cstr| {
        let mut guard = cstr.lock().unwrap();
        if let Some(ref c) = *guard {
            return c.as_ptr();
        }

        // Try to get the error string and convert it to CString
        LAST_ERROR.with(|error| {
            let err_guard = error.lock().unwrap();
            if let Some(ref msg) = *err_guard {
                if let Ok(c) = CString::new(msg.as_str()) {
                    let ptr = c.as_ptr();
                    *guard = Some(c);
                    ptr
                } else {
                    std::ptr::null()
                }
            } else {
                std::ptr::null()
            }
        })
    })
}

/// Get the ABI version of the library.
///
/// # Returns
///
/// A 32-bit unsigned integer encoding the ABI version.
/// Format: MAJOR << 16 | MINOR << 8 | PATCH
///
/// For version 0.1.0, this returns 0x00000100 (256 decimal).
/// For version 1.2.3, this would return 0x010203 (66051 decimal).
///
/// C callers can use this to verify the loaded library matches their
/// compiled header's expectations.
#[no_mangle]
pub extern "C" fn pdftract_abi_version() -> u32 {
    const MAJOR: u8 = 0;
    const MINOR: u8 = 1;
    const PATCH: u8 = 0;

    (MAJOR as u32) << 16 | (MINOR as u32) << 8 | (PATCH as u32)
}

/// Verify a visual citation receipt against a PDF file.
///
/// # Arguments
///
/// * `path` - Path to the PDF file (null-terminated UTF-8 string)
/// * `receipt_json` - JSON string containing the receipt to verify
///
/// # Returns
///
/// An int32_t exit code:
/// - 0: receipt verifies successfully
/// - 1: extraction failed (PDF unreadable, encrypted, etc.)
/// - 10: pdf_fingerprint mismatch
/// - 11: bbox mismatch (no span meets 90% IoU threshold)
/// - 12: content_hash mismatch (best-IoU span's text differs)
///
/// On error, use pdftract_last_error() to get a detailed message.
#[no_mangle]
pub extern "C" fn pdftract_verify_receipt(
    path: *const c_char,
    receipt_json: *const c_char,
) -> i32 {
    clear_last_error();

    let result = catch_unwind(|| unsafe {
        let pdf_path = match cstr_to_string(path) {
            Ok(s) => s,
            Err(_) => {
                set_last_error(json_error(error_codes::NULL_POINTER, "path pointer is null"));
                return exit_code::EXTRACTION_FAILED;
            }
        };

        let receipt_str = match cstr_to_string(receipt_json) {
            Ok(s) => s,
            Err(_) => {
                set_last_error(json_error(error_codes::NULL_POINTER, "receipt_json pointer is null"));
                return exit_code::EXTRACTION_FAILED;
            }
        };

        // Parse the receipt JSON
        let receipt: Receipt = match serde_json::from_str(&receipt_str) {
            Ok(r) => r,
            Err(e) => {
                set_last_error(json_error(error_codes::INVALID_JSON, &format!("Invalid receipt JSON: {}", e)));
                return exit_code::EXTRACTION_FAILED;
            }
        };

        // Extract the PDF to get spans and fingerprint
        let pdf_path_obj = Path::new(&pdf_path);
        let extraction_result = match extract_pdf(pdf_path_obj, &ExtractionOptions::default()) {
            Ok(result) => result,
            Err(e) => {
                set_last_error(anyhow_to_json_error(e));
                return exit_code::EXTRACTION_FAILED;
            }
        };

        // Get the page specified in the receipt
        let page = if receipt.page_index < extraction_result.pages.len() {
            &extraction_result.pages[receipt.page_index]
        } else {
            set_last_error(json_error(error_codes::EXTRACTION_ERROR,
                &format!("receipt page_index {} out of bounds (PDF has {} pages)",
                    receipt.page_index, extraction_result.pages.len())));
            return exit_code::EXTRACTION_FAILED;
        };

        // Collect spans from the page
        let spans: Vec<SpanData> = page.spans.iter()
            .map(|span| SpanData {
                text: span.text.clone(),
                bbox: span.bbox,
            })
            .collect();

        // Verify the receipt
        let verify_result = verify_receipt(&receipt, &spans, &extraction_result.fingerprint);

        match verify_result {
            VerificationResult::Ok { .. } => exit_code::SUCCESS,
            VerificationResult::FingerprintMismatch { .. } => exit_code::FINGERPRINT_MISMATCH,
            VerificationResult::BboxMismatch { .. } => exit_code::BBOX_MISMATCH,
            VerificationResult::ContentMismatch { .. } => exit_code::CONTENT_MISMATCH,
        }
    });

    match result {
        Ok(code) => code,
        Err(_) => {
            set_last_error(json_error(error_codes::PANIC, "panic in pdftract_verify_receipt"));
            exit_code::EXTRACTION_FAILED
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    /// Create a minimal valid PDF for testing.
    fn create_minimal_pdf(path: &Path) -> std::io::Result<()> {
        let pdf_data = br#"%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>>>>>endobj
xref
0 4
0000000000 65535 f
0000000009 00000 n
0000000052 00000 n
0000000109 00000 n
trailer<</Size 4/Root 1 0 R>>
startxref
206
%%EOF
"#;
        let mut file = fs::File::create(path)?;
        file.write_all(pdf_data)?;
        Ok(())
    }

    #[test]
    fn test_json_error() {
        let err = json_error("TEST_CODE", "test message");
        assert!(err.contains(r#""error":"TEST_CODE""#));
        assert!(err.contains(r#""message":"test message""#));
    }

    #[test]
    fn test_escape_json() {
        let escaped = escape_json("hello\nworld\"test\\");
        assert_eq!(escaped, "hello\\nworld\\\"test\\\\");
    }

    #[test]
    fn test_pdftract_version_not_null() {
        let version = unsafe {
            CStr::from_ptr(pdftract_version())
                .to_str()
                .unwrap()
        };
        assert!(!version.is_empty());
    }
}
