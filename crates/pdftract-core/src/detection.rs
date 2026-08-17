//! Document detection module for JavaScript, XFA, and conformance.
//!
//! This module provides detectors for document-level metadata flags:
//! - JavaScript presence (contains_javascript)
//! - XFA forms (contains_xfa)
//! - PDF/A conformance (conformance)
//!
//! # Security Boundary (TH-04)
//!
//! **JavaScript is NEVER EXECUTED.** Per the TH-04 threat model, pdftract detects
//! JavaScript actions in PDFs (in `/OpenAction`, `/AA`, page `/AA`, and annotation `/A`
//! entries) but does not execute them. Detection only flags presence for downstream
//! security review via diagnostics and `metadata.javascript_actions[]` output.
//!
//! Per INV-8, all detection functions are resilient and never panic.

use crate::parser::catalog::Catalog;
use crate::parser::object::{PdfDict, PdfObject};
use crate::parser::pages::PageDict;
use crate::parser::xref::XrefResolver;
use tracing::{debug, info};

/// Detect JavaScript presence in a PDF document.
///
/// This function walks the document tree checking for JavaScript actions in:
/// - Catalog /OpenAction
/// - Catalog /AA (Additional Actions)
/// - Page-level /AA dicts
/// - AcroForm field /AA dicts
/// - Annotation /A and /AA dicts
///
/// JavaScript is NEVER EXECUTED; only its presence is flagged.
///
/// # Arguments
///
/// * `catalog` - The document catalog
/// * `pages` - All page dictionaries in the document
/// * `acroform` - The AcroForm dictionary (if present)
/// * `resolver` - The xref resolver for dereferencing indirect objects
///
/// # Returns
///
/// `true` if any JavaScript action is found, `false` otherwise.
///
/// # Behavior
///
/// Per INV-8, this function never panics. Malformed or unresolvable
/// objects are silently skipped (treated as no-JS).
pub fn detect_javascript(
    catalog: &Catalog,
    pages: &[PageDict],
    acroform: &Option<PdfDict>,
    resolver: &XrefResolver,
) -> bool {
    info!("JavaScript detection started - scanning document for JavaScript actions");
    debug!("JavaScript detection: pdftract NEVER executes embedded JavaScript (per TH-04 threat model)");

    // Check catalog /OpenAction
    if has_js_action(&catalog.open_action, resolver) {
        info!("JavaScript DETECTED: catalog /OpenAction contains JavaScript action");
        debug!("JavaScript found but NOT executed - detection only (per TH-04 threat model)");
        return true;
    }

    // Check catalog /AA
    if has_js_in_aa(&catalog.aa, resolver) {
        info!("JavaScript DETECTED: catalog /AA (Additional Actions) contains JavaScript");
        debug!("JavaScript found but NOT executed - detection only (per TH-04 threat model)");
        return true;
    }

    // Check each page for /AA and annotations
    for (page_idx, page) in pages.iter().enumerate() {
        // Check page /AA
        if has_js_in_aa(&page.aa, resolver) {
            info!(
                "JavaScript DETECTED: page {} /AA (Additional Actions) contains JavaScript",
                page_idx
            );
            debug!("JavaScript found but NOT executed - detection only (per TH-04 threat model)");
            return true;
        }

        // Check page annotations for /A and /AA entries
        for &annot_ref in &page.annots {
            if let Ok(annot_obj) = resolver.resolve(annot_ref) {
                if let Some(annot_dict) = annot_obj.as_dict() {
                    // Check /A (primary action)
                    if let Some(action) = annot_dict.get("A") {
                        if has_js_action(&Some(action.clone()), resolver) {
                            info!("JavaScript DETECTED: page {} annotation /A (primary action) contains JavaScript", page_idx);
                            debug!("JavaScript found but NOT executed - detection only (per TH-04 threat model)");
                            return true;
                        }
                    }
                    // Check /AA (additional actions)
                    if let Some(aa) = annot_dict.get("AA") {
                        if has_js_in_aa(&Some(aa.clone()), resolver) {
                            info!("JavaScript DETECTED: page {} annotation /AA (additional actions) contains JavaScript", page_idx);
                            debug!("JavaScript found but NOT executed - detection only (per TH-04 threat model)");
                            return true;
                        }
                    }
                }
            }
        }
    }

    // Check AcroForm fields for /AA
    if let Some(form_dict) = acroform {
        if has_js_in_acroform(form_dict, resolver) {
            info!("JavaScript DETECTED: AcroForm fields /AA (Additional Actions) contain JavaScript");
            debug!("JavaScript found but NOT executed - detection only (per TH-04 threat model)");
            return true;
        }
    }

    info!("JavaScript detection complete - no JavaScript actions found");
    false
}

/// Check if a PdfObject represents a JavaScript action.
///
/// This detects dictionaries with /S == /JavaScript or /JS entries.
fn has_js_action(obj: &Option<PdfObject>, resolver: &XrefResolver) -> bool {
    let obj = match obj {
        None => return false,
        Some(o) => o,
    };

    // Resolve if it's a reference
    let resolved = match obj {
        PdfObject::Ref(r) => match resolver.resolve(*r) {
            Ok(o) => o,
            Err(_) => return false,
        },
        _ => obj.clone(),
    };

    // Check if it's a dictionary with /S == /JavaScript
    if let Some(dict) = resolved.as_dict() {
        // Check for /S (subtype) == /JavaScript or /JS
        if let Some(s_obj) = dict.get("S") {
            if let Some(s_name) = s_obj.as_name() {
                if s_name == "JavaScript" || s_name == "JS" {
                    debug!("JavaScript action detected with /S == {}", s_name);
                    return true;
                }
            }
        }
        // Check for /JS entry (JavaScript code)
        if dict.get("JS").is_some() {
            debug!("JavaScript action detected with /JS entry");
            return true;
        }
    }

    false
}

/// Check if an /AA (Additional Actions) dictionary contains JavaScript.
///
/// /AA dictionaries can have keys like /O (open), /C (close), /D (down),
/// etc. Each value can be an action dictionary with JavaScript.
fn has_js_in_aa(aa: &Option<PdfObject>, resolver: &XrefResolver) -> bool {
    let aa = match aa {
        None => return false,
        Some(a) => a,
    };

    // Resolve if it's a reference
    let aa_dict = match aa {
        PdfObject::Ref(r) => match resolver.resolve(*r) {
            Ok(o) => o,
            Err(_) => return false,
        },
        _ => aa.clone(),
    };

    if let Some(dict) = aa_dict.as_dict() {
        // Common action keys in /AA dictionaries
        // /O=Open, /C=Close, /D=MouseDown, /U=MouseUp, /E=Enter, /X=Exit, /FO=FocusIn, /PO=FocusOut
        let action_keys = ["O", "C", "D", "U", "E", "X", "FO", "PO", "PC", "PV", "PI"];

        for key in &action_keys {
            if let Some(action_obj) = dict.get(*key) {
                if has_js_action(&Some(action_obj.clone()), resolver) {
                    debug!(
                        "JavaScript detected in /AA dictionary with action key: /{}",
                        key
                    );
                    return true;
                }
            }
        }
    }

    false
}

/// Check if AcroForm fields contain JavaScript actions.
///
/// Walks the /Fields array recursively and checks each field's /AA dict.
fn has_js_in_acroform(acroform: &PdfDict, resolver: &XrefResolver) -> bool {
    // Get the /Fields array
    let fields = match acroform.get("Fields") {
        None => return false,
        Some(f) => f,
    };

    let fields_array = match fields {
        PdfObject::Ref(r) => match resolver.resolve(*r) {
            Ok(o) => o,
            Err(_) => return false,
        },
        _ => fields.clone(),
    };

    if let Some(array) = fields_array.as_array() {
        for field_obj in array.as_ref() {
            let field = match field_obj {
                PdfObject::Ref(r) => match resolver.resolve(*r) {
                    Ok(f) => f,
                    Err(_) => continue,
                },
                _ => field_obj.clone(),
            };

            if let Some(field_dict) = field.as_dict() {
                // Check this field's /AA
                if let Some(aa) = field_dict.get("AA") {
                    if has_js_in_aa(&Some(aa.clone()), resolver) {
                        return true;
                    }
                }

                // Recurse into nested fields (some fields are field groups)
                // Kids entries can contain sub-fields
                if let Some(kids) = field_dict.get("Kids") {
                    if let Some(kids_array) = kids.as_array() {
                        for kid in kids_array.as_ref() {
                            if let Some(kid_dict) = kid.as_dict() {
                                if let Some(aa) = kid_dict.get("AA") {
                                    if has_js_in_aa(&Some(aa.clone()), resolver) {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    false
}

/// Detect XFA (XML Forms Architecture) presence in a PDF document.
///
/// Checks for the /XFA key in the AcroForm dictionary. If /XFA is present
/// and non-null, the document contains XFA forms.
///
/// # Arguments
///
/// * `acroform` - The AcroForm dictionary (if present)
///
/// # Returns
///
/// `true` if XFA is present, `false` otherwise.
///
/// # Behavior
///
/// Per INV-8, this function never panics. Missing or malformed AcroForm
/// dictionaries return false.
pub fn detect_xfa(acroform: &Option<PdfDict>) -> bool {
    match acroform {
        None => false,
        Some(dict) => {
            // Check if /XFA key exists and is non-null
            match dict.get("XFA") {
                None => false,
                Some(PdfObject::Null) => false,
                Some(_) => true,
            }
        }
    }
}

/// Detect PDF/A conformance from XMP metadata.
///
/// Parses the XMP XML to extract pdfaid:part and pdfaid:conformance
/// namespace elements, then combines them as "PDF/A-{part}{conformance}"
/// (e.g. "PDF/A-1b", "PDF/A-2u", "PDF/A-3a").
///
/// # Arguments
///
/// * `metadata_stream` - Optional byte slice containing the XMP metadata stream
///
/// # Returns
///
/// * `Some(String)` - PDF/A conformance string if detected (e.g., "PDF/A-1b")
/// * `None` - No PDF/A conformance detected or malformed XML
///
/// # Graceful Failure
///
/// Per INV-8, this function never panics. Malformed XML, missing elements,
/// or any parsing error returns None rather than propagating errors.
///
/// # Example
///
/// ```ignore
/// use pdftract_core::detection::detect_conformance;
///
/// // XMP with pdfaid:part="1" and pdfaid:conformance="b"
/// let xmp = br#"<?xpacket begin='...'?>
/// <rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>
///   <rdf:Description rdf:about=''
///     xmlns:pdfaid='http://www.aiim.org/pdfa/ns/id/'>
///     <pdfaid:part>1</pdfaid:part>
///     <pdfaid:conformance>b</pdfaid:conformance>
///   </rdf:Description>
/// </rdf:RDF>"#;
///
/// let result = detect_conformance(Some(xmp));
/// assert_eq!(result, Some("PDF/A-1b".to_string()));
/// ```
pub fn detect_conformance(metadata_stream: Option<&[u8]>) -> Option<String> {
    crate::conformance::detect_conformance(metadata_stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_detect_xfa_none() {
        assert!(!detect_xfa(&None));
    }

    #[test]
    fn test_detect_xfa_no_xfa_key() {
        let mut dict = PdfDict::new();
        dict.insert(Arc::from("Fields"), PdfObject::Array(Box::new(vec![])));
        assert!(!detect_xfa(&Some(dict)));
    }

    #[test]
    fn test_detect_xfa_null() {
        let mut dict = PdfDict::new();
        dict.insert(Arc::from("XFA"), PdfObject::Null);
        assert!(!detect_xfa(&Some(dict)));
    }

    #[test]
    fn test_detect_xfa_present() {
        let mut dict = PdfDict::new();
        dict.insert(Arc::from("XFA"), PdfObject::Integer(1));
        assert!(detect_xfa(&Some(dict)));
    }

    #[test]
    fn test_detect_xfa_with_array() {
        // XFA is typically an array of streams
        let mut dict = PdfDict::new();
        let xfa_array = vec![
            PdfObject::Ref(ObjRef::new(10, 0)),
            PdfObject::String(Box::new(b"form".to_vec())),
        ];
        dict.insert(Arc::from("XFA"), PdfObject::Array(Box::new(xfa_array)));
        assert!(detect_xfa(&Some(dict)));
    }

    #[test]
    fn test_detect_javascript_empty() {
        let catalog = Catalog::new(ObjRef::new(1, 0), PdfObject::Dict(Box::new(PdfDict::new())));
        let pages = Vec::new();
        let acroform = None;
        let resolver = XrefResolver::new();

        assert!(!detect_javascript(&catalog, &pages, &acroform, &resolver));
    }

    #[test]
    fn test_detect_javascript_with_catalog_openaction_js() {
        let resolver = XrefResolver::new();
        let mut catalog = Catalog::new(ObjRef::new(1, 0), PdfObject::Dict(Box::new(PdfDict::new())));

        // Create a JavaScript action dict
        let mut js_dict = PdfDict::new();
        js_dict.insert(Arc::from("S"), PdfObject::Name(Arc::from("JavaScript")));
        js_dict.insert(
            Arc::from("JS"),
            PdfObject::String(Box::new(b"app.alert('hello')".to_vec())),
        );
        let js_obj = PdfObject::Dict(Box::new(js_dict));

        catalog.open_action = Some(js_obj);

        let pages = Vec::new();
        let acroform = None;

        assert!(detect_javascript(&catalog, &pages, &acroform, &resolver));
    }

    #[test]
    fn test_detect_javascript_with_catalog_aa_js() {
        let resolver = XrefResolver::new();
        let mut catalog = Catalog::new(ObjRef::new(1, 0), PdfObject::Dict(Box::new(PdfDict::new())));

        // Create an /AA dict with JavaScript
        let mut aa_dict = PdfDict::new();
        let mut js_dict = PdfDict::new();
        js_dict.insert(Arc::from("S"), PdfObject::Name(Arc::from("JavaScript")));
        js_dict.insert(
            Arc::from("JS"),
            PdfObject::String(Box::new(b"app.alert('open')".to_vec())),
        );
        aa_dict.insert(Arc::from("O"), PdfObject::Dict(Box::new(js_dict)));
        let aa_obj = PdfObject::Dict(Box::new(aa_dict));

        catalog.aa = Some(aa_obj);

        let pages = Vec::new();
        let acroform = None;

        assert!(detect_javascript(&catalog, &pages, &acroform, &resolver));
    }

    #[test]
    fn test_detect_javascript_no_javascript() {
        let resolver = XrefResolver::new();
        let catalog = Catalog::new(ObjRef::new(1, 0), PdfObject::Dict(Box::new(PdfDict::new())));

        let mut page = PageDict::default();
        page.obj_ref = ObjRef::new(2, 0);
        let pages = vec![page];
        let acroform = None;

        assert!(!detect_javascript(&catalog, &pages, &acroform, &resolver));
    }

    #[test]
    fn test_detect_javascript_with_annotation_js() {
        let resolver = XrefResolver::new();
        let catalog = Catalog::new(ObjRef::new(1, 0), PdfObject::Dict(Box::new(PdfDict::new())));

        // Create a page with an annotation that has JavaScript in /A
        let mut page = PageDict::default();
        page.obj_ref = ObjRef::new(2, 0);

        // Create an annotation with JavaScript action
        let annot_ref = ObjRef::new(10, 0);
        let mut annot_dict = PdfDict::new();
        let mut js_dict = PdfDict::new();
        js_dict.insert(Arc::from("S"), PdfObject::Name(Arc::from("JavaScript")));
        js_dict.insert(
            Arc::from("JS"),
            PdfObject::String(Box::new(b"app.alert('annot')".to_vec())),
        );
        annot_dict.insert(Arc::from("A"), PdfObject::Dict(Box::new(js_dict)));
        resolver.cache_object(annot_ref, PdfObject::Dict(Box::new(annot_dict)));

        page.annots.push(annot_ref);
        let pages = vec![page];

        let acroform = None;

        assert!(detect_javascript(&catalog, &pages, &acroform, &resolver));
    }

    #[test]
    fn test_detect_javascript_with_page_aa_js() {
        let resolver = XrefResolver::new();
        let catalog = Catalog::new(ObjRef::new(1, 0), PdfObject::Dict(Box::new(PdfDict::new())));

        // Create a page with /AA containing JavaScript
        let mut page = PageDict::default();
        page.obj_ref = ObjRef::new(2, 0);

        let mut aa_dict = PdfDict::new();
        let mut js_dict = PdfDict::new();
        js_dict.insert(Arc::from("S"), PdfObject::Name(Arc::from("JavaScript")));
        js_dict.insert(
            Arc::from("JS"),
            PdfObject::String(Box::new(b"app.alert('page')".to_vec())),
        );
        aa_dict.insert(Arc::from("O"), PdfObject::Dict(Box::new(js_dict)));
        page.aa = Some(PdfObject::Dict(Box::new(aa_dict)));

        let pages = vec![page];
        let acroform = None;

        assert!(detect_javascript(&catalog, &pages, &acroform, &resolver));
    }

    #[test]
    fn test_detect_javascript_with_acroform_field_js() {
        let resolver = XrefResolver::new();
        let catalog = Catalog::new(ObjRef::new(1, 0), PdfObject::Dict(Box::new(PdfDict::new())));

        let page = PageDict::default();
        let pages = vec![page];

        // Create AcroForm with a field that has JavaScript in /AA
        let mut acroform = PdfDict::new();
        let mut field_dict = PdfDict::new();
        let mut aa_dict = PdfDict::new();
        let mut js_dict = PdfDict::new();
        js_dict.insert(Arc::from("S"), PdfObject::Name(Arc::from("JavaScript")));
        js_dict.insert(
            Arc::from("JS"),
            PdfObject::String(Box::new(b"app.alert('field')".to_vec())),
        );
        aa_dict.insert(Arc::from("C"), PdfObject::Dict(Box::new(js_dict)));
        field_dict.insert(Arc::from("AA"), PdfObject::Dict(Box::new(aa_dict)));

        let fields = vec![PdfObject::Dict(Box::new(field_dict))];
        acroform.insert(Arc::from("Fields"), PdfObject::Array(Box::new(fields)));

        assert!(detect_javascript(
            &catalog,
            &pages,
            &Some(acroform),
            &resolver
        ));
    }

    #[test]
    fn test_has_js_action_with_s_javascript() {
        let resolver = XrefResolver::new();

        let mut dict = PdfDict::new();
        dict.insert(Arc::from("S"), PdfObject::Name(Arc::from("JavaScript")));
        dict.insert(
            Arc::from("JS"),
            PdfObject::String(Box::new(b"test".to_vec())),
        );
        let obj = PdfObject::Dict(Box::new(dict));

        assert!(has_js_action(&Some(obj), &resolver));
    }

    #[test]
    fn test_has_js_action_with_s_js() {
        let resolver = XrefResolver::new();

        let mut dict = PdfDict::new();
        dict.insert(Arc::from("S"), PdfObject::Name(Arc::from("JS")));
        dict.insert(
            Arc::from("JS"),
            PdfObject::String(Box::new(b"test".to_vec())),
        );
        let obj = PdfObject::Dict(Box::new(dict));

        assert!(has_js_action(&Some(obj), &resolver));
    }

    #[test]
    fn test_has_js_action_no_js() {
        let resolver = XrefResolver::new();

        let mut dict = PdfDict::new();
        dict.insert(Arc::from("S"), PdfObject::Name(Arc::from("GoTo")));
        dict.insert(Arc::from("D"), PdfObject::Name(Arc::from("NextPage")));
        let obj = PdfObject::Dict(Box::new(dict));

        assert!(!has_js_action(&Some(obj), &resolver));
    }

    #[test]
    fn test_detect_conformance_pdf_a_1b() {
        let xmp = br#"<?xpacket begin='...'?>
<rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>
  <rdf:Description rdf:about=''
    xmlns:pdfaid='http://www.aiim.org/pdfa/ns/id/'>
    <pdfaid:part>1</pdfaid:part>
    <pdfaid:conformance>b</pdfaid:conformance>
  </rdf:Description>
</rdf:RDF>"#;

        let result = detect_conformance(Some(xmp));
        assert_eq!(result, Some("PDF/A-1b".to_string()));
    }

    #[test]
    fn test_detect_conformance_none() {
        let result = detect_conformance(None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_detect_conformance_malformed() {
        let xmp = b"<not-valid-xml<<<<";
        let result = detect_conformance(Some(xmp));
        assert_eq!(result, None);
    }
}
