//! Markdown output module.
//!
//! This module provides Markdown emission functionality for pdftract.
//! It includes support for block-level Markdown emission, inline span styling,
//! footnote emission (when Phase 7 footnote detection is implemented), and
//! inline link emission (when Phase 7.6 link annotations are available).

pub mod footnotes;
pub mod links;

pub use footnotes::{emit_footnote_def, emit_footnote_defs, emit_footnote_ref, PageFootnotes};
pub use links::{
    concatenate_anchor_text, emit_inline_link, emit_page_links_from_json, find_spans_in_link_json,
    resolve_link_target_from_json, LinkTarget,
};
