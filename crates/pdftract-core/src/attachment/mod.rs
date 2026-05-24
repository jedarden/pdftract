//! Attachment extraction module.
//!
//! This module handles extraction of embedded files and attachments from PDF documents.
//!
//! # Submodules
//!
//! - [`associated_files`]: PDF 2.0 /AF (Associated Files) array walker

pub mod associated_files;

// Re-export key types for convenience
pub use associated_files::{walk_af_array, AssociatedFileEntry};
