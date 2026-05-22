//! Resource dictionary handling with inheritance.
//!
//! PDF 1.7, Section 7.7.3.3 "Resource Dictionary"
//!
//! This module implements per-page resource dictionary merging across
//! the /Pages tree hierarchy. Each page receives a merged ResourceDict
//! containing all resources from its ancestor /Pages nodes, with per-key
//! last-write-wins semantics at the page level.

use crate::parser::object::{ObjRef, PdfObject, PdfDict, intern};
use std::sync::Arc;
use indexmap::IndexMap;

/// A merged resource dictionary for a page.
///
/// Contains all resource namespaces from the page's ancestors,
/// merged according to PDF inheritance rules.
#[derive(Debug, Clone)]
pub struct ResourceDict {
    /// /Font namespace: maps font names to font dictionaries
    pub fonts: IndexMap<Arc<str>, ObjRef>,
    /// /XObject namespace: maps XObject names to form/image XObjects
    pub xobjects: IndexMap<Arc<str>, ObjRef>,
    /// /ExtGState namespace: maps graphics state names to ExtGState dictionaries
    pub ext_gstates: IndexMap<Arc<str>, ObjRef>,
    /// /ColorSpace namespace: maps color space names to color space definitions
    /// Can be either indirect references (most common) or direct arrays (inline)
    pub color_spaces: IndexMap<Arc<str>, PdfObject>,
    /// /Shading namespace: maps shading names to shading dictionaries
    pub shadings: IndexMap<Arc<str>, ObjRef>,
    /// /Pattern namespace: maps pattern names to pattern dictionaries
    pub patterns: IndexMap<Arc<str>, ObjRef>,
    /// /Properties namespace: maps property names to property dictionaries
    /// Used for marked content and OCG references
    pub properties: IndexMap<Arc<str>, ObjRef>,
    /// /ProcSet array (deprecated in PDF 1.7+)
    /// Informational only; preserved but not enforced
    pub proc_set: Vec<Arc<str>>,
}

impl Default for ResourceDict {
    fn default() -> Self {
        ResourceDict {
            fonts: IndexMap::new(),
            xobjects: IndexMap::new(),
            ext_gstates: IndexMap::new(),
            color_spaces: IndexMap::new(),
            shadings: IndexMap::new(),
            patterns: IndexMap::new(),
            properties: IndexMap::new(),
            proc_set: Vec::new(),
        }
    }
}

impl ResourceDict {
    /// Create an empty ResourceDict.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if this ResourceDict is completely empty (no resources in any namespace).
    pub fn is_empty(&self) -> bool {
        self.fonts.is_empty()
            && self.xobjects.is_empty()
            && self.ext_gstates.is_empty()
            && self.color_spaces.is_empty()
            && self.shadings.is_empty()
            && self.patterns.is_empty()
            && self.properties.is_empty()
            && self.proc_set.is_empty()
    }

    /// Get the total number of resources across all namespaces.
    pub fn total_count(&self) -> usize {
        self.fonts.len()
            + self.xobjects.len()
            + self.ext_gstates.len()
            + self.color_spaces.len()
            + self.shadings.len()
            + self.patterns.len()
            + self.properties.len()
            + self.proc_set.len()
    }
}

/// Merge a child /Resources dictionary into an ancestor ResourceDict.
///
/// This function implements PDF resource inheritance: each namespace is merged
/// independently, with per-key last-write-wins semantics. If a page declares
/// a resource with the same name as an ancestor, the page's version wins.
///
/// # Arguments
/// * `ancestor` - The merged ResourceDict from parent /Pages nodes
/// * `child` - The /Resources dictionary from the current node (may be null)
///
/// # Returns
/// A new ResourceDict containing the merged resources.
///
/// # Example
/// ```ignore
/// // Ancestor has /F1 and /F2 fonts
/// let ancestor = ResourceDict {
///     fonts: map!["F1" => ref1, "F2" => ref2],
///     ...
/// };
///
/// // Page adds /F3 and overrides /F1
/// let child_resources = dict!{
///     "Font" => dict!{"F1" => new_ref1, "F3" => ref3}
/// };
///
/// // Merged: F1 from page, F2 from ancestor, F3 from page
/// let merged = merge_resources(&ancestor, &child_resources);
/// assert_eq!(merged.fonts["F1"], new_ref1);
/// assert_eq!(merged.fonts["F2"], ref2);
/// assert_eq!(merged.fonts["F3"], ref3);
/// ```
pub fn merge_resources(ancestor: &ResourceDict, child: &PdfObject) -> ResourceDict {
    // Start with a clone of the ancestor
    let mut merged = ancestor.clone();

    // If child has no /Resources, return ancestor as-is
    let child_dict = match child {
        PdfObject::Null => return merged,
        PdfObject::Dict(d) => &**d,
        PdfObject::Ref(_) => {
            // Indirect reference - we can't resolve it here without the resolver
            // This case is handled by the caller during page tree traversal
            return merged;
        }
        _ => return merged,
    };

    // Merge /Font namespace
    if let Some(font_obj) = child_dict.get("Font") {
        if let Some(font_dict) = font_obj.as_dict() {
            for (name, obj) in font_dict.iter() {
                if let Some(ref_) = obj.as_ref() {
                    merged.fonts.insert(name.clone(), ref_);
                }
                // Direct dictionaries in /Font are rare but legal; we skip them
                // because they should have been indirect in a well-formed PDF
            }
        }
    }

    // Merge /XObject namespace
    if let Some(xobj_obj) = child_dict.get("XObject") {
        if let Some(xobj_dict) = xobj_obj.as_dict() {
            for (name, obj) in xobj_dict.iter() {
                if let Some(ref_) = obj.as_ref() {
                    merged.xobjects.insert(name.clone(), ref_);
                }
            }
        }
    }

    // Merge /ExtGState namespace
    if let Some(gs_obj) = child_dict.get("ExtGState") {
        if let Some(gs_dict) = gs_obj.as_dict() {
            for (name, obj) in gs_dict.iter() {
                if let Some(ref_) = obj.as_ref() {
                    merged.ext_gstates.insert(name.clone(), ref_);
                }
            }
        }
    }

    // Merge /ColorSpace namespace (can be inline arrays OR refs)
    if let Some(cs_obj) = child_dict.get("ColorSpace") {
        if let Some(cs_dict) = cs_obj.as_dict() {
            for (name, obj) in cs_dict.iter() {
                // Preserve both refs and direct arrays
                merged.color_spaces.insert(name.clone(), obj.clone());
            }
        }
    }

    // Merge /Shading namespace
    if let Some(shade_obj) = child_dict.get("Shading") {
        if let Some(shade_dict) = shade_obj.as_dict() {
            for (name, obj) in shade_dict.iter() {
                if let Some(ref_) = obj.as_ref() {
                    merged.shadings.insert(name.clone(), ref_);
                }
            }
        }
    }

    // Merge /Pattern namespace
    if let Some(pattern_obj) = child_dict.get("Pattern") {
        if let Some(pattern_dict) = pattern_obj.as_dict() {
            for (name, obj) in pattern_dict.iter() {
                if let Some(ref_) = obj.as_ref() {
                    merged.patterns.insert(name.clone(), ref_);
                }
            }
        }
    }

    // Merge /Properties namespace
    if let Some(prop_obj) = child_dict.get("Properties") {
        if let Some(prop_dict) = prop_obj.as_dict() {
            for (name, obj) in prop_dict.iter() {
                if let Some(ref_) = obj.as_ref() {
                    merged.properties.insert(name.clone(), ref_);
                }
            }
        }
    }

    // Merge /ProcSet (deprecated; just collect names)
    if let Some(procset_obj) = child_dict.get("ProcSet") {
        if let Some(procset_arr) = procset_obj.as_array() {
            for obj in procset_arr.iter() {
                if let Some(name) = obj.as_name() {
                    let name_arc = intern(name);
                    if !merged.proc_set.contains(&name_arc) {
                        merged.proc_set.push(name_arc);
                    }
                }
            }
        }
    }

    merged
}

/// Extract a ResourceDict from a /Resources dictionary object.
///
/// This function is called when we first encounter a /Resources dict
/// (typically at the root /Pages node). It converts the raw PdfObject
/// into a ResourceDict structure.
///
/// # Arguments
/// * `resources_obj` - The /Resources dictionary (may be null)
///
/// # Returns
/// A ResourceDict containing all resources from the dictionary.
pub fn extract_resources(resources_obj: &PdfObject) -> ResourceDict {
    let empty = ResourceDict::default();
    merge_resources(&empty, resources_obj)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_resource_dict() {
        let dict = ResourceDict::new();
        assert!(dict.is_empty());
        assert_eq!(dict.total_count(), 0);
    }

    #[test]
    fn test_resource_dict_not_empty() {
        let mut dict = ResourceDict::new();
        dict.fonts.insert(intern("F1"), ObjRef::new(1, 0));
        assert!(!dict.is_empty());
        assert_eq!(dict.total_count(), 1);
    }

    #[test]
    fn test_merge_fonts_last_write_wins() {
        // Ancestor has /F1 and /F2
        let mut ancestor = ResourceDict::new();
        ancestor.fonts.insert(intern("F1"), ObjRef::new(1, 0));
        ancestor.fonts.insert(intern("F2"), ObjRef::new(2, 0));

        // Child overrides /F1 and adds /F3
        let mut child_resources = PdfDict::new();
        let mut child_font = PdfDict::new();
        child_font.insert(intern("F1"), PdfObject::Ref(ObjRef::new(10, 0)));
        child_font.insert(intern("F3"), PdfObject::Ref(ObjRef::new(3, 0)));
        child_resources.insert(intern("Font"), PdfObject::Dict(Box::new(child_font)));

        let child_obj = PdfObject::Dict(Box::new(child_resources));

        // Merged should have F1 from child, F2 from ancestor, F3 from child
        let merged = merge_resources(&ancestor, &child_obj);

        assert_eq!(merged.fonts.len(), 3);
        assert_eq!(merged.fonts.get(&intern("F1")), Some(&ObjRef::new(10, 0))); // Overridden
        assert_eq!(merged.fonts.get(&intern("F2")), Some(&ObjRef::new(2, 0)));  // Inherited
        assert_eq!(merged.fonts.get(&intern("F3")), Some(&ObjRef::new(3, 0)));  // New
    }

    #[test]
    fn test_merge_xobjects() {
        let mut ancestor = ResourceDict::new();
        ancestor.xobjects.insert(intern("Im1"), ObjRef::new(5, 0));

        let mut child_resources = PdfDict::new();
        let mut child_xobj = PdfDict::new();
        child_xobj.insert(intern("Im2"), PdfObject::Ref(ObjRef::new(6, 0)));
        child_resources.insert(intern("XObject"), PdfObject::Dict(Box::new(child_xobj)));

        let merged = merge_resources(&ancestor, &PdfObject::Dict(Box::new(child_resources)));

        assert_eq!(merged.xobjects.len(), 2);
        assert_eq!(merged.xobjects.get(&intern("Im1")), Some(&ObjRef::new(5, 0)));
        assert_eq!(merged.xobjects.get(&intern("Im2")), Some(&ObjRef::new(6, 0)));
    }

    #[test]
    fn test_merge_colorspace_inline_array() {
        // ColorSpace can be an inline array (not just a ref)
        let mut ancestor = ResourceDict::new();

        let mut child_resources = PdfDict::new();
        let mut child_cs = PdfDict::new();

        // Inline color space array: [/CalRGB << /Gamma [1 1 1] >>]
        let mut gamma_arr = PdfDict::new();
        gamma_arr.insert(intern("Gamma"), PdfObject::Array(Box::new(vec![
            PdfObject::Integer(1),
            PdfObject::Integer(1),
            PdfObject::Integer(1),
        ])));

        child_cs.insert(
            intern("CS1"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Name(intern("CalRGB")),
                PdfObject::Dict(Box::new(gamma_arr)),
            ])),
        );

        child_resources.insert(intern("ColorSpace"), PdfObject::Dict(Box::new(child_cs)));

        let merged = merge_resources(&ancestor, &PdfObject::Dict(Box::new(child_resources)));

        assert_eq!(merged.color_spaces.len(), 1);
        let cs1 = merged.color_spaces.get(&intern("CS1")).unwrap();
        assert!(cs1.as_array().is_some());
    }

    #[test]
    fn test_merge_procset_dedup() {
        let ancestor = ResourceDict::new();

        let mut child_resources = PdfDict::new();
        // /ProcSet can have duplicates (legal but weird)
        child_resources.insert(
            intern("ProcSet"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Name(intern("PDF")),
                PdfObject::Name(intern("Text")),
                PdfObject::Name(intern("PDF")), // Duplicate
            ])),
        );

        let merged = merge_resources(&ancestor, &PdfObject::Dict(Box::new(child_resources)));

        // Should deduplicate
        assert_eq!(merged.proc_set.len(), 2);
    }

    #[test]
    fn test_merge_null_child_returns_ancestor() {
        let mut ancestor = ResourceDict::new();
        ancestor.fonts.insert(intern("F1"), ObjRef::new(1, 0));

        let merged = merge_resources(&ancestor, &PdfObject::Null);

        assert_eq!(merged.fonts.len(), 1);
        assert_eq!(merged.fonts.get(&intern("F1")), Some(&ObjRef::new(1, 0)));
    }

    #[test]
    fn test_three_level_inheritance() {
        // Critical test: resources from grandparent + parent + page
        let mut grandparent = ResourceDict::new();
        grandparent.fonts.insert(intern("F1"), ObjRef::new(1, 0));

        // Parent adds F2
        let mut parent_resources = PdfDict::new();
        let mut parent_fonts = PdfDict::new();
        parent_fonts.insert(intern("F2"), PdfObject::Ref(ObjRef::new(2, 0)));
        parent_resources.insert(intern("Font"), PdfObject::Dict(Box::new(parent_fonts)));

        let parent = merge_resources(&grandparent, &PdfObject::Dict(Box::new(parent_resources)));

        // Page adds F3
        let mut page_resources = PdfDict::new();
        let mut page_fonts = PdfDict::new();
        page_fonts.insert(intern("F3"), PdfObject::Ref(ObjRef::new(3, 0)));
        page_resources.insert(intern("Font"), PdfObject::Dict(Box::new(page_fonts)));

        let page = merge_resources(&parent, &PdfObject::Dict(Box::new(page_resources)));

        // All three fonts should be present
        assert_eq!(page.fonts.len(), 3);
        assert_eq!(page.fonts.get(&intern("F1")), Some(&ObjRef::new(1, 0)));
        assert_eq!(page.fonts.get(&intern("F2")), Some(&ObjRef::new(2, 0)));
        assert_eq!(page.fonts.get(&intern("F3")), Some(&ObjRef::new(3, 0)));
    }

    #[test]
    fn test_merge_all_namespaces() {
        let ancestor = ResourceDict::new();

        let mut child_resources = PdfDict::new();

        // /Font
        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("F1"), PdfObject::Ref(ObjRef::new(1, 0)));
        child_resources.insert(intern("Font"), PdfObject::Dict(Box::new(font_dict)));

        // /XObject
        let mut xobj_dict = PdfDict::new();
        xobj_dict.insert(intern("Im1"), PdfObject::Ref(ObjRef::new(5, 0)));
        child_resources.insert(intern("XObject"), PdfObject::Dict(Box::new(xobj_dict)));

        // /ExtGState
        let mut gs_dict = PdfDict::new();
        gs_dict.insert(intern("GS1"), PdfObject::Ref(ObjRef::new(10, 0)));
        child_resources.insert(intern("ExtGState"), PdfObject::Dict(Box::new(gs_dict)));

        // /ColorSpace
        let mut cs_dict = PdfDict::new();
        cs_dict.insert(intern("CS1"), PdfObject::Ref(ObjRef::new(15, 0)));
        child_resources.insert(intern("ColorSpace"), PdfObject::Dict(Box::new(cs_dict)));

        // /Shading
        let mut shade_dict = PdfDict::new();
        shade_dict.insert(intern("Sh1"), PdfObject::Ref(ObjRef::new(20, 0)));
        child_resources.insert(intern("Shading"), PdfObject::Dict(Box::new(shade_dict)));

        // /Pattern
        let mut pat_dict = PdfDict::new();
        pat_dict.insert(intern("P1"), PdfObject::Ref(ObjRef::new(25, 0)));
        child_resources.insert(intern("Pattern"), PdfObject::Dict(Box::new(pat_dict)));

        // /Properties
        let mut prop_dict = PdfDict::new();
        prop_dict.insert(intern("MC1"), PdfObject::Ref(ObjRef::new(30, 0)));
        child_resources.insert(intern("Properties"), PdfObject::Dict(Box::new(prop_dict)));

        let merged = merge_resources(&ancestor, &PdfObject::Dict(Box::new(child_resources)));

        assert_eq!(merged.fonts.len(), 1);
        assert_eq!(merged.xobjects.len(), 1);
        assert_eq!(merged.ext_gstates.len(), 1);
        assert_eq!(merged.color_spaces.len(), 1);
        assert_eq!(merged.shadings.len(), 1);
        assert_eq!(merged.patterns.len(), 1);
        assert_eq!(merged.properties.len(), 1);
    }
}
