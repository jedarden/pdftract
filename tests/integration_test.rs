// Integration tests for pdftract
//
// This file serves as the main entry point for integration tests,
// organizing them into logical modules for maintainability.

// Standard library imports
use std::path::PathBuf;

// PyPdfProcessor import from pdftract-py crate
// Note: import path uses the [lib] name from Cargo.toml (pdftract), not the package name
use pdftract::PyPdfProcessor;

// Core types from pdftract-core
use pdftract_core::{AttachmentJson, ExtractionOptions, PageResult, TableJson};

// Exception types from pdftract crate (PyO3 exceptions)
use pdftract::{
    CorruptPdfError,
    EncryptionError,
    PdftractError,
    ReceiptVerifyError,
    RemoteFetchInterruptedError,
    SourceUnreachableError,
    TlsError,
    UnsupportedOperationError,
};

// PyO3 imports for Python bindings testing
// Uncomment when testing Python integration
// use pyo3::{Python, PyResult, types::PyDict};

// Test helper utilities and fixtures
mod test_helpers;

// Individual integration test cases
mod test_cases;
