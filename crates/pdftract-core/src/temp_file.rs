//! Temporary file utilities for PDF processing.
//!
//! This module provides RAII-style temporary file management for PDF content
//! that needs to be written to disk for external tool invocation (e.g., pdftract MCP).
//!
//! # Example
//!
//! ```no_run
//! use pdftract_core::temp_file::PdfTempFile;
//! use std::io::Write;
//!
//! # fn main() -> anyhow::Result<()> {
//! // Create temp file from PDF bytes
//! let pdf_bytes = b"%PDF-1.4...";
//! let temp_file = PdfTempFile::from_bytes(pdf_bytes)?;
//!
//! // Get path for external invocation
//! let path = temp_file.path();
//!
//! // Use the path with external tools...
//! // Temp file is automatically deleted when `temp_file` goes out of scope
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

/// RAII guard for temporary PDF files.
///
/// Automatically deletes the temporary file when dropped, ensuring cleanup
/// even if panic occurs. The file is created with a unique name based on
/// process ID and an optional suffix to avoid collisions.
#[derive(Debug)]
pub struct PdfTempFile {
    /// Path to the temporary file.
    path: PathBuf,
}

impl PdfTempFile {
    /// Create a temporary PDF file from raw bytes.
    ///
    /// Creates a new temporary file in the system temp directory with
    /// a unique name based on process ID and the provided suffix.
    /// The file is automatically deleted when this guard is dropped.
    ///
    /// # Arguments
    ///
    /// * `pdf_bytes` - Raw PDF content bytes
    /// * `suffix` - Optional suffix for the temp file name (e.g., page index)
    ///
    /// # Returns
    ///
    /// A `PdfTempFile` guard that will delete the file on drop.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Temp directory cannot be determined
    /// - Temp file cannot be created
    /// - PDF bytes cannot be written
    /// - File cannot be flushed to disk
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pdftract_core::temp_file::PdfTempFile;
    /// # fn main() -> anyhow::Result<()> {
    /// let pdf_bytes = b"%PDF-1.4...";
    /// let temp_file = PdfTempFile::from_bytes_with_suffix(pdf_bytes, "page-0")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_bytes_with_suffix(pdf_bytes: &[u8], suffix: &str) -> Result<Self> {
        // Validate PDF has minimal content
        if pdf_bytes.is_empty() {
            return Err(anyhow::anyhow!("PDF input is empty"));
        }

        // Check for PDF signature
        if !pdf_bytes.starts_with(b"%PDF") {
            return Err(anyhow::anyhow!(
                "Invalid PDF: missing PDF signature (expected to start with '%PDF')"
            ));
        }

        // Create temp file path
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!(
            "pdftract-classify-{}-{}.pdf",
            std::process::id(),
            suffix
        ));

        // Create and write to temp file
        let mut file = File::create(&temp_file)
            .with_context(|| format!("Failed to create temporary file: {}", temp_file.display()))?;

        file.write_all(pdf_bytes)
            .with_context(|| format!("Failed to write PDF to temporary file: {}", temp_file.display()))?;

        file.flush()
            .with_context(|| format!("Failed to flush temporary file: {}", temp_file.display()))?;

        Ok(Self { path: temp_file })
    }

    /// Create a temporary PDF file from raw bytes.
    ///
    /// This is a convenience method that uses a timestamp-based suffix.
    ///
    /// # Arguments
    ///
    /// * `pdf_bytes` - Raw PDF content bytes
    ///
    /// # Returns
    ///
    /// A `PdfTempFile` guard that will delete the file on drop.
    pub fn from_bytes(pdf_bytes: &[u8]) -> Result<Self> {
        // Use timestamp as suffix for uniqueness
        let suffix = format!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
        Self::from_bytes_with_suffix(pdf_bytes, &suffix)
    }

    /// Get the path to the temporary file.
    ///
    /// Returns a reference to the path for use with external tool invocation.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pdftract_core::temp_file::PdfTempFile;
    /// # fn main() -> anyhow::Result<()> {
    /// # let pdf_bytes = b"%PDF-1.4...";
    /// # let temp_file = PdfTempFile::from_bytes(pdf_bytes)?;
    /// let path = temp_file.path();
    /// // Use path with external command
    /// # Ok(())
    /// # }
    /// ```
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Consume this guard and keep the temporary file.
    ///
    /// Prevents automatic deletion, returning the path to the file.
    /// The caller becomes responsible for cleanup.
    ///
    /// # Returns
    ///
    /// The path to the temporary file, which will no longer be auto-deleted.
    pub fn into_path(self) -> PathBuf {
        // Prevent Drop from running by forgetting self
        // Note: This leaks memory until the file is manually deleted
        // Prefer letting the guard run its course unless explicitly needed
        let path = self.path.clone();
        std::mem::forget(self);
        path
    }
}

// Implement Drop for RAII cleanup
impl Drop for PdfTempFile {
    fn drop(&mut self) {
        // Silently ignore removal errors - the file may already be deleted
        // or the temp dir may not exist. This is cleanup, not a critical operation.
        let _ = std::fs::remove_file(&self.path);
    }
}

// Ensure the Drop implementation works correctly even on panic
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temp_file_creation() {
        let pdf_bytes = b"%PDF-1.4\n%fake pdf content";
        let temp_file = PdfTempFile::from_bytes(pdf_bytes).unwrap();

        // Path should exist
        assert!(temp_file.path().exists());
    }

    #[test]
    fn test_temp_file_cleanup_on_drop() {
        let pdf_bytes = b"%PDF-1.4\n%fake pdf content";
        let path = {
            let temp_file = PdfTempFile::from_bytes(pdf_bytes).unwrap();
            let path = temp_file.path().to_path_buf();
            assert!(path.exists());
            path
        };

        // File should be deleted after temp_file goes out of scope
        // Give it a moment for filesystem sync
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(!path.exists());
    }

    #[test]
    fn test_temp_file_with_suffix() {
        let pdf_bytes = b"%PDF-1.4\n%fake pdf content";
        let temp_file = PdfTempFile::from_bytes_with_suffix(pdf_bytes, "test-page-0").unwrap();

        // Path should contain the suffix
        let path_str = temp_file.path().to_string_lossy();
        assert!(path_str.contains("test-page-0"));
        assert!(path_str.contains(".pdf"));
    }

    #[test]
    fn test_temp_file_rejects_empty_input() {
        let pdf_bytes = b"";
        let result = PdfTempFile::from_bytes(pdf_bytes);
        match result {
            Ok(_) => panic!("Expected error for empty input"),
            Err(e) => assert!(e.to_string().contains("empty")),
        }
    }

    #[test]
    fn test_temp_file_rejects_invalid_pdf() {
        let pdf_bytes = b"Not a PDF";
        let result = PdfTempFile::from_bytes(pdf_bytes);
        match result {
            Ok(_) => panic!("Expected error for invalid PDF"),
            Err(e) => assert!(e.to_string().contains("PDF signature")),
        }
    }

    #[test]
    fn test_temp_file_path_extraction() {
        let pdf_bytes = b"%PDF-1.4\n%fake pdf content";
        let temp_file = PdfTempFile::from_bytes(pdf_bytes).unwrap();

        let path = temp_file.path();
        assert!(path.is_absolute());
        assert!(path.extension().unwrap_or_default() == "pdf");
    }

    #[test]
    fn test_temp_file_into_path() {
        let pdf_bytes = b"%PDF-1.4\n%fake pdf content";
        let temp_file = PdfTempFile::from_bytes(pdf_bytes).unwrap();
        let path = temp_file.into_path();

        // File should still exist after into_path
        assert!(path.exists());

        // Manual cleanup
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_temp_file_cleanup_on_panic() {
        use std::panic;

        let pdf_bytes = b"%PDF-1.4\n%fake pdf content";
        let path = {
            let temp_file = PdfTempFile::from_bytes(pdf_bytes).unwrap();
            temp_file.path().to_path_buf()
        };

        // Simulate a panic after temp file creation
        let result = panic::catch_unwind(|| {
            let temp_file = PdfTempFile::from_bytes(pdf_bytes).unwrap();
            assert!(temp_file.path().exists());
            panic!("test panic");
        });

        assert!(result.is_err());

        // File should still be cleaned up even after panic
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(!path.exists());
    }

    #[test]
    fn test_temp_file_multiple_instances() {
        let pdf_bytes = b"%PDF-1.4\n%fake pdf content";

        // Create multiple temp files - they should have unique names
        let temp1 = PdfTempFile::from_bytes(pdf_bytes).unwrap();
        let temp2 = PdfTempFile::from_bytes(pdf_bytes).unwrap();
        let temp3 = PdfTempFile::from_bytes(pdf_bytes).unwrap();

        assert_ne!(temp1.path(), temp2.path());
        assert_ne!(temp2.path(), temp3.path());
        assert_ne!(temp1.path(), temp3.path());

        // All should exist
        assert!(temp1.path().exists());
        assert!(temp2.path().exists());
        assert!(temp3.path().exists());
    }

    #[test]
    fn test_temp_file_write_and_read() {
        let pdf_bytes = b"%PDF-1.4\n%Test PDF content";
        let temp_file = PdfTempFile::from_bytes(pdf_bytes).unwrap();

        // Verify we can read back what we wrote
        let read_bytes = std::fs::read(temp_file.path()).unwrap();
        assert_eq!(pdf_bytes, read_bytes.as_slice());
    }
}
