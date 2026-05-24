//! Non-link annotation extraction (Phase 7.6.3).
//!
//! This module extracts non-link annotations such as Highlight, Stamp,
//! FreeText, Note, Squiggly, StrikeOut, Underline, etc.

use crate::annotation::AnnotationCommon;
use crate::parser::object::PdfDict;

/// A non-link annotation extracted from a PDF page.
///
/// Represents markup annotations like highlights, text notes, stamps,
/// and other non-link annotations.
#[derive(Debug, Clone)]
pub struct Annotation {
    /// Common annotation fields (subtype, rect, contents, etc.).
    pub common: AnnotationCommon,
}

/// Extract a non-link annotation from an annotation dictionary.
///
/// This function implements Phase 7.6.3: it extracts non-link annotations
/// (all subtypes except Link, Widget, and Popup).
///
/// # Arguments
///
/// * `dict` - The annotation dictionary
/// * `common` - Pre-extracted common annotation fields
///
/// # Returns
///
/// Some(Annotation) for valid non-link annotations, None for skipped types.
pub(crate) fn extract_annotation(_dict: &PdfDict, common: AnnotationCommon) -> Option<Annotation> {
    // For now, all non-link, non-widget, non-popup annotations are valid
    // The common struct already contains all the shared fields
    Some(Annotation { common })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::AnnotationCommon;
    use crate::parser::object::PdfObject;
    use indexmap::IndexMap;
    use std::sync::Arc;

    #[test]
    fn test_extract_highlight_annotation() {
        let mut dict = IndexMap::new();

        // Add /Contents
        dict.insert(
            Arc::from("/Contents"),
            PdfObject::String(Box::new(b"Important text".to_vec())),
        );

        let common = AnnotationCommon {
            subtype: "Highlight".to_string(),
            rect: Some([10.0, 20.0, 100.0, 30.0]),
            contents: Some("Important text".to_string()),
            author: None,
            modified: None,
            color: Some(vec![1.0, 1.0, 0.0]), // Yellow highlight
            opacity: Some(0.5),
            flags: 0,
            name_id: None,
            subject: None,
            page_index: 0,
        };

        let result = extract_annotation(&dict, common);
        assert!(result.is_some());
        let annotation = result.unwrap();
        assert_eq!(annotation.common.subtype, "Highlight");
        assert_eq!(
            annotation.common.contents,
            Some("Important text".to_string())
        );
        assert_eq!(annotation.common.color, Some(vec![1.0, 1.0, 0.0]));
    }

    #[test]
    fn test_extract_text_annotation() {
        let dict = IndexMap::new();

        let common = AnnotationCommon {
            subtype: "Text".to_string(),
            rect: Some([50.0, 100.0, 70.0, 120.0]),
            contents: Some("Review this section".to_string()),
            author: Some("John Doe".to_string()),
            modified: Some("2023-05-15T14:30:45Z".to_string()),
            color: None,
            opacity: None,
            flags: 0,
            name_id: Some("note-1".to_string()),
            subject: Some("Review".to_string()),
            page_index: 2,
        };

        let result = extract_annotation(&dict, common);
        assert!(result.is_some());
        let annotation = result.unwrap();
        assert_eq!(annotation.common.subtype, "Text");
        assert_eq!(annotation.common.author, Some("John Doe".to_string()));
        assert_eq!(annotation.common.name_id, Some("note-1".to_string()));
    }

    #[test]
    fn test_extract_annotation_with_no_contents() {
        let dict = IndexMap::new();

        let common = AnnotationCommon {
            subtype: "Underline".to_string(),
            rect: Some([0.0, 0.0, 50.0, 10.0]),
            contents: None, // No /Contents
            author: None,
            modified: None,
            color: None,
            opacity: None,
            flags: 0,
            name_id: None,
            subject: None,
            page_index: 1,
        };

        let result = extract_annotation(&dict, common);
        assert!(result.is_some());
        let annotation = result.unwrap();
        assert_eq!(annotation.common.subtype, "Underline");
        assert!(annotation.common.contents.is_none());
    }
}
