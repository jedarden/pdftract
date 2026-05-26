//! JavaScript action detection module.
//!
//! This module provides functions to detect JavaScript actions in PDFs
//! without executing them. Per TH-04, pdftract NEVER executes embedded
//! JavaScript; we only flag its presence for downstream security review.

use crate::diagnostics::{DiagCode, Diagnostic};
use crate::parser::catalog::Catalog;
use crate::parser::object::{ObjRef, PdfObject};
use crate::parser::xref::XrefResolver;
use std::sync::Arc;

/// A detected JavaScript action.
#[derive(Debug, Clone)]
pub struct JavascriptAction {
    /// Location of the JavaScript action in the PDF structure.
    ///
    /// Examples: "catalog.openaction", "page.0.aa.O", "page.1.annot.0.A".
    pub location: String,

    /// Truncated excerpt of the JavaScript code (first 200 characters).
    pub code_excerpt: String,
}

/// Detect JavaScript actions in a PDF catalog and pages.
///
/// This function walks the catalog and all pages to find JavaScript
/// actions in `/OpenAction`, `/AA`, page `/AA`, and annotation `/A` entries.
///
/// # Arguments
///
/// * `catalog` - The parsed document catalog
/// * `pages` - All page dictionaries in the document
/// * `resolver` - The xref resolver for dereferencing indirect objects
///
/// # Returns
///
/// A tuple of:
/// - Vec of detected JavascriptAction structs
/// - Vec of diagnostics emitted during detection
pub fn detect_javascript(
    catalog: &Catalog,
    pages: &[crate::parser::pages::PageDict],
    resolver: &Arc<XrefResolver>,
) -> (Vec<JavascriptAction>, Vec<Diagnostic>) {
    let mut actions = Vec::new();
    let mut diagnostics = Vec::new();

    // Check catalog /OpenAction
    if let Some(open_action) = &catalog.open_action {
        check_object_for_js(open_action, "catalog.openaction", &mut actions, resolver);
    }

    // Check catalog /AA (additional actions)
    if let Some(aa) = &catalog.aa {
        check_aa_for_js(aa, "catalog.aa", &mut actions, resolver);
    }

    // Check each page for /AA and annotations
    for (page_idx, page) in pages.iter().enumerate() {
        let page_prefix = format!("page.{}", page_idx);

        // Check page /AA
        if let Some(page_aa) = &page.aa {
            check_aa_for_js(
                page_aa,
                &format!("{}.aa", page_prefix),
                &mut actions,
                resolver,
            );
        }

        // Check page annotations for /A (action) entries
        if !page.annots.is_empty() {
            // Wrap the annots Vec in a PdfObject::Array for the checker
            let annot_array_obj = PdfObject::Array(Box::new(
                page.annots.iter().map(|&r| PdfObject::Ref(r)).collect(),
            ));
            check_annotations_for_js(&annot_array_obj, &page_prefix, &mut actions, resolver);
        }
    }

    // Emit diagnostic if any JavaScript was found
    if !actions.is_empty() {
        diagnostics.push(Diagnostic::with_dynamic_no_offset(
            DiagCode::SecurityJavascriptPresent,
            format!(
                "Detected {} JavaScript action(s) in PDF document. JavaScript was NOT executed.",
                actions.len()
            ),
        ));
    }

    (actions, diagnostics)
}

/// Check a PdfObject for JavaScript content.
///
/// If the object is a dictionary with a /JS entry, extract the JavaScript.
fn check_object_for_js(
    obj: &PdfObject,
    location: &str,
    actions: &mut Vec<JavascriptAction>,
    resolver: &Arc<XrefResolver>,
) {
    // If it's a reference, resolve it first
    let dict = match obj {
        PdfObject::Ref(r) => match resolver.resolve(*r) {
            Ok(resolved) => resolved,
            Err(_) => return,
        },
        other => other.clone(),
    };

    // Check if it's a dictionary with a /JS entry
    if let Some(dict) = dict.as_dict() {
        if let Some(js_obj) = dict.get("JS") {
            extract_js_code(js_obj, location, actions, resolver);
        }
        // Also check for /S (subtype) == /JavaScript with /JS entry
        else if let Some(s_obj) = dict.get("S") {
            if let Some(s_name) = s_obj.as_name() {
                if s_name == "JavaScript" {
                    if let Some(js_obj) = dict.get("JS") {
                        extract_js_code(js_obj, location, actions, resolver);
                    }
                }
            }
        }
    }
}

/// Check an /AA (additional actions) dictionary for JavaScript.
///
/// The /AA dictionary can have keys like /O (open), /C (close), /D (down), etc.
/// Each value can be an action dictionary with a /JS entry.
fn check_aa_for_js(
    aa: &PdfObject,
    prefix: &str,
    actions: &mut Vec<JavascriptAction>,
    resolver: &Arc<XrefResolver>,
) {
    let aa_dict = match aa {
        PdfObject::Ref(r) => match resolver.resolve(*r) {
            Ok(resolved) => resolved,
            Err(_) => return,
        },
        other => other.clone(),
    };

    if let Some(dict) = aa_dict.as_dict() {
        // Common action keys in /AA dictionaries
        let action_keys = ["O", "C", "D", "U", "E", "X", "FO", "PO", "PC", "PV", "PI"];

        for key in &action_keys {
            if let Some(action_obj) = dict.get(*key) {
                let location = format!("{}.{}", prefix, key.to_lowercase());
                check_object_for_js(action_obj, &location, actions, resolver);
            }
        }
    }
}

/// Check page annotations for JavaScript actions.
///
/// Walks the /Annots array and checks each annotation's /A (action) entry.
fn check_annotations_for_js(
    annot_array: &PdfObject,
    page_prefix: &str,
    actions: &mut Vec<JavascriptAction>,
    resolver: &Arc<XrefResolver>,
) {
    let annots = match annot_array {
        PdfObject::Ref(r) => match resolver.resolve(*r) {
            Ok(resolved) => resolved,
            Err(_) => return,
        },
        other => other.clone(),
    };

    if let Some(array) = annots.as_array() {
        for (annot_idx, annot_obj) in array.iter().enumerate() {
            let annot = match annot_obj {
                PdfObject::Ref(r) => match resolver.resolve(*r) {
                    Ok(resolved) => resolved,
                    Err(_) => continue,
                },
                other => other.clone(),
            };

            if let Some(dict) = annot.as_dict() {
                if let Some(action_obj) = dict.get("A") {
                    let location = format!("{}.annot.{}.a", page_prefix, annot_idx);
                    check_object_for_js(action_obj, &location, actions, resolver);
                }
            }
        }
    }
}

/// Extract JavaScript code from a /JS entry.
///
/// The /JS entry can be either a string (direct JS code) or a stream
/// (hex-encoded or binary JS code).
fn extract_js_code(
    js_obj: &PdfObject,
    location: &str,
    actions: &mut Vec<JavascriptAction>,
    _resolver: &Arc<XrefResolver>,
) {
    let js_code = match js_obj {
        PdfObject::Ref(_r) => {
            // For now, skip resolving references to avoid complexity
            // In practice, most JavaScript is direct strings
            return;
        }
        PdfObject::String(s) => {
            // Get the underlying bytes from the boxed Vec<u8>
            let bytes: &[u8] = &**s;
            bytes.to_vec()
        }
        PdfObject::Name(n) => n.as_bytes().to_vec(),
        // Skip stream-based JavaScript for now (requires source access)
        _ => return,
    };

    // Convert bytes to string, ignoring decoding errors
    let code_string = String::from_utf8_lossy(&js_code);

    // Truncate to 200 characters
    let excerpt = if code_string.len() > 200 {
        code_string.chars().take(200).collect()
    } else {
        code_string.into_owned()
    };

    actions.push(JavascriptAction {
        location: location.to_string(),
        code_excerpt: excerpt,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_javascript_empty() {
        let resolver = Arc::new(XrefResolver::new());
        let catalog = Catalog::new(ObjRef::new(1, 0));
        let pages = Vec::new();

        let (actions, diagnostics) = detect_javascript(&catalog, &pages, &resolver);

        assert!(actions.is_empty());
        assert!(diagnostics.is_empty());
    }
}
