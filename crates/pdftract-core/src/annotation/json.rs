//! JSON conversion for annotations and links (Phase 7.6.4).
//!
//! This module provides conversion functions from internal annotation types
//! to their JSON-serializable equivalents.

use crate::annotation::links::{DestArray, FitType, LinkAnnotation};
use crate::annotation::other::{Annotation, AnnotationSpecific};
use crate::schema::{
    AnnotationJson, AnnotationSpecificJson, DestArrayJson, DestTypeJson, LinkJson,
};
use std::cmp::Ordering;
use std::collections::HashMap;

/// Convert a `LinkAnnotation` to `LinkJson`.
///
/// This function implements the JSON serialization path for link annotations,
/// extracting the relevant fields for JSON output.
///
/// # Arguments
///
/// * `link` - The link annotation to convert
/// * `page_ref_to_index` - Optional map from page refs to page indices for dest_array resolution
///
/// # Returns
///
/// A `LinkJson` suitable for JSON serialization.
pub fn link_to_json(
    link: &LinkAnnotation,
    page_ref_to_index: &Option<HashMap<u64, usize>>,
) -> LinkJson {
    let rect = link.common.rect.unwrap_or([0.0, 0.0, 0.0, 0.0]);
    let page_index = link.common.page_index;

    let dest_array = link.dest_array.as_ref().map(|dest| DestArrayJson {
        page_index: dest.page_index,
        dest: fit_type_to_json(&dest.fit),
    });

    LinkJson {
        page_index,
        rect,
        uri: link.uri.clone(),
        dest: link.dest.clone(),
        dest_array,
    }
}

/// Convert a `FitType` to `DestTypeJson`.
fn fit_type_to_json(fit: &FitType) -> DestTypeJson {
    match fit {
        FitType::Xyz { left, top, zoom } => DestTypeJson::Xyz {
            left: left.map(|f| f as f64),
            top: top.map(|f| f as f64),
            zoom: zoom.map(|f| f as f64),
        },
        FitType::Fit => DestTypeJson::Fit,
        FitType::FitH(top) => DestTypeJson::FitH {
            top: top.map(|f| f as f64),
        },
        FitType::FitV(left) => DestTypeJson::FitV {
            left: left.map(|f| f as f64),
        },
        FitType::FitR(left, bottom, right, top) => DestTypeJson::FitR {
            left: *left as f64,
            bottom: *bottom as f64,
            right: *right as f64,
            top: *top as f64,
        },
        FitType::FitB => DestTypeJson::FitB,
        FitType::FitBH(top) => DestTypeJson::FitBH {
            top: top.map(|f| f as f64),
        },
        FitType::FitBV(left) => DestTypeJson::FitBV {
            left: left.map(|f| f as f64),
        },
    }
}

/// Convert an `Annotation` to `AnnotationJson`.
///
/// This function implements the JSON serialization path for non-link annotations,
/// extracting the relevant fields for JSON output.
///
/// # Arguments
///
/// * `annotation` - The annotation to convert
///
/// # Returns
///
/// An `AnnotationJson` suitable for JSON serialization.
pub fn annotation_to_json(annotation: &Annotation) -> AnnotationJson {
    let common = &annotation.common;

    // Convert subtype-specific fields
    let specific = match &annotation.specific {
        AnnotationSpecific::TextMarkup { quads } => Some(AnnotationSpecificJson::TextMarkup {
            quads: quads.clone(),
        }),
        AnnotationSpecific::Stamp { name } => {
            Some(AnnotationSpecificJson::Stamp { name: name.clone() })
        }
        AnnotationSpecific::FreeText { da } => {
            Some(AnnotationSpecificJson::FreeText { da: da.clone() })
        }
        AnnotationSpecific::Text {
            open,
            state,
            state_model,
        } => Some(AnnotationSpecificJson::Text {
            open: *open,
            state: state.clone(),
            state_model: state_model.clone(),
        }),
        AnnotationSpecific::Ink { strokes } => Some(AnnotationSpecificJson::Ink {
            strokes: strokes.clone(),
        }),
        AnnotationSpecific::Line { endpoints } => Some(AnnotationSpecificJson::Line {
            endpoints: *endpoints,
        }),
        AnnotationSpecific::Polygon { vertices } => Some(AnnotationSpecificJson::Polygon {
            vertices: vertices.clone(),
        }),
        AnnotationSpecific::FileAttachment { fs_ref } => {
            Some(AnnotationSpecificJson::FileAttachment {
                fs_ref: fs_ref.map(|r| (r.object << 16 | (r.generation as u32)) as u32),
            })
        }
        AnnotationSpecific::Other => None,
    };

    AnnotationJson {
        subtype: common.subtype.clone(),
        rect: common.rect,
        contents: common.contents.clone(),
        author: common.author.clone(),
        modified: common.modified.clone(),
        color: common.color.clone(),
        opacity: common.opacity,
        name_id: common.name_id.clone(),
        subject: common.subject.clone(),
        specific,
    }
}

/// Sort links by (page_index, rect.y0 desc, rect.x0).
///
/// Per the plan, links are sorted deterministically for stable output.
pub fn sort_links(links: &mut Vec<LinkJson>) {
    links.sort_by(|a, b| {
        a.page_index
            .cmp(&b.page_index)
            .then_with(|| {
                // Sort by y0 descending (top of page first)
                let a_y0 = a.rect[1];
                let b_y0 = b.rect[1];
                b_y0.partial_cmp(&a_y0).unwrap_or(Ordering::Equal)
            })
            .then_with(|| {
                // Then by x0 ascending (left to right)
                let a_x0 = a.rect[0];
                let b_x0 = b.rect[0];
                a_x0.partial_cmp(&b_x0).unwrap_or(Ordering::Equal)
            })
    });
}

/// Sort annotations by (rect.y0 desc, rect.x0).
///
/// Per the plan, page-level annotations are sorted deterministically for stable output.
pub fn sort_annotations(annotations: &mut Vec<AnnotationJson>) {
    annotations.sort_by(|a, b| {
        match (&a.rect, &b.rect) {
            (Some(a_rect), Some(b_rect)) => {
                // Sort by y0 descending (top of page first)
                let y_cmp = b_rect[1].partial_cmp(&a_rect[1]).unwrap_or(Ordering::Equal);
                if y_cmp != Ordering::Equal {
                    return y_cmp;
                }
                // Then by x0 ascending (left to right)
                a_rect[0].partial_cmp(&b_rect[0]).unwrap_or(Ordering::Equal)
            }
            // Annotations without rect come last
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::links::LinkAnnotation;
    use crate::annotation::other::{Annotation, AnnotationSpecific};
    use crate::annotation::AnnotationCommon;

    fn make_common_link() -> AnnotationCommon {
        AnnotationCommon {
            subtype: "Link".to_string(),
            rect: Some([100.0, 200.0, 300.0, 220.0]),
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
    fn test_link_to_json_uri() {
        let link = LinkAnnotation {
            common: make_common_link(),
            uri: Some("https://example.com".to_string()),
            dest: None,
            dest_array: None,
        };

        let json = link_to_json(&link, &None);

        assert_eq!(json.page_index, 0);
        assert_eq!(json.rect, [100.0, 200.0, 300.0, 220.0]);
        assert_eq!(json.uri, Some("https://example.com".to_string()));
        assert!(json.dest.is_none());
        assert!(json.dest_array.is_none());
    }

    #[test]
    fn test_link_to_json_named_dest() {
        let link = LinkAnnotation {
            common: make_common_link(),
            uri: None,
            dest: Some("Section1".to_string()),
            dest_array: None,
        };

        let json = link_to_json(&link, &None);

        assert_eq!(json.uri, None);
        assert_eq!(json.dest, Some("Section1".to_string()));
        assert!(json.dest_array.is_none());
    }

    #[test]
    fn test_link_to_json_explicit_dest() {
        let dest_array = DestArray {
            page_index: 5,
            fit: FitType::Xyz {
                left: Some(100.0),
                top: Some(200.0),
                zoom: Some(1.5),
            },
        };

        let link = LinkAnnotation {
            common: make_common_link(),
            uri: None,
            dest: None,
            dest_array: Some(dest_array),
        };

        let json = link_to_json(&link, &None);

        assert!(json.dest_array.is_some());
        let dest_json = json.dest_array.as_ref().unwrap();
        assert_eq!(dest_json.page_index, 5);
        match &dest_json.dest {
            DestTypeJson::Xyz { left, top, zoom } => {
                assert_eq!(*left.as_ref().unwrap(), 100.0);
                assert_eq!(*top.as_ref().unwrap(), 200.0);
                assert_eq!(*zoom.as_ref().unwrap(), 1.5);
            }
            _ => panic!("Expected Xyz dest type"),
        }
    }

    #[test]
    fn test_annotation_to_json_highlight() {
        let common = AnnotationCommon {
            subtype: "Highlight".to_string(),
            rect: Some([50.0, 100.0, 200.0, 120.0]),
            contents: Some("Important text".to_string()),
            author: Some("John Doe".to_string()),
            modified: Some("2023-01-15T14:30:45Z".to_string()),
            color: Some(vec![1.0, 1.0, 0.0]),
            opacity: Some(0.5),
            flags: 0,
            name_id: Some("annot1".to_string()),
            subject: Some("Review".to_string()),
            page_index: 0,
        };

        let annotation = Annotation {
            common,
            specific: AnnotationSpecific::TextMarkup {
                quads: vec![[50.0, 100.0, 200.0, 100.0, 200.0, 120.0, 50.0, 120.0]],
            },
        };

        let json = annotation_to_json(&annotation);

        assert_eq!(json.subtype, "Highlight");
        assert_eq!(json.rect, Some([50.0, 100.0, 200.0, 120.0]));
        assert_eq!(json.contents, Some("Important text".to_string()));
        assert_eq!(json.author, Some("John Doe".to_string()));
        assert_eq!(json.modified, Some("2023-01-15T14:30:45Z".to_string()));
        assert_eq!(json.color, Some(vec![1.0, 1.0, 0.0]));
        assert_eq!(json.opacity, Some(0.5));
        assert_eq!(json.name_id, Some("annot1".to_string()));
        assert_eq!(json.subject, Some("Review".to_string()));

        match json.specific {
            Some(AnnotationSpecificJson::TextMarkup { ref quads }) => {
                assert_eq!(quads.len(), 1);
                assert_eq!(
                    quads[0],
                    [50.0, 100.0, 200.0, 100.0, 200.0, 120.0, 50.0, 120.0]
                );
            }
            _ => panic!("Expected TextMarkup specific fields"),
        }
    }

    #[test]
    fn test_annotation_to_json_text_note() {
        let common = AnnotationCommon {
            subtype: "Text".to_string(),
            rect: Some([100.0, 200.0, 120.0, 220.0]),
            contents: Some("Check this".to_string()),
            author: Some("Jane".to_string()),
            modified: None,
            color: None,
            opacity: None,
            flags: 0,
            name_id: None,
            subject: None,
            page_index: 1,
        };

        let annotation = Annotation {
            common,
            specific: AnnotationSpecific::Text {
                open: Some(true),
                state: Some("Reviewed".to_string()),
                state_model: Some("Marked".to_string()),
            },
        };

        let json = annotation_to_json(&annotation);

        assert_eq!(json.subtype, "Text");

        match json.specific {
            Some(AnnotationSpecificJson::Text {
                open,
                ref state,
                ref state_model,
            }) => {
                assert_eq!(open, Some(true));
                assert_eq!(state.as_ref().unwrap(), "Reviewed");
                assert_eq!(state_model.as_ref().unwrap(), "Marked");
            }
            _ => panic!("Expected Text specific fields"),
        }
    }

    #[test]
    fn test_sort_links() {
        let mut links = vec![
            LinkJson {
                page_index: 1,
                rect: [100.0, 300.0, 200.0, 320.0],
                uri: None,
                dest: None,
                dest_array: None,
            },
            LinkJson {
                page_index: 0,
                rect: [100.0, 200.0, 200.0, 220.0],
                uri: None,
                dest: None,
                dest_array: None,
            },
            LinkJson {
                page_index: 0,
                rect: [100.0, 100.0, 200.0, 120.0],
                uri: None,
                dest: None,
                dest_array: None,
            },
            LinkJson {
                page_index: 0,
                rect: [50.0, 100.0, 100.0, 120.0],
                uri: None,
                dest: None,
                dest_array: None,
            },
        ];

        sort_links(&mut links);

        // Expected order:
        // 1. page_index 0, y0=100, x0=50 (leftmost of the two at y=100)
        // 2. page_index 0, y0=100, x0=100
        // 3. page_index 0, y0=200 (higher y0 comes first due to desc order)
        // 4. page_index 1
        assert_eq!(links[0].rect[0], 50.0);
        assert_eq!(links[1].rect[0], 100.0);
        assert_eq!(links[2].rect[1], 200.0);
        assert_eq!(links[3].page_index, 1);
    }

    #[test]
    fn test_sort_annotations() {
        let mut annotations = vec![
            AnnotationJson {
                subtype: "Highlight".to_string(),
                rect: Some([100.0, 200.0, 200.0, 220.0]),
                contents: None,
                author: None,
                modified: None,
                color: None,
                opacity: None,
                name_id: None,
                subject: None,
                specific: None,
            },
            AnnotationJson {
                subtype: "Text".to_string(),
                rect: Some([50.0, 100.0, 100.0, 120.0]),
                contents: None,
                author: None,
                modified: None,
                color: None,
                opacity: None,
                name_id: None,
                subject: None,
                specific: None,
            },
            AnnotationJson {
                subtype: "Stamp".to_string(),
                rect: Some([100.0, 100.0, 150.0, 120.0]),
                contents: None,
                author: None,
                modified: None,
                color: None,
                opacity: None,
                name_id: None,
                subject: None,
                specific: None,
            },
            AnnotationJson {
                subtype: "Note".to_string(),
                rect: None,
                contents: None,
                author: None,
                modified: None,
                color: None,
                opacity: None,
                name_id: None,
                subject: None,
                specific: None,
            },
        ];

        sort_annotations(&mut annotations);

        // Expected order:
        // 1. y0=200 comes first (desc order)
        // 2. y0=100, x0=50 (left of the two at y=100)
        // 3. y0=100, x0=100
        // 4. rect=None comes last
        assert_eq!(annotations[0].rect.unwrap()[1], 200.0);
        assert_eq!(annotations[1].rect.unwrap()[0], 50.0);
        assert_eq!(annotations[2].rect.unwrap()[0], 100.0);
        assert!(annotations[3].rect.is_none());
    }

    #[test]
    fn test_fit_type_to_json_all_variants() {
        // Test all fit type variants round-trip correctly
        let cases = vec![
            (
                FitType::Xyz {
                    left: Some(10.0),
                    top: Some(20.0),
                    zoom: Some(1.5),
                },
                "xyz",
            ),
            (FitType::Fit, "fit"),
            (FitType::FitH(Some(100.0)), "fith"),
            (FitType::FitV(Some(50.0)), "fitv"),
            (FitType::FitR(10.0, 20.0, 100.0, 200.0), "fitr"),
            (FitType::FitB, "fitb"),
            (FitType::FitBH(Some(75.0)), "fitbh"),
            (FitType::FitBV(Some(25.0)), "fitbv"),
        ];

        for (fit, expected_tag) in cases {
            let json = fit_type_to_json(&fit);
            let tag = match &json {
                DestTypeJson::Xyz { .. } => "xyz",
                DestTypeJson::Fit => "fit",
                DestTypeJson::FitH { .. } => "fith",
                DestTypeJson::FitV { .. } => "fitv",
                DestTypeJson::FitR { .. } => "fitr",
                DestTypeJson::FitB => "fitb",
                DestTypeJson::FitBH { .. } => "fitbh",
                DestTypeJson::FitBV { .. } => "fitbv",
            };
            assert_eq!(tag, expected_tag, "Fit type mismatch for {:?}", fit);
        }
    }

    #[test]
    fn test_annotation_roundtrip_serialization() {
        // Test that annotation JSON serializes correctly
        let common = AnnotationCommon {
            subtype: "Highlight".to_string(),
            rect: Some([50.0, 100.0, 200.0, 120.0]),
            contents: Some("Test".to_string()),
            author: None,
            modified: None,
            color: Some(vec![1.0, 0.0, 0.0]),
            opacity: None,
            flags: 0,
            name_id: None,
            subject: None,
            page_index: 0,
        };

        let annotation = Annotation {
            common,
            specific: AnnotationSpecific::TextMarkup {
                quads: vec![[0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]],
            },
        };

        let json = annotation_to_json(&annotation);
        let json_str = serde_json::to_string(&json).unwrap();
        let deserialized: AnnotationJson = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.subtype, json.subtype);
        assert_eq!(deserialized.rect, json.rect);
        assert_eq!(deserialized.contents, json.contents);
        assert_eq!(deserialized.color, json.color);
    }

    #[test]
    fn test_link_roundtrip_serialization() {
        // Test that link JSON serializes correctly
        let link = LinkAnnotation {
            common: make_common_link(),
            uri: Some("https://example.com".to_string()),
            dest: None,
            dest_array: None,
        };

        let json = link_to_json(&link, &None);
        let json_str = serde_json::to_string(&json).unwrap();
        let deserialized: LinkJson = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.page_index, json.page_index);
        assert_eq!(deserialized.rect, json.rect);
        assert_eq!(deserialized.uri, json.uri);
    }
}
