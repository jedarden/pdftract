//! Table detection and structure reconstruction.
//!
//! This module implements table detection from PDF content streams using two methods:
//!
//! ## Line-based detection (7.2.1)
//! For bordered tables with ruling lines:
//! 1. Collecting horizontal and vertical path segments from stroke operators
//! 2. Clustering collinear segments within epsilon tolerance
//! 3. Finding intersection points between horizontal and vertical segments
//! 4. Building candidate grids from the intersections
//!
//! ## Borderless detection (7.2.2)
//! For tables without ruling lines, using x0 alignment heuristics:
//! 1. Collect text positions from content stream (Tm, Td, TD, T*, Tj, TJ operators)
//! 2. Group by x0 positions (within 2.0 pt tolerance)
//! 3. Find column candidates (3+ spans at same x0 on different y positions)
//! 4. Find row candidates (y positions where >= 2 column candidates have spans)
//! 5. Validate: 3+ rows AND 3+ columns, contiguous y range, no gap > 100 pt

mod detector;
mod segment;
mod grid;
mod cell;

pub use detector::TableDetector;
pub use segment::{Segment, SegmentOrientation};
pub use grid::GridCandidate;
pub use cell::{Cell, TableSpan};

use crate::parser::pages::PageDict;

/// Page context for table detection.
///
/// Contains the information needed to detect tables on a page.
#[derive(Debug, Clone)]
pub struct PageContext<'a> {
    /// The page dictionary.
    pub page: &'a PageDict,
    /// Decoded content stream bytes for this page.
    pub content_bytes: &'a [u8],
}

impl<'a> PageContext<'a> {
    /// Create a new page context from a page dict and content bytes.
    pub fn new(page: &'a PageDict, content_bytes: &'a [u8]) -> Self {
        Self { page, content_bytes }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_context_creation() {
        // Minimal test to verify the module compiles
        use std::sync::Arc;
        use crate::parser::object::ObjRef;
        use crate::parser::resources::ResourceDict;

        let page = PageDict {
            obj_ref: ObjRef::new(1, 0),
            media_box: [0.0, 0.0, 612.0, 792.0],
            resources: Arc::new(ResourceDict::default()),
            contents: vec![],
            annots: vec![],
            actual_text: None,
            lang: None,
            aa: None,
            struct_parents: None,
            crop_box: None,
            bleed_box: None,
            trim_box: None,
            art_box: None,
            rotate: 0,
        };
        let content = b"";
        let ctx = PageContext::new(&page, content);
        assert_eq!(ctx.page.media_box[0], 0.0);
        assert_eq!(ctx.content_bytes.len(), 0);
    }
}
