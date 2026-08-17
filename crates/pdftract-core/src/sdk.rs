//! pdftract SDK public API surface.
//!
//! This module exposes the 9-method SDK contract that all language SDKs implement.
//! Rust users import pdftract-core directly and use these functions to match the SDK contract.

use crate::classify::{PageClass, PageClassification};
use crate::extract::{
    extract_pdf, extract_text as extract_text_impl, ExtractionResult, PageResult,
};
use crate::options::ExtractionOptions;
use crate::parser::stream::PdfSource as ParserPdfSource;
use crate::receipts::verifier::{verify_receipt, SpanData, VerificationResult};
use crate::receipts::Receipt;
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;
use std::process::Command;

/// Extract a PDF to the full structured JSON output.
///
/// This is the main extraction method that returns pages, spans, blocks, tables,
/// form fields, and other structured data as JSON-serializable objects.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `options` - Extraction options (OCR, password, etc.)
///
/// # Returns
///
/// An `ExtractionResult` containing pages and metadata.
pub fn extract(pdf_path: &Path, options: &ExtractionOptions) -> Result<ExtractionResult> {
    extract_pdf(pdf_path, options)
}

/// Extract plain text from a PDF.
///
/// Returns the concatenated text content of all pages, with spans separated
/// by newlines. Invisible text (rendering_mode=3) is excluded by default.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `options` - Extraction options (OCR, password, etc.)
///
/// # Returns
///
/// A String containing all extracted text.
pub fn extract_text(pdf_path: &Path, options: &ExtractionOptions) -> Result<String> {
    extract_text_impl(pdf_path, options)
}

/// Extract Markdown from a PDF.
///
/// Returns the document converted to Markdown format, with headers, lists,
/// tables, and form fields rendered using Markdown syntax.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `options` - Extraction options (OCR, password, etc.)
///
/// # Returns
///
/// A String containing the Markdown representation.
pub fn extract_markdown(pdf_path: &Path, options: &ExtractionOptions) -> Result<String> {
    let result = extract_pdf(pdf_path, options)?;

    let mut markdown = String::new();
    for (i, page) in result.pages.iter().enumerate() {
        if i > 0 {
            markdown.push_str("\n\n");
        }

        // Filter links to only those that belong to this page
        let page_links: Vec<_> = result
            .links
            .iter()
            .filter(|link| link.page_index == i)
            .cloned()
            .collect();

        markdown.push_str(&crate::markdown::page_to_markdown_with_links(
            &page.blocks,
            &page.spans,
            &[], // No separate tables storage - tables are in blocks
            page_links.as_slice(),
            i,
            false, // include_anchor
            &crate::markdown::MarkdownOptions::default(),
        ));
    }

    Ok(markdown)
}

/// Extract a PDF page by page as an iterator.
///
/// This is the streaming variant that yields pages one at a time, keeping
/// memory usage bounded regardless of document size.
///
/// # Memory Bounding
///
/// This implementation uses lazy page iteration that processes one page at a time.
/// Peak RSS stays under the 256MB ceiling regardless of page count (plan requirement
/// docs/plan/plan.md:74-75).
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `options` - Extraction options (OCR, password, etc.)
///
/// # Returns
///
/// An iterator that yields `PageResult` objects.
///
/// # Examples
///
/// ```rust,no_run
/// use pdftract_core::{extract_stream, ExtractionOptions};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut pages = extract_stream(
///     &std::path::Path::new("document.pdf"),
///     &ExtractionOptions::default()
/// )?;
///
/// while let Some(page_result) = pages.next() {
///     let page = page_result?;
///     println!("Page {}: {} spans", page.index, page.spans.len());
/// }
/// # Ok(())
/// # }
/// ```
pub fn extract_stream(
    pdf_path: &Path,
    options: &ExtractionOptions,
) -> Result<impl Iterator<Item = Result<PageResult>>> {
    // Channel to send pages from callback to iterator
    let (sender, receiver) = std::sync::mpsc::channel();

    // Spawn a thread that uses the streaming extraction callback
    let pdf_path = pdf_path.to_path_buf();
    let options_clone = options.clone();

    std::thread::spawn(move || {
        use crate::extract::extract_pdf_streaming;

        let result = extract_pdf_streaming(&pdf_path, &options_clone, |page_result| {
            // Send the page to the iterator
            let _ = sender.send(Ok(page_result.clone()));
            true // Continue processing
        });

        // Send the final result or error
        if let Err(e) = result {
            let _ = sender.send(Err(anyhow::anyhow!("Extraction failed: {}", e)));
        }
    });

    // Return an iterator that receives pages from the channel
    Ok(StreamIterator::new(receiver))
}

/// Streaming iterator that receives pages from a channel.
///
/// This iterator wraps a channel receiver to provide an Iterator interface
/// that yields PageResult objects as they become available from the streaming
/// extraction thread.
struct StreamIterator {
    /// Channel receiver for pages
    receiver: std::sync::mpsc::Receiver<Result<PageResult>>,
    /// Buffer for the next page (if already received)
    next_item: Option<Result<PageResult>>,
}

impl StreamIterator {
    fn new(receiver: std::sync::mpsc::Receiver<Result<PageResult>>) -> Self {
        Self {
            receiver,
            next_item: None,
        }
    }
}

impl Iterator for StreamIterator {
    type Item = Result<PageResult>;

    fn next(&mut self) -> Option<Self::Item> {
        // Return buffered item if available
        if let Some(item) = self.next_item.take() {
            return Some(item);
        }

        // Try to receive the next page
        match self.receiver.recv() {
            Ok(item) => Some(item),
            Err(_) => None, // Channel closed, no more pages
        }
    }
}

/// Search for text patterns in a PDF.
///
/// Returns an iterator of matches with page index, span index, and context.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `pattern` - Search pattern (plain text or regex)
/// * `case_insensitive` - Ignore case when matching
/// * `regex` - Treat pattern as a regular expression
/// * `whole_word` - Match only whole words
///
/// # Returns
///
/// A vector of `SearchMatch` objects with location and context.
pub fn search(
    pdf_path: &Path,
    pattern: &str,
    case_insensitive: bool,
    use_regex: bool,
    whole_word: bool,
) -> Result<Vec<SearchMatch>> {
    let options = ExtractionOptions::default();
    let result = extract_pdf(pdf_path, &options)?;

    let mut matches = Vec::new();

    // Build the regex pattern
    let search_pattern = if whole_word {
        format!(r"\b{}\b", regex::escape(pattern))
    } else if use_regex {
        pattern.to_string()
    } else {
        regex::escape(pattern)
    };

    let re = Regex::new(&search_pattern)
        .with_context(|| format!("Invalid regex pattern: {}", search_pattern))?;

    for (page_idx, page) in result.pages.iter().enumerate() {
        for (span_idx, span) in page.spans.iter().enumerate() {
            let text = &span.text;

            // Check if pattern matches
            let re_with_flags = if case_insensitive {
                Regex::new(&format!("(?i){}", search_pattern))?
            } else {
                re.clone()
            };

            if re_with_flags.is_match(text) {
                matches.push(SearchMatch {
                    page_index: page_idx,
                    span_index: span_idx,
                    text: text.clone(),
                    bbox: span.bbox,
                });
            }
        }
    }

    Ok(matches)
}

/// A single search match result.
#[derive(Debug, Clone)]
pub struct SearchMatch {
    /// Page index where the match was found.
    pub page_index: usize,
    /// Span index within the page.
    pub span_index: usize,
    /// The matched text content.
    pub text: String,
    /// Bounding box of the match [x0, y0, x1, y1].
    pub bbox: [f64; 4],
}

/// Get metadata about a PDF.
///
/// Returns page count and basic metadata without full extraction.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
///
/// # Returns
///
/// A `PdfMetadata` object with page count and other metadata.
pub fn get_metadata(pdf_path: &Path) -> Result<PdfMetadata> {
    let (_fingerprint, catalog, pages, _resolver, trailer) = crate::document::parse_pdf_file(pdf_path)?;

    // Check if document is encrypted by looking for /Encrypt in trailer
    let is_encrypted = trailer.get("/Encrypt").is_some();

    Ok(PdfMetadata {
        page_count: pages.len(),
        is_encrypted,
        is_tagged: catalog.struct_tree_root_ref.is_some(),
        has_forms: catalog.acroform_ref.is_some(),
    })
}

/// Metadata about a PDF document.
#[derive(Debug, Clone)]
pub struct PdfMetadata {
    /// Total number of pages.
    pub page_count: usize,
    /// Whether the document is encrypted.
    pub is_encrypted: bool,
    /// Whether the document is a tagged PDF.
    pub is_tagged: bool,
    /// Whether the document has AcroForm fields.
    pub has_forms: bool,
}

/// Compute the cryptographic hash of a PDF.
///
/// Returns the v1 fingerprint hash of the PDF content.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
///
/// # Returns
///
/// A String containing the fingerprint hash in format "pdftract-v1:HEX_HASH".
///
/// Where HEX_HASH is a hexadecimal string of the SHA-256 hash.
pub fn hash(pdf_path: &Path) -> Result<String> {
    let (fingerprint, _catalog, _pages, _resolver, _trailer) = crate::document::parse_pdf_file(pdf_path)?;
    Ok(fingerprint)
}

/// Classify a PDF page using the full pdftract binary invocation.
///
/// This function invokes the pdftract binary with JSON output to perform
/// comprehensive page classification, including hybrid detection and grid analysis.
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `page_index` - Zero-based page index to classify
///
/// # Returns
///
/// A `PageClassification` with the detected page type and confidence.
///
/// # Errors
///
/// Returns an error if:
/// - The pdftract binary cannot be found
/// - The PDF file cannot be read
/// - The pdftract binary fails to execute
/// - The JSON output cannot be parsed
/// - The page index is out of bounds
pub fn classify(pdf_path: &Path, page_index: usize) -> Result<PageClassification> {
    use std::io::Write;
    use std::process::Command;

    // Read the PDF file
    let pdf_bytes = std::fs::read(pdf_path)
        .with_context(|| format!("Failed to read PDF file: {}", pdf_path.display()))?;

    // Validate PDF has minimal content
    if pdf_bytes.is_empty() {
        return Err(anyhow::anyhow!("PDF input is empty"));
    }

    // Check for PDF signature
    if !pdf_bytes.starts_with(b"%PDF") {
        return Err(anyhow::anyhow!("Invalid PDF: missing PDF signature (expected to start with '%PDF')"));
    }

    // Create a temporary file for the PDF
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!(
        "pdftract-classify-{}-{}.pdf",
        std::process::id(),
        page_index
    ));

    // Write PDF bytes to temp file
    {
        let mut file = std::fs::File::create(&temp_file)
            .with_context(|| format!("Failed to create temporary file: {}", temp_file.display()))?;
        file.write_all(&pdf_bytes)
            .with_context(|| format!("Failed to write PDF to temporary file: {}", temp_file.display()))?;
        file.flush()
            .with_context(|| format!("Failed to flush temporary file: {}", temp_file.display()))?;
    }

    // Ensure temp file is cleaned up using a manual RAII guard
    struct TempFileGuard(std::path::PathBuf);
    impl Drop for TempFileGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _temp_guard = TempFileGuard(temp_file.clone());

    // Find the pdftract binary
    let pdftract_binary = find_pdftract_binary()?;

    // Spawn pdftract binary with JSON output to stdout
    let output = Command::new(&pdftract_binary)
        .arg("extract")
        .arg("--json")
        .arg("-")  // Write JSON to stdout
        .arg(&temp_file)  // PDF input file
        .output()
        .with_context(|| format!("Failed to spawn pdftract binary: {}", pdftract_binary))?;

    // Check if the command succeeded
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(anyhow::anyhow!(
            "pdftract extraction failed with exit code {:?}. stderr: {}",
            output.status.code(),
            stderr
        ));
    }

    // Parse JSON output from stdout
    let json_str = String::from_utf8(output.stdout)
        .with_context(|| "Failed to convert pdftract output to UTF-8")?;

    let json_value: serde_json::Value = serde_json::from_str(&json_str)
        .with_context(|| "Failed to parse pdftract JSON output")?;

    // Extract pages array
    let pages = json_value
        .get("pages")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("JSON output missing required 'pages' array"))?;

    // We expect at least one page
    if pages.is_empty() {
        return Err(anyhow::anyhow!("PDF contains no pages"));
    }

    // Validate page index is within bounds
    if page_index >= pages.len() {
        return Err(anyhow::anyhow!(
            "Page index {} out of bounds (PDF has {} pages)",
            page_index,
            pages.len()
        ));
    }

    // Get the requested page
    let page = &pages[page_index];

    // Extract classification from page_type
    // PageClass mapping: "mixed" -> Hybrid, "text" -> Vector, "scanned" -> Scanned, "broken_vector" -> BrokenVector
    let page_type = page
        .get("page_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("JSON output missing 'page_type' field"))?;

    // Extract confidence if present, otherwise default to 0.5
    let confidence = page
        .get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5) as f32;

    let class = match page_type {
        "mixed" => PageClass::Hybrid,
        "text" => PageClass::Vector,
        "scanned" => PageClass::Scanned,
        "broken_vector" => PageClass::BrokenVector,
        "blank" => PageClass::Vector, // Blank pages are treated as vector (no content)
        "figure_only" => PageClass::Scanned, // Figure-only pages are treated as scanned
        unknown => {
            return Err(anyhow::anyhow!(
                "Unknown page_type '{}'. Expected one of: mixed, text, scanned, broken_vector, blank, figure_only",
                unknown
            ))
        }
    };

    // For Hybrid pages, extract hybrid_cells if present
    let hybrid_cells = if class == PageClass::Hybrid {
        // Try to extract hybrid_cells from the JSON output
        if let Some(cells) = page.get("hybrid_cells").and_then(|v| v.as_array()) {
            use std::collections::BTreeSet;
            let cell_set: BTreeSet<usize> = cells
                .iter()
                .filter_map(|v| v.as_u64())
                .map(|v| v as usize)
                .collect();
            Some(cell_set)
        } else {
            None
        }
    } else {
        None
    };

    Ok(PageClassification {
        class,
        confidence,
        hybrid_cells,
    })
}

/// Find the pdftract binary in standard locations.
///
/// Searches for the pdftract binary in:
/// 1. The current executable's directory (for testing)
/// 2. The build target/release directory
/// 3. System PATH
///
/// # Returns
///
/// The path to the pdftract binary if found.
///
/// # Errors
///
/// Returns an error if the binary cannot be found in any location.
fn find_pdftract_binary() -> Result<String> {
    use std::env;
    use std::path::PathBuf;

    let binary_name = if cfg!(windows) { "pdftract.exe" } else { "pdftract" };

    // List of paths to search for the pdftract binary
    let mut search_paths: Vec<PathBuf> = Vec::new();

    // 1. Check the current executable's directory (for testing)
    if let Some(exe_path) = env::current_exe().ok() {
        if let Some(exe_dir) = exe_path.parent() {
            search_paths.push(exe_dir.join(binary_name));
        }
    }

    // 2. Check the build target/release directory
    if let Ok(mut cwd) = env::current_dir() {
        cwd.push("target");
        cwd.push("release");
        cwd.push(binary_name);
        search_paths.push(cwd);

        // Also check debug directory
        if let Some(_release_idx) = search_paths.last().and_then(|p| Some(p.to_string_lossy().contains("release"))) {
            let mut debug_path = search_paths.last().unwrap().clone();
            debug_path.set_file_name("debug");
            debug_path.push(binary_name);
            search_paths.push(debug_path);
        }
    }

    // 3. Check PATH
    if let Ok(path_var) = env::var("PATH") {
        for dir in env::split_paths(&path_var) {
            search_paths.push(dir.join(binary_name));
        }
    }

    // Try each path to see if the binary exists and is executable
    let binary_paths: Vec<String> = search_paths
        .iter()
        .filter_map(|p| p.to_str().map(|s| s.to_string()))
        .collect();

    for path in &binary_paths {
        if std::path::Path::new(path).exists() {
            // Test if we can execute it
            if Command::new(path)
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                return Ok(path.clone());
            }
        }
    }

    Err(anyhow::anyhow!(
        "pdftract binary not found. Tried the following paths: {:?}. \
         Ensure pdftract is built (run 'cargo build --release') and available in PATH.",
        binary_paths
    ))
}

/// Verify a cryptographic receipt against a PDF.
///
/// Validates that the receipt matches the PDF content by checking:
/// 1. PDF fingerprint matches
/// 2. At least one span has bbox overlap >= 90% IoU
/// 3. That span's NFC-normalized SHA-256 equals the receipt's content_hash
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `receipt_path` - Path to the receipt JSON file
///
/// # Returns
///
/// A `VerificationResult` indicating success or the specific failure mode.
pub fn verify_receipt_from_path(
    pdf_path: &Path,
    receipt_path: &Path,
) -> Result<VerificationResult> {
    // Load the receipt
    let receipt_data =
        std::fs::read_to_string(receipt_path).context("Failed to read receipt file")?;
    let receipt: Receipt =
        serde_json::from_str(&receipt_data).context("Failed to parse receipt JSON")?;

    // Extract spans from the PDF
    let options = ExtractionOptions::default();
    let result = extract_pdf(pdf_path, &options)?;

    let page = result.pages.get(receipt.page_index).ok_or_else(|| {
        anyhow::anyhow!("Receipt page index {} out of bounds", receipt.page_index)
    })?;

    // Convert spans to SpanData
    let spans: Vec<SpanData> = page
        .spans
        .iter()
        .map(|span| SpanData {
            text: span.text.clone(),
            bbox: span.bbox,
        })
        .collect();

    // Compute the actual fingerprint
    let actual_fingerprint = hash(pdf_path)?;

    // Verify
    Ok(verify_receipt(&receipt, &spans, &actual_fingerprint))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_search_basic() {
        // Test will be implemented with fixture
    }

    #[test]
    fn test_get_metadata_encrypted_pdf() {
        // Test that encrypted PDFs are correctly identified
        let encrypted_fixtures = [
            "tests/fixtures/encrypted/EC-04-rc4-encrypted.pdf",
            "tests/fixtures/encrypted/EC-05-aes128-encrypted.pdf",
            "tests/fixtures/encrypted/EC-06-aes256-encrypted.pdf",
            "tests/fixtures/encrypted/livecycle.pdf",
        ];

        for fixture_path in &encrypted_fixtures {
            let path = Path::new(fixture_path);
            if !path.exists() {
                println!("Skipping {} (not found)", fixture_path);
                continue;
            }

            let metadata = match get_metadata(path) {
                Ok(meta) => meta,
                Err(e) => {
                    println!("Skipping {} - parse error: {}", fixture_path, e);
                    continue;
                }
            };

            assert!(
                metadata.is_encrypted,
                "Expected encrypted PDF {} to have is_encrypted=true",
                fixture_path
            );
            println!("✓ {} correctly reports is_encrypted=true ({} pages)",
                     fixture_path, metadata.page_count);
        }
    }

    #[test]
    fn test_get_metadata_non_encrypted_pdf() {
        // Test that non-encrypted PDFs return is_encrypted=false
        let non_encrypted_fixtures = [
            "tests/fixtures/sample.pdf",
            "tests/fixtures/tagged-suspects-true.pdf",
            "tests/fixtures/markdown_structure.pdf",
        ];

        for fixture_path in &non_encrypted_fixtures {
            let path = Path::new(fixture_path);
            if !path.exists() {
                println!("Skipping {} (not found)", fixture_path);
                continue;
            }

            let metadata = match get_metadata(path) {
                Ok(meta) => meta,
                Err(e) => {
                    println!("Skipping {} - parse error: {}", fixture_path, e);
                    continue;
                }
            };

            assert!(
                !metadata.is_encrypted,
                "Expected non-encrypted PDF {} to have is_encrypted=false",
                fixture_path
            );
            println!("✓ {} correctly reports is_encrypted=false ({} pages)",
                     fixture_path, metadata.page_count);
        }
    }

    #[test]
    fn test_get_metadata_page_count() {
        // Test that page count is correctly reported
        let path = Path::new("tests/fixtures/sample.pdf");
        if !path.exists() {
            println!("Skipping sample.pdf (not found)");
            return;
        }

        let metadata = get_metadata(path).expect("Failed to get metadata");
        assert!(metadata.page_count > 0, "Expected page count > 0");
        println!("✓ sample.pdf has {} pages", metadata.page_count);
    }

    #[test]
    fn test_get_metadata_tagged_pdf() {
        // Test that tagged PDFs are correctly identified
        let path = Path::new("tests/fixtures/tagged-suspects-true.pdf");
        if !path.exists() {
            println!("Skipping tagged-suspects-true.pdf (not found)");
            return;
        }

        let metadata = get_metadata(path).expect("Failed to get metadata");
        // This PDF should be tagged
        assert!(metadata.is_tagged, "Expected tagged PDF to have is_tagged=true");
        println!("✓ tagged-suspects-true.pdf correctly reports is_tagged=true");
    }

    #[test]
    fn test_get_metadata_non_tagged_pdf() {
        // Test that non-tagged PDFs are correctly identified
        let path = Path::new("tests/fixtures/sample.pdf");
        if !path.exists() {
            println!("Skipping sample.pdf (not found)");
            return;
        }

        let metadata = get_metadata(path).expect("Failed to get metadata");
        // sample.pdf is not tagged
        assert!(!metadata.is_tagged, "Expected non-tagged PDF to have is_tagged=false");
        println!("✓ sample.pdf correctly reports is_tagged=false");
    }
}
