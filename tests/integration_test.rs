// Integration tests for pdftract
//
// This file serves as the main entry point for integration tests,
// organizing them into logical modules for maintainability.

// Standard library imports
use std::path::PathBuf;

// PyPdfProcessor import from pdftract-py crate
// Note: import path uses the [lib] name from Cargo.toml (pdftract), not the package name
use pdftract::PyPdfProcessor;

// Additional commonly used imports from pdftract-py crate for testing
// These will be used when actual test cases are implemented
// Uncomment as needed:
// use pdftract::extract; // Extract function (Python-facing)
// use pdftract_core::{ExtractionOptions, PageResult, TableJson}; // Core types from pdftract-core
// Note: Exception types are Python-specific and not exposed in Rust API

// PyO3 imports for Python bindings testing
// Uncomment when testing Python integration
// use pyo3::{Python, PyResult, types::PyDict};

// Test helper utilities and fixtures
mod test_helpers;

// Individual integration test cases
mod test_cases;
