//! Test error type imports from pdftract_core crate
//!
//! This test verifies that all error types can be imported
//! from the pdftract_core crate.

use pdftract_core::{
    document::DocumentError,
    page_extraction_error::PageExtractionError,
};

#[cfg(feature = "decrypt")]
use pdftract_core::encryption::DecryptError;

#[cfg(feature = "decrypt")]
use pdftract_core::encryption::decryptor::DecryptionError;

#[test]
fn test_pdftract_core_error_types_import() {
    // This test verifies that the core error types are accessible
    assert!(true);
}

#[cfg(feature = "decrypt")]
#[test]
fn test_encryption_error_types_import() {
    // This test verifies that encryption error types are accessible
    assert!(true);
}
