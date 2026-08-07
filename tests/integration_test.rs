// Integration tests for pdftract
//
// This file serves as the main entry point for integration tests,
// organizing them into logical modules for maintainability.

// Standard library imports
use std::path::PathBuf;

// PyPdfProcessor import from pdftract-py crate
// Note: lib name is "pdftract" (defined in crates/pdftract-py/Cargo.toml)
// Using the correct lib name as specified in crates/pdftract-py/Cargo.toml [lib]
use pdftract::PyPdfProcessor;

// Additional commonly used imports for testing
// These will be used when actual test cases are implemented
// use pdftract::extract; // Re-exported when needed
// use pdftract::{ExtractionOptions, PageResult, TableJson}; // Re-exported types when needed

// Exception types from pdftract-py crate
use pdftract::{
    PdftractError,
    EncryptionError,
    CorruptPdfError,
    SourceUnreachableError,
    RemoteFetchInterruptedError,
    TlsError,
    ReceiptVerifyError,
    UnsupportedOperationError,
};

// PyO3 imports for Python bindings testing
// Uncomment when testing Python integration
// use pyo3::{Python, PyResult, types::PyDict};

// Test helper utilities and fixtures
mod test_helpers;

// Individual integration test cases
mod test_cases;
