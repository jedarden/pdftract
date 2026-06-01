//! Markdown footnote emission.
//!
//! This module implements footnote emission for the Markdown sink.
//! Each footnote reference span gets a unique numeric ID assigned in
//! document order; emits [^N] in body where the ref appears; emits
//! [^N]: footnote text definitions at end of page (per v1.0 decision).
//!
//! # Footnote emission format
//!
//! This module uses GitHub Flavored Markdown (GFM) footnote syntax:
//! - Footnote reference in body: `[^N]` where N is a numeric ID
//! - Footnote definition at page end: `[^N]: <text>`
//!
//! # Phase 7 integration
//!
//! Footnote detection is implemented in Phase 7. This module provides
//! the emission infrastructure that will be used by Phase 7 when
//! footnote data is available. For documents without footnotes (current
//! state, as Phase 7 is not yet implemented), this code path is a no-op.
//!
//! # Future: end-of-document option
//!
//! Per v1.0 decision, footnote definitions are emitted at the end of
//! each page. A future option may allow emitting all footnotes at the
//! end of the document instead (tradeoff: proximity vs flow).

use std::collections::HashMap;

/// Footnote data for a single page.
///
/// This structure represents the footnote information that will be
/// provided by Phase 7 footnote detection. For now, it's a stub that
/// can be populated when Phase 7 is implemented.
///
/// # Fields
///
/// * `refs` - Map from span index to footnote ID (assigned in document order)
/// * `definitions` - Map from footnote ID to footnote text
#[derive(Debug, Clone, Default)]
pub struct PageFootnotes {
    /// Map from span index (within the page's spans array) to footnote ID.
    ///
    /// When Phase 7 footnote detection is implemented, this will be populated
    /// with the span indices that contain footnote references, mapped to their
    /// assigned footnote IDs.
    pub refs: HashMap<usize, u32>,

    /// Map from footnote ID to footnote text.
    ///
    /// When Phase 7 footnote detection is implemented, this will contain
    /// the actual footnote text for each footnote ID.
    pub definitions: HashMap<u32, String>,
}

impl PageFootnotes {
    /// Create a new empty PageFootnotes.
    ///
    /// Returns a structure with no footnote references or definitions.
    /// This is the default state for pages without footnotes.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if this page has any footnotes.
    ///
    /// Returns true if there are any footnote references or definitions.
    pub fn is_empty(&self) -> bool {
        self.refs.is_empty() && self.definitions.is_empty()
    }

    /// Add a footnote reference.
    ///
    /// # Arguments
    ///
    /// * `span_index` - The span index (within the page's spans array)
    /// * `footnote_id` - The footnote ID (numeric, assigned in document order)
    pub fn add_ref(&mut self, span_index: usize, footnote_id: u32) {
        self.refs.insert(span_index, footnote_id);
    }

    /// Add a footnote definition.
    ///
    /// # Arguments
    ///
    /// * `footnote_id` - The footnote ID
    /// * `text` - The footnote text
    pub fn add_definition(&mut self, footnote_id: u32, text: String) {
        self.definitions.insert(footnote_id, text);
    }

    /// Get the footnote ID for a given span index.
    ///
    /// Returns None if the span is not a footnote reference.
    pub fn get_footnote_id(&self, span_index: usize) -> Option<u32> {
        self.refs.get(&span_index).copied()
    }

    /// Get the footnote text for a given footnote ID.
    ///
    /// Returns None if the footnote ID has no definition.
    pub fn get_definition(&self, footnote_id: u32) -> Option<&str> {
        self.definitions.get(&footnote_id).map(|s| s.as_str())
    }
}

/// Emit a footnote reference as Markdown.
///
/// This function emits a footnote reference in GFM syntax: `[^N]`
/// where N is the footnote ID.
///
/// # Arguments
///
/// * `footnote_id` - The footnote ID
///
/// # Returns
///
/// A markdown string containing the footnote reference.
///
/// # Example
///
/// ```
/// use pdftract_core::output::markdown::footnotes::emit_footnote_ref;
///
/// let md = emit_footnote_ref(1);
/// assert_eq!(md, "[^1]");
/// ```
pub fn emit_footnote_ref(footnote_id: u32) -> String {
    format!("[^{}]", footnote_id)
}

/// Emit a footnote definition as Markdown.
///
/// This function emits a footnote definition in GFM syntax: `[^N]: <text>`
/// where N is the footnote ID and text is the footnote text.
///
/// Per the acceptance criteria, empty footnote text emits `[^N]: (empty)`
/// as a placeholder so the reference is at least visible.
///
/// # Arguments
///
/// * `footnote_id` - The footnote ID
/// * `text` - The footnote text (may be empty)
///
/// # Returns
///
/// A markdown string containing the footnote definition.
///
/// # Example
///
/// ```
/// use pdftract_core::output::markdown::footnotes::emit_footnote_def;
///
/// let md = emit_footnote_def(1, "Footnote text");
/// assert_eq!(md, "[^1]: Footnote text\n");
///
/// let md_empty = emit_footnote_def(2, "");
/// assert_eq!(md_empty, "[^2]: (empty)\n");
/// ```
pub fn emit_footnote_def(footnote_id: u32, text: &str) -> String {
    let text = if text.is_empty() {
        "(empty)".to_string()
    } else {
        text.to_string()
    };
    format!("[^{}]: {}\n", footnote_id, text)
}

/// Emit all footnote definitions for a page.
///
/// This function collects all footnote definitions for the page and
/// emits them at the end of the page content, per the v1.0 decision.
///
/// The output includes a blank line before the definitions block for
/// pretty formatting.
///
/// # Arguments
///
/// * `footnotes` - The page footnotes data
///
/// # Returns
///
/// A markdown string containing all footnote definitions, or an empty
/// string if there are no footnotes.
///
/// # Example
///
/// ```
/// use pdftract_core::output::markdown::footnotes::{emit_footnote_defs, PageFootnotes};
///
/// let mut footnotes = PageFootnotes::new();
/// footnotes.add_definition(1, "First footnote".to_string());
/// footnotes.add_definition(2, "Second footnote".to_string());
///
/// let md = emit_footnote_defs(&footnotes);
/// assert!(md.contains("\n[^1]: First footnote\n"));
/// assert!(md.contains("[^2]: Second footnote\n"));
/// ```
pub fn emit_footnote_defs(footnotes: &PageFootnotes) -> String {
    if footnotes.is_empty() {
        return String::new();
    }

    let mut result = String::from("\n"); // Blank line before definitions

    // Collect and sort footnote IDs for deterministic output
    let mut ids: Vec<u32> = footnotes.definitions.keys().copied().collect();
    ids.sort();

    for id in ids {
        if let Some(text) = footnotes.get_definition(id) {
            result.push_str(&emit_footnote_def(id, text));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_footnotes_new() {
        let footnotes = PageFootnotes::new();
        assert!(footnotes.is_empty());
        assert!(footnotes.refs.is_empty());
        assert!(footnotes.definitions.is_empty());
    }

    #[test]
    fn test_page_footnotes_add_ref() {
        let mut footnotes = PageFootnotes::new();
        footnotes.add_ref(0, 1);
        footnotes.add_ref(5, 2);

        assert_eq!(footnotes.get_footnote_id(0), Some(1));
        assert_eq!(footnotes.get_footnote_id(5), Some(2));
        assert_eq!(footnotes.get_footnote_id(3), None);
    }

    #[test]
    fn test_page_footnotes_add_definition() {
        let mut footnotes = PageFootnotes::new();
        footnotes.add_definition(1, "First footnote".to_string());
        footnotes.add_definition(2, "Second footnote".to_string());

        assert_eq!(footnotes.get_definition(1), Some("First footnote"));
        assert_eq!(footnotes.get_definition(2), Some("Second footnote"));
        assert_eq!(footnotes.get_definition(3), None);
    }

    #[test]
    fn test_page_footnotes_is_empty() {
        let footnotes = PageFootnotes::new();
        assert!(footnotes.is_empty());

        let mut footnotes = PageFootnotes::new();
        footnotes.add_ref(0, 1);
        assert!(!footnotes.is_empty());
    }

    #[test]
    fn test_emit_footnote_ref() {
        assert_eq!(emit_footnote_ref(1), "[^1]");
        assert_eq!(emit_footnote_ref(5), "[^5]");
        assert_eq!(emit_footnote_ref(100), "[^100]");
    }

    #[test]
    fn test_emit_footnote_def_with_text() {
        let md = emit_footnote_def(1, "Footnote text");
        assert_eq!(md, "[^1]: Footnote text\n");

        let md = emit_footnote_def(2, "Multi-line\ntext");
        assert_eq!(md, "[^2]: Multi-line\ntext\n");
    }

    #[test]
    fn test_emit_footnote_def_empty_text() {
        let md = emit_footnote_def(1, "");
        assert_eq!(md, "[^1]: (empty)\n");
    }

    #[test]
    fn test_emit_footnote_defs_empty() {
        let footnotes = PageFootnotes::new();
        let md = emit_footnote_defs(&footnotes);
        assert_eq!(md, "");
    }

    #[test]
    fn test_emit_footnote_defs_single() {
        let mut footnotes = PageFootnotes::new();
        footnotes.add_definition(1, "First footnote".to_string());

        let md = emit_footnote_defs(&footnotes);
        assert_eq!(md, "\n[^1]: First footnote\n");
    }

    #[test]
    fn test_emit_footnote_defs_multiple_sorted() {
        let mut footnotes = PageFootnotes::new();
        footnotes.add_definition(3, "Third footnote".to_string());
        footnotes.add_definition(1, "First footnote".to_string());
        footnotes.add_definition(2, "Second footnote".to_string());

        let md = emit_footnote_defs(&footnotes);
        // Definitions should be emitted in sorted order by ID
        assert!(md.starts_with("\n[^1]: First footnote\n"));
        assert!(md.contains("[^2]: Second footnote\n"));
        assert!(md.contains("[^3]: Third footnote\n"));
    }

    #[test]
    fn test_emit_footnote_defs_with_empty_text() {
        let mut footnotes = PageFootnotes::new();
        footnotes.add_definition(1, "Has text".to_string());
        footnotes.add_definition(2, "".to_string());

        let md = emit_footnote_defs(&footnotes);
        assert!(md.contains("[^1]: Has text\n"));
        assert!(md.contains("[^2]: (empty)\n"));
    }
}
