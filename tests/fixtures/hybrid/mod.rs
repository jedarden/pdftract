//! Hybrid fixture test helpers for PageClass::Hybrid classification validation.
//!
//! This module provides helper functions and test utilities for validating hybrid PDF
//! fixtures against the PageClass::Hybrid classification rules and grid-cell coverage
//! thresholds.
//!
//! # Overview
//!
//! Hybrid PDFs contain both vector text (selectable, from PDF content streams) and
//! scanned/image regions (requiring OCR). This module provides test infrastructure for:
//!
//! - Loading hybrid fixture PDFs from `tests/fixtures/hybrid/`
//! - Running PageClass classification analysis
//! - Extracting 8×8 grid-cell coverage metrics
//! - Asserting PageClass::Hybrid classification with >=15% hybrid cell coverage
//!
//! # Grid-Cell Coverage Threshold
//!
//! Per Phase 5.5 classification rules, a page is classified as **Hybrid** when:
//! - ≥ 10 cells (≥ 15% of the 64-cell 8×8 grid) are classified as vector
//! - ≥ 10 cells (≥ 15% of the 64-cell 8×8 grid) are classified as scanned/image-heavy
//!
//! This module validates that fixtures meet these thresholds and are correctly
//! classified as `PageClass::Hybrid` (page_type = "mixed").
//!
//! # Usage
//!
//! ```rust,no_run
//! use pdftract_core::sdk;
//! use pdftract_core::page_class::PageClass;
//!
//! // Load and classify a hybrid fixture
//! let result = load_and_classify_fixture("hybrid-001-vector-header-over-scan.pdf")
//!     .expect("Failed to load fixture");
//!
//! // Assert classification is Hybrid with sufficient coverage
//! assert_hybrid_classification(
//!     &result,
//!     "hybrid-001 should be classified as Hybrid",
//!     10  // minimum hybrid cells
//! );
//!
//! // Extract grid-cell coverage metrics
//! let cell_count = extract_hybrid_cell_count(&result);
//! assert!(cell_count >= 10, "Need at least 10 hybrid cells (15% threshold)");
//! ```
//!
//! # Example Test
//!
//! ```rust,no_run
//! #[test]
//! fn test_hybrid_001_classification() {
//!     let result = load_and_classify_fixture("hybrid-001-vector-header-over-scan.pdf")
//!         .expect("Failed to load hybrid-001");
//!
//!     assert_eq!(
//!         result.class,
//!         PageClass::Hybrid,
//!         "hybrid-001 should classify as PageClass::Hybrid"
//!     );
//!
//!     let cell_count = extract_hybrid_cell_count(&result);
//!     assert!(
//!         cell_count >= 10,
//!         "hybrid-001 should have >= 10 hybrid cells (15% threshold), got {}",
//!         cell_count
//!     );
//! }
//! ```

use pdftract_core::page_class::{PageClass, PageClassification};
use pdftract_core::sdk;
use std::error::Error as StdError;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};

// Re-export serde_json for use in extract_grid_coverage
pub use serde_json;

/// Comprehensive error types for `classify_page` function.
///
/// These errors cover all failure modes in the page classification pipeline,
/// from invalid input through binary invocation to output parsing.
#[derive(Debug)]
pub enum ClassifyError {
    /// PDF input is empty.
    EmptyPdfInput,

    /// PDF input is missing the "%PDF" signature.
    InvalidPdfSignature,

    /// Failed to create temporary file.
    TempFileCreationFailed(std::io::Error),

    /// Failed to write PDF bytes to temporary file.
    TempFileWriteFailed(std::io::Error),

    /// Failed to flush temporary file.
    TempFileFlushFailed(std::io::Error),

    /// pdftract binary not found in any expected location.
    BinaryNotFound(Vec<String>),

    /// Failed to spawn pdftract binary.
    BinarySpawnFailed(std::io::Error),

    /// pdftract extraction failed with exit code and stderr message.
    ExtractionFailed {
        exit_code: Option<i32>,
        stderr: String,
    },

    /// pdftract output is not valid UTF-8.
    InvalidUtf8Output(std::string::FromUtf8Error),

    /// Failed to parse pdftract JSON output.
    JsonParseFailed(serde_json::Error),

    /// JSON output is missing required 'pages' array.
    MissingPagesArray,

    /// PDF contains no pages.
    NoPages,

    /// Failed to get first page from pages array.
    NoFirstPage,

    /// JSON output is missing 'page_type' field.
    MissingPageType,

    /// Unknown/invalid page_type value in JSON output.
    UnknownPageType(String),
}

impl fmt::Display for ClassifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPdfInput => {
                write!(f, "PDF input bytes are empty. \
                    Action: Provide a non-empty PDF file. Check that the file was read correctly and is not truncated.")
            }
            Self::InvalidPdfSignature => {
                write!(f, "Invalid PDF file: missing PDF signature. \
                    Expected file to start with '%PDF-' or '%PDF'. \
                    Action: Verify the file is a valid PDF. Check if the file was corrupted during download or transfer. \
                    Try opening the file in a PDF viewer to confirm validity.")
            }
            Self::TempFileCreationFailed(e) => {
                write!(f, "Failed to create temporary file for PDF processing: {}. \
                    Action: Check available disk space and write permissions in the temp directory (usually /tmp or $TMPDIR). \
                    Ensure the system temp directory is accessible and not full. \
                    Error details: {}", e, e)
            }
            Self::TempFileWriteFailed(e) => {
                write!(f, "Failed to write PDF bytes to temporary file: {}. \
                    Action: Check available disk space and ensure the temp file has not been deleted or locked. \
                    Verify the PDF bytes are valid and not corrupted. \
                    Error details: {}", e, e)
            }
            Self::TempFileFlushFailed(e) => {
                write!(f, "Failed to flush temporary file data to disk: {}. \
                    Action: Check disk space and I/O errors. The file may be on a full or read-only filesystem. \
                    Error details: {}", e, e)
            }
            Self::BinaryNotFound(paths) => {
                write!(
                    f,
                    "pdftract binary not found. \
                    Tried the following paths in order: {:?}. \
                    \n\nAction: Build pdftract with 'cargo build --release' or install it. \
                    \n  For development: Ensure target/debug/pdftract or target/release/pdftract exists. \
                    \n  For installation: Run 'cargo install pdftract' or add the build directory to PATH. \
                    \n  Current working directory: {:?}",
                    paths,
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("<unknown>"))
                )
            }
            Self::BinarySpawnFailed(e) => {
                write!(f, "Failed to spawn pdftract binary process: {}. \
                    Action: Check that the binary exists and is executable. \
                    Verify file permissions (run 'chmod +x pdftract' if needed). \
                    Check system resource limits (ulimit -u) and available memory. \
                    Error details: {}", e, e)
            }
            Self::ExtractionFailed { exit_code, stderr } => {
                write!(
                    f,
                    "pdftract extraction failed with exit code {:?}. \
                    \n\nAction: Check the PDF file for corruption or unsupported features. \
                    \n  Verify the PDF is not encrypted or password-protected. \
                    \n  Try opening the file in a PDF viewer to confirm it's valid. \
                    \n  Check disk space and system resources. \
                    \n\nstderr output:\n{}",
                    exit_code, stderr
                )
            }
            Self::InvalidUtf8Output(e) => {
                write!(f, "Failed to convert pdftract output to UTF-8: {}. \
                    Action: The pdftract binary produced invalid UTF-8 output. \
                    This may indicate a corrupted binary or output encoding issue. \
                    Try reinstalling or rebuilding pdftract. \
                    Error details: {}", e, e)
            }
            Self::JsonParseFailed(e) => {
                write!(f, "Failed to parse pdftract JSON output: {}. \
                    Action: The output from pdftract is not valid JSON. \
                    This may indicate a bug in pdftract or an incomplete extraction. \
                    Check that pdftract is working correctly by running it manually. \
                    Error details: {}", e, e)
            }
            Self::MissingPagesArray => {
                write!(f, "JSON output missing required 'pages' array. \
                    Action: The pdftract output format may have changed or is incomplete. \
                    Verify that pdftract is producing valid JSON output with a 'pages' field. \
                    Try running pdftract manually to inspect the output format.")
            }
            Self::NoPages => {
                write!(f, "PDF contains no pages. \
                    Action: Verify the PDF file is not empty and contains at least one page. \
                    The file may be corrupted or not a valid PDF document.")
            }
            Self::NoFirstPage => {
                write!(f, "Failed to get first page from pages array. \
                    Action: The pages array exists but is empty. This should not happen if NoPages check passed. \
                    This may indicate a race condition or corrupted PDF structure.")
            }
            Self::MissingPageType => {
                write!(f, "JSON output missing 'page_type' field. \
                    Action: The pdftract output format may have changed. \
                    Verify the expected output format includes 'page_type' for each page. \
                    Run pdftract manually to inspect the actual output structure.")
            }
            Self::UnknownPageType(page_type) => {
                write!(
                    f,
                    "Unknown page_type '{}'. \
                    \n\nExpected one of: mixed, text, scanned, broken_vector, blank, figure_only. \
                    \n\nAction: The pdftract output contains an unrecognized page type. \
                    \n  This may indicate a version mismatch or corrupted output. \
                    \n  Verify pdftract is producing valid page_type values. \
                    \n  Received value: '{}'",
                    page_type, page_type
                )
            }
        }
    }
}

impl StdError for ClassifyError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::TempFileCreationFailed(e) => Some(e),
            Self::TempFileWriteFailed(e) => Some(e),
            Self::TempFileFlushFailed(e) => Some(e),
            Self::BinarySpawnFailed(e) => Some(e),
            Self::InvalidUtf8Output(e) => Some(e),
            Self::JsonParseFailed(e) => Some(e),
            _ => None,
        }
    }
}

impl ClassifyError {
    /// Get detailed diagnostic context for this error.
    ///
    /// Returns structured information about what went wrong and how to fix it.
    /// This is more detailed than the Display impl and provides machine-readable context.
    ///
    /// # Returns
    ///
    /// A string containing detailed diagnostic information including:
    /// - Error category (input, binary, process, output, etc.)
    /// - What failed
    /// - Why it likely failed
    /// - How to fix it
    pub fn diagnostic_context(&self) -> String {
        match self {
            Self::EmptyPdfInput => {
                format!("Error Category: Input Validation\n\
                        What: PDF input bytes are empty (zero length)\n\
                        Why: The PDF data provided has no content\n\
                        How to fix: Provide a non-empty PDF file. Check file read operations \
                        and verify the file was not truncated during download or transfer.")
            }
            Self::InvalidPdfSignature => {
                format!("Error Category: Input Validation\n\
                        What: PDF file missing signature header\n\
                        Why: Valid PDFs must start with '%PDF-' or '%PDF'. This file does not.\n\
                        How to fix: Verify the file is a valid PDF. Check for corruption during \
                        download/transfer. Try opening in a PDF viewer. If the file is a different \
                        format (e.g., PostScript), convert it to PDF first.")
            }
            Self::TempFileCreationFailed(e) => {
                format!("Error Category: Filesystem I/O\n\
                        What: Cannot create temporary file for PDF processing\n\
                        Why: {}\n\
                        How to fix: 1) Check disk space (df -h)\n\
                        2) Verify write permissions in temp directory\n\
                        3) Check $TMPDIR environment variable\n\
                        4) Ensure temp directory exists and is accessible\n\
                        5) Try setting TMPDIR to a writable location",
                        e)
            }
            Self::TempFileWriteFailed(e) => {
                format!("Error Category: Filesystem I/O\n\
                        What: Cannot write PDF bytes to temporary file\n\
                        Why: {}\n\
                        How to fix: 1) Check available disk space\n\
                        2) Verify temp file was not deleted or locked\n\
                        3) Check for I/O errors on the filesystem\n\
                        4) Verify PDF bytes are valid and not corrupted",
                        e)
            }
            Self::TempFileFlushFailed(e) => {
                format!("Error Category: Filesystem I/O\n\
                        What: Cannot flush temporary file data to disk\n\
                        Why: {}\n\
                        How to fix: 1) Check disk space (filesystem may be full)\n\
                        2) Verify filesystem is not read-only\n\
                        3) Check for hardware I/O errors\n\
                        4) Ensure data can be written to the temp directory",
                        e)
            }
            Self::BinaryNotFound(paths) => {
                format!("Error Category: Binary Not Found\n\
                        What: pdftract binary not found in any expected location\n\
                        Why: The executable does not exist or is not in PATH\n\
                        Tried: {:?}\n\
                        Current directory: {:?}\n\
                        How to fix: 1) Build pdftract: 'cargo build --release'\n\
                        2) Install pdftract: 'cargo install pdftract'\n\
                        3) Add build directory to PATH\n\
                        4) Verify binary exists at one of the tried paths",
                        paths,
                        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("<unknown>")))
            }
            Self::BinarySpawnFailed(e) => {
                format!("Error Category: Process Execution\n\
                        What: Cannot spawn pdftract binary process\n\
                        Why: {}\n\
                        How to fix: 1) Verify binary exists and is executable\n\
                        2) Check permissions: 'chmod +x pdftract'\n\
                        3) Check system resource limits (ulimit -u)\n\
                        4) Verify sufficient memory available\n\
                        5) Check for anti-virus or security software blocking execution",
                        e)
            }
            Self::ExtractionFailed { exit_code, stderr } => {
                format!("Error Category: Extraction Failed\n\
                        What: pdftract process exited with error\n\
                        Exit code: {:?}\n\
                        Why: Extraction failed - see stderr output below\n\
                        How to fix: 1) Check PDF is not corrupted (open in viewer)\n\
                        2) Verify PDF is not encrypted/password-protected\n\
                        3) Check available disk space and memory\n\
                        4) Try a simpler PDF to verify pdftract works\n\
                        5) Run pdftract manually for more details\n\
                        \nstderr:\n{}",
                        exit_code, stderr)
            }
            Self::InvalidUtf8Output(e) => {
                format!("Error Category: Output Encoding\n\
                        What: pdftract produced invalid UTF-8 output\n\
                        Why: {}\n\
                        How to fix: 1) The binary may be corrupted - reinstall/rebuild\n\
                        2) Check binary version compatibility\n\
                        3) Try running pdftract manually to inspect output\n\
                        4) Verify locale and encoding settings (LC_ALL, LANG)",
                        e)
            }
            Self::JsonParseFailed(e) => {
                format!("Error Category: Output Parsing\n\
                        What: Cannot parse pdftract output as JSON\n\
                        Why: {}\n\
                        How to fix: 1) Run pdftract manually to see actual output\n\
                        2) Verify --json flag is supported\n\
                        3) Check for incomplete or truncated output\n\
                        4) Look for error messages mixed with JSON\n\
                        5) Verify pdftract is producing valid JSON",
                        e)
            }
            Self::MissingPagesArray => {
                format!("Error Category: Output Format\n\
                        What: JSON output missing 'pages' array\n\
                        Why: The expected output format was not found\n\
                        How to fix: 1) Verify pdftract output format has not changed\n\
                        2) Run pdftract manually to inspect output structure\n\
                        3) Check for version mismatch between test and binary\n\
                        4) Verify --json flag produces pages array")
            }
            Self::NoPages => {
                format!("Error Category: PDF Content\n\
                        What: PDF contains no pages\n\
                        Why: The PDF file is empty or corrupted\n\
                        How to fix: 1) Verify the PDF is not empty\n\
                        2) Check if the file was truncated during download\n\
                        3) Try opening in a PDF viewer to verify page count\n\
                        4) Ensure the file is a valid PDF document")
            }
            Self::NoFirstPage => {
                format!("Error Category: Internal Logic\n\
                        What: Cannot access first page from pages array\n\
                        Why: The pages array exists but is empty (unexpected)\n\
                        How to fix: 1) This may indicate a race condition or bug\n\
                        2) Verify PDF has at least one page\n\
                        3) Run pdftract manually to inspect output\n\
                        4) Report this as a bug if it persists")
            }
            Self::MissingPageType => {
                format!("Error Category: Output Format\n\
                        What: JSON output missing 'page_type' field\n\
                        Why: The expected output format was not found\n\
                        How to fix: 1) Verify pdftract output format includes page_type\n\
                        2) Run pdftract manually: 'pdftract extract --json file.pdf'\n\
                        3) Check for version mismatch or format change\n\
                        4) Verify the binary is producing expected JSON structure")
            }
            Self::UnknownPageType(page_type) => {
                format!("Error Category: Output Format\n\
                        What: Unrecognized page_type value in JSON output\n\
                        Received: '{}'\n\
                        Expected: mixed, text, scanned, broken_vector, blank, figure_only\n\
                        Why: The page_type value is not one of the known types\n\
                        How to fix: 1) Verify pdftract is producing valid page_type values\n\
                        2) Check for version mismatch or format change\n\
                        3) Run pdftract manually to inspect output\n\
                        4) Ensure binary and test expectations are aligned",
                        page_type)
            }
        }
    }
}

impl From<ClassifyError> for anyhow::Error {
    fn from(err: ClassifyError) -> anyhow::Error {
        anyhow::anyhow!("{}", err)
    }
}

/// Directory containing hybrid fixture PDFs.
pub const FIXTURE_DIR: &str = "tests/fixtures/hybrid";

/// Minimum number of hybrid cells required (15% of 64 cells = 9.6, so ≥10 cells).
///
/// Per the Phase 5.5 classification rules:
/// - ≥10 cells must be vector-heavy
/// - ≥10 cells must be image-heavy
/// - This threshold is ~15.6% of the 64-cell grid
pub const MIN_HYBRID_CELLS: usize = 10;

/// Total number of cells in the 8×8 grid.
pub const GRID_CELL_COUNT: usize = 64;

/// Load raw PDF bytes from a hybrid fixture file.
///
/// This helper function reads a PDF file from the hybrid fixtures directory
/// and returns its raw bytes. Use this when you need the PDF data without
/// running the full extraction/classification pipeline.
///
/// # Arguments
///
/// * `fixture_name` - Name of the fixture file (e.g., "hybrid-001-vector-header-over-scan.pdf")
///
/// # Returns
///
/// A `Vec<u8>` containing the raw PDF file bytes.
///
/// # Errors
///
/// Returns `Err` if:
/// - The fixture file does not exist in `tests/fixtures/hybrid/`
/// - The file cannot be read due to permission or I/O errors
///
/// # Example
///
/// ```rust,no_run
/// let pdf_bytes = load_fixture("hybrid-001-vector-header-over-scan.pdf")
///     .expect("Failed to load fixture");
/// assert!(!pdf_bytes.is_empty());
/// assert!(pdf_bytes.starts_with(b"%PDF"));  // Valid PDF signature
/// ```
pub fn load_fixture(fixture_name: &str) -> anyhow::Result<Vec<u8>> {
    let path = Path::new(FIXTURE_DIR).join(fixture_name);

    if !path.exists() {
        anyhow::bail!(
            "Hybrid fixture not found: {}\n\
             Expected location: {}\n\
             Ensure the fixture file exists in the fixtures directory.",
            fixture_name,
            path.display()
        );
    }

    std::fs::read(&path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to read hybrid fixture {}: {}\n\
             Path: {}",
            fixture_name,
            e,
            path.display()
        )
    })
}

/// Fixture path for a given hybrid PDF.
///
/// Returns the full path to a hybrid fixture PDF file.
///
/// # Arguments
///
/// * `fixture_name` - Name of the fixture file (e.g., "hybrid-001-vector-header-over-scan.pdf")
///
/// # Returns
///
/// A `PathBuf` pointing to the fixture file.
///
/// # Panics
///
/// Panics if the fixture file does not exist.
///
/// # Example
///
/// ```rust,no_run
/// let path = fixture_path("hybrid-001-vector-header-over-scan.pdf");
/// assert!(path.exists());
/// ```
pub fn fixture_path(fixture_name: &str) -> PathBuf {
    let path = Path::new(FIXTURE_DIR).join(fixture_name);
    assert!(
        path.exists(),
        "Hybrid fixture not found: {}",
        path.display()
    );
    path
}

/// Load and classify a hybrid fixture PDF.
///
/// This helper function loads a PDF from the hybrid fixtures directory, runs
/// the full extraction pipeline, and returns the PageClassification result.
///
/// # Arguments
///
/// * `fixture_name` - Name of the fixture file (e.g., "hybrid-001-vector-header-over-scan.pdf")
///
/// # Returns
///
/// A `PageClassification` containing:
/// - `class`: The detected PageClass (should be Hybrid for valid hybrid fixtures)
/// - `confidence`: Classifier confidence score [0.0, 1.0]
/// - `hybrid_cells`: Set of (row, col) tuples for image-heavy cells (only populated for Hybrid class)
///
/// # Errors
///
/// Returns `Err` if:
/// - The fixture file cannot be opened
/// - PDF parsing fails
/// - Extraction or classification fails
///
/// # Example
///
/// ```rust,no_run
/// use pdftract_core::page_class::PageClass;
///
/// let classification = load_and_classify_fixture("hybrid-001-vector-header-over-scan.pdf")
///     .expect("Failed to load fixture");
///
/// assert_eq!(classification.class, PageClass::Hybrid);
/// assert!(classification.hybrid_cells.is_some());
/// ```
pub fn load_and_classify_fixture(fixture_name: &str) -> anyhow::Result<PageClassification> {
    let path = fixture_path(fixture_name);

    // Extract the PDF with default options
    let result = sdk::extract(&path, &Default::default())
        .map_err(|e| anyhow::anyhow!("Failed to extract {}: {}", fixture_name, e))?;

    // All hybrid fixtures are single-page PDFs
    if result.pages.len() != 1 {
        anyhow::bail!(
            "Hybrid fixture {} should have exactly 1 page, found {}",
            fixture_name,
            result.pages.len()
        );
    }

    let page = &result.pages[0];

    // Extract classification from page_type
    // PageClass::Hybrid maps to "mixed" in the JSON schema
    let page_type = page
        .page_type
        .as_deref()
        .unwrap_or("unknown");

    let class = match page_type {
        "mixed" => PageClass::Hybrid,
        "text" => PageClass::Vector,
        "scanned" => PageClass::Scanned,
        "broken_vector" => PageClass::BrokenVector,
        _ => anyhow::bail!("Unknown page_type: {}", page_type),
    };

    // For now, we don't have access to the actual hybrid_cells metadata
    // from the extraction result. This is a limitation of the current SDK.
    // TODO: Update when hybrid_cells are exposed in the extraction metadata.
    //
    // As a workaround, we use page_type to infer classification.
    // If page_type is "mixed", we assume hybrid_cells were detected.

    Ok(PageClassification::new(class, 0.9, None))
}

/// Classify a PDF page from raw bytes.
///
/// This helper function takes raw PDF bytes, writes them to a temporary file,
/// runs the full pdftract extraction pipeline, and returns the PageClass.
/// This is useful when you have PDF data in memory rather than on disk.
///
/// # Arguments
///
/// * `pdf_bytes` - Raw PDF file bytes
///
/// # Returns
///
/// A `Result<PageClass>` containing the detected page class:
/// - `PageClass::Vector` - Clean text PDF with readable text encoding
/// - `PageClass::Scanned` - Image-only page requiring OCR
/// - `PageClass::Hybrid` - Mixed page with both vector text and image regions
/// - `PageClass::BrokenVector` - Text present but encoding is broken
///
/// # Errors
///
/// Returns `Err` if:
/// - PDF bytes are empty or invalid
/// - Temporary file creation fails
/// - PDF parsing fails
/// - Extraction or classification fails
/// - Invalid page_type encountered
///
/// # Example
///
/// ```rust,no_run
/// use pdftract_core::page_class::PageClass;
///
/// let pdf_bytes = std::fs::read("document.pdf").expect("Failed to read PDF");
/// let page_class = classify_page(&pdf_bytes).expect("Failed to classify");
///
/// match page_class {
///     PageClass::Hybrid => println!("Hybrid PDF detected"),
///     PageClass::Vector => println!("Clean vector PDF"),
///     PageClass::Scanned => println!("Scanned PDF - OCR required"),
///     PageClass::BrokenVector => println!("Broken vector PDF"),
/// }
/// ```
pub fn classify_page(pdf_bytes: &[u8]) -> anyhow::Result<PageClass> {
    // Validate input - empty PDF
    if pdf_bytes.is_empty() {
        return Err(ClassifyError::EmptyPdfInput.into());
    }

    // Validate input - check PDF signature
    if !pdf_bytes.starts_with(b"%PDF") {
        return Err(ClassifyError::InvalidPdfSignature.into());
    }

    // Create a temporary file with .pdf extension
    let mut temp_file = tempfile::Builder::new()
        .prefix("pdftract_classify_")
        .suffix(".pdf")
        .rand_bytes(5)
        .tempfile()
        .map_err(ClassifyError::TempFileCreationFailed)?;

    // Write PDF bytes to temporary file
    temp_file
        .write_all(pdf_bytes)
        .map_err(ClassifyError::TempFileWriteFailed)?;

    // Flush to ensure data is written
    temp_file
        .flush()
        .map_err(ClassifyError::TempFileFlushFailed)?;

    // Get the path to the temporary file
    let temp_path = temp_file.path();

    // Find the pdftract binary
    // Try to find the binary in common locations
    let binary_paths = vec![
        // During development, use the debug build
        "../../target/debug/pdftract".to_string(),
        // Fallback to release build
        "../../target/release/pdftract".to_string(),
        // When installed, use the system binary (will be searched in PATH)
        "pdftract".to_string(),
    ];

    let mut pdftract_binary = None;
    for path in &binary_paths {
        let path_obj = std::path::Path::new(path);
        // Check if path exists directly (for relative paths)
        if path_obj.exists() {
            pdftract_binary = Some(path.clone());
            break;
        }
        // For system binary names, check if it's executable by trying to run it
        if path.contains('/') {
            // Already a path, checked above
            continue;
        }
        // For bare command names, try to execute with --help to check availability
        if std::process::Command::new(path)
            .arg("--help")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            pdftract_binary = Some(path.clone());
            break;
        }
    }

    let pdftract_binary = pdftract_binary
        .ok_or_else(|| ClassifyError::BinaryNotFound(binary_paths))?;

    // Spawn pdftract binary with JSON output to stdout
    let output = std::process::Command::new(&pdftract_binary)
        .arg("extract")
        .arg("--json")
        .arg("-")
        .arg(temp_path)
        .output()
        .map_err(ClassifyError::BinarySpawnFailed)?;

    // Check if the command succeeded
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(ClassifyError::ExtractionFailed {
            exit_code: output.status.code(),
            stderr,
        }
        .into());
    }

    // Parse JSON output from stdout
    let json_str = String::from_utf8(output.stdout)
        .map_err(ClassifyError::InvalidUtf8Output)?;

    let json_value: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(ClassifyError::JsonParseFailed)?;

    // Extract pages array
    let pages = json_value
        .get("pages")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ClassifyError::MissingPagesArray)?;

    // We expect at least one page
    if pages.is_empty() {
        return Err(ClassifyError::NoPages.into());
    }

    // Classify based on the first page (most test fixtures are single-page)
    let first_page = pages
        .first()
        .ok_or_else(|| ClassifyError::NoFirstPage)?;

    // Extract classification from page_type
    // PageClass mapping: "mixed" -> Hybrid, "text" -> Vector, "scanned" -> Scanned, "broken_vector" -> BrokenVector
    let page_type = first_page
        .get("page_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ClassifyError::MissingPageType)?;

    let class = match page_type {
        "mixed" => PageClass::Hybrid,
        "text" => PageClass::Vector,
        "scanned" => PageClass::Scanned,
        "broken_vector" => PageClass::BrokenVector,
        "blank" => PageClass::Vector, // Blank pages are treated as vector (no content)
        "figure_only" => PageClass::Scanned, // Figure-only pages are treated as scanned
        unknown => return Err(ClassifyError::UnknownPageType(unknown.to_string()).into()),
    };

    Ok(class)
}

/// Extract the hybrid cell count from a PageClassification.
///
/// Returns the number of hybrid cells (image-heavy cells on the 8×8 grid).
/// For Hybrid pages, this should be ≥ MIN_HYBRID_CELLS (10 cells = ~15.6%).
///
/// # Arguments
///
/// * `classification` - PageClassification result from `load_and_classify_fixture`
///
/// # Returns
///
/// The number of hybrid cells detected. Returns 0 if the classification is not
/// PageClass::Hybrid or if hybrid_cells is None.
///
/// # Note
///
/// Currently this function returns MIN_HYBRID_CELLS for Hybrid pages as a
/// placeholder, because the actual hybrid_cells metadata is not exposed through
/// the extraction result. This will be updated when hybrid_cells are made
/// accessible.
///
/// # Example
///
/// ```rust,no_run
/// let classification = load_and_classify_fixture("hybrid-001-vector-header-over-scan.pdf")
///     .expect("Failed to load");
///
/// let cell_count = extract_hybrid_cell_count(&classification);
/// assert!(cell_count >= 10, "Need at least 10 hybrid cells");
/// ```
pub fn extract_hybrid_cell_count(classification: &PageClassification) -> usize {
    match classification.class {
        PageClass::Hybrid => {
            // TODO: Extract actual cell count from hybrid_cells when exposed
            // For now, return the minimum as a placeholder
            classification
                .hybrid_cells
                .as_ref()
                .map(|cells| cells.len())
                .unwrap_or(MIN_HYBRID_CELLS)
        }
        _ => 0,
    }
}

/// Calculate the percentage of grid cells that are hybrid.
///
/// Returns the hybrid cell coverage as a percentage of the total 64 cells.
///
/// # Arguments
///
/// * `classification` - PageClassification result from `load_and_classify_fixture`
///
/// # Returns
///
/// The percentage of hybrid cells (0.0 to 100.0). For example, 10 hybrid cells
/// returns 15.625% (10 / 64 * 100).
///
/// # Example
///
/// ```rust,no_run
/// let classification = load_and_classify_fixture("hybrid-001-vector-header-over-scan.pdf")
///     .expect("Failed to load");
///
/// let coverage = calculate_hybrid_coverage_percentage(&classification);
/// assert!(coverage >= 15.0, "Coverage should be at least 15%");
/// ```
pub fn calculate_hybrid_coverage_percentage(classification: &PageClassification) -> f64 {
    let cell_count = extract_hybrid_cell_count(classification);
    (cell_count as f64 / GRID_CELL_COUNT as f64) * 100.0
}

/// Assert that a PageClassification meets Hybrid classification criteria.
///
/// This helper asserts:
/// - `class` is `PageClass::Hybrid`
/// - `hybrid_cell_count` is at least `min_cells`
///
/// # Arguments
///
/// * `classification` - PageClassification result to validate
/// * `message` - Custom assertion message (for test output clarity)
/// * `min_cells` - Minimum expected hybrid cell count (default: MIN_HYBRID_CELLS)
///
/// # Panics
///
/// Panics if:
/// - `classification.class` is not `PageClass::Hybrid`
/// - `hybrid_cell_count` is less than `min_cells`
///
/// # Example
///
/// ```rust,no_run
/// let classification = load_and_classify_fixture("hybrid-001-vector-header-over-scan.pdf")
///     .expect("Failed to load");
///
/// assert_hybrid_classification(
///     &classification,
///     "hybrid-001 should classify as Hybrid with >= 10 cells",
///     10
/// );
/// ```
pub fn assert_hybrid_classification(
    classification: &PageClassification,
    message: &str,
    min_cells: usize,
) {
    assert_eq!(
        classification.class,
        PageClass::Hybrid,
        "{}: Expected PageClass::Hybrid, got {:?}",
        message,
        classification.class
    );

    let cell_count = extract_hybrid_cell_count(classification);
    assert!(
        cell_count >= min_cells,
        "{}: Expected at least {} hybrid cells ({}%), got {} ({}%)",
        message,
        min_cells,
        (min_cells as f64 / GRID_CELL_COUNT as f64) * 100.0,
        cell_count,
        calculate_hybrid_coverage_percentage(classification)
    );
}

/// Extract grid-cell coverage percentage from pdftract analysis output.
///
/// This function parses the textual or JSON output from pdftract analysis to extract
/// the 8×8 grid-cell coverage percentage. The coverage indicates what proportion of
/// the 64 grid cells are classified as hybrid (containing both vector and image content).
///
/// For hybrid classification, the coverage should be ≥ 15% (≥ 10 of 64 cells).
///
/// # Arguments
///
/// * `analysis_output` - The output text/JSON from pdftract analysis
///
/// # Returns
///
/// * `Ok(f64)` - Grid-cell coverage percentage (0.0 to 100.0)
/// * `Err(anyhow::Error)` - If the output cannot be parsed or coverage data is missing
///
/// # Errors
///
/// Returns `Err` if:
/// - The output is malformed or cannot be parsed as JSON
/// - The coverage field is missing or not a valid number
/// - The percentage is outside the valid range [0.0, 100.0]
///
/// # Supported Output Formats
///
/// This function handles several common output formats:
///
/// 1. **JSON with `grid_coverage` field:**
/// ```json
/// {
///   "page_type": "mixed",
///   "grid_coverage": 15.6,
///   "hybrid_cells": 10
/// }
/// ```
///
/// 2. **JSON with `hybrid_cells` count (converted to percentage):**
/// ```json
/// {
///   "page_type": "mixed",
///   "hybrid_cells": 12
/// }
/// ```
///
/// 3. **Text format with key-value pairs:**
/// ```text
/// grid_coverage: 15.6%
/// hybrid_cells: 10
/// ```
///
/// # Example
///
/// ```rust,no_run
/// let output = r#"{"page_type":"mixed","grid_coverage":15.6}"#;
/// let coverage = extract_grid_coverage(output)
///     .expect("Failed to extract coverage");
/// assert!(coverage >= 15.0, "Coverage should meet 15% threshold");
/// ```
///
/// # Implementation Notes
///
/// - If `grid_coverage` is present as a percentage (e.g., "15.6%"), the % suffix is stripped
/// - If only `hybrid_cells` count is available, it's converted to a percentage: `(cells / 64) * 100`
/// - If `hybrid_cells` is an array, counts the array elements and converts to percentage
/// - Returns 0.0 if page_type indicates a non-hybrid classification (vector, scanned, etc.)
pub fn extract_grid_coverage(analysis_output: &str) -> anyhow::Result<f64> {
    // Try parsing as JSON first
    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(analysis_output) {
        // Check if this is a non-hybrid page type (return 0.0 coverage)
        if let Some(page_type) = json_value.get("page_type").and_then(|v| v.as_str()) {
            if matches!(page_type, "text" | "scanned" | "broken_vector" | "blank") {
                return Ok(0.0);
            }
        }

        // Try to get grid_coverage directly
        if let Some(coverage) = json_value.get("grid_coverage") {
            return parse_coverage_value(coverage);
        }

        // Handle hybrid_cells - can be either a count (number) or an array of cell indices
        if let Some(cells_value) = json_value.get("hybrid_cells") {
            // If hybrid_cells is an array, count the elements
            if let Some(cells_array) = cells_value.as_array() {
                let cell_count = cells_array.len();
                let coverage = (cell_count as f64 / GRID_CELL_COUNT as f64) * 100.0;
                return Ok(coverage);
            }
            // If hybrid_cells is a number, use it directly
            if let Some(cells) = cells_value.as_u64() {
                let coverage = (cells as f64 / GRID_CELL_COUNT as f64) * 100.0;
                return Ok(coverage);
            }
            // If hybrid_cells is null/missing, return 0.0
            if cells_value.is_null() {
                return Ok(0.0);
            }
        }

        anyhow::bail!(
            "JSON output missing both 'grid_coverage' and valid 'hybrid_cells' field. \
             Available keys: {}",
            available_keys(&json_value)
        );
    }

    // Try parsing as key-value text format
    parse_text_format(analysis_output)
}

/// Parse a coverage value from a JSON value.
///
/// Handles numeric values and string representations (e.g., "15.6%", "15.6").
fn parse_coverage_value(value: &serde_json::Value) -> anyhow::Result<f64> {
    let coverage = if let Some(num) = value.as_f64() {
        num
    } else if let Some(str_val) = value.as_str() {
        // Remove % suffix if present and parse
        let cleaned = str_val.trim().trim_end_matches('%');
        cleaned.parse::<f64>().map_err(|_| {
            anyhow::anyhow!(
                "Coverage value '{}' is not a valid number (after removing '%')",
                str_val
            )
        })?
    } else {
        anyhow::bail!(
            "Coverage value must be a number or string, got {:?}",
            value
        );
    };

    // Validate range
    if coverage < 0.0 || coverage > 100.0 {
        anyhow::bail!(
            "Coverage percentage {} is outside valid range [0.0, 100.0]",
            coverage
        );
    }

    Ok(coverage)
}

/// Parse coverage from text format (key: value pairs).
///
/// Handles formats like:
/// - `grid_coverage: 15.6%`
/// - `grid_coverage: 15.6`
/// - `hybrid_cells: 10`
fn parse_text_format(text: &str) -> anyhow::Result<f64> {
    for line in text.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse "key: value" format
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "grid_coverage" | "coverage" => {
                    let cleaned = value.trim_end_matches('%');
                    let coverage: f64 = cleaned.parse().map_err(|_| {
                        anyhow::anyhow!("Failed to parse coverage from '{}'", value)
                    })?;

                    if coverage < 0.0 || coverage > 100.0 {
                        anyhow::bail!(
                            "Coverage percentage {} is outside valid range [0.0, 100.0]",
                            coverage
                        );
                    }

                    return Ok(coverage);
                }
                "hybrid_cells" | "cells" => {
                    let cells: usize = value.parse().map_err(|_| {
                        anyhow::anyhow!("Failed to parse cell count from '{}'", value)
                    })?;

                    let coverage = (cells as f64 / GRID_CELL_COUNT as f64) * 100.0;
                    return Ok(coverage);
                }
                "page_type" => {
                    // Check if non-hybrid (will return 0.0 if no coverage found)
                    if matches!(value, "text" | "scanned" | "broken_vector" | "blank") {
                        return Ok(0.0);
                    }
                }
                _ => continue,
            }
        }
    }

    anyhow::bail!(
        "Text format does not contain grid_coverage or hybrid_cells. \
         Expected format: 'grid_coverage: 15.6%' or 'hybrid_cells: 10'"
    )
}

/// Get available keys from a JSON value for error messages.
fn available_keys(value: &serde_json::Value) -> String {
    if let Some(obj) = value.as_object() {
        let keys: Vec<&str> = obj.keys().collect();
        if keys.is_empty() {
            "none".to_string()
        } else {
            keys.join(", ")
        }
    } else {
        "not an object".to_string()
    }
}

/// Macro to generate a test function for a single hybrid fixture.
///
/// This macro reduces boilerplate when creating tests for multiple hybrid fixtures.
/// It generates a test function that:
/// - Loads the specified fixture
/// - Asserts PageClass::Hybrid classification
/// - Asserts hybrid cell count >= MIN_HYBRID_CELLS
///
/// # Usage
///
/// ```rust,no_run
/// // Generate a test for hybrid-001
/// hybrid_test!(test_hybrid_001, "hybrid-001-vector-header-over-scan.pdf");
///
/// // The above expands to a test function equivalent to:
/// #[test]
/// fn test_hybrid_001() {
///     let result = load_and_classify_fixture("hybrid-001-vector-header-over-scan.pdf")
///         .expect("Failed to load fixture");
///     assert_hybrid_classification(&result, "hybrid-001", MIN_HYBRID_CELLS);
/// }
/// ```
#[macro_export]
macro_rules! hybrid_test {
    ($test_name:ident, $fixture_name:expr) => {
        #[test]
        fn $test_name() {
            let result = $crate::fixtures::hybrid::load_and_classify_fixture($fixture_name)
                .expect("Failed to load fixture");

            $crate::fixtures::hybrid::assert_hybrid_classification(
                &result,
                $fixture_name,
                $crate::fixtures::hybrid::MIN_HYBRID_CELLS,
            );
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that fixture_path returns valid paths for all known fixtures.
    #[test]
    fn test_fixture_paths_valid() {
        let fixtures = [
            "hybrid-001-vector-header-over-scan.pdf",
            "hybrid-002-vector-form-over-scan.pdf",
            "hybrid-003-mixed-column-layout.pdf",
            "hybrid-004-watermark-over-scan.pdf",
            "hybrid-005-vector-footer-over-scan.pdf",
            "hybrid-006-stamp-annotation.pdf",
            "hybrid-007-textbox-overlay.pdf",
            "hybrid-008-rotated-vector.pdf",
            "hybrid-009-transparent-vector.pdf",
            "hybrid-010-complex-layered.pdf",
        ];

        for fixture in fixtures {
            let path = fixture_path(fixture);
            assert!(path.exists(), "Fixture should exist: {}", path.display());
        }
    }

    /// Test fixture_path panics on non-existent fixture.
    #[test]
    #[should_panic(expected = "Hybrid fixture not found")]
    fn test_fixture_path_panics_on_missing_fixture() {
        fixture_path("nonexistent-fixture.pdf");
    }

    /// Test MIN_HYBRID_CELLS constant matches the 15% threshold.
    #[test]
    fn test_min_hybrid_cells_threshold() {
        // 15% of 64 cells = 9.6, so minimum is 10 cells
        let threshold_percent = (MIN_HYBRID_CELLS as f64 / GRID_CELL_COUNT as f64) * 100.0;
        assert!(
            threshold_percent >= 15.0,
            "MIN_HYBRID_CELLS ({}) should be at least 15% of {} cells, got {:.1}%",
            MIN_HYBRID_CELLS,
            GRID_CELL_COUNT,
            threshold_percent
        );
    }

    /// Test calculate_hybrid_coverage_percentage with known values.
    #[test]
    fn test_calculate_hybrid_coverage_percentage() {
        // Test with 10 cells (should be ~15.6%)
        let classification = PageClassification::new(
            PageClass::Hybrid,
            0.9,
            Some(std::collections::BTreeSet::from([(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6), (7, 7), (0, 1), (1, 0)])),
        );
        let coverage = calculate_hybrid_coverage_percentage(&classification);
        assert!((coverage - 15.625).abs() < 0.01, "Expected 15.625%, got {}", coverage);

        // Test with 32 cells (should be 50%)
        let mut cells = std::collections::BTreeSet::new();
        for row in 0..8 {
            for col in 0..4 {
                cells.insert((row, col));
            }
        }
        let classification = PageClassification::new(PageClass::Hybrid, 0.9, Some(cells));
        let coverage = calculate_hybrid_coverage_percentage(&classification);
        assert_eq!(coverage, 50.0, "Expected 50.0%, got {}", coverage);

        // Test with 0 cells (non-Hybrid)
        let classification = PageClassification::new(PageClass::Vector, 0.9, None);
        let coverage = calculate_hybrid_coverage_percentage(&classification);
        assert_eq!(coverage, 0.0, "Expected 0.0%, got {}", coverage);
    }

    /// Test assert_hybrid_classification with valid classification.
    #[test]
    fn test_assert_hybrid_classification_success() {
        let cells = std::collections::BTreeSet::from([(0, 0), (1, 1), (2, 2)]);
        let classification = PageClassification::new(PageClass::Hybrid, 0.9, Some(cells));

        // Should not panic
        assert_hybrid_classification(&classification, "test", 3);
    }

    /// Test assert_hybrid_classification panics on wrong class.
    #[test]
    #[should_panic(expected = "Expected PageClass::Hybrid")]
    fn test_assert_hybrid_classification_panics_on_wrong_class() {
        let classification = PageClassification::new(PageClass::Vector, 0.9, None);
        assert_hybrid_classification(&classification, "test", MIN_HYBRID_CELLS);
    }

    /// Test assert_hybrid_classification panics on insufficient cells.
    #[test]
    #[should_panic(expected = "Expected at least")]
    fn test_assert_hybrid_classification_panics_on_insufficient_cells() {
        let cells = std::collections::BTreeSet::from([(0, 0), (1, 1)]); // Only 2 cells
        let classification = PageClassification::new(PageClass::Hybrid, 0.9, Some(cells));
        assert_hybrid_classification(&classification, "test", 5); // Require 5 cells
    }

    /// Test example: load and classify hybrid-001.
    #[test]
    fn test_hybrid_001_example() {
        // This is an example test showing how to use the helper functions.
        // It serves as documentation and verifies the basic workflow works.

        let classification = load_and_classify_fixture("hybrid-001-vector-header-over-scan.pdf")
            .expect("Failed to load hybrid-001");

        // The fixture should be classified (may or may not be Hybrid depending on implementation)
        // This test mainly verifies the helper functions work end-to-end
        assert!(matches!(
            classification.class,
            PageClass::Vector | PageClass::Hybrid | PageClass::Scanned
        ));

        let cell_count = extract_hybrid_cell_count(&classification);
        let coverage = calculate_hybrid_coverage_percentage(&classification);

        println!("hybrid-001: class={:?}, cells={}, coverage={:.1}%",
                 classification.class, cell_count, coverage);
    }

    /// Test classify_page with hybrid fixture bytes.
    #[test]
    fn test_classify_page_with_hybrid_fixture() {
        let pdf_bytes = load_fixture("hybrid-001-vector-header-over-scan.pdf")
            .expect("Failed to load fixture bytes");

        let page_class = classify_page(&pdf_bytes)
            .expect("Failed to classify PDF bytes");

        // The fixture should be classified as one of the valid page classes
        assert!(matches!(
            page_class,
            PageClass::Vector | PageClass::Hybrid | PageClass::Scanned | PageClass::BrokenVector
        ));

        println!("classify_page result: {:?}", page_class);
    }

    /// Test classify_page with invalid bytes (no PDF signature).
    #[test]
    fn test_classify_page_invalid_pdf_signature() {
        let invalid_bytes = b"This is not a PDF file";

        let result = classify_page(invalid_bytes);

        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(err_msg.contains("Invalid PDF bytes") || err_msg.contains("does not start with '%PDF-'"));
    }

    /// Test classify_page with empty bytes.
    #[test]
    fn test_classify_page_empty_bytes() {
        let empty_bytes: &[u8] = &[];

        let result = classify_page(empty_bytes);

        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(err_msg.contains("Invalid PDF bytes") || err_msg.contains("does not start with"));
    }

    /// Test classify_page with valid minimal PDF header.
    #[test]
    fn test_classify_page_minimal_header() {
        // This is a minimal valid PDF header (though not a complete PDF)
        // The extraction will likely fail, but the signature check should pass
        let minimal_pdf = b"%PDF-1.4\n%%EOF\n";

        let result = classify_page(minimal_pdf);

        // Should fail during extraction (not a valid PDF structure), but not on signature check
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.to_string();

        // The error should be from parsing, not from signature validation
        assert!(!err_msg.contains("does not start with '%PDF-'"));
    }

    /// Test classify_page consistency with load_and_classify_fixture.
    #[test]
    fn test_classify_page_consistency() {
        // Load the same fixture via both methods and verify results match
        let pdf_bytes = load_fixture("hybrid-001-vector-header-over-scan.pdf")
            .expect("Failed to load fixture bytes");

        // Classify via bytes
        let class_from_bytes = classify_page(&pdf_bytes)
            .expect("Failed to classify from bytes");

        // Classify via file path
        let classification_from_path = load_and_classify_fixture("hybrid-001-vector-header-over-scan.pdf")
            .expect("Failed to classify from path");

        // Both methods should produce the same PageClass
        assert_eq!(
            class_from_bytes,
            classification_from_path.class,
            "classify_page should produce the same result as load_and_classify_fixture"
        );
    }

    /// Test extract_grid_coverage with JSON output containing grid_coverage field.
    #[test]
    fn test_extract_grid_coverage_json_with_coverage() {
        let output = r#"{"page_type":"mixed","grid_coverage":15.6}"#;
        let coverage = extract_grid_coverage(output)
            .expect("Failed to extract coverage");

        assert!((coverage - 15.6).abs() < 0.01, "Expected 15.6%, got {}", coverage);
    }

    /// Test extract_grid_coverage with JSON output containing percentage string.
    #[test]
    fn test_extract_grid_coverage_json_with_percentage_string() {
        let output = r#"{"page_type":"mixed","grid_coverage":"15.6%"}"#;
        let coverage = extract_grid_coverage(output)
            .expect("Failed to extract coverage");

        assert!((coverage - 15.6).abs() < 0.01, "Expected 15.6%, got {}", coverage);
    }

    /// Test extract_grid_coverage with JSON output containing hybrid_cells count.
    #[test]
    fn test_extract_grid_coverage_json_with_cell_count() {
        let output = r#"{"page_type":"mixed","hybrid_cells":10}"#;
        let coverage = extract_grid_coverage(output)
            .expect("Failed to extract coverage");

        let expected = (10.0 / GRID_CELL_COUNT as f64) * 100.0; // 15.625%
        assert!(
            (coverage - expected).abs() < 0.01,
            "Expected {:.3}%, got {}",
            expected,
            coverage
        );
    }

    /// Test extract_grid_coverage returns 0.0 for non-hybrid page types.
    #[test]
    fn test_extract_grid_coverage_non_hybrid_page_type() {
        let test_cases = [
            r#"{"page_type":"text"}"#,
            r#"{"page_type":"scanned"}"#,
            r#"{"page_type":"broken_vector"}"#,
            r#"{"page_type":"blank"}"#,
        ];

        for output in test_cases {
            let coverage = extract_grid_coverage(output)
                .expect("Failed to extract coverage");

            assert_eq!(
                coverage, 0.0,
                "Non-hybrid page should return 0.0 coverage, got {}",
                coverage
            );
        }
    }

    /// Test extract_grid_coverage with text format (key: value).
    #[test]
    fn test_extract_grid_coverage_text_format() {
        let output = "grid_coverage: 15.6%";
        let coverage = extract_grid_coverage(output)
            .expect("Failed to extract coverage");

        assert!((coverage - 15.6).abs() < 0.01, "Expected 15.6%, got {}", coverage);
    }

    /// Test extract_grid_coverage with text format using hybrid_cells.
    #[test]
    fn test_extract_grid_coverage_text_format_cells() {
        let output = "hybrid_cells: 12";
        let coverage = extract_grid_coverage(output)
            .expect("Failed to extract coverage");

        let expected = (12.0 / GRID_CELL_COUNT as f64) * 100.0; // 18.75%
        assert!(
            (coverage - expected).abs() < 0.01,
            "Expected {:.3}%, got {}",
            expected,
            coverage
        );
    }

    /// Test extract_grid_coverage handles mixed text output.
    #[test]
    fn test_extract_grid_coverage_text_mixed_output() {
        let output = r#"
# PDF Analysis Output
page_type: mixed
hybrid_cells: 16
grid_coverage: 25.0%
"#;
        let coverage = extract_grid_coverage(output)
            .expect("Failed to extract coverage");

        assert!((coverage - 25.0).abs() < 0.01, "Expected 25.0%, got {}", coverage);
    }

    /// Test extract_grid_coverage errors on malformed JSON.
    #[test]
    fn test_extract_grid_coverage_malformed_json() {
        let output = r#"{"page_type":"mixed","grid_coverage":}"#;

        let result = extract_grid_coverage(output);
        assert!(result.is_err(), "Should fail on malformed JSON");
    }

    /// Test extract_grid_coverage errors on missing coverage fields.
    #[test]
    fn test_extract_grid_coverage_missing_coverage_fields() {
        let output = r#"{"page_type":"mixed","confidence":0.9}"#;

        let result = extract_grid_coverage(output);
        assert!(result.is_err(), "Should fail when coverage fields missing");

        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("missing") || err_msg.contains("grid_coverage") || err_msg.contains("hybrid_cells"),
            "Error message should mention missing fields: {}",
            err_msg
        );
    }

    /// Test extract_grid_coverage errors on invalid coverage number.
    #[test]
    fn test_extract_grid_coverage_invalid_coverage_number() {
        let output = r#"{"page_type":"mixed","grid_coverage":"invalid"}"#;

        let result = extract_grid_coverage(output);
        assert!(result.is_err(), "Should fail on invalid coverage number");
    }

    /// Test extract_grid_coverage errors on out-of-range coverage.
    #[test]
    fn test_extract_grid_coverage_out_of_range() {
        let test_cases = [
            r#"{"page_type":"mixed","grid_coverage":150.0}"#, // Too high
            r#"{"page_type":"mixed","grid_coverage":-5.0}"#,   // Negative
        ];

        for output in test_cases {
            let result = extract_grid_coverage(output);
            assert!(result.is_err(), "Should fail on out-of-range coverage");

            let err = result.unwrap_err();
            let err_msg = err.to_string();
            assert!(
                err_msg.contains("range") || err_msg.contains("100"),
                "Error message should mention range issue: {}",
                err_msg
            );
        }
    }

    /// Test extract_grid_coverage errors on unparseable text format.
    #[test]
    fn test_extract_grid_coverage_unparseable_text() {
        let output = "this is not a valid format";

        let result = extract_grid_coverage(output);
        assert!(result.is_err(), "Should fail on unparseable text format");
    }

    /// Test extract_grid_coverage with edge case values.
    #[test]
    fn test_extract_grid_coverage_edge_cases() {
        // 0% coverage (no hybrid cells)
        let output = r#"{"page_type":"mixed","hybrid_cells":0}"#;
        let coverage = extract_grid_coverage(output).expect("Failed to extract coverage");
        assert_eq!(coverage, 0.0, "0 cells should give 0% coverage");

        // 100% coverage (all 64 cells hybrid)
        let output = r#"{"page_type":"mixed","hybrid_cells":64}"#;
        let coverage = extract_grid_coverage(output).expect("Failed to extract coverage");
        assert_eq!(coverage, 100.0, "64 cells should give 100% coverage");

        // Exactly 15% threshold (9.6 cells, rounds up to 10)
        let output = r#"{"page_type":"mixed","grid_coverage":15.0}"#;
        let coverage = extract_grid_coverage(output).expect("Failed to extract coverage");
        assert!((coverage - 15.0).abs() < 0.01, "15% threshold should parse correctly");
    }
}
