//! Markdown output module.
//!
//! This module provides Markdown emission functionality for pdftract.
//! It includes support for block-level Markdown emission, inline span styling,
//! and footnote emission (when Phase 7 footnote detection is implemented).

pub mod footnotes;

pub use footnotes::{emit_footnote_def, emit_footnote_defs, emit_footnote_ref, PageFootnotes};
