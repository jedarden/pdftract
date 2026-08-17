// Integration tests for pdftract
//
// This file serves as the main entry point for integration tests,
// organizing them into logical modules for maintainability.

// Standard library imports
use std::path::PathBuf;

// PyPdfProcessor import from pdftract-py crate
// Note: The import path uses the [lib] name from Cargo.toml (pdftract_py), matching the package name
use pdftract_py::PyPdfProcessor;

// Exception types from pdftract-py crate (PyO3 exceptions)
use pdftract_py::{
    CorruptPdfError, EncryptionError, PdftractError, ReceiptVerifyError,
    RemoteFetchInterruptedError, SourceUnreachableError, TlsError, UnsupportedOperationError,
};

// Core types from pdftract-core (safe for test binaries)
use pdftract_core::{AttachmentJson, ExtractionOptions, PageResult, TableJson};

// Note: PyO3-specific exceptions and Python-only types still require Python context.
// PyPdfProcessor is available for import as it's a Rust-exposed type.

// Test helper utilities and fixtures
mod test_helpers;

// Individual integration test cases
mod test_cases;
