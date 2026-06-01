//! Attachment extraction module.
//!
//! This module handles extraction of embedded files and attachments from PDF documents.
//!
//! # Submodules
//!
//! - [`associated_files`]: PDF 2.0 /AF (Associated Files) array walker
//! - [`filespec`]: Filespec dictionary and EF stream decoder (PDF 1.7+)
//! - [`name_tree`]: /EmbeddedFiles name tree walker (PDF 1.7)

pub mod associated_files;
pub mod filespec;
pub mod name_tree;

// Re-export key types for convenience
pub use associated_files::{walk_af_array, AssociatedFileEntry};
pub use filespec::{extract_one, AttachmentBuilder};
pub use name_tree::{walk_embedded_files, EmbeddedFileEntry};
