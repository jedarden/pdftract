//! Non-link annotation extraction (Phase 7.6.3).
//!
//! This module extracts non-link annotations such as Highlight, Stamp,
//! FreeText, Note, Squiggly, StrikeOut, Underline, etc.

use crate::annotation::AnnotationCommon;
use crate::parser::object::{PdfDict, PdfObject};
use crate::parser::xref::XrefResolver;

/// Subtype-specific fields for non-link annotations.
///
/// Different annotation subtypes have additional fields beyond the common
/// fields. This enum captures those subtype-specific extras.
#[derive(Debug, Clone)]
pub enum AnnotationSpecific {
    /// Highlight, Squiggly, StrikeOut, Underline: quad points for the highlighted regions.
    TextMarkup {
        /// Array of 8-float quads representing the highlighted regions
        /// (each quad is x1,y1,x2,y2,x3,y3,x4,y4 in reading order)
        quads: Vec<[f32; 8]>,
    },
    /// Stamp annotation: icon name.
    Stamp {
        /// Icon name for the stamp (e.g., "Approved", "Draft", "Confidential")
        name: Option<String>,
    },
    /// FreeText annotation: default appearance string.
    FreeText {
        /// Default appearance string for the text (e.g., "1 Tf 0 g")
        da: Option<String>,
    },
    /// Text (sticky note) annotation: open state and model.
    Text {
        /// Whether the note is initially open when the page is viewed
        open: Option<bool>,
        /// State string for the note (e.g., "Reviewed", "Accepted")
        state: Option<String>,
        /// State model (e.g., "Marked", "Review")
        state_model: Option<String>,
    },
    /// Ink annotation: stroke paths.
    Ink {
        /// Array of stroke paths, where each path is a series of (x, y) points
        strokes: Vec<Vec<[f32; 2]>>,
    },
    /// Line annotation: endpoints.
    Line {
        /// Line endpoints as [x1, y1, x2, y2]
        endpoints: Option<[f32; 4]>,
    },
    /// Polygon or PolyLine annotation: vertices.
    Polygon {
        /// Array of (x, y) coordinate pairs for the polygon/polyline vertices
        vertices: Vec<[f32; 2]>,
    },
    /// FileAttachment annotation: filespec reference.
    FileAttachment {
        /// Reference to the file specification dictionary for the attached file
        fs_ref: Option<crate::parser::object::ObjRef>,
    },
    /// Circle, Square, Caret, Redact, Sound, Movie, Screen, PrinterMark, TrapNet, Watermark, 3D:
    /// No additional subtype-specific fields extracted.
    Other,
}

/// A non-link annotation extracted from a PDF page.
///
/// Represents markup annotations like highlights, text notes, stamps,
/// and other non-link annotations.
#[derive(Debug, Clone)]
pub struct Annotation {
    /// Common annotation fields (subtype, rect, contents, etc.).
    pub common: AnnotationCommon,
    /// Subtype-specific fields.
    pub specific: AnnotationSpecific,
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
/// * `resolver` - The Xref resolver for dereferencing indirect objects
///
/// # Returns
///
/// Some(Annotation) for valid non-link annotations, None for skipped types.
pub(crate) fn extract_annotation(
    dict: &PdfDict,
    common: AnnotationCommon,
    resolver: &XrefResolver,
) -> Option<Annotation> {
    let subtype = &common.subtype;

    // Dispatch based on subtype to extract subtype-specific fields
    let specific = match subtype.as_str() {
        "Highlight" | "Squiggly" | "StrikeOut" | "Underline" => extract_text_markup(dict, resolver),
        "Stamp" => extract_stamp(dict),
        "FreeText" => extract_freetext(dict),
        "Text" => extract_text_note(dict),
        "Ink" => extract_ink(dict, resolver),
        "Line" => extract_line(dict),
        "Polygon" | "PolyLine" => extract_polygon(dict, resolver),
        "FileAttachment" => extract_file_attachment(dict),
        "Circle" | "Square" | "Caret" | "Redact" | "Sound" | "Movie" | "Screen" | "PrinterMark"
        | "TrapNet" | "Watermark" | "3D" => AnnotationSpecific::Other,
        _ => {
            // Unknown subtype - emit as Other with a note
            // In production, this would emit a diagnostic
            AnnotationSpecific::Other
        }
    };

    Some(Annotation { common, specific })
}

/// Extract quad points from text markup annotations (Highlight, Squiggly, StrikeOut, Underline).
///
/// Per PDF 1.7 spec, /QuadPoints is an array of 8*N floats representing N quads,
/// where each quad is (x1, y1, x2, y2, x3, y3, x4, y4) in reading order.
fn extract_text_markup(dict: &PdfDict, _resolver: &XrefResolver) -> AnnotationSpecific {
    let quads = dict
        .get("/QuadPoints")
        .and_then(|obj| extract_quad_array(obj));
    AnnotationSpecific::TextMarkup {
        quads: quads.unwrap_or_default(),
    }
}

/// Extract an array of 8-float quads from a PdfObject.
fn extract_quad_array(obj: &PdfObject) -> Option<Vec<[f32; 8]>> {
    let arr = obj.as_array()?;
    if arr.len() % 8 != 0 {
        return None;
    }

    let mut quads = Vec::new();
    for chunk in arr.chunks(8) {
        if chunk.len() == 8 {
            let coords: Vec<Option<f32>> = chunk.iter().map(|o| as_f32(o)).collect();

            if coords.iter().all(|c| c.is_some()) {
                quads.push([
                    coords[0].unwrap(),
                    coords[1].unwrap(),
                    coords[2].unwrap(),
                    coords[3].unwrap(),
                    coords[4].unwrap(),
                    coords[5].unwrap(),
                    coords[6].unwrap(),
                    coords[7].unwrap(),
                ]);
            }
        }
    }

    if quads.is_empty() {
        None
    } else {
        Some(quads)
    }
}

/// Extract the /Name field from a Stamp annotation.
fn extract_stamp(dict: &PdfDict) -> AnnotationSpecific {
    let name = dict
        .get("/Name")
        .and_then(|o| o.as_name())
        .map(|s| s.to_string());
    AnnotationSpecific::Stamp { name }
}

/// Extract the /DA (default appearance) field from a FreeText annotation.
fn extract_freetext(dict: &PdfDict) -> AnnotationSpecific {
    let da = dict
        .get("/DA")
        .and_then(|o| o.as_string())
        .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());
    AnnotationSpecific::FreeText { da }
}

/// Extract the /Open, /State, /StateModel fields from a Text (sticky note) annotation.
fn extract_text_note(dict: &PdfDict) -> AnnotationSpecific {
    let open = dict.get("/Open").and_then(|o| o.as_bool());
    let state = dict
        .get("/State")
        .and_then(|o| o.as_string())
        .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());
    let state_model = dict
        .get("/StateModel")
        .and_then(|o| o.as_name())
        .map(|s| s.to_string());
    AnnotationSpecific::Text {
        open,
        state,
        state_model,
    }
}

/// Extract the /InkList field from an Ink annotation.
///
/// /InkList is an array of stroke arrays, where each stroke is an array of (x, y) points.
fn extract_ink(dict: &PdfDict, resolver: &XrefResolver) -> AnnotationSpecific {
    let strokes = dict
        .get("/InkList")
        .and_then(|obj| extract_ink_list(obj, resolver));
    AnnotationSpecific::Ink {
        strokes: strokes.unwrap_or_default(),
    }
}

/// Extract an ink list from a PdfObject.
fn extract_ink_list(obj: &PdfObject, resolver: &XrefResolver) -> Option<Vec<Vec<[f32; 2]>>> {
    let arr = obj.as_array()?;
    let mut strokes = Vec::new();

    for stroke_obj in arr {
        let stroke_arr = match stroke_obj {
            PdfObject::Array(arr) => arr.to_vec(),
            PdfObject::Ref(r) => match resolver.resolve(*r) {
                Ok(PdfObject::Array(arr)) => arr.to_vec(),
                _ => continue,
            },
            _ => continue,
        };

        let mut points = Vec::new();
        for chunk in stroke_arr.chunks(2) {
            if chunk.len() == 2 {
                if let (Some(x), Some(y)) = (as_f32(&chunk[0]), as_f32(&chunk[1])) {
                    points.push([x, y]);
                }
            }
        }

        if !points.is_empty() {
            strokes.push(points);
        }
    }

    if strokes.is_empty() {
        None
    } else {
        Some(strokes)
    }
}

/// Extract the /L field from a Line annotation.
///
/// /L is an array of 4 floats: [x1, y1, x2, y2].
fn extract_line(dict: &PdfDict) -> AnnotationSpecific {
    let endpoints = dict.get("/L").and_then(|obj| {
        let arr = obj.as_array()?;
        if arr.len() != 4 {
            return None;
        }

        let coords: Vec<Option<f32>> = arr.iter().map(|o| as_f32(o)).collect();

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
    });

    AnnotationSpecific::Line { endpoints }
}

/// Extract the /Vertices field from a Polygon or PolyLine annotation.
///
/// /Vertices is an array of (x, y) coordinate pairs.
fn extract_polygon(dict: &PdfDict, resolver: &XrefResolver) -> AnnotationSpecific {
    let vertices = dict
        .get("/Vertices")
        .and_then(|obj| extract_vertices(obj, resolver));
    AnnotationSpecific::Polygon {
        vertices: vertices.unwrap_or_default(),
    }
}

/// Extract vertices from a PdfObject.
fn extract_vertices(obj: &PdfObject, resolver: &XrefResolver) -> Option<Vec<[f32; 2]>> {
    let arr = match obj {
        PdfObject::Array(arr) => arr.to_vec(),
        PdfObject::Ref(r) => match resolver.resolve(*r) {
            Ok(PdfObject::Array(arr)) => arr.to_vec(),
            _ => return None,
        },
        _ => return None,
    };

    let mut vertices = Vec::new();
    for chunk in arr.chunks(2) {
        if chunk.len() == 2 {
            if let (Some(x), Some(y)) = (as_f32(&chunk[0]), as_f32(&chunk[1])) {
                vertices.push([x, y]);
            }
        }
    }

    if vertices.is_empty() {
        None
    } else {
        Some(vertices)
    }
}

/// Extract the /FS field from a FileAttachment annotation.
fn extract_file_attachment(dict: &PdfDict) -> AnnotationSpecific {
    let fs_ref = dict.get("/FS").and_then(|o| o.as_ref());
    AnnotationSpecific::FileAttachment { fs_ref }
}

/// Convert a PdfObject to f32, handling both Real and Integer types.
fn as_f32(obj: &PdfObject) -> Option<f32> {
    obj.as_real()
        .map(|f| f as f32)
        .or_else(|| obj.as_int().map(|i| i as f32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::AnnotationCommon;
    use crate::parser::object::PdfObject;
    use crate::parser::xref::XrefResolver;
    use indexmap::IndexMap;
    use std::sync::Arc;

    fn make_resolver() -> XrefResolver {
        XrefResolver::new()
    }

    fn make_common(subtype: &str) -> AnnotationCommon {
        AnnotationCommon {
            subtype: subtype.to_string(),
            rect: Some([10.0, 20.0, 100.0, 30.0]),
            contents: Some("Test content".to_string()),
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
    fn test_extract_highlight_annotation_with_quads() {
        let resolver = make_resolver();
        let mut dict = IndexMap::new();

        // Add /QuadPoints for a highlight (2 quads = 16 floats)
        let mut quads = Vec::new();
        for i in 0..16 {
            quads.push(PdfObject::Real(i as f64));
        }
        dict.insert(Arc::from("/QuadPoints"), PdfObject::Array(Box::new(quads)));

        let common = make_common("Highlight");
        let result = extract_annotation(&dict, common, &resolver);
        assert!(result.is_some());
        let annotation = result.unwrap();
        assert_eq!(annotation.common.subtype, "Highlight");

        match annotation.specific {
            AnnotationSpecific::TextMarkup { ref quads } => {
                assert_eq!(quads.len(), 2);
                assert_eq!(quads[0], [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);
            }
            _ => panic!("Expected TextMarkup specific fields"),
        }
    }

    #[test]
    fn test_extract_highlight_annotation_no_quads() {
        let resolver = make_resolver();
        let dict = IndexMap::new();

        let common = make_common("Highlight");
        let result = extract_annotation(&dict, common, &resolver);
        assert!(result.is_some());
        let annotation = result.unwrap();

        match annotation.specific {
            AnnotationSpecific::TextMarkup { ref quads } => {
                assert!(quads.is_empty());
            }
            _ => panic!("Expected TextMarkup specific fields"),
        }
    }

    #[test]
    fn test_extract_stamp_annotation() {
        let resolver = make_resolver();
        let mut dict = IndexMap::new();

        dict.insert(Arc::from("/Name"), PdfObject::Name("Approved".into()));

        let common = make_common("Stamp");
        let result = extract_annotation(&dict, common, &resolver);
        assert!(result.is_some());
        let annotation = result.unwrap();
        assert_eq!(annotation.common.subtype, "Stamp");

        match annotation.specific {
            AnnotationSpecific::Stamp { ref name } => {
                assert_eq!(name.as_deref(), Some("Approved"));
            }
            _ => panic!("Expected Stamp specific fields"),
        }
    }

    #[test]
    fn test_extract_stamp_no_name() {
        let resolver = make_resolver();
        let dict = IndexMap::new();

        let common = make_common("Stamp");
        let result = extract_annotation(&dict, common, &resolver);
        assert!(result.is_some());
        let annotation = result.unwrap();

        match annotation.specific {
            AnnotationSpecific::Stamp { ref name } => {
                assert!(name.is_none());
            }
            _ => panic!("Expected Stamp specific fields"),
        }
    }

    #[test]
    fn test_extract_freetext_annotation() {
        let resolver = make_resolver();
        let mut dict = IndexMap::new();

        dict.insert(
            Arc::from("/DA"),
            PdfObject::String(Box::new(b"1 Tf 0 g".to_vec())),
        );

        let common = make_common("FreeText");
        let result = extract_annotation(&dict, common, &resolver);
        assert!(result.is_some());
        let annotation = result.unwrap();
        assert_eq!(annotation.common.subtype, "FreeText");

        match annotation.specific {
            AnnotationSpecific::FreeText { ref da } => {
                assert_eq!(da.as_deref(), Some("1 Tf 0 g"));
            }
            _ => panic!("Expected FreeText specific fields"),
        }
    }

    #[test]
    fn test_extract_text_note_annotation() {
        let resolver = make_resolver();
        let mut dict = IndexMap::new();

        dict.insert(Arc::from("/Open"), PdfObject::Bool(true));
        dict.insert(
            Arc::from("/State"),
            PdfObject::String(Box::new(b"Reviewed".to_vec())),
        );
        dict.insert(Arc::from("/StateModel"), PdfObject::Name("Marked".into()));

        let common = make_common("Text");
        let result = extract_annotation(&dict, common, &resolver);
        assert!(result.is_some());
        let annotation = result.unwrap();
        assert_eq!(annotation.common.subtype, "Text");

        match annotation.specific {
            AnnotationSpecific::Text {
                open,
                ref state,
                ref state_model,
            } => {
                assert_eq!(open, Some(true));
                assert_eq!(state.as_deref(), Some("Reviewed"));
                assert_eq!(state_model.as_deref(), Some("Marked"));
            }
            _ => panic!("Expected Text specific fields"),
        }
    }

    #[test]
    fn test_extract_ink_annotation() {
        let resolver = make_resolver();
        let mut dict = IndexMap::new();

        // /InkList with two strokes: first stroke has 2 points, second has 3 points
        let stroke1 = vec![
            PdfObject::Real(10.0),
            PdfObject::Real(20.0),
            PdfObject::Real(30.0),
            PdfObject::Real(40.0),
        ];
        let stroke2 = vec![
            PdfObject::Real(50.0),
            PdfObject::Real(60.0),
            PdfObject::Real(70.0),
            PdfObject::Real(80.0),
            PdfObject::Real(90.0),
            PdfObject::Real(100.0),
        ];
        dict.insert(
            Arc::from("/InkList"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Array(Box::new(stroke1)),
                PdfObject::Array(Box::new(stroke2)),
            ])),
        );

        let common = make_common("Ink");
        let result = extract_annotation(&dict, common, &resolver);
        assert!(result.is_some());
        let annotation = result.unwrap();
        assert_eq!(annotation.common.subtype, "Ink");

        match annotation.specific {
            AnnotationSpecific::Ink { ref strokes } => {
                assert_eq!(strokes.len(), 2);
                assert_eq!(strokes[0].len(), 2);
                assert_eq!(strokes[0][0], [10.0, 20.0]);
                assert_eq!(strokes[1].len(), 3);
            }
            _ => panic!("Expected Ink specific fields"),
        }
    }

    #[test]
    fn test_extract_line_annotation() {
        let resolver = make_resolver();
        let mut dict = IndexMap::new();

        dict.insert(
            Arc::from("/L"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Real(10.0),
                PdfObject::Real(20.0),
                PdfObject::Real(100.0),
                PdfObject::Real(200.0),
            ])),
        );

        let common = make_common("Line");
        let result = extract_annotation(&dict, common, &resolver);
        assert!(result.is_some());
        let annotation = result.unwrap();
        assert_eq!(annotation.common.subtype, "Line");

        match annotation.specific {
            AnnotationSpecific::Line { ref endpoints } => {
                assert_eq!(endpoints.as_ref(), Some(&[10.0, 20.0, 100.0, 200.0]));
            }
            _ => panic!("Expected Line specific fields"),
        }
    }

    #[test]
    fn test_extract_polygon_annotation() {
        let resolver = make_resolver();
        let mut dict = IndexMap::new();

        // /Vertices with 3 points (triangle)
        dict.insert(
            Arc::from("/Vertices"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Real(10.0),
                PdfObject::Real(20.0),
                PdfObject::Real(30.0),
                PdfObject::Real(40.0),
                PdfObject::Real(50.0),
                PdfObject::Real(60.0),
            ])),
        );

        let common = make_common("Polygon");
        let result = extract_annotation(&dict, common, &resolver);
        assert!(result.is_some());
        let annotation = result.unwrap();
        assert_eq!(annotation.common.subtype, "Polygon");

        match annotation.specific {
            AnnotationSpecific::Polygon { ref vertices } => {
                assert_eq!(vertices.len(), 3);
                assert_eq!(vertices[0], [10.0, 20.0]);
                assert_eq!(vertices[1], [30.0, 40.0]);
                assert_eq!(vertices[2], [50.0, 60.0]);
            }
            _ => panic!("Expected Polygon specific fields"),
        }
    }

    #[test]
    fn test_extract_polyline_annotation() {
        let resolver = make_resolver();
        let mut dict = IndexMap::new();

        dict.insert(
            Arc::from("/Vertices"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Real(0.0),
                PdfObject::Real(0.0),
                PdfObject::Real(10.0),
                PdfObject::Real(10.0),
                PdfObject::Real(20.0),
                PdfObject::Real(20.0),
            ])),
        );

        let common = make_common("PolyLine");
        let result = extract_annotation(&dict, common, &resolver);
        assert!(result.is_some());
        let annotation = result.unwrap();
        assert_eq!(annotation.common.subtype, "PolyLine");

        match annotation.specific {
            AnnotationSpecific::Polygon { ref vertices } => {
                assert_eq!(vertices.len(), 3);
            }
            _ => panic!("Expected Polygon specific fields"),
        }
    }

    #[test]
    fn test_extract_file_attachment_annotation() {
        let resolver = make_resolver();
        let mut dict = IndexMap::new();

        let fs_ref = crate::parser::object::ObjRef::new(42, 0);
        dict.insert(Arc::from("/FS"), PdfObject::Ref(fs_ref));

        let common = make_common("FileAttachment");
        let result = extract_annotation(&dict, common, &resolver);
        assert!(result.is_some());
        let annotation = result.unwrap();
        assert_eq!(annotation.common.subtype, "FileAttachment");

        match annotation.specific {
            AnnotationSpecific::FileAttachment { fs_ref } => {
                assert_eq!(fs_ref, Some(crate::parser::object::ObjRef::new(42, 0)));
            }
            _ => panic!("Expected FileAttachment specific fields"),
        }
    }

    #[test]
    fn test_extract_circle_annotation() {
        let resolver = make_resolver();
        let dict = IndexMap::new();

        let common = make_common("Circle");
        let result = extract_annotation(&dict, common, &resolver);
        assert!(result.is_some());
        let annotation = result.unwrap();

        match annotation.specific {
            AnnotationSpecific::Other => {}
            _ => panic!("Expected Other specific fields for Circle"),
        }
    }

    #[test]
    fn test_extract_square_annotation() {
        let resolver = make_resolver();
        let dict = IndexMap::new();

        let common = make_common("Square");
        let result = extract_annotation(&dict, common, &resolver);
        assert!(result.is_some());
        let annotation = result.unwrap();

        match annotation.specific {
            AnnotationSpecific::Other => {}
            _ => panic!("Expected Other specific fields for Square"),
        }
    }

    #[test]
    fn test_extract_unknown_subtype() {
        let resolver = make_resolver();
        let dict = IndexMap::new();

        let common = make_common("UnknownSubtype");
        let result = extract_annotation(&dict, common, &resolver);
        assert!(result.is_some());
        let annotation = result.unwrap();
        assert_eq!(annotation.common.subtype, "UnknownSubtype");

        // Unknown subtypes should get Other specific fields
        match annotation.specific {
            AnnotationSpecific::Other => {}
            _ => panic!("Expected Other specific fields for unknown subtype"),
        }
    }

    #[test]
    fn test_extract_quad_array_invalid_length() {
        // QuadPoints with invalid length (not divisible by 8)
        let arr = vec![
            PdfObject::Real(1.0),
            PdfObject::Real(2.0),
            PdfObject::Real(3.0),
        ];

        let result = extract_quad_array(&PdfObject::Array(Box::new(arr)));
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_quad_array_single_quad() {
        let arr = vec![
            PdfObject::Real(0.0),
            PdfObject::Real(1.0),
            PdfObject::Real(2.0),
            PdfObject::Real(3.0),
            PdfObject::Real(4.0),
            PdfObject::Real(5.0),
            PdfObject::Real(6.0),
            PdfObject::Real(7.0),
        ];

        let result = extract_quad_array(&PdfObject::Array(Box::new(arr)));
        assert!(result.is_some());
        let quads = result.unwrap();
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0], [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);
    }

    #[test]
    fn test_extract_quad_array_with_nulls() {
        // Null values in the quad array should be skipped
        let arr = vec![
            PdfObject::Real(0.0),
            PdfObject::Real(1.0),
            PdfObject::Null, // This quad should be skipped
            PdfObject::Real(3.0),
            PdfObject::Real(4.0),
            PdfObject::Real(5.0),
            PdfObject::Real(6.0),
            PdfObject::Real(7.0),
            PdfObject::Real(8.0),
            PdfObject::Real(9.0),
            PdfObject::Real(10.0),
            PdfObject::Real(11.0),
            PdfObject::Real(12.0),
            PdfObject::Real(13.0),
            PdfObject::Real(14.0),
            PdfObject::Real(15.0),
        ];

        let result = extract_quad_array(&PdfObject::Array(Box::new(arr)));
        assert!(result.is_some());
        let quads = result.unwrap();
        // Only the second valid quad should be extracted
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0], [8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0]);
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
    fn test_squiggly_subtype() {
        let resolver = make_resolver();
        let dict = IndexMap::new();

        let common = make_common("Squiggly");
        let result = extract_annotation(&dict, common, &resolver);
        assert!(result.is_some());
        let annotation = result.unwrap();
        assert_eq!(annotation.common.subtype, "Squiggly");

        match annotation.specific {
            AnnotationSpecific::TextMarkup { .. } => {}
            _ => panic!("Expected TextMarkup for Squiggly"),
        }
    }

    #[test]
    fn test_strikeout_subtype() {
        let resolver = make_resolver();
        let dict = IndexMap::new();

        let common = make_common("StrikeOut");
        let result = extract_annotation(&dict, common, &resolver);
        assert!(result.is_some());
        let annotation = result.unwrap();
        assert_eq!(annotation.common.subtype, "StrikeOut");

        match annotation.specific {
            AnnotationSpecific::TextMarkup { .. } => {}
            _ => panic!("Expected TextMarkup for StrikeOut"),
        }
    }

    #[test]
    fn test_underline_subtype() {
        let resolver = make_resolver();
        let dict = IndexMap::new();

        let common = make_common("Underline");
        let result = extract_annotation(&dict, common, &resolver);
        assert!(result.is_some());
        let annotation = result.unwrap();
        assert_eq!(annotation.common.subtype, "Underline");

        match annotation.specific {
            AnnotationSpecific::TextMarkup { .. } => {}
            _ => panic!("Expected TextMarkup for Underline"),
        }
    }
}
