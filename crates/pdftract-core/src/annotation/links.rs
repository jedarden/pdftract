//! Link annotation extraction (Phase 7.6.2).
//!
//! This module extracts URI hyperlinks and internal destination links from
//! `/Subtype /Link` annotations.

use crate::annotation::AnnotationCommon;
use crate::parser::object::{PdfDict, PdfObject};

/// A link annotation extracted from a PDF page.
///
/// Represents either a URI hyperlink (external link) or an internal destination
/// link (named or explicit destination within the same document).
#[derive(Debug, Clone)]
pub struct LinkAnnotation {
    /// Common annotation fields (subtype, rect, etc.).
    pub common: AnnotationCommon,
    /// The URI target for external links (from /A /S /URI /URI).
    /// None for internal destination links or malformed URIs.
    pub uri: Option<String>,
    /// The internal destination name (from /Dest as a name string).
    /// None for URI links or explicit destination arrays.
    pub dest: Option<String>,
}

/// Extract a link annotation from a Link annotation dictionary.
///
/// This function implements Phase 7.6.2: it extracts the URI or destination
/// from a `/Subtype /Link` annotation.
///
/// # Arguments
///
/// * `dict` - The Link annotation dictionary
/// * `common` - Pre-extracted common annotation fields
///
/// # Returns
///
/// Some(LinkAnnotation) if the link has a valid URI or destination, None otherwise.
pub(crate) fn extract_link(dict: &PdfDict, common: AnnotationCommon) -> Option<LinkAnnotation> {
    // Try to extract /A (action) dictionary - PDF dict keys include the leading /
    let (uri, dest) = if let Some(action_obj) = dict.get("/A") {
        // Resolve indirect reference if needed
        let action_dict = match action_obj {
            PdfObject::Dict(action_dict) => action_dict,
            PdfObject::Ref(_) => {
                // Indirect reference - for now, skip (could resolve in future)
                return None;
            }
            _ => {
                return None;
            }
        };

        // Check /S (action type)
        let action_type = action_dict.get("/S").and_then(|o| o.as_name());

        match action_type {
            Some(name) if name == "URI" => {
                // URI action: extract /URI
                let uri = action_dict
                    .get("/URI")
                    .and_then(|o| o.as_string())
                    .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());

                (uri, None)
            }
            Some(name) if name == "GoTo" => {
                // GoTo action: extract /D (destination)
                let dest = extract_destination_name(action_dict.get("/D"));
                (None, dest)
            }
            _ => {
                // Other action types: ignore for now
                return None;
            }
        }
    } else if let Some(dest_obj) = dict.get("/Dest") {
        // Direct /Dest entry (no /A)
        let dest = extract_destination_name(Some(dest_obj));
        (None, dest)
    } else {
        // No /A and no /Dest: not a valid link
        return None;
    };

    // At least one of uri or dest should be Some
    if uri.is_none() && dest.is_none() {
        return None;
    }

    Some(LinkAnnotation { common, uri, dest })
}

/// Extract a destination name from a /Dest or /D entry.
///
/// Destinations can be:
/// - A name string (e.g., "SectionTwo")
/// - An explicit destination array (ignored for now, returns None)
fn extract_destination_name(dest_obj: Option<&PdfObject>) -> Option<String> {
    match dest_obj? {
        PdfObject::Name(name) => Some(name.to_string()),
        PdfObject::String(bytes) => String::from_utf8(bytes.to_vec()).ok(),
        PdfObject::Array(_) => {
            // Explicit destination array: could be expanded but skip for now
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::PdfObject;
    use indexmap::IndexMap;
    use std::sync::Arc;

    #[test]
    fn test_extract_link_uri() {
        let mut dict = IndexMap::new();

        // Create /A dictionary with /S /URI and /URI
        let mut action_dict = IndexMap::new();
        action_dict.insert(Arc::from("/S"), PdfObject::Name("URI".into()));
        action_dict.insert(
            Arc::from("/URI"),
            PdfObject::String(Box::new(b"https://example.com".to_vec())),
        );

        dict.insert(Arc::from("/A"), PdfObject::Dict(Box::new(action_dict)));

        let common = AnnotationCommon {
            subtype: "Link".to_string(),
            rect: Some([0.0, 0.0, 100.0, 20.0]),
            contents: None,
            author: None,
            modified: None,
            color: None,
            opacity: None,
            flags: 0,
            name_id: None,
            subject: None,
            page_index: 0,
        };

        let result = extract_link(&dict, common);
        assert!(result.is_some());
        let link = result.unwrap();
        assert_eq!(link.uri, Some("https://example.com".to_string()));
        assert_eq!(link.dest, None);
    }

    #[test]
    fn test_extract_link_named_dest() {
        let mut dict = IndexMap::new();

        // Direct /Dest as a name
        dict.insert(Arc::from("/Dest"), PdfObject::Name("SectionTwo".into()));

        let common = AnnotationCommon {
            subtype: "Link".to_string(),
            rect: Some([0.0, 0.0, 100.0, 20.0]),
            contents: None,
            author: None,
            modified: None,
            color: None,
            opacity: None,
            flags: 0,
            name_id: None,
            subject: None,
            page_index: 0,
        };

        let result = extract_link(&dict, common);
        assert!(result.is_some());
        let link = result.unwrap();
        assert_eq!(link.uri, None);
        assert_eq!(link.dest, Some("SectionTwo".to_string()));
    }

    #[test]
    fn test_extract_link_goto_action() {
        let mut dict = IndexMap::new();

        // Create /A dictionary with /S /GoTo and /D
        let mut action_dict = IndexMap::new();
        action_dict.insert(Arc::from("/S"), PdfObject::Name("GoTo".into()));
        action_dict.insert(Arc::from("/D"), PdfObject::Name("Appendix".into()));

        dict.insert(Arc::from("/A"), PdfObject::Dict(Box::new(action_dict)));

        let common = AnnotationCommon {
            subtype: "Link".to_string(),
            rect: Some([0.0, 0.0, 100.0, 20.0]),
            contents: None,
            author: None,
            modified: None,
            color: None,
            opacity: None,
            flags: 0,
            name_id: None,
            subject: None,
            page_index: 0,
        };

        let result = extract_link(&dict, common);
        assert!(result.is_some());
        let link = result.unwrap();
        assert_eq!(link.uri, None);
        assert_eq!(link.dest, Some("Appendix".to_string()));
    }
}
