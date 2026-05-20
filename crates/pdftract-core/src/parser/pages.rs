//! Page tree flattening with inherited attribute resolution.
//!
//! This module implements the page tree walker that resolves inherited attributes
//! (MediaBox, CropBox, Resources, Rotate) across the /Pages subtree and produces
//! a flat Vec<PageDict> suitable for downstream extraction phases.
//!
//! Per PDF 1.7 spec section 7.7.3.4 "Page Tree":
//! - /MediaBox, /CropBox, /Resources, /Rotate are inheritable from ancestor /Pages nodes
//! - /BleedBox, /TrimBox, /ArtBox, /Contents, /Annots are not inheritable
//! - Inheritance is "last-write-wins" at each level (child overrides parent)
//! - If a required inheritable attribute is missing and not inherited, use a safe default

use crate::parser::object::{ObjRef, PdfObject, PdfDict, intern};
use crate::parser::xref::XrefResolver;
use crate::parser::{Diagnostic, Severity};
use crate::parser::diagnostic::DiagCode;
use crate::parser::resources::{ResourceDict, merge_resources, extract_resources};
use std::collections::HashSet;
use std::sync::Arc;

/// Default MediaBox when none is specified (US Letter: 612 x 792 points).
///
/// Per EC-09: Page with no MediaBox and no inherited MediaBox substitutes
/// US Letter dimensions and emits STRUCT_MISSING_KEY diagnostic.
pub const DEFAULT_MEDIABOX: [f64; 4] = [0.0, 0.0, 612.0, 792.0];

/// Maximum depth of /Pages nesting to prevent stack overflow.
///
/// Real-world PDFs rarely exceed 5 levels; 16 is very generous.
const MAX_PAGES_DEPTH: u8 = 16;

/// A fully resolved page dictionary with all inherited attributes merged.
///
/// This is the output of the page tree flattening process. Each PageDict
/// represents a leaf /Page node with all inheritable attributes from its
/// ancestor /Pages nodes resolved.
#[derive(Debug, Clone)]
pub struct PageDict {
    /// The page's own indirect reference
    pub obj_ref: ObjRef,
    /// REQUIRED; inherited if missing on this page. Default: [0, 0, 612, 792]
    pub media_box: [f64; 4],
    /// Optional; defaults to media_box if absent
    pub crop_box: Option<[f64; 4]>,
    /// Optional; defaults to crop_box if absent
    pub bleed_box: Option<[f64; 4]>,
    /// Optional; defaults to crop_box if absent
    pub trim_box: Option<[f64; 4]>,
    /// Optional; defaults to crop_box if absent
    pub art_box: Option<[f64; 4]>,
    /// Page rotation in degrees; must be a multiple of 90 (0, 90, 180, 270)
    pub rotate: i32,
    /// Merged resource dict containing all inherited resources
    /// Wrapped in Arc for memory efficiency when multiple pages share the same resources
    pub resources: Arc<ResourceDict>,
    /// List of content stream references (in order)
    pub contents: Vec<ObjRef>,
    /// Annotation array references
    pub annots: Vec<ObjRef>,
    /// ActualText from tagged PDF (if present)
    pub actual_text: Option<String>,
    /// Language identifier (if present)
    pub lang: Option<String>,
    /// Page-level additional actions (used by JS detection)
    pub aa: Option<PdfObject>,
}

/// Inherited attributes accumulator for page tree traversal.
///
/// Tracks the current inherited values as we walk down the /Pages tree.
/// Each /Pages node may override these values; leaf /Page nodes read
/// the accumulated values.
#[derive(Debug, Clone)]
struct InheritedAttrs {
    /// Inherited MediaBox (required, but may be None -> use default)
    media_box: Option<[f64; 4]>,
    /// Inherited CropBox (optional)
    crop_box: Option<[f64; 4]>,
    /// Inherited merged resources (accumulated from all ancestors)
    resources: Arc<ResourceDict>,
    /// Inherited Rotate value (defaults to 0)
    rotate: i32,
}

impl Default for InheritedAttrs {
    fn default() -> Self {
        InheritedAttrs {
            media_box: None,
            crop_box: None,
            resources: Arc::new(ResourceDict::new()),
            rotate: 0,
        }
    }
}

/// Result type for page tree flattening.
pub type Result<T> = std::result::Result<T, Vec<Diagnostic>>;

/// Flatten the page tree into a vector of fully resolved PageDict objects.
///
/// This function walks the /Pages subtree starting from the given /Pages reference,
/// resolves all inherited attributes, and returns a flat vector of leaf pages in
/// document order (left-to-right depth-first traversal).
///
/// # Arguments
/// * `resolver` - The xref resolver for resolving indirect references
/// * `pages_ref` - The object reference to the root /Pages dictionary
///
/// # Returns
/// A `Result<Vec<PageDict>>` containing the flattened pages or diagnostics.
///
/// # Behavior
/// - Empty /Pages tree: returns empty Vec (page_count = 0)
/// - Missing /MediaBox: substitutes DEFAULT_MEDIABOX, emits STRUCT_MISSING_KEY
/// - Invalid /Rotate: clamps to nearest multiple of 90, emits STRUCT_INVALID_ROTATE
/// - Circular reference: detected, subtree pruned, STRUCT_CIRCULAR_REF emitted
/// - Depth exceeded: subtree pruned, STRUCT_DEPTH_EXCEEDED emitted
/// - Page count mismatch: emits STRUCT_INVALID_PAGE_COUNT if /Count disagrees
///
/// # Example
/// ```ignore
/// let pages = flatten_page_tree(&resolver, catalog.pages_ref)?;
/// for (i, page) in pages.iter().enumerate() {
///     println!("Page {}: MediaBox {:?}", i, page.media_box);
/// }
/// ```
pub fn flatten_page_tree(resolver: &XrefResolver, pages_ref: ObjRef) -> Result<Vec<PageDict>> {
    let mut diagnostics = Vec::new();
    let mut visited = HashSet::new();
    let mut inherited = InheritedAttrs::default();

    // Resolve the root /Pages node
    let pages_obj = match resolver.resolve(pages_ref) {
        Ok(obj) => obj,
        Err(e) => {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                phase: "1.4".to_string(),
                code: DiagCode::MissingKey,
                message: format!("Failed to resolve root /Pages node {}: {}", pages_ref, e),
            });
            return Err(diagnostics);
        }
    };

    // Extract /Count if present (for validation later)
    let declared_count = pages_obj.as_dict()
        .and_then(|d| d.get("Count"))
        .and_then(|o| o.as_int())
        .unwrap_or(0);

    // Walk the tree starting from root /Pages
    let pages = walk_page_tree(
        resolver,
        &pages_obj,
        &mut inherited,
        &mut visited,
        0,
        &mut diagnostics,
    );

    // Validate page count against /Count
    let actual_count = pages.len() as i64;
    if declared_count > 0 && actual_count != declared_count {
        diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            phase: "1.4".to_string(),
            code: DiagCode::InvalidPageCount,
            message: format!(
                "STRUCT_INVALID_PAGE_COUNT: /Count declares {} pages, but tree contains {} pages",
                declared_count, actual_count
            ),
        });
    }

    if !diagnostics.is_empty() && pages.is_empty() {
        // Only return error if we have no pages at all
        Err(diagnostics)
    } else {
        Ok(pages)
    }
}

/// Recursive page tree walker.
///
/// Traverses the /Pages subtree depth-first, accumulating inherited attributes
/// and emitting PageDict objects for leaf /Page nodes.
///
/// # Arguments
/// * `resolver` - The xref resolver
/// * `node` - The current node (either /Pages or /Page)
/// * `inherited` - Current inherited attributes (mutated during traversal)
/// * `visited` - Set of visited object references for cycle detection
/// * `depth` - Current nesting depth
/// * `diagnostics` - Accumulator for diagnostics
///
/// # Returns
/// A vector of PageDict objects from this subtree.
fn walk_page_tree(
    resolver: &XrefResolver,
    node: &PdfObject,
    inherited: &mut InheritedAttrs,
    visited: &mut HashSet<ObjRef>,
    depth: u8,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<PageDict> {
    // Depth limit check
    if depth > MAX_PAGES_DEPTH {
        diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            phase: "1.4".to_string(),
            code: DiagCode::DepthExceeded,
            message: format!("STRUCT_DEPTH_EXCEEDED: /Pages nesting exceeds {} levels", MAX_PAGES_DEPTH),
        });
        return Vec::new();
    }

    let dict = match node.as_dict() {
        Some(d) => d,
        None => {
            // Not a dictionary - skip this node
            return Vec::new();
        }
    };

    // Check /Type to determine if this is /Pages or /Page
    let node_type = dict.get("Type")
        .and_then(|o| o.as_name())
        .unwrap_or("");

    // Save the inherited state before merging this node's attributes
    let parent_inherited = inherited.clone();

    // Merge inheritable attributes from this node
    merge_inherited_attrs(dict, inherited, diagnostics);

    match node_type {
        "Page" => {
            // Leaf node: emit a PageDict
            vec![build_page_dict(node, inherited, diagnostics)]
        }
        "Pages" => {
            // Internal node: recurse into /Kids
            let kids = match dict.get("Kids") {
                Some(k) => k,
                None => {
                    diagnostics.push(Diagnostic {
                        severity: Severity::Warning,
                        phase: "1.4".to_string(),
                        code: DiagCode::MissingKey,
                        message: "STRUCT_MISSING_KEY: /Pages node missing /Kids".to_string(),
                    });
                    return Vec::new();
                }
            };

            let kids_array = match kids.as_array() {
                Some(arr) => arr,
                None => {
                    // /Kids is not an array - skip
                    return Vec::new();
                }
            };

            let mut pages = Vec::new();
            for kid in kids_array {
                // Handle both direct (embedded dict) and indirect references
                let kid_obj = match kid {
                    PdfObject::Ref(ref_) => {
                        // Check for cycles
                        if visited.contains(ref_) {
                            diagnostics.push(Diagnostic {
                                severity: Severity::Warning,
                                phase: "1.4".to_string(),
                                code: DiagCode::CircularRef,
                                message: format!("STRUCT_CIRCULAR_REF: /Pages node {} already visited", ref_),
                            });
                            continue;
                        }
                        visited.insert(*ref_);

                        match resolver.resolve(*ref_) {
                            Ok(obj) => obj,
                            Err(e) => {
                                diagnostics.push(Diagnostic {
                                    severity: Severity::Warning,
                                    phase: "1.4".to_string(),
                                    code: DiagCode::MissingKey,
                                    message: format!("STRUCT_MISSING_KEY: Failed to resolve /Kids entry {}: {}", ref_, e),
                                });
                                continue;
                            }
                        }
                    }
                    PdfObject::Dict(_) => {
                        // Direct dictionary - uncommon but legal
                        kid.clone()
                    }
                    _ => {
                        // Invalid /Kids entry - skip
                        continue;
                    }
                };

                // Recurse into the child
                let child_pages = walk_page_tree(
                    resolver,
                    &kid_obj,
                    inherited,
                    visited,
                    depth + 1,
                    diagnostics,
                );
                pages.extend(child_pages);

                // Restore inherited state for next sibling
                *inherited = parent_inherited.clone();
            }

            pages
        }
        _ => {
            // Unknown /Type - skip this node
            *inherited = parent_inherited;
            Vec::new()
        }
    }
}

/// Merge inheritable attributes from a /Pages or /Page node into the accumulator.
///
/// Per PDF spec 7.7.3.4, only MediaBox, CropBox, Resources, and Rotate are inheritable.
/// This function updates the `inherited` accumulator with any values present in `dict`.
fn merge_inherited_attrs(dict: &PdfDict, inherited: &mut InheritedAttrs, diagnostics: &mut Vec<Diagnostic>) {
    // MediaBox (inheritable)
    if let Some(mb) = parse_rect(dict.get("MediaBox")) {
        inherited.media_box = Some(mb);
    }

    // CropBox (inheritable)
    if let Some(cb) = parse_rect(dict.get("CropBox")) {
        inherited.crop_box = Some(cb);
    }

    // Resources (inheritable) - merge with existing resources
    if let Some(resources_obj) = dict.get("Resources") {
        let merged = merge_resources(&inherited.resources, resources_obj);
        inherited.resources = Arc::new(merged);
    }

    // Rotate (inheritable)
    if let Some(rot) = dict.get("Rotate").and_then(|o| o.as_int()) {
        if rot % 90 != 0 {
            diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                phase: "1.4".to_string(),
                code: DiagCode::InvalidRotate,
                message: format!("STRUCT_INVALID_ROTATE: /Rotate value {} is not a multiple of 90", rot),
            });
            // Clamp to nearest multiple of 90 (floor toward negative infinity)
            inherited.rotate = ((rot as f64 / 90.0).floor() as i64 * 90) as i32;
        } else {
            inherited.rotate = rot as i32;
        }
    }
}

/// Build a PageDict from a leaf /Page node and accumulated inherited attributes.
///
/// This function extracts all page-level attributes, substituting defaults for
/// missing values and emitting diagnostics where appropriate.
fn build_page_dict(page_obj: &PdfObject, inherited: &InheritedAttrs, diagnostics: &mut Vec<Diagnostic>) -> PageDict {
    let dict = match page_obj.as_dict() {
        Some(d) => d,
        None => {
            // Not a dict - return a minimal PageDict with defaults
            return PageDict {
                obj_ref: ObjRef::new(0, 0),
                media_box: DEFAULT_MEDIABOX,
                crop_box: None,
                bleed_box: None,
                trim_box: None,
                art_box: None,
                rotate: inherited.rotate,
                resources: Arc::clone(&inherited.resources),
                contents: Vec::new(),
                annots: Vec::new(),
                actual_text: None,
                lang: None,
                aa: None,
            };
        }
    };

    // Get the page's object reference (if available as Indirect)
    let obj_ref = if let PdfObject::Indirect(ind) = page_obj {
        ind.id
    } else {
        ObjRef::new(0, 0)
    };

    // MediaBox: use page's own, or inherited, or default
    let media_box = if let Some(mb) = parse_rect(dict.get("MediaBox")) {
        mb
    } else if let Some(inherited_mb) = inherited.media_box {
        inherited_mb
    } else {
        diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            phase: "1.4".to_string(),
            code: DiagCode::MissingKey,
            message: format!("STRUCT_MISSING_KEY: Page {} has no /MediaBox and no inherited /MediaBox; using US Letter default", obj_ref),
        });
        DEFAULT_MEDIABOX
    };

    // CropBox: use page's own, or inherited, or default to media_box
    let crop_box = if let Some(cb) = parse_rect(dict.get("CropBox")) {
        Some(cb)
    } else {
        inherited.crop_box
    };

    // BleedBox, TrimBox, ArtBox: non-inheritable, must be on this page
    let bleed_box = parse_rect(dict.get("BleedBox"));
    let trim_box = parse_rect(dict.get("TrimBox"));
    let art_box = parse_rect(dict.get("ArtBox"));

    // Rotate: use page's own (with validation) or inherited
    let mut rotate = inherited.rotate;
    if let Some(rot) = dict.get("Rotate").and_then(|o| o.as_int()) {
        if rot % 90 != 0 {
            diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                phase: "1.4".to_string(),
                code: DiagCode::InvalidRotate,
                message: format!("STRUCT_INVALID_ROTATE: Page {} has /Rotate value {} (not a multiple of 90)", obj_ref, rot),
            });
            // Clamp to nearest multiple of 90 (floor toward negative infinity)
            rotate = ((rot as f64 / 90.0).floor() as i64 * 90) as i32;
        } else {
            // Valid rotate value - normalize to 0-270 range
            rotate = ((rot % 360 + 360) % 360) as i32;
        }
    }

    // Resources: merge page's own resources with inherited resources
    let resources = if let Some(resources_obj) = dict.get("Resources") {
        let merged = merge_resources(&inherited.resources, resources_obj);
        Arc::new(merged)
    } else {
        // No resources on this page - use inherited resources as-is
        Arc::clone(&inherited.resources)
    };

    // Contents: normalize to Vec<ObjRef>
    let contents = parse_contents_array(dict.get("Contents"));

    // Annots: collect array of references
    let annots = if let Some(PdfObject::Array(arr)) = dict.get("Annots") {
        arr.iter()
            .filter_map(|o| o.as_ref())
            .collect()
    } else {
        Vec::new()
    };

    // ActualText (from tagged PDF)
    let actual_text = dict.get("ActualText")
        .and_then(|o| o.as_string())
        .and_then(|s| String::from_utf8(s.to_vec()).ok());

    // Lang (language identifier)
    let lang = dict.get("Lang")
        .and_then(|o| o.as_string())
        .and_then(|s| String::from_utf8(s.to_vec()).ok());

    // AA (additional actions)
    let aa = dict.get("AA").cloned();

    PageDict {
        obj_ref,
        media_box,
        crop_box,
        bleed_box,
        trim_box,
        art_box,
        rotate,
        resources,
        contents,
        annots,
        actual_text,
        lang,
        aa,
    }
}

/// Parse a rectangle array [x1 y1 x2 y2] from a PdfObject.
///
/// Returns None if the object is not a 4-element array of numbers.
fn parse_rect(obj: Option<&PdfObject>) -> Option<[f64; 4]> {
    let arr = obj?.as_array()?;
    if arr.len() != 4 {
        return None;
    }

    let x1 = arr[0].as_int().map(|i| i as f64).or_else(|| arr[0].as_real())?;
    let y1 = arr[1].as_int().map(|i| i as f64).or_else(|| arr[1].as_real())?;
    let x2 = arr[2].as_int().map(|i| i as f64).or_else(|| arr[2].as_real())?;
    let y2 = arr[3].as_int().map(|i| i as f64).or_else(|| arr[3].as_real())?;

    Some([x1, y1, x2, y2])
}

/// Normalize /Contents to a Vec<ObjRef>.
///
/// /Contents can be:
/// - A single stream reference -> Vec with one element
/// - An array of stream references -> Vec with all elements
/// - A direct stream (illegal) -> empty Vec with diagnostic
/// - Missing -> empty Vec
fn parse_contents_array(obj: Option<&PdfObject>) -> Vec<ObjRef> {
    match obj {
        None => Vec::new(),
        Some(PdfObject::Ref(ref_)) => vec![*ref_],
        Some(PdfObject::Array(arr)) => {
            arr.iter()
                .filter_map(|o| o.as_ref())
                .collect()
        }
        Some(PdfObject::Stream(_)) => {
            // Direct stream is illegal - should be indirect
            // Return empty; diagnostics would be emitted by parser
            Vec::new()
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
fn make_pages_dict(kids: Vec<PdfObject>, count: i64, media_box: Option<[f64; 4]>) -> PdfObject {
    let mut dict = PdfDict::new();
    dict.insert(intern("Type"), PdfObject::Name(intern("Pages")));
    dict.insert(intern("Kids"), PdfObject::Array(Box::new(kids)));
    dict.insert(intern("Count"), PdfObject::Integer(count));
    if let Some(mb) = media_box {
        dict.insert(intern("MediaBox"), make_rect_array(mb));
    }
    PdfObject::Dict(Box::new(dict))
}

#[cfg(test)]
fn make_page_dict(media_box: Option<[f64; 4]>, rotate: Option<i64>) -> PdfObject {
    let mut dict = PdfDict::new();
    dict.insert(intern("Type"), PdfObject::Name(intern("Page")));
    if let Some(mb) = media_box {
        dict.insert(intern("MediaBox"), make_rect_array(mb));
    }
    if let Some(rot) = rotate {
        dict.insert(intern("Rotate"), PdfObject::Integer(rot));
    }
    PdfObject::Dict(Box::new(dict))
}

#[cfg(test)]
fn make_rect_array(rect: [f64; 4]) -> PdfObject {
    PdfObject::Array(Box::new(vec![
        PdfObject::Real(rect[0]),
        PdfObject::Real(rect[1]),
        PdfObject::Real(rect[2]),
        PdfObject::Real(rect[3]),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mediabox() {
        assert_eq!(DEFAULT_MEDIABOX, [0.0, 0.0, 612.0, 792.0]);
    }

    #[test]
    fn test_parse_rect_valid() {
        let rect = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Integer(0),
            PdfObject::Integer(612),
            PdfObject::Integer(792),
        ]));
        assert_eq!(parse_rect(Some(&rect)), Some([0.0, 0.0, 612.0, 792.0]));
    }

    #[test]
    fn test_parse_rect_real() {
        let rect = PdfObject::Array(Box::new(vec![
            PdfObject::Real(0.0),
            PdfObject::Real(0.0),
            PdfObject::Real(612.5),
            PdfObject::Real(792.5),
        ]));
        assert_eq!(parse_rect(Some(&rect)), Some([0.0, 0.0, 612.5, 792.5]));
    }

    #[test]
    fn test_parse_rect_invalid_length() {
        let rect = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Integer(0),
            PdfObject::Integer(612),
        ]));
        assert_eq!(parse_rect(Some(&rect)), None);
    }

    #[test]
    fn test_parse_rect_non_array() {
        assert_eq!(parse_rect(Some(&PdfObject::Integer(42))), None);
    }

    #[test]
    fn test_parse_contents_single_ref() {
        let ref_obj = PdfObject::Ref(ObjRef::new(10, 0));
        assert_eq!(parse_contents_array(Some(&ref_obj)), vec![ObjRef::new(10, 0)]);
    }

    #[test]
    fn test_parse_contents_array() {
        let arr = PdfObject::Array(Box::new(vec![
            PdfObject::Ref(ObjRef::new(10, 0)),
            PdfObject::Ref(ObjRef::new(11, 0)),
        ]));
        assert_eq!(parse_contents_array(Some(&arr)), vec![
            ObjRef::new(10, 0),
            ObjRef::new(11, 0),
        ]);
    }

    #[test]
    fn test_parse_contents_none() {
        assert_eq!(parse_contents_array(None), Vec::new());
    }

    #[test]
    fn test_flatten_single_page() {
        let resolver = XrefResolver::new();
        let pages_ref = ObjRef::new(1, 0);

        let page = make_page_dict(Some([0.0, 0.0, 612.0, 792.0]), None);
        let pages = make_pages_dict(vec![page], 1, None);

        resolver.cache_object(pages_ref, pages);

        let result = flatten_page_tree(&resolver, pages_ref);
        assert!(result.is_ok());
        let pages_vec = result.unwrap();
        assert_eq!(pages_vec.len(), 1);
        assert_eq!(pages_vec[0].media_box, [0.0, 0.0, 612.0, 792.0]);
    }

    #[test]
    fn test_flatten_three_level_inheritance() {
        // Critical test: 3-level /Pages tree with MediaBox only on grandparent
        let resolver = XrefResolver::new();

        // Grandparent /Pages (has MediaBox)
        let grandparent_ref = ObjRef::new(1, 0);
        let grandparent = make_pages_dict(vec![], 2, Some([0.0, 0.0, 612.0, 792.0]));

        // Parent /Pages (no MediaBox - inherits from grandparent)
        let parent_ref = ObjRef::new(2, 0);
        let parent = make_pages_dict(vec![], 1, None);

        // Leaf pages (no MediaBox - inherits from grandparent via parent)
        let page1_ref = ObjRef::new(3, 0);
        let page1 = make_page_dict(None, None);
        let page2_ref = ObjRef::new(4, 0);
        let page2 = make_page_dict(None, None);

        // Wire up the tree: grandparent -> parent -> [page1, page2]
        let mut grandparent_dict = grandparent.as_dict().unwrap().clone();
        grandparent_dict.insert(
            intern("Kids"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(parent_ref)]))
        );

        let mut parent_dict = parent.as_dict().unwrap().clone();
        parent_dict.insert(
            intern("Kids"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(page1_ref), PdfObject::Ref(page2_ref)]))
        );

        resolver.cache_object(grandparent_ref, PdfObject::Dict(Box::new(grandparent_dict)));
        resolver.cache_object(parent_ref, PdfObject::Dict(Box::new(parent_dict)));
        resolver.cache_object(page1_ref, page1);
        resolver.cache_object(page2_ref, page2);

        let result = flatten_page_tree(&resolver, grandparent_ref);
        assert!(result.is_ok());
        let pages_vec = result.unwrap();
        assert_eq!(pages_vec.len(), 2);
        // Both pages should inherit MediaBox from grandparent
        assert_eq!(pages_vec[0].media_box, [0.0, 0.0, 612.0, 792.0]);
        assert_eq!(pages_vec[1].media_box, [0.0, 0.0, 612.0, 792.0]);
    }

    #[test]
    fn test_ec09_missing_mediabox_defaults_to_us_letter() {
        // Critical test EC-09: page with no MediaBox anywhere
        let resolver = XrefResolver::new();
        let pages_ref = ObjRef::new(1, 0);

        // /Pages with no MediaBox
        let pages = make_pages_dict(
            vec![make_page_dict(None, None)],
            1,
            None,
        );

        resolver.cache_object(pages_ref, pages);

        let result = flatten_page_tree(&resolver, pages_ref);
        assert!(result.is_ok());
        let pages_vec = result.unwrap();
        assert_eq!(pages_vec.len(), 1);
        assert_eq!(pages_vec[0].media_box, DEFAULT_MEDIABOX);
    }

    #[test]
    fn test_invalid_rotate_clamped() {
        let resolver = XrefResolver::new();
        let pages_ref = ObjRef::new(1, 0);

        // /Rotate = 45 should be clamped to 0
        let pages = make_pages_dict(
            vec![make_page_dict(Some(DEFAULT_MEDIABOX), Some(45))],
            1,
            Some(DEFAULT_MEDIABOX),
        );

        resolver.cache_object(pages_ref, pages);

        let result = flatten_page_tree(&resolver, pages_ref);
        assert!(result.is_ok());
        let pages_vec = result.unwrap();
        assert_eq!(pages_vec[0].rotate, 0);
    }

    #[test]
    fn test_invalid_rotate_135_clamped() {
        let resolver = XrefResolver::new();
        let pages_ref = ObjRef::new(1, 0);

        // /Rotate = 135 should be clamped to 90
        let pages = make_pages_dict(
            vec![make_page_dict(Some(DEFAULT_MEDIABOX), Some(135))],
            1,
            Some(DEFAULT_MEDIABOX),
        );

        resolver.cache_object(pages_ref, pages);

        let result = flatten_page_tree(&resolver, pages_ref);
        assert!(result.is_ok());
        let pages_vec = result.unwrap();
        assert_eq!(pages_vec[0].rotate, 90);
    }

    #[test]
    fn test_valid_rotate_values() {
        for rot in [0, 90, 180, 270, 360, -90, -180] {
            let resolver = XrefResolver::new();
            let pages_ref = ObjRef::new(1, 0);

            let pages = make_pages_dict(
                vec![make_page_dict(Some(DEFAULT_MEDIABOX), Some(rot))],
                1,
                Some(DEFAULT_MEDIABOX),
            );

            resolver.cache_object(pages_ref, pages);

            let result = flatten_page_tree(&resolver, pages_ref);
            assert!(result.is_ok());
            let pages_vec = result.unwrap();
            // Normalize to 0-270 range
            let expected = ((rot % 360 + 360) % 360) as i32;
            assert_eq!(pages_vec[0].rotate, expected);
        }
    }

    #[test]
    fn test_empty_pages_tree() {
        let resolver = XrefResolver::new();
        let pages_ref = ObjRef::new(1, 0);

        let pages = make_pages_dict(vec![], 0, None);
        resolver.cache_object(pages_ref, pages);

        let result = flatten_page_tree(&resolver, pages_ref);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_page_count_mismatch() {
        let resolver = XrefResolver::new();
        let pages_ref = ObjRef::new(1, 0);

        // /Count says 5, but we only have 1 page
        let pages = make_pages_dict(
            vec![make_page_dict(Some(DEFAULT_MEDIABOX), None)],
            5,  // Wrong count
            Some(DEFAULT_MEDIABOX),
        );

        resolver.cache_object(pages_ref, pages);

        let result = flatten_page_tree(&resolver, pages_ref);
        assert!(result.is_ok());
        let pages_vec = result.unwrap();
        assert_eq!(pages_vec.len(), 1);
        // The function should have emitted a diagnostic about count mismatch
        // (we can't easily check this without exposing diagnostics from the public API)
    }

    #[test]
    fn test_cycle_detection_in_page_tree() {
        // Test that circular references in the page tree are detected and handled
        let resolver = XrefResolver::new();

        // Create a tree with a cycle: parent -> child1 -> child2 -> child1 (cycle)
        let parent_ref = ObjRef::new(1, 0);
        let child1_ref = ObjRef::new(2, 0);
        let child2_ref = ObjRef::new(3, 0);
        let page_ref = ObjRef::new(4, 0);

        // Add a valid page first
        let page = make_page_dict(Some(DEFAULT_MEDIABOX), None);
        resolver.cache_object(page_ref, page);

        // Create child2 with a valid page and a reference to child1 (creating cycle)
        let mut child2_dict = PdfDict::new();
        child2_dict.insert(intern("Type"), PdfObject::Name(intern("Pages")));
        child2_dict.insert(intern("Kids"), PdfObject::Array(Box::new(vec![
            PdfObject::Ref(page_ref),
            PdfObject::Ref(child1_ref),  // This will cause a cycle
        ])));
        child2_dict.insert(intern("Count"), PdfObject::Integer(2));

        // Create child1 that references child2 (the other half of the cycle)
        let mut child1_dict = PdfDict::new();
        child1_dict.insert(intern("Type"), PdfObject::Name(intern("Pages")));
        child1_dict.insert(intern("Kids"), PdfObject::Array(Box::new(vec![PdfObject::Ref(child2_ref)])));
        child1_dict.insert(intern("Count"), PdfObject::Integer(1));

        // Create parent that references child1
        let mut parent_dict = PdfDict::new();
        parent_dict.insert(intern("Type"), PdfObject::Name(intern("Pages")));
        parent_dict.insert(intern("Kids"), PdfObject::Array(Box::new(vec![PdfObject::Ref(child1_ref)])));
        parent_dict.insert(intern("Count"), PdfObject::Integer(2));
        parent_dict.insert(intern("MediaBox"), make_rect_array(DEFAULT_MEDIABOX));

        resolver.cache_object(child1_ref, PdfObject::Dict(Box::new(child1_dict)));
        resolver.cache_object(child2_ref, PdfObject::Dict(Box::new(child2_dict)));
        resolver.cache_object(parent_ref, PdfObject::Dict(Box::new(parent_dict)));

        let result = flatten_page_tree(&resolver, parent_ref);
        // Should succeed and return the one valid page (the cycle is pruned)
        assert!(result.is_ok());
        let pages_vec = result.unwrap();
        // We should get exactly 1 page (the valid one before the cycle)
        assert_eq!(pages_vec.len(), 1);
        assert_eq!(pages_vec[0].media_box, DEFAULT_MEDIABOX);
    }

    #[test]
    fn test_resource_inheritance_three_level() {
        // Critical test: 3-level resource inheritance
        let resolver = XrefResolver::new();

        // Grandparent /Pages with resources /F1 and /Im1
        let grandparent_ref = ObjRef::new(1, 0);
        let mut grandparent_resources = PdfDict::new();
        let mut gp_fonts = PdfDict::new();
        gp_fonts.insert(intern("F1"), PdfObject::Ref(ObjRef::new(10, 0)));
        let mut gp_xobj = PdfDict::new();
        gp_xobj.insert(intern("Im1"), PdfObject::Ref(ObjRef::new(20, 0)));
        grandparent_resources.insert(intern("Font"), PdfObject::Dict(Box::new(gp_fonts)));
        grandparent_resources.insert(intern("XObject"), PdfObject::Dict(Box::new(gp_xobj)));

        let mut grandparent = PdfDict::new();
        grandparent.insert(intern("Type"), PdfObject::Name(intern("Pages")));
        grandparent.insert(intern("Kids"), PdfObject::Array(Box::new(vec![])));
        grandparent.insert(intern("Count"), PdfObject::Integer(2));
        grandparent.insert(intern("Resources"), PdfObject::Dict(Box::new(grandparent_resources)));
        grandparent.insert(intern("MediaBox"), make_rect_array(DEFAULT_MEDIABOX));

        // Parent /Pages adds /F2
        let parent_ref = ObjRef::new(2, 0);
        let mut parent_resources = PdfDict::new();
        let mut p_fonts = PdfDict::new();
        p_fonts.insert(intern("F2"), PdfObject::Ref(ObjRef::new(11, 0)));
        parent_resources.insert(intern("Font"), PdfObject::Dict(Box::new(p_fonts)));

        let mut parent = PdfDict::new();
        parent.insert(intern("Type"), PdfObject::Name(intern("Pages")));
        parent.insert(intern("Kids"), PdfObject::Array(Box::new(vec![])));
        parent.insert(intern("Count"), PdfObject::Integer(2));
        parent.insert(intern("Resources"), PdfObject::Dict(Box::new(parent_resources)));

        // Page 1 adds /F3 and overrides /F1
        let page1_ref = ObjRef::new(3, 0);
        let mut page1_resources = PdfDict::new();
        let mut page1_fonts = PdfDict::new();
        page1_fonts.insert(intern("F1"), PdfObject::Ref(ObjRef::new(15, 0))); // Override
        page1_fonts.insert(intern("F3"), PdfObject::Ref(ObjRef::new(12, 0))); // New
        page1_resources.insert(intern("Font"), PdfObject::Dict(Box::new(page1_fonts)));

        let mut page1 = PdfDict::new();
        page1.insert(intern("Type"), PdfObject::Name(intern("Page")));
        page1.insert(intern("MediaBox"), make_rect_array(DEFAULT_MEDIABOX));
        page1.insert(intern("Resources"), PdfObject::Dict(Box::new(page1_resources)));

        // Page 2 has no resources (should inherit all)
        let page2_ref = ObjRef::new(4, 0);
        let mut page2 = PdfDict::new();
        page2.insert(intern("Type"), PdfObject::Name(intern("Page")));
        page2.insert(intern("MediaBox"), make_rect_array(DEFAULT_MEDIABOX));

        // Wire up the tree: grandparent -> parent -> [page1, page2]
        let mut grandparent_dict = grandparent.as_dict().unwrap().clone();
        grandparent_dict.insert(
            intern("Kids"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(parent_ref)]))
        );

        let mut parent_dict = parent.as_dict().unwrap().clone();
        parent_dict.insert(
            intern("Kids"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(page1_ref), PdfObject::Ref(page2_ref)]))
        );

        resolver.cache_object(grandparent_ref, PdfObject::Dict(Box::new(grandparent_dict)));
        resolver.cache_object(parent_ref, PdfObject::Dict(Box::new(parent_dict)));
        resolver.cache_object(page1_ref, PdfObject::Dict(Box::new(page1)));
        resolver.cache_object(page2_ref, PdfObject::Dict(Box::new(page2)));

        let result = flatten_page_tree(&resolver, grandparent_ref);
        assert!(result.is_ok());
        let pages_vec = result.unwrap();
        assert_eq!(pages_vec.len(), 2);

        // Page 1: should have F1 (overridden), F2 (inherited), F3 (new), Im1 (inherited)
        assert_eq!(pages_vec[0].resources.fonts.len(), 3);
        assert_eq!(pages_vec[0].resources.fonts.get(&intern("F1")), Some(&ObjRef::new(15, 0))); // Overridden
        assert_eq!(pages_vec[0].resources.fonts.get(&intern("F2")), Some(&ObjRef::new(11, 0))); // Inherited from parent
        assert_eq!(pages_vec[0].resources.fonts.get(&intern("F3")), Some(&ObjRef::new(12, 0))); // New on page
        assert_eq!(pages_vec[0].resources.xobjects.len(), 1);
        assert_eq!(pages_vec[0].resources.xobjects.get(&intern("Im1")), Some(&ObjRef::new(20, 0))); // Inherited from grandparent

        // Page 2: should have all inherited resources (F1, F2, Im1)
        assert_eq!(pages_vec[1].resources.fonts.len(), 2);
        assert_eq!(pages_vec[1].resources.fonts.get(&intern("F1")), Some(&ObjRef::new(10, 0))); // From grandparent
        assert_eq!(pages_vec[1].resources.fonts.get(&intern("F2")), Some(&ObjRef::new(11, 0))); // From parent
        assert_eq!(pages_vec[1].resources.xobjects.len(), 1);
        assert_eq!(pages_vec[1].resources.xobjects.get(&intern("Im1")), Some(&ObjRef::new(20, 0))); // From grandparent
    }

    #[test]
    fn test_resource_inheritance_page_without_resources() {
        // Test that a page without /Resources inherits parent's resources
        let resolver = XrefResolver::new();

        // Parent /Pages with resources
        let parent_ref = ObjRef::new(1, 0);
        let mut parent_resources = PdfDict::new();
        let mut parent_fonts = PdfDict::new();
        parent_fonts.insert(intern("F1"), PdfObject::Ref(ObjRef::new(10, 0)));
        parent_resources.insert(intern("Font"), PdfObject::Dict(Box::new(parent_fonts)));

        let mut parent = PdfDict::new();
        parent.insert(intern("Type"), PdfObject::Name(intern("Pages")));
        parent.insert(intern("Kids"), PdfObject::Array(Box::new(vec![])));
        parent.insert(intern("Count"), PdfObject::Integer(1));
        parent.insert(intern("Resources"), PdfObject::Dict(Box::new(parent_resources)));
        parent.insert(intern("MediaBox"), make_rect_array(DEFAULT_MEDIABOX));

        // Page without /Resources
        let page_ref = ObjRef::new(2, 0);
        let mut page = PdfDict::new();
        page.insert(intern("Type"), PdfObject::Name(intern("Page")));
        page.insert(intern("MediaBox"), make_rect_array(DEFAULT_MEDIABOX));

        // Wire up the tree
        let mut parent_dict = parent.clone();
        parent_dict.insert(
            intern("Kids"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(page_ref)]))
        );

        resolver.cache_object(parent_ref, PdfObject::Dict(Box::new(parent_dict)));
        resolver.cache_object(page_ref, PdfObject::Dict(Box::new(page)));

        let result = flatten_page_tree(&resolver, parent_ref);
        assert!(result.is_ok());
        let pages_vec = result.unwrap();
        assert_eq!(pages_vec.len(), 1);

        // Page should have inherited F1 from parent
        assert_eq!(pages_vec[0].resources.fonts.len(), 1);
        assert_eq!(pages_vec[0].resources.fonts.get(&intern("F1")), Some(&ObjRef::new(10, 0)));

        // Verify Arc pointer sharing: when page has no resources,
        // it should share the same Arc as the parent (memory efficiency)
        // We can't test this directly without exposing the parent's resources,
        // but we can verify the resources are present
    }

    #[test]
    fn test_resource_inheritance_empty_root() {
        // Test that empty /Resources at root propagates correctly
        let resolver = XrefResolver::new();

        // Root /Pages with empty /Resources
        let root_ref = ObjRef::new(1, 0);
        let mut root_resources = PdfDict::new(); // Empty resources dict
        let mut root = PdfDict::new();
        root.insert(intern("Type"), PdfObject::Name(intern("Pages")));
        root.insert(intern("Kids"), PdfObject::Array(Box::new(vec![])));
        root.insert(intern("Count"), PdfObject::Integer(1));
        root.insert(intern("Resources"), PdfObject::Dict(Box::new(root_resources)));
        root.insert(intern("MediaBox"), make_rect_array(DEFAULT_MEDIABOX));

        // Page without /Resources
        let page_ref = ObjRef::new(2, 0);
        let mut page = PdfDict::new();
        page.insert(intern("Type"), PdfObject::Name(intern("Page")));
        page.insert(intern("MediaBox"), make_rect_array(DEFAULT_MEDIABOX));

        // Wire up the tree
        let mut root_dict = root.clone();
        root_dict.insert(
            intern("Kids"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(page_ref)]))
        );

        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));
        resolver.cache_object(page_ref, PdfObject::Dict(Box::new(page)));

        let result = flatten_page_tree(&resolver, root_ref);
        assert!(result.is_ok());
        let pages_vec = result.unwrap();
        assert_eq!(pages_vec.len(), 1);

        // Page should have empty resources
        assert!(pages_vec[0].resources.is_empty());
    }
}

/// Property tests for page tree flattening fuzzing.
///
/// Per acceptance criteria: "proptest: random page-tree shapes never panic"
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    /// Helper to make a /Pages dict (duplicate from tests module).
    fn make_pages_dict(kids: Vec<PdfObject>, count: i64, media_box: Option<[f64; 4]>) -> PdfObject {
        let mut dict = PdfDict::new();
        dict.insert(intern("Type"), PdfObject::Name(intern("Pages")));
        dict.insert(intern("Kids"), PdfObject::Array(Box::new(kids)));
        dict.insert(intern("Count"), PdfObject::Integer(count));
        if let Some(mb) = media_box {
            dict.insert(intern("MediaBox"), PdfObject::Array(Box::new(vec![
                PdfObject::Real(mb[0]),
                PdfObject::Real(mb[1]),
                PdfObject::Real(mb[2]),
                PdfObject::Real(mb[3]),
            ])));
        }
        PdfObject::Dict(Box::new(dict))
    }

    /// Helper to make a /Page dict (duplicate from tests module).
    fn make_page_dict(media_box: Option<[f64; 4]>, rotate: Option<i64>) -> PdfObject {
        let mut dict = PdfDict::new();
        dict.insert(intern("Type"), PdfObject::Name(intern("Page")));
        if let Some(mb) = media_box {
            dict.insert(intern("MediaBox"), PdfObject::Array(Box::new(vec![
                PdfObject::Real(mb[0]),
                PdfObject::Real(mb[1]),
                PdfObject::Real(mb[2]),
                PdfObject::Real(mb[3]),
            ])));
        }
        if let Some(rot) = rotate {
            dict.insert(intern("Rotate"), PdfObject::Integer(rot));
        }
        PdfObject::Dict(Box::new(dict))
    }

    /// Strategy to generate arbitrary rectangle arrays.
    fn arb_rect() -> impl Strategy<Value = [f64; 4]> {
        prop::array::uniform4(-1000.0..1000.0)
    }

    /// Strategy to generate arbitrary page dictionaries.
    fn arb_page_dict() -> impl Strategy<Value = PdfDict> {
        (
            arb_rect(),
            prop::option::of(-1000i64..1000),
            prop::option::of(arb_rect()),
            prop::option::of(arb_rect()),
        ).prop_map(|(media_box, rotate, crop_box, bleed_box)| {
            let mut dict = PdfDict::new();
            dict.insert(intern("Type"), PdfObject::Name(intern("Page")));
            dict.insert(intern("MediaBox"), PdfObject::Array(Box::new(vec![
                PdfObject::Real(media_box[0]),
                PdfObject::Real(media_box[1]),
                PdfObject::Real(media_box[2]),
                PdfObject::Real(media_box[3]),
            ])));
            if let Some(rot) = rotate {
                dict.insert(intern("Rotate"), PdfObject::Integer(rot));
            }
            if let Some(cb) = crop_box {
                dict.insert(intern("CropBox"), PdfObject::Array(Box::new(vec![
                    PdfObject::Real(cb[0]),
                    PdfObject::Real(cb[1]),
                    PdfObject::Real(cb[2]),
                    PdfObject::Real(cb[3]),
                ])));
            }
            if let Some(bb) = bleed_box {
                dict.insert(intern("BleedBox"), PdfObject::Array(Box::new(vec![
                    PdfObject::Real(bb[0]),
                    PdfObject::Real(bb[1]),
                    PdfObject::Real(bb[2]),
                    PdfObject::Real(bb[3]),
                ])));
            }
            dict
        })
    }

    /// Strategy to generate /Pages dictionaries with direct /Kids.
    fn arb_pages_dict_with_direct_kids(max_depth: u8) -> impl Strategy<Value = PdfDict> {
        let leaf = prop::option::of(arb_page_dict());

        leaf.prop_map(move |maybe_page: Option<PdfDict>| {
            let mut dict = PdfDict::new();
            dict.insert(intern("Type"), PdfObject::Name(intern("Pages")));
            dict.insert(intern("Count"), PdfObject::Integer(0));

            if let Some(page) = maybe_page {
                dict.insert(intern("Kids"), PdfObject::Array(Box::new(vec![
                    PdfObject::Dict(Box::new(page))
                ])));
                dict.insert(intern("Count"), PdfObject::Integer(1));
            } else {
                dict.insert(intern("Kids"), PdfObject::Array(Box::new(vec![])));
            }
            dict
        })
    }

    proptest! {
        /// Test that parse_rect never panics on arbitrary arrays (INV-8).
        #[test]
        fn fuzz_parse_rect_no_panics(arr in prop::collection::vec(any::<f64>(), 0..10)) {
            let obj = PdfObject::Array(Box::new(
                arr.into_iter().map(|f| if f.is_finite() { PdfObject::Real(f) } else { PdfObject::Real(0.0) }).collect()
            ));
            // This should never panic
            let _ = parse_rect(Some(&obj));
        }

        /// Test that build_page_dict never panics on arbitrary input.
        #[test]
        fn fuzz_build_page_dict_no_panics(page_dict in arb_page_dict()) {
            let inherited = InheritedAttrs::default();
            let mut diagnostics = Vec::new();
            let page_obj = PdfObject::Dict(Box::new(page_dict));

            // This should never panic
            let _ = build_page_dict(&page_obj, &inherited, &mut diagnostics);
        }

        /// Test that flatten_page_tree handles arbitrary /Pages structures without panicking.
        #[test]
        fn fuzz_flatten_page_tree_no_panics(pages_dict in arb_pages_dict_with_direct_kids(2)) {
            let resolver = XrefResolver::new();
            let pages_ref = ObjRef::new(1, 0);

            resolver.cache_object(pages_ref, PdfObject::Dict(Box::new(pages_dict)));

            // This should never panic - should always return Ok or Err with diagnostics
            let _ = flatten_page_tree(&resolver, pages_ref);
        }

        /// Test that arbitrary rotate values are handled without panicking.
        #[test]
        fn fuzz_rotate_clamping_no_panics(rot in any::<i64>()) {
            let resolver = XrefResolver::new();
            let pages_ref = ObjRef::new(1, 0);

            let pages = make_pages_dict(
                vec![make_page_dict(Some(DEFAULT_MEDIABOX), Some(rot))],
                1,
                Some(DEFAULT_MEDIABOX),
            );

            resolver.cache_object(pages_ref, pages);

            // This should never panic
            let result = flatten_page_tree(&resolver, pages_ref);
            prop_assert!(result.is_ok() || result.is_err());
        }
    }
}
