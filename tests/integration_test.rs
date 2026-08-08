// Integration tests for pdftract
//
// This file serves as the main entry point for integration tests,
// organizing them into logical modules for maintainability.

// Standard library imports
use std::path::PathBuf;

// Core types from pdftract-core (safe for test binaries)
use pdftract_core::{AttachmentJson, ExtractionOptions, PageResult, TableJson};

// Note: Python binding types (PyPdfProcessor, PyO3 exceptions) cannot be imported
// directly in test binaries due to linker constraints. They should only be used
// within a Python context or tested via Python-based test harnesses.

// Test helper utilities and fixtures
mod test_helpers;

// Individual integration test cases
mod test_cases;
