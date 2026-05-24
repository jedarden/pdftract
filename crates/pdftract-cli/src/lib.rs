//! pdftract CLI library.
//!
//! This library exports the CLI's internal modules for integration testing.

pub mod grep;
pub mod inspect;
pub mod mcp;

// Re-export diagnostics for testing
pub use pdftract_core::diagnostics::{DiagCode, DiagInfo, DIAGNOSTIC_CATALOG};
