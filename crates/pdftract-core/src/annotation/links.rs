//! Link annotation extraction (Phase 7.6.2).
//!
//! This module extracts URI hyperlinks and internal destination links from
//! `/Subtype /Link` annotations.

use crate::annotation::AnnotationCommon;
use crate::parser::object::{PdfDict, PdfObject};
use crate::parser::xref::XrefResolver;

/// Destination anchor types for explicit link destinations.
///
/// Per PDF 1.7 spec section 12.3.2.2 "Explicit Destinations":
/// - /XYZ: left, top, zoom (null = retain current view)
/// - /Fit: fit page to window
/// - /FitH: fit width, top coordinate
/// - /FitV: left coordinate, fit height
/// - /FitR: fit rectangle (left, bottom, right, top)
/// - /FitB: fit bounding box to window
/// - /FitBH: fit bbox width, top coordinate
/// - /FitBV: left coordinate, fit bbox height
#[derive(Debug, Clone, PartialEq)]
pub enum FitType {
    /// XYZ destination (left, top, zoom)
    /// Any null value means "retain current view"
    Xyz {
        /// Left coordinate of the viewport (null = retain current left position)
        left: Option<f32>,
        /// Top coordinate of the viewport (null = retain current top position)
        top: Option<f32>,
        /// Zoom factor (null = retain current zoom)
        zoom: Option<f32>,
    },
    /// Fit page to window
    Fit,
    /// Fit horizontally (top coordinate)
    FitH(Option<f32>),
    /// Fit vertically (left coordinate)
    FitV(Option<f32>),
    /// Fit rectangle (left, bottom, right, top)
    FitR(f32, f32, f32, f32),
    /// Fit bounding box to window
    FitB,
    /// Fit bounding box horizontally (top coordinate)
    FitBH(Option<f32>),
    /// Fit bounding box vertically (left coordinate)
    FitBV(Option<f32>),
}

/// An explicit destination array from a /Dest entry.
///
/// Per PDF 1.7 spec section 12.3.2.2, explicit destinations are arrays:
/// [page_ref, /FitTypeName, params...]
#[derive(Debug, Clone)]
pub struct DestArray {
    /// Page index (0-based) within the document
    pub page_index: usize,
    /// Fit type and coordinates
    pub fit: FitType,
}

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
    /// Explicit destination array (from /Dest as an array).
    /// None for URI links or named destinations.
    pub dest_array: Option<DestArray>,
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
/// * `resolver` - The Xref resolver for dereferencing indirect objects
/// * `dests_dict` - Optional /Catalog /Dests dictionary (legacy PDF 1.2)
/// * `names_dests_ref` - Optional /Catalog /Names /Dests reference (PDF 1.3+ name tree)
///
/// # Returns
///
/// Some(LinkAnnotation) if the link has a valid URI or destination, None otherwise.
pub(crate) fn extract_link(
    dict: &PdfDict,
    common: AnnotationCommon,
    resolver: &XrefResolver,
    dests_dict: Option<&PdfDict>,
    names_dests_ref: Option<crate::parser::object::ObjRef>,
) -> Option<LinkAnnotation> {
    // Try to extract /A (action) dictionary - PDF dict keys include the leading /
    if let Some(action_obj) = dict.get("/A") {
        // Resolve indirect reference if needed
        let action_dict = match action_obj {
            PdfObject::Dict(action_dict) => action_dict.clone(),
            PdfObject::Ref(action_ref) => {
                match resolver.resolve(*action_ref) {
                    Ok(PdfObject::Dict(resolved)) => resolved.clone(),
                    _ => {
                        // Failed to resolve or not a dict - emit diagnostic placeholder
                        return Some(LinkAnnotation {
                            common,
                            uri: None,
                            dest: Some("link_action_resolve_failed".to_string()),
                            dest_array: None,
                        });
                    }
                }
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

                return Some(LinkAnnotation {
                    common,
                    uri,
                    dest: None,
                    dest_array: None,
                });
            }
            Some(name) if name == "GoTo" => {
                // GoTo action: extract /D (destination)
                return extract_destination(
                    action_dict.get("/D"),
                    &common,
                    resolver,
                    dests_dict,
                    names_dests_ref,
                );
            }
            Some(name) if name == "JavaScript" => {
                // JavaScript action: emit diagnostic with truncated code
                let js_code = action_dict
                    .get("/JS")
                    .and_then(|o| o.as_string())
                    .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());

                let truncated = js_code.as_ref().map(|s| {
                    if s.len() > 100 {
                        format!("{}...", &s[..100])
                    } else {
                        s.clone()
                    }
                });

                return Some(LinkAnnotation {
                    common,
                    uri: Some(format!("javascript:{}", truncated.unwrap_or_default())),
                    dest: None,
                    dest_array: None,
                });
            }
            _ => {
                // Other action types: ignore for now
                return None;
            }
        }
    }

    // Check for direct /Dest entry (no /A)
    if let Some(dest_obj) = dict.get("/Dest") {
        return extract_destination(
            Some(dest_obj),
            &common,
            resolver,
            dests_dict,
            names_dests_ref,
        );
    }

    // No /A and no /Dest: emit diagnostic for link without target
    Some(LinkAnnotation {
        common,
        uri: None,
        dest: Some("link_without_target".to_string()),
        dest_array: None,
    })
}

/// Extract a destination (named or explicit) from a /Dest or /D entry.
///
/// # Arguments
///
/// * `dest_obj` - The destination object (Name, String, or Array)
/// * `common` - Pre-extracted common annotation fields
/// * `resolver` - The Xref resolver for dereferencing
/// * `dests_dict` - Optional /Catalog /Dests dictionary
/// * `names_dests_ref` - Optional /Catalog /Names /Dests reference
///
/// # Returns
///
/// Some(LinkAnnotation) with dest or dest_array populated, None if invalid.
fn extract_destination(
    dest_obj: Option<&PdfObject>,
    common: &AnnotationCommon,
    resolver: &XrefResolver,
    dests_dict: Option<&PdfDict>,
    names_dests_ref: Option<crate::parser::object::ObjRef>,
) -> Option<LinkAnnotation> {
    let dest_obj = dest_obj?;

    match dest_obj {
        PdfObject::Name(name) => {
            // Named destination - try to resolve
            let resolved = resolve_named_destination(name, resolver, dests_dict, names_dests_ref);
            match resolved {
                Ok(resolved_dest) => Some(LinkAnnotation {
                    common: common.clone(),
                    uri: None,
                    dest: Some(name.to_string()),
                    dest_array: Some(resolved_dest),
                }),
                Err(_) => Some(LinkAnnotation {
                    common: common.clone(),
                    uri: None,
                    dest: Some(name.to_string()),
                    dest_array: None,
                }),
            }
        }
        PdfObject::String(bytes) => {
            // String destination - treat as name
            if let Ok(name) = String::from_utf8(bytes.to_vec()) {
                let resolved =
                    resolve_named_destination(&name, resolver, dests_dict, names_dests_ref);
                match resolved {
                    Ok(resolved_dest) => Some(LinkAnnotation {
                        common: common.clone(),
                        uri: None,
                        dest: Some(name),
                        dest_array: Some(resolved_dest),
                    }),
                    Err(_) => Some(LinkAnnotation {
                        common: common.clone(),
                        uri: None,
                        dest: Some(name),
                        dest_array: None,
                    }),
                }
            } else {
                None
            }
        }
        PdfObject::Array(arr) => {
            // Explicit destination array: [page_ref, fit_name, ...args]
            parse_explicit_destination(arr, common, resolver)
        }
        _ => None,
    }
}

/// Parse an explicit destination array.
///
/// Array format: [page_ref, /FitTypeName, params...]
fn parse_explicit_destination(
    arr: &[PdfObject],
    common: &AnnotationCommon,
    resolver: &XrefResolver,
) -> Option<LinkAnnotation> {
    if arr.len() < 2 {
        return None;
    }

    // First element is the page reference
    let page_ref = arr.first()?;
    let page_index = resolve_page_index(page_ref, resolver)?;

    // Second element is the fit type name
    let fit_name = arr.get(1)?.as_name()?;

    // Parse the fit type and coordinates
    let fit = parse_fit_type(fit_name, arr, 2)?;

    Some(LinkAnnotation {
        common: common.clone(),
        uri: None,
        dest: None,
        dest_array: Some(DestArray { page_index, fit }),
    })
}

/// Parse a fit type from a destination array.
///
/// # Arguments
///
/// * `fit_name` - The fit type name (e.g., "XYZ", "Fit", "FitH")
/// * `arr` - The destination array
/// * `start_idx` - Index where fit parameters start (usually 2)
fn parse_fit_type(fit_name: &str, arr: &[PdfObject], start_idx: usize) -> Option<FitType> {
    match fit_name {
        "XYZ" => {
            // /XYZ left top zoom
            let left = arr.get(start_idx).and_then(|o| as_f32(o));
            let top = arr.get(start_idx + 1).and_then(|o| as_f32(o));
            let zoom = arr.get(start_idx + 2).and_then(|o| as_f32(o));
            Some(FitType::Xyz { left, top, zoom })
        }
        "Fit" => Some(FitType::Fit),
        "FitH" => {
            let top = arr.get(start_idx).and_then(|o| as_f32(o));
            Some(FitType::FitH(top))
        }
        "FitV" => {
            let left = arr.get(start_idx).and_then(|o| as_f32(o));
            Some(FitType::FitV(left))
        }
        "FitR" => {
            let left = arr.get(start_idx).and_then(|o| as_f32(o))?;
            let bottom = arr.get(start_idx + 1).and_then(|o| as_f32(o))?;
            let right = arr.get(start_idx + 2).and_then(|o| as_f32(o))?;
            let top = arr.get(start_idx + 3).and_then(|o| as_f32(o))?;
            Some(FitType::FitR(left, bottom, right, top))
        }
        "FitB" => Some(FitType::FitB),
        "FitBH" => {
            let top = arr.get(start_idx).and_then(|o| as_f32(o));
            Some(FitType::FitBH(top))
        }
        "FitBV" => {
            let left = arr.get(start_idx).and_then(|o| as_f32(o));
            Some(FitType::FitBV(left))
        }
        _ => None,
    }
}

/// Resolve a page reference to a page index.
///
/// This is a simplified version - in a full implementation, this would
/// look up the page in the catalog's page tree.
fn resolve_page_index(_page_ref: &PdfObject, _resolver: &XrefResolver) -> Option<usize> {
    // For now, we can't resolve page indices without access to the page tree.
    // This is a placeholder that returns None.
    // In a full implementation, this would:
    // 1. Resolve the page reference to a page dictionary
    // 2. Look up the page in the catalog's page tree
    // 3. Return the 0-based page index
    None
}

/// Resolve a named destination via /Catalog /Dests or /Catalog /Names /Dests.
///
/// Per PDF 1.7 spec:
/// - /Catalog /Dests is a dictionary (PDF 1.2 legacy, preferred when present)
/// - /Catalog /Names /Dests is a name tree (PDF 1.3+, fallback)
///
/// Returns Ok(DestArray) if resolved, Err(()) if not found (but the link is still valid).
fn resolve_named_destination(
    name: &str,
    resolver: &XrefResolver,
    dests_dict: Option<&PdfDict>,
    names_dests_ref: Option<crate::parser::object::ObjRef>,
) -> Result<DestArray, ()> {
    // First try /Catalog /Dests (legacy dict, preferred when present)
    if let Some(dests) = dests_dict {
        // Build key with leading slash
        let key = format!("/{}", name);
        if let Some(dest_obj) = dests.get(&*key) {
            if let Some(_arr) = dest_obj.as_array() {
                // Parse the explicit destination array
                if let Some(dest_array) = parse_dest_array_from_obj(dest_obj, resolver) {
                    return Ok(dest_array);
                }
            }
        }
    }

    // Fall back to /Catalog /Names /Dests (name tree)
    if let Some(names_ref) = names_dests_ref {
        if let Ok(names_obj) = resolver.resolve(names_ref) {
            if let Some(names_dict) = names_obj.as_dict() {
                if let Some(dests_obj) = names_dict.get("/Dests") {
                    // Walk the name tree to find the destination
                    if let Some(dest_array) = walk_name_tree_for_dest(dests_obj, name, resolver) {
                        return Ok(dest_array);
                    }
                }
            }
        }
    }

    Err(())
}

/// Walk a name tree to find a named destination.
///
/// Name trees have /Limits (optional) and /Kids or /Nums entries.
fn walk_name_tree_for_dest(
    node: &PdfObject,
    target_name: &str,
    resolver: &XrefResolver,
) -> Option<DestArray> {
    let dict = node.as_dict()?;

    // Check for /Nums (leaf node - alternating key-value pairs)
    if let Some(nums_obj) = dict.get("/Nums") {
        if let Some(nums_arr) = nums_obj.as_array() {
            for chunk in nums_arr.chunks(2) {
                if chunk.len() == 2 {
                    if let Some(key_str) = chunk[0]
                        .as_string()
                        .or_else(|| chunk[0].as_name().map(|s| s.as_bytes()))
                    {
                        if let Ok(key) = String::from_utf8(key_str.to_vec()) {
                            if key == target_name {
                                // Found it - parse the destination array
                                return parse_dest_array_from_obj(&chunk[1], resolver);
                            }
                        }
                    }
                }
            }
        }
    }

    // Check for /Kids (internal node - recursive)
    if let Some(kids_obj) = dict.get("/Kids") {
        if let Some(kids_arr) = kids_obj.as_array() {
            for kid in kids_arr {
                let resolved = match kid {
                    PdfObject::Ref(ref_) => resolver.resolve(*ref_).ok(),
                    PdfObject::Dict(_) => Some(kid.clone()),
                    _ => None,
                };

                if let Some(resolved) = resolved {
                    if let Some(result) = walk_name_tree_for_dest(&resolved, target_name, resolver)
                    {
                        return Some(result);
                    }
                }
            }
        }
    }

    None
}

/// Parse a destination array from a PdfObject.
fn parse_dest_array_from_obj(obj: &PdfObject, resolver: &XrefResolver) -> Option<DestArray> {
    let arr = obj.as_array()?;
    if arr.len() < 2 {
        return None;
    }

    let page_ref = arr.first()?;
    let page_index = resolve_page_index(page_ref, resolver)?;

    let fit_name = arr.get(1)?.as_name()?;
    let fit = parse_fit_type(fit_name, &arr, 2)?;

    Some(DestArray { page_index, fit })
}

/// Convert a PdfObject to f32, handling both Real and Integer types.
fn as_f32(obj: &PdfObject) -> Option<f32> {
    obj.as_real()
        .map(|f| f as f32)
        .or_else(|| obj.as_int().map(|i| i as f32))
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

    fn make_resolver() -> XrefResolver {
        XrefResolver::new()
    }

    fn make_common() -> AnnotationCommon {
        AnnotationCommon {
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
        }
    }

    #[test]
    fn test_extract_link_uri() {
        let resolver = make_resolver();
        let mut dict = IndexMap::new();

        // Create /A dictionary with /S /URI and /URI
        let mut action_dict = IndexMap::new();
        action_dict.insert(Arc::from("/S"), PdfObject::Name("URI".into()));
        action_dict.insert(
            Arc::from("/URI"),
            PdfObject::String(Box::new(b"https://example.com".to_vec())),
        );

        dict.insert(Arc::from("/A"), PdfObject::Dict(Box::new(action_dict)));

        let common = make_common();

        let result = extract_link(&dict, common, &resolver, None, None);
        assert!(result.is_some());
        let link = result.unwrap();
        assert_eq!(link.uri, Some("https://example.com".to_string()));
        assert_eq!(link.dest, None);
        assert!(link.dest_array.is_none());
    }

    #[test]
    fn test_extract_link_named_dest() {
        let resolver = make_resolver();
        let mut dict = IndexMap::new();

        // Direct /Dest as a name
        dict.insert(Arc::from("/Dest"), PdfObject::Name("SectionTwo".into()));

        let common = make_common();

        let result = extract_link(&dict, common, &resolver, None, None);
        assert!(result.is_some());
        let link = result.unwrap();
        assert_eq!(link.uri, None);
        assert_eq!(link.dest, Some("SectionTwo".to_string()));
        assert!(link.dest_array.is_none());
    }

    #[test]
    fn test_extract_link_goto_action() {
        let resolver = make_resolver();
        let mut dict = IndexMap::new();

        // Create /A dictionary with /S /GoTo and /D
        let mut action_dict = IndexMap::new();
        action_dict.insert(Arc::from("/S"), PdfObject::Name("GoTo".into()));
        action_dict.insert(Arc::from("/D"), PdfObject::Name("Appendix".into()));

        dict.insert(Arc::from("/A"), PdfObject::Dict(Box::new(action_dict)));

        let common = make_common();

        let result = extract_link(&dict, common, &resolver, None, None);
        assert!(result.is_some());
        let link = result.unwrap();
        assert_eq!(link.uri, None);
        assert_eq!(link.dest, Some("Appendix".to_string()));
        assert!(link.dest_array.is_none());
    }

    #[test]
    fn test_extract_link_javascript_action() {
        let resolver = make_resolver();
        let mut dict = IndexMap::new();

        // Create /A dictionary with /S /JavaScript and /JS
        let mut action_dict = IndexMap::new();
        action_dict.insert(Arc::from("/S"), PdfObject::Name("JavaScript".into()));
        action_dict.insert(
            Arc::from("/JS"),
            PdfObject::String(Box::new(b"app.alert('Hello');".to_vec())),
        );

        dict.insert(Arc::from("/A"), PdfObject::Dict(Box::new(action_dict)));

        let common = make_common();

        let result = extract_link(&dict, common, &resolver, None, None);
        assert!(result.is_some());
        let link = result.unwrap();
        assert!(link.uri.is_some());
        assert_eq!(link.dest, None);
        assert!(link.dest_array.is_none());
        // URI should contain "javascript:" prefix
        let uri = link.uri.unwrap();
        assert!(uri.starts_with("javascript:"));
        assert!(uri.contains("app.alert"));
    }

    #[test]
    fn test_extract_link_javascript_truncation() {
        let resolver = make_resolver();
        let mut dict = IndexMap::new();

        // Create a long JavaScript action
        let long_js = "app.alert('Hello'); ".repeat(20); // > 100 chars
        let mut action_dict = IndexMap::new();
        action_dict.insert(Arc::from("/S"), PdfObject::Name("JavaScript".into()));
        action_dict.insert(
            Arc::from("/JS"),
            PdfObject::String(Box::new(long_js.as_bytes().to_vec())),
        );

        dict.insert(Arc::from("/A"), PdfObject::Dict(Box::new(action_dict)));

        let common = make_common();

        let result = extract_link(&dict, common, &resolver, None, None);
        assert!(result.is_some());
        let link = result.unwrap();
        let uri = link.uri.unwrap();
        // Should be truncated with "..." suffix
        assert!(uri.len() < long_js.len() + 20); // +20 for "javascript:" prefix
        assert!(uri.ends_with("..."));
    }

    #[test]
    fn test_extract_link_without_target() {
        let resolver = make_resolver();
        let dict = IndexMap::new();

        // No /A and no /Dest
        let common = make_common();

        let result = extract_link(&dict, common, &resolver, None, None);
        assert!(result.is_some());
        let link = result.unwrap();
        assert_eq!(link.uri, None);
        assert_eq!(link.dest, Some("link_without_target".to_string()));
        assert!(link.dest_array.is_none());
    }

    #[test]
    fn test_parse_fit_type_xyz() {
        let mut arr = Vec::new();
        arr.push(PdfObject::Ref(crate::parser::object::ObjRef::new(1, 0)));
        arr.push(PdfObject::Name("XYZ".into()));
        arr.push(PdfObject::Real(100.0));
        arr.push(PdfObject::Real(200.0));
        arr.push(PdfObject::Real(1.5));

        let fit = parse_fit_type("XYZ", &arr, 2);
        assert!(fit.is_some());
        match fit.unwrap() {
            FitType::Xyz { left, top, zoom } => {
                assert_eq!(left, Some(100.0));
                assert_eq!(top, Some(200.0));
                assert_eq!(zoom, Some(1.5));
            }
            _ => panic!("Expected XYZ fit type"),
        }
    }

    #[test]
    fn test_parse_fit_type_xyz_nulls() {
        let mut arr = Vec::new();
        arr.push(PdfObject::Ref(crate::parser::object::ObjRef::new(1, 0)));
        arr.push(PdfObject::Name("XYZ".into()));
        arr.push(PdfObject::Null); // null left
        arr.push(PdfObject::Real(200.0)); // top
        arr.push(PdfObject::Null); // null zoom

        let fit = parse_fit_type("XYZ", &arr, 2);
        assert!(fit.is_some());
        match fit.unwrap() {
            FitType::Xyz { left, top, zoom } => {
                assert_eq!(left, None);
                assert_eq!(top, Some(200.0));
                assert_eq!(zoom, None);
            }
            _ => panic!("Expected XYZ fit type"),
        }
    }

    #[test]
    fn test_parse_fit_type_fit() {
        let arr = vec![
            PdfObject::Ref(crate::parser::object::ObjRef::new(1, 0)),
            PdfObject::Name("Fit".into()),
        ];

        let fit = parse_fit_type("Fit", &arr, 2);
        assert_eq!(fit, Some(FitType::Fit));
    }

    #[test]
    fn test_parse_fit_type_fith() {
        let mut arr = Vec::new();
        arr.push(PdfObject::Ref(crate::parser::object::ObjRef::new(1, 0)));
        arr.push(PdfObject::Name("FitH".into()));
        arr.push(PdfObject::Real(300.0));

        let fit = parse_fit_type("FitH", &arr, 2);
        assert!(fit.is_some());
        match fit.unwrap() {
            FitType::FitH(top) => {
                assert_eq!(top, Some(300.0));
            }
            _ => panic!("Expected FitH fit type"),
        }
    }

    #[test]
    fn test_parse_fit_type_fitr() {
        let mut arr = Vec::new();
        arr.push(PdfObject::Ref(crate::parser::object::ObjRef::new(1, 0)));
        arr.push(PdfObject::Name("FitR".into()));
        arr.push(PdfObject::Real(10.0)); // left
        arr.push(PdfObject::Real(20.0)); // bottom
        arr.push(PdfObject::Real(400.0)); // right
        arr.push(PdfObject::Real(500.0)); // top

        let fit = parse_fit_type("FitR", &arr, 2);
        assert!(fit.is_some());
        match fit.unwrap() {
            FitType::FitR(left, bottom, right, top) => {
                assert_eq!(left, 10.0);
                assert_eq!(bottom, 20.0);
                assert_eq!(right, 400.0);
                assert_eq!(top, 500.0);
            }
            _ => panic!("Expected FitR fit type"),
        }
    }

    #[test]
    fn test_parse_fit_type_fitb() {
        let arr = vec![
            PdfObject::Ref(crate::parser::object::ObjRef::new(1, 0)),
            PdfObject::Name("FitB".into()),
        ];

        let fit = parse_fit_type("FitB", &arr, 2);
        assert_eq!(fit, Some(FitType::FitB));
    }

    #[test]
    fn test_parse_fit_type_fitbh() {
        let mut arr = Vec::new();
        arr.push(PdfObject::Ref(crate::parser::object::ObjRef::new(1, 0)));
        arr.push(PdfObject::Name("FitBH".into()));
        arr.push(PdfObject::Real(150.0));

        let fit = parse_fit_type("FitBH", &arr, 2);
        assert!(fit.is_some());
        match fit.unwrap() {
            FitType::FitBH(top) => {
                assert_eq!(top, Some(150.0));
            }
            _ => panic!("Expected FitBH fit type"),
        }
    }

    #[test]
    fn test_parse_fit_type_fitbv() {
        let mut arr = Vec::new();
        arr.push(PdfObject::Ref(crate::parser::object::ObjRef::new(1, 0)));
        arr.push(PdfObject::Name("FitBV".into()));
        arr.push(PdfObject::Real(75.0));

        let fit = parse_fit_type("FitBV", &arr, 2);
        assert!(fit.is_some());
        match fit.unwrap() {
            FitType::FitBV(left) => {
                assert_eq!(left, Some(75.0));
            }
            _ => panic!("Expected FitBV fit type"),
        }
    }

    #[test]
    fn test_parse_fit_type_unknown() {
        let arr = vec![
            PdfObject::Ref(crate::parser::object::ObjRef::new(1, 0)),
            PdfObject::Name("UnknownFit".into()),
        ];

        let fit = parse_fit_type("UnknownFit", &arr, 2);
        assert!(fit.is_none());
    }

    #[test]
    fn test_as_f32_with_real() {
        let obj = PdfObject::Real(42.5);
        assert_eq!(as_f32(&obj), Some(42.5_f32));
    }

    #[test]
    fn test_as_f32_with_int() {
        let obj = PdfObject::Integer(42);
        assert_eq!(as_f32(&obj), Some(42.0_f32));
    }

    #[test]
    fn test_as_f32_with_null() {
        let obj = PdfObject::Null;
        assert_eq!(as_f32(&obj), None);
    }

    #[test]
    fn test_extract_link_dest_as_string() {
        let resolver = make_resolver();
        let mut dict = IndexMap::new();

        // /Dest as a string (hex-encoded)
        dict.insert(
            Arc::from("/Dest"),
            PdfObject::String(Box::new(b"Chapter1".to_vec())),
        );

        let common = make_common();

        let result = extract_link(&dict, common, &resolver, None, None);
        assert!(result.is_some());
        let link = result.unwrap();
        assert_eq!(link.uri, None);
        assert_eq!(link.dest, Some("Chapter1".to_string()));
    }

    #[test]
    fn test_extract_link_dest_array_unresolved_page() {
        let resolver = make_resolver();
        let mut dict = IndexMap::new();

        // Explicit destination array
        let mut dest_arr = Vec::new();
        dest_arr.push(PdfObject::Ref(crate::parser::object::ObjRef::new(5, 0)));
        dest_arr.push(PdfObject::Name("XYZ".into()));
        dest_arr.push(PdfObject::Real(100.0));
        dest_arr.push(PdfObject::Real(200.0));
        dest_arr.push(PdfObject::Real(1.0));

        dict.insert(Arc::from("/Dest"), PdfObject::Array(Box::new(dest_arr)));

        let common = make_common();

        let result = extract_link(&dict, common, &resolver, None, None);
        // Should return None because we can't resolve the page index yet
        // (page index resolution requires access to the page tree)
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_named_destination_from_dests_dict() {
        let resolver = make_resolver();

        // Create a /Dests dictionary
        let mut dests = IndexMap::new();
        let mut dest_arr = Vec::new();
        dest_arr.push(PdfObject::Ref(crate::parser::object::ObjRef::new(3, 0)));
        dest_arr.push(PdfObject::Name("Fit".into()));
        dests.insert(Arc::from("Section1"), PdfObject::Array(Box::new(dest_arr)));

        let result = resolve_named_destination("Section1", &resolver, Some(&dests), None);
        // Should return Err because page index resolution is not implemented
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_named_destination_not_found() {
        let resolver = make_resolver();
        let dests = IndexMap::new();

        let result = resolve_named_destination("NonExistent", &resolver, Some(&dests), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_walk_name_tree_leaf_node() {
        let resolver = make_resolver();

        // Create a name tree leaf node with /Nums
        let mut nums = Vec::new();
        nums.push(PdfObject::String(Box::new(b"Dest1".to_vec())));
        let mut dest_arr = Vec::new();
        dest_arr.push(PdfObject::Ref(crate::parser::object::ObjRef::new(2, 0)));
        dest_arr.push(PdfObject::Name("Fit".into()));
        nums.push(PdfObject::Array(Box::new(dest_arr)));

        let mut node = IndexMap::new();
        node.insert(Arc::from("/Nums"), PdfObject::Array(Box::new(nums)));

        let result = walk_name_tree_for_dest(&PdfObject::Dict(Box::new(node)), "Dest1", &resolver);
        // Should return None because page index resolution is not implemented
        assert!(result.is_none());
    }

    #[test]
    fn test_walk_name_tree_not_found() {
        let resolver = make_resolver();

        let mut nums = Vec::new();
        nums.push(PdfObject::String(Box::new(b"OtherDest".to_vec())));
        let mut dest_arr = Vec::new();
        dest_arr.push(PdfObject::Ref(crate::parser::object::ObjRef::new(2, 0)));
        dest_arr.push(PdfObject::Name("Fit".into()));
        nums.push(PdfObject::Array(Box::new(dest_arr)));

        let mut node = IndexMap::new();
        node.insert(Arc::from("/Nums"), PdfObject::Array(Box::new(nums)));

        let result =
            walk_name_tree_for_dest(&PdfObject::Dict(Box::new(node)), "NotFound", &resolver);
        assert!(result.is_none());
    }

    #[test]
    fn test_fit_type_partial_eq() {
        // Test FitType PartialEq implementation
        assert_eq!(FitType::Fit, FitType::Fit);
        assert_eq!(FitType::FitB, FitType::FitB);
        assert_eq!(FitType::FitH(Some(100.0)), FitType::FitH(Some(100.0)));
        assert_ne!(FitType::FitH(Some(100.0)), FitType::FitH(Some(200.0)));
        assert_eq!(
            FitType::Xyz {
                left: Some(10.0),
                top: Some(20.0),
                zoom: Some(1.5)
            },
            FitType::Xyz {
                left: Some(10.0),
                top: Some(20.0),
                zoom: Some(1.5)
            }
        );
    }
}
