// Integration tests for pdftract
//
// This file serves as the main entry point for integration tests,
// organizing them into logical modules for maintainability.

// Standard library imports
use std::path::{Path, PathBuf};

// Core pdftract imports for integration testing
use pdftract_core::{ExtractionOptions, OutputOptions};

// PyPdfProcessor import from pdftract-py crate (lib name "pdftract" in Cargo.toml)
use pdftract::PyPdfProcessor;

// PyO3 imports for Python bindings testing
// Uncomment when testing Python integration
// use pyo3::{Python, PyResult, types::PyDict};

// Test helper utilities and fixtures
mod test_helpers;

// Individual integration test cases
mod test_cases;
