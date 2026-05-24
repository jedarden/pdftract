//! Annotation and hyperlink extraction from PDF pages.
//!
//! This module implements Phase 7.6: Hyperlink and Annotation Extraction.
//!
//! ## Architecture
//!
//! - **Dispatcher** (7.6.1): Walk `/Annots` arrays and dispatch by `/Subtype`
//! - **Link extractor** (7.6.2): Extract URI and internal destination links
//! - **Annotation extractor** (7.6.3): Extract non-link annotations (Highlight, Note, etc.)
//!
//! ## Reuse
//!
//! The `AnnotationCommon` struct is shared by both link and annotation extractors,
//! ensuring consistent parsing of common fields like dates, colors, and strings.

pub mod links;
pub mod other;

use crate::parser::xref::XrefResolver;
use links::LinkAnnotation;
use other::Annotation;
use std::collections::HashSet;

/// Common fields shared by all annotation subtypes.
///
/// This struct contains the fields that are extracted once and reused by
/// both link and annotation extractors, ensuring consistency.
#[derive(Debug, Clone)]
pub struct AnnotationCommon {
    /// The annotation subtype (e.g., "Link", "Highlight", "Text", "Stamp").
    pub subtype: String,
    /// The bounding rectangle `[x0, y0, x1, y1]` in PDF user-space units.
    /// None if the /Rect entry is missing or invalid.
    pub rect: Option<[f32; 4]>,
    /// The annotation's content text (from /Contents).
    /// None if /Contents is missing or not a string.
    pub contents: Option<String>,
    /// The annotation's author (from /T).
    /// None if /T is missing or not a string.
    pub author: Option<String>,
    /// The modification date (from /M) as an ISO 8601 string.
    /// None if /M is missing, malformed, or fails to parse.
    pub modified: Option<String>,
    /// The color array (from /C) as RGB/Grayscale components.
    /// None if /C is missing. Length is 1 (grayscale), 3 (RGB), or 4 (CMYK).
    pub color: Option<Vec<f32>>,
    /// The opacity (from /CA), defaulting to 1.0.
    pub opacity: Option<f32>,
    /// The annotation flags bitmask (from /F).
    pub flags: u32,
    /// The name identifier (from /NM).
    /// None if /NM is missing.
    pub name_id: Option<String>,
    /// The subject (from /Subj).
    /// None if /Subj is missing.
    pub subject: Option<String>,
    /// The zero-based page index containing this annotation.
    pub page_index: usize,
}

/// Dispatch annotations by subtype, separating links from other annotations.
///
/// This function implements Phase 7.6.1: it walks the `/Annots` array for each
/// page and dispatches each annotation based on its `/Subtype`:
///
/// - `/Link` → routed to link extractor (7.6.2)
/// - `/Widget` → skipped (handled by form field extractor 7.4)
/// - `/Popup` → skipped (companion to other annotations)
/// - All other subtypes → routed to annotation extractor (7.6.3)
///
/// # Arguments
///
/// * `resolver` - The Xref resolver for dereferencing indirect objects
/// * `pages` - Slice of page dictionaries with their annotation references
/// * `dests_dict` - Optional /Catalog /Dests dictionary for named destination resolution
/// * `names_dests_ref` - Optional /Catalog /Names /Dests reference for name tree resolution
///
/// # Returns
///
/// A tuple of `(Vec<LinkAnnotation>, Vec<Annotation>)` containing all extracted
/// link annotations and non-link annotations across all pages.
///
/// # Behavior
///
/// - Pages with no `/Annots` entry or an empty array contribute empty lists.
/// - Annotations with missing `/Subtype` are skipped with a diagnostic.
/// - Dereference loops are detected and skipped with a diagnostic.
/// - Output order follows document order (the order of /Annots arrays).
pub fn dispatch_annotations(
    resolver: &XrefResolver,
    pages: &[crate::parser::pages::PageDict],
    dests_dict: Option<&crate::parser::object::PdfDict>,
    names_dests_ref: Option<crate::parser::object::ObjRef>,
) -> (Vec<LinkAnnotation>, Vec<Annotation>) {
    let mut all_links = Vec::new();
    let mut all_annotations = Vec::new();
    let mut visited = HashSet::new();

    for (page_index, page) in pages.iter().enumerate() {
        let page_annot_refs = &page.annots;

        if page_annot_refs.is_empty() {
            continue;
        }

        for &annot_ref in page_annot_refs {
            // Detect dereference loops
            if !visited.insert(annot_ref) {
                // Create a placeholder link for loop detection
                all_links.push(LinkAnnotation {
                    common: AnnotationCommon {
                        subtype: "Loop".to_string(),
                        rect: None,
                        contents: None,
                        author: None,
                        modified: None,
                        color: None,
                        opacity: None,
                        flags: 0,
                        name_id: None,
                        subject: None,
                        page_index,
                    },
                    uri: None,
                    dest: None,
                    dest_array: None,
                });
                continue;
            }

            // Resolve the annotation dictionary
            let annot_dict = match resolver.resolve(annot_ref) {
                Ok(crate::parser::object::PdfObject::Dict(dict)) => dict,
                Ok(_) => {
                    // Not a dictionary - skip
                    continue;
                }
                Err(_) => {
                    // Failed to resolve - skip
                    continue;
                }
            };

            // Extract the subtype (keys in PDF dicts include the leading /)
            let subtype = match annot_dict.get("/Subtype").and_then(|o| o.as_name()) {
                Some(name) => name.to_string(),
                None => {
                    // Missing subtype - skip
                    continue;
                }
            };

            // Skip Widget (form fields handled by 7.4) and Popup (companion subtype)
            if subtype == "Widget" || subtype == "Popup" {
                continue;
            }

            // Extract common fields
            let common = extract_common_fields(&annot_dict, &subtype, page_index, resolver);

            // Dispatch by subtype
            if subtype == "Link" {
                if let Some(link) =
                    links::extract_link(&annot_dict, common, resolver, dests_dict, names_dests_ref)
                {
                    all_links.push(link);
                }
            } else {
                if let Some(annotation) = other::extract_annotation(&annot_dict, common, resolver) {
                    all_annotations.push(annotation);
                }
            }
        }
    }

    (all_links, all_annotations)
}

/// Extract common annotation fields from an annotation dictionary.
///
/// This function parses the shared fields used by all annotation types:
/// /Rect, /Contents, /T, /M, /C, /CA, /F, /NM, /Subj.
///
/// # Arguments
///
/// * `dict` - The annotation dictionary
/// * `subtype` - The annotation subtype (already extracted)
/// * `page_index` - The zero-based page index
/// * `resolver` - The Xref resolver for dereferencing indirect objects
///
/// # Returns
///
/// An `AnnotationCommon` struct with all extractable fields.
fn extract_common_fields(
    dict: &crate::parser::object::PdfDict,
    subtype: &str,
    page_index: usize,
    _resolver: &XrefResolver,
) -> AnnotationCommon {
    // Extract /Rect (bounding box) - PDF dict keys include the leading /
    let rect = dict.get("/Rect").and_then(|obj| {
        if let Some(arr) = obj.as_array() {
            if arr.len() == 4 {
                let coords: Vec<Option<f32>> = arr
                    .iter()
                    .map(|o| {
                        o.as_real()
                            .map(|f| f as f32)
                            .or_else(|| o.as_int().map(|i| i as f32))
                    })
                    .collect();

                if coords.iter().all(|c| c.is_some()) {
                    Some([
                        coords[0].unwrap(),
                        coords[1].unwrap(),
                        coords[2].unwrap(),
                        coords[3].unwrap(),
                    ])
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    });

    // Extract /Contents (annotation text)
    let contents = dict
        .get("/Contents")
        .and_then(|o| o.as_string())
        .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());

    // Extract /T (author)
    let author = dict
        .get("/T")
        .and_then(|o| o.as_string())
        .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());

    // Extract /M (modification date) and parse to ISO 8601
    let modified = dict
        .get("/M")
        .and_then(|o| o.as_string())
        .and_then(parse_pdf_date);

    // Extract /C (color array)
    let color = dict.get("/C").and_then(|obj| {
        if let Some(arr) = obj.as_array() {
            let colors: Vec<Option<f32>> = arr
                .iter()
                .map(|o| {
                    o.as_real()
                        .map(|f| f as f32)
                        .or_else(|| o.as_int().map(|i| i as f32))
                })
                .collect();

            if colors.iter().all(|c| c.is_some()) {
                Some(colors.into_iter().map(|c| c.unwrap()).collect())
            } else {
                None
            }
        } else {
            obj.as_real()
                .map(|f| vec![f as f32])
                .or_else(|| obj.as_int().map(|i| vec![i as f32]))
        }
    });

    // Extract /CA (opacity), default 1.0
    let opacity = dict
        .get("/CA")
        .and_then(|o| o.as_real())
        .map(|f| f as f32)
        .or_else(|| dict.get("/CA").and_then(|o| o.as_int()).map(|i| i as f32));

    // Extract /F (flags), default 0
    let flags = dict.get("/F").and_then(|o| o.as_int()).unwrap_or(0) as u32;

    // Extract /NM (name identifier)
    let name_id = dict
        .get("/NM")
        .and_then(|o| o.as_string())
        .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());

    // Extract /Subj (subject)
    let subject = dict
        .get("/Subj")
        .and_then(|o| o.as_string())
        .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());

    AnnotationCommon {
        subtype: subtype.to_string(),
        rect,
        contents,
        author,
        modified,
        color,
        opacity,
        flags,
        name_id,
        subject,
        page_index,
    }
}

/// Parse a PDF date string to ISO 8601 format.
///
/// PDF date format: `D:YYYYMMDDHHmmSSOHH'mm'`
/// - Truncation is allowed (date only, date+time only)
/// - Timezone can be `Z`, `+HH'mm'`, `-HH'mm'`, or omitted (defaults to UTC)
///
/// Returns ISO 8601 format (RFC 3339) or None if parsing fails.
fn parse_pdf_date(pdf_date: &[u8]) -> Option<String> {
    let date_str = std::str::from_utf8(pdf_date).ok()?;

    // Strip "D:" prefix if present
    let date_str = date_str.strip_prefix("D:").unwrap_or(date_str);

    // Minimum required: YYYYMMDD (8 characters after stripping D:)
    if date_str.len() < 8 {
        return None;
    }

    // Parse date components
    let year = date_str[0..4].parse::<u32>().ok()?;
    let month = date_str[4..6].parse::<u32>().ok()?;
    let day = date_str[6..8].parse::<u32>().ok()?;

    // Validate date ranges
    if month == 0 || month > 12 || day == 0 || day > 31 {
        return None;
    }

    // Parse time components if present
    let (hour, minute, second) = if date_str.len() >= 14 {
        let hour = date_str[8..10].parse::<u32>().ok()?;
        let minute = date_str[10..12].parse::<u32>().ok()?;
        let second = date_str[12..14].parse::<u32>().ok()?;

        // Validate time ranges
        if hour > 23 || minute > 59 || second > 59 {
            return None;
        }
        (hour, minute, second)
    } else {
        // Default to midnight if time not present
        (0, 0, 0)
    };

    // Parse timezone if present
    let tz_str = if date_str.len() > 14 {
        &date_str[14..]
    } else {
        ""
    };

    // Build ISO 8601 string
    let mut iso_string = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        year, month, day, hour, minute, second
    );

    // Handle timezone
    if tz_str.is_empty() || tz_str == "Z" {
        iso_string.push('Z');
    } else if let Some(offset_str) = tz_str.strip_prefix('+') {
        // Parse +HH'mm' or +HHmm
        let offset_clean = offset_str.replace("'", "");
        if offset_clean.len() >= 3 {
            let tz_hour: u32 = offset_clean[0..2].parse().unwrap_or(0);
            let tz_min: u32 = if offset_clean.len() >= 4 {
                offset_clean[2..4].parse().unwrap_or(0)
            } else {
                0
            };
            iso_string.push_str(&format!("+{:02}:{:02}", tz_hour, tz_min));
        } else {
            iso_string.push('Z');
        }
    } else if let Some(offset_str) = tz_str.strip_prefix('-') {
        // Parse -HH'mm' or -HHmm
        let offset_clean = offset_str.replace("'", "");
        if offset_clean.len() >= 3 {
            let tz_hour: u32 = offset_clean[0..2].parse().unwrap_or(0);
            let tz_min: u32 = if offset_clean.len() >= 4 {
                offset_clean[2..4].parse().unwrap_or(0)
            } else {
                0
            };
            iso_string.push_str(&format!("-{:02}:{:02}", tz_hour, tz_min));
        } else {
            iso_string.push('Z');
        }
    } else {
        iso_string.push('Z');
    }

    Some(iso_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pdf_date_full_with_timezone() {
        let date = b"D:20230515143045+05'30'";
        let result = parse_pdf_date(date);
        assert_eq!(result, Some("2023-05-15T14:30:45+05:30".to_string()));
    }

    #[test]
    fn test_parse_pdf_date_utc() {
        let date = b"D:20230515143045Z";
        let result = parse_pdf_date(date);
        assert_eq!(result, Some("2023-05-15T14:30:45Z".to_string()));
    }

    #[test]
    fn test_parse_pdf_date_negative_timezone() {
        let date = b"D:20230515143045-08'00'";
        let result = parse_pdf_date(date);
        assert_eq!(result, Some("2023-05-15T14:30:45-08:00".to_string()));
    }

    #[test]
    fn test_parse_pdf_date_only() {
        let date = b"D:20230515";
        let result = parse_pdf_date(date);
        assert_eq!(result, Some("2023-05-15T00:00:00Z".to_string()));
    }

    #[test]
    fn test_parse_pdf_date_no_timezone() {
        let date = b"D:20230515143045";
        let result = parse_pdf_date(date);
        assert_eq!(result, Some("2023-05-15T14:30:45Z".to_string()));
    }

    #[test]
    fn test_parse_pdf_date_malformed() {
        let date = b"invalid";
        let result = parse_pdf_date(date);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_pdf_date_without_d_prefix() {
        let date = b"20230515";
        let result = parse_pdf_date(date);
        assert_eq!(result, Some("2023-05-15T00:00:00Z".to_string()));
    }
}
