//! Digital signature field discovery and metadata extraction.
//!
//! This module implements Phase 7.3 of the plan: digital signature metadata.
//! It walks the AcroForm /Fields array to discover signature fields, extracts
//! metadata from signature dictionaries, and computes coverage statistics.
//!
//! ## Architecture
//!
//! - **Discovery** (this module): Walk /Fields recursively, filter to /FT /Sig
//! - **Metadata extraction** (future): Extract /V dict properties (signer, date, reason, etc.)
//! - **Validation** (out of scope): Cryptographic validation requires certificate chains
//!
//! ## Reuse
//!
//! The `walk_acroform_fields` helper is designed for reuse by Phase 7.4 (form fields),
//! which walks the same tree but filters to all field types, not just /Sig.

use crate::parser::catalog::Catalog;
use crate::parser::object::{ObjRef, PdfObject, PdfDict, intern};
use crate::parser::xref::XrefResolver;
use crate::diagnostics::{Diagnostic, DiagCode};
use std::sync::Arc;

/// Result type for signature operations.
pub type Result<T> = std::result::Result<T, Vec<Diagnostic>>;

/// A reference to a signature field in the AcroForm.
///
/// Represents a discovered signature field with its location and metadata.
/// This is the output of the discovery phase (7.3.1); metadata extraction
/// happens in 7.3.2.
#[derive(Debug, Clone, PartialEq)]
pub struct SigFieldRef {
    /// Absolute (dot-joined) field name, e.g., "employer_signature" or "form.employee_sig"
    pub full_name: String,

    /// Indirect reference to the /V dictionary (signature value) if present.
    ///
    /// Absent means the field exists but is unsigned (blank signature field).
    /// Present means the field has been signed at least once.
    pub v_ref: Option<ObjRef>,

    /// Bounding rectangle for the signature appearance on the page.
    ///
    /// Format: [x0, y0, x1, y1] in PDF user-space points.
    /// None if the field has no visual appearance (form-only signature).
    pub rect: Option<[f32; 4]>,

    /// Index of the page containing this signature field's widget annotation.
    ///
    /// None if the field has no widget on any page (form-only signature).
    pub page_index: Option<usize>,

    /// The field's own indirect reference.
    pub field_ref: ObjRef,
}

/// A field reference from AcroForm walking.
///
/// Internal type used by `walk_acroform_fields` to represent any field
/// (signature, text, button, choice). This is the reusable primitive that
/// 7.4 will consume directly.
#[derive(Debug, Clone)]
struct FieldRef {
    /// Absolute (dot-joined) field name
    full_name: String,

    /// Field type (/FT): Tx, Btn, Ch, Sig (or None if inherited)
    field_type: Option<String>,

    /// Indirect reference to /V (current value) if present
    v_ref: Option<ObjRef>,

    /// Bounding rectangle if present
    rect: Option<[f32; 4]>,

    /// Page index if resolvable
    page_index: Option<usize>,

    /// The field's own indirect reference
    field_ref: ObjRef,

    /// Parent field type (for /FT inheritance)
    parent_ft: Option<String>,
}

impl FieldRef {
    /// Check if this field is a signature field.
    ///
    /// A field is a signature field if its /FT (or inherited /FT) is /Sig.
    fn is_signature(&self) -> bool {
        let ft = self.field_type.as_ref().or(self.parent_ft.as_ref());
        ft.map(|t| t == "Sig").unwrap_or(false)
    }

    /// Convert to SigFieldRef if this is a signature field.
    fn into_sig_field(self) -> Option<SigFieldRef> {
        if self.is_signature() {
            Some(SigFieldRef {
                full_name: self.full_name,
                v_ref: self.v_ref,
                rect: self.rect,
                page_index: self.page_index,
                field_ref: self.field_ref,
            })
        } else {
            None
        }
    }
}

/// Walk the AcroForm /Fields array recursively and collect all fields.
///
/// This is the reusable walker that both signature discovery (7.3) and
/// form field extraction (7.4) will use. It performs DFS traversal of
/// the /Kids hierarchy, resolves /FT inheritance, and constructs absolute
/// field names.
///
/// # Arguments
///
/// * `resolver` - Xref resolver for dereferencing indirect objects
/// * `catalog` - Document catalog containing the AcroForm reference
///
/// # Returns
///
/// A `Vec<FieldRef>` containing all discovered fields (not just signatures).
///
/// # Behavior
///
/// - If /AcroForm is absent, returns empty vec (not an error)
/// - If /Fields is absent or empty, returns empty vec
/// - Descends recursively via /Kids arrays
/// - Resolves /FT inheritance from parent to child fields
/// - Constructs absolute names by joining /T values with "."
/// - Emits diagnostics for malformed structures but continues
fn walk_acroform_fields(
    resolver: &XrefResolver,
    catalog: &Catalog,
) -> Vec<FieldRef> {
    let mut fields = Vec::new();
    let mut diagnostics = Vec::new();

    // AcroForm is optional; absent means no fields
    let acroform_ref = match catalog.acroform_ref {
        Some(ref_) => ref_,
        None => return fields,
    };

    // Resolve the AcroForm dictionary
    let acroform = match resolver.resolve(acroform_ref) {
        Ok(obj) => obj,
        Err(_) => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!("Failed to resolve /AcroForm reference {}", acroform_ref),
            ));
            return fields;
        }
    };

    let acroform_dict = match acroform.as_dict() {
        Some(d) => d,
        None => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!("/AcroForm is not a dictionary (type: {})", acroform.type_name()),
            ));
            return fields;
        }
    };

    // /Fields is an array of indirect references to field dictionaries
    let fields_array = match acroform_dict.get("Fields").and_then(|o| o.as_array()) {
        Some(arr) => arr,
        None => return fields, // No /Fields means no form fields
    };

    // Walk each field in the /Fields array
    for field_obj in fields_array.iter() {
        let field_ref = match field_obj {
            PdfObject::Ref(ref_) => *ref_,
            _ => continue, // Skip non-reference entries
        };

        walk_field_recursive(
            resolver,
            field_ref,
            &mut fields,
            String::new(),
            None,
            &mut diagnostics,
        );
    }

    fields
}

/// Recursively walk a field dictionary and its /Kids.
///
/// This helper function performs DFS traversal of the field hierarchy,
/// building absolute field names and tracking /FT inheritance.
///
/// # Arguments
///
/// * `resolver` - Xref resolver
/// * `field_ref` - Indirect reference to the current field dictionary
/// * `fields` - Output accumulator for discovered fields
/// * `parent_name` - Accumulated absolute name from parent path
/// * `parent_ft` - Inherited field type from parent (/FT value)
/// * `diagnostics` - Diagnostic accumulator
fn walk_field_recursive(
    resolver: &XrefResolver,
    field_ref: ObjRef,
    fields: &mut Vec<FieldRef>,
    parent_name: String,
    parent_ft: Option<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Resolve the field dictionary
    let field_obj = match resolver.resolve(field_ref) {
        Ok(obj) => obj,
        Err(_) => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!("Failed to resolve field reference {}", field_ref),
            ));
            return;
        }
    };

    let field_dict = match field_obj.as_dict() {
        Some(d) => d,
        None => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!("Field {} is not a dictionary", field_ref),
            ));
            return;
        }
    };

    // Extract /T (partial name) for building absolute name
    let partial_name = field_dict.get("T")
        .and_then(|o| o.as_string())
        .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());

    // Build the absolute field name
    let full_name = if let Some(ref pn) = partial_name {
        if parent_name.is_empty() {
            pn.clone()
        } else {
            format!("{}.{}", parent_name, pn)
        }
    } else {
        parent_name.clone()
    };

    // Extract /FT (field type) - may be absent on child fields (inherit from parent)
    let field_type = field_dict.get("FT")
        .and_then(|o| o.as_name())
        .map(|n| n.to_string());

    // Use parent's /FT if this field doesn't have one
    let effective_ft = field_type.as_ref().or(parent_ft.as_ref());

    // Extract /V (current value) if present
    let v_ref = field_dict.get("V")
        .and_then(|o| match o {
            PdfObject::Ref(r) => Some(*r),
            _ => None,
        });

    // Extract /Rect (bounding rectangle) if present
    let rect = field_dict.get("Rect")
        .and_then(|o| o.as_array())
        .and_then(|arr| {
            if arr.len() == 4 {
                let coords: Vec<Option<f64>> = arr.iter()
                    .map(|o| o.as_real().or_else(|| o.as_int().map(|i| i as f64)))
                    .collect();
                if coords.iter().all(|c| c.is_some()) {
                    Some([
                        coords[0].unwrap() as f32,
                        coords[1].unwrap() as f32,
                        coords[2].unwrap() as f32,
                        coords[3].unwrap() as f32,
                    ])
                } else {
                    None
                }
            } else {
                None
            }
        });

    // TODO: Resolve page_index by searching page /Annots arrays
    // This requires access to the page tree, which we don't have here.
    // For now, page_index is always None.
    let page_index = None;

    // Check for /Kids (nested fields)
    let kids = field_dict.get("Kids").and_then(|o| o.as_array());

    if let Some(kids_array) = kids {
        // This is a parent field with children - recurse into /Kids
        for kid_obj in kids_array.iter() {
            let kid_ref = match kid_obj {
                PdfObject::Ref(ref_) => *ref_,
                _ => continue,
            };

            walk_field_recursive(
                resolver,
                kid_ref,
                fields,
                full_name.clone(),
                effective_ft.map(|s| s.clone()),
                diagnostics,
            );
        }
    } else {
        // This is a leaf field - emit it
        fields.push(FieldRef {
            full_name,
            field_type,
            v_ref,
            rect,
            page_index,
            field_ref,
            parent_ft,
        });
    }
}

/// Discover all signature fields in the PDF document.
///
/// This is the main entry point for Phase 7.3.1: signature field discovery.
/// It walks the AcroForm /Fields array and filters to fields whose /FT
/// (field type) is /Sig.
///
/// # Arguments
///
/// * `resolver` - Xref resolver for dereferencing indirect objects
/// * `catalog` - Document catalog containing the AcroForm reference
///
/// # Returns
///
/// A `Vec<SigFieldRef>` containing all discovered signature fields.
/// Returns empty vec if the PDF has no AcroForm or no signature fields.
///
/// # Example
///
/// ```ignore
/// use pdftract_core::signature::discover;
///
/// let sig_fields = discover(&resolver, &catalog);
/// for sig in sig_fields {
///     println!("Signature field: {}", sig.full_name);
///     if let Some(v_ref) = sig.v_ref {
///         println!("  Signed: {}", v_ref);
///     } else {
///         println!("  Unsigned (blank)");
///     }
/// }
/// ```
pub fn discover(
    resolver: &XrefResolver,
    catalog: &Catalog,
) -> Vec<SigFieldRef> {
    walk_acroform_fields(resolver, catalog)
        .into_iter()
        .filter_map(|f| f.into_sig_field())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::{intern, PdfObject};

    /// Helper to create a test catalog with an AcroForm.
    fn make_test_acroform(fields: Vec<PdfObject>) -> (Catalog, XrefResolver) {
        let mut resolver = XrefResolver::new();

        // Create the AcroForm dictionary
        let mut acroform_dict = indexmap::IndexMap::new();
        acroform_dict.insert(intern("Fields"), PdfObject::Array(Box::new(fields)));

        let acroform_ref = ObjRef::new(10, 0);
        resolver.cache_object(acroform_ref, PdfObject::Dict(Box::new(acroform_dict)));

        // Create a minimal catalog
        let mut catalog = Catalog::new(ObjRef::new(1, 0));
        catalog.acroform_ref = Some(acroform_ref);

        (catalog, resolver)
    }

    /// Helper to create a field dictionary with a specific ID.
    fn make_field_dict_with_id(
        id: u32,
        ft: Option<&str>,
        t: Option<&str>,
        v: Option<ObjRef>,
        rect: Option<[f32; 4]>,
        kids: Option<Vec<ObjRef>>,
    ) -> (ObjRef, PdfObject) {
        let mut dict = indexmap::IndexMap::new();

        if let Some(ft_val) = ft {
            dict.insert(intern("FT"), PdfObject::Name(intern(ft_val)));
        }

        if let Some(t_val) = t {
            dict.insert(intern("T"), PdfObject::String(Box::new(t_val.as_bytes().to_vec())));
        }

        if let Some(v_ref) = v {
            dict.insert(intern("V"), PdfObject::Ref(v_ref));
        }

        if let Some(rect_val) = rect {
            let rect_array: Vec<PdfObject> = rect_val.iter()
                .map(|&c| PdfObject::Real(c as f64))
                .collect();
            dict.insert(intern("Rect"), PdfObject::Array(Box::new(rect_array)));
        }

        if let Some(kids_refs) = kids {
            let kids_array: Vec<PdfObject> = kids_refs.iter()
                .map(|&r| PdfObject::Ref(r))
                .collect();
            dict.insert(intern("Kids"), PdfObject::Array(Box::new(kids_array)));
        }

        let field_ref = ObjRef::new(100 + id, 0);
        (field_ref, PdfObject::Dict(Box::new(dict)))
    }

    #[test]
    fn test_discover_no_acroform() {
        let catalog = Catalog::new(ObjRef::new(1, 0));
        let resolver = XrefResolver::new();

        let sig_fields = discover(&resolver, &catalog);

        assert!(sig_fields.is_empty());
    }

    #[test]
    fn test_discover_no_fields() {
        let mut resolver = XrefResolver::new();

        let acroform_ref = ObjRef::new(10, 0);
        let acroform_dict = indexmap::IndexMap::new();
        resolver.cache_object(acroform_ref, PdfObject::Dict(Box::new(acroform_dict)));

        let mut catalog = Catalog::new(ObjRef::new(1, 0));
        catalog.acroform_ref = Some(acroform_ref);

        let sig_fields = discover(&resolver, &catalog);

        assert!(sig_fields.is_empty());
    }

    #[test]
    fn test_discover_two_flat_signatures() {
        let (field1_ref, field1) = make_field_dict_with_id(
            1,
            Some("Sig"),
            Some("employer_sig"),
            None,
            None,
            None,
        );

        let (field2_ref, field2) = make_field_dict_with_id(
            2,
            Some("Sig"),
            Some("employee_sig"),
            None,
            None,
            None,
        );

        let fields = vec![
            PdfObject::Ref(field1_ref),
            PdfObject::Ref(field2_ref),
        ];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(field1_ref, field1);
        resolver.cache_object(field2_ref, field2);

        let sig_fields = discover(&resolver, &catalog);

        assert_eq!(sig_fields.len(), 2);

        let sig1 = sig_fields.iter().find(|s| s.full_name == "employer_sig").unwrap();
        assert_eq!(sig1.full_name, "employer_sig");
        assert!(sig1.v_ref.is_none());

        let sig2 = sig_fields.iter().find(|s| s.full_name == "employee_sig").unwrap();
        assert_eq!(sig2.full_name, "employee_sig");
        assert!(sig2.v_ref.is_none());
    }

    #[test]
    fn test_discover_non_signature_fields_excluded() {
        let (text_field_ref, text_field) = make_field_dict_with_id(
            1,
            Some("Tx"),
            Some("employee_name"),
            None,
            None,
            None,
        );

        let (sig_field_ref, sig_field) = make_field_dict_with_id(
            2,
            Some("Sig"),
            Some("employee_sig"),
            None,
            None,
            None,
        );

        let fields = vec![
            PdfObject::Ref(text_field_ref),
            PdfObject::Ref(sig_field_ref),
        ];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(text_field_ref, text_field);
        resolver.cache_object(sig_field_ref, sig_field);

        let sig_fields = discover(&resolver, &catalog);

        assert_eq!(sig_fields.len(), 1);
        assert_eq!(sig_fields[0].full_name, "employee_sig");
    }

    #[test]
    fn test_discover_nested_signature_inherits_ft() {
        // Parent field with /FT /Sig and /Kids array
        let (kid_field_ref, kid_field) = make_field_dict_with_id(
            2,
            None, // No /FT on child - inherits from parent
            Some("sub_sig"),
            None,
            None,
            None,
        );

        let (parent_field_ref, parent_field) = make_field_dict_with_id(
            1,
            Some("Sig"), // Parent has /FT /Sig
            Some("parent_sig"),
            None,
            None,
            Some(vec![kid_field_ref]),
        );

        let fields = vec![PdfObject::Ref(parent_field_ref)];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(parent_field_ref, parent_field);
        resolver.cache_object(kid_field_ref, kid_field);

        let sig_fields = discover(&resolver, &catalog);

        // Should find the nested signature field
        assert_eq!(sig_fields.len(), 1);
        assert_eq!(sig_fields[0].full_name, "parent_sig.sub_sig");
    }

    #[test]
    fn test_discover_nested_mixed_field_types() {
        // Parent with /FT /Sig has two kids: one inherits, one overrides
        let (kid1_ref, kid1) = make_field_dict_with_id(
            2,
            None, // Inherits /FT /Sig from parent
            Some("kid1"),
            None,
            None,
            None,
        );

        let (kid2_ref, kid2) = make_field_dict_with_id(
            3,
            Some("Tx"), // Overrides to text field
            Some("kid2"),
            None,
            None,
            None,
        );

        let (parent_ref, parent) = make_field_dict_with_id(
            1,
            Some("Sig"),
            Some("parent"),
            None,
            None,
            Some(vec![kid1_ref, kid2_ref]),
        );

        let fields = vec![PdfObject::Ref(parent_ref)];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(parent_ref, parent);
        resolver.cache_object(kid1_ref, kid1);
        resolver.cache_object(kid2_ref, kid2);

        let sig_fields = discover(&resolver, &catalog);

        // Only kid1 should be a signature (inherits /FT /Sig)
        assert_eq!(sig_fields.len(), 1);
        assert_eq!(sig_fields[0].full_name, "parent.kid1");
    }

    #[test]
    fn test_discover_with_rect() {
        let (field_ref, field) = make_field_dict_with_id(
            1,
            Some("Sig"),
            Some("signature"),
            None,
            Some([100.0, 200.0, 300.0, 400.0]),
            None,
        );

        let fields = vec![PdfObject::Ref(field_ref)];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(field_ref, field);

        let sig_fields = discover(&resolver, &catalog);

        assert_eq!(sig_fields.len(), 1);
        assert_eq!(sig_fields[0].rect, Some([100.0, 200.0, 300.0, 400.0]));
    }

    #[test]
    fn test_discover_with_v_ref() {
        let v_ref = ObjRef::new(999, 0);

        let (field_ref, field) = make_field_dict_with_id(
            1,
            Some("Sig"),
            Some("signature"),
            Some(v_ref),
            None,
            None,
        );

        let fields = vec![PdfObject::Ref(field_ref)];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(field_ref, field);

        let sig_fields = discover(&resolver, &catalog);

        assert_eq!(sig_fields.len(), 1);
        assert_eq!(sig_fields[0].v_ref, Some(v_ref));
    }

    #[test]
    fn test_walk_acroform_fields_reusable() {
        // Verify that walk_acroform_fields returns all field types
        let (text_ref, text) = make_field_dict_with_id(
            1,
            Some("Tx"),
            Some("text_field"),
            None,
            None,
            None,
        );

        let (sig_ref, sig) = make_field_dict_with_id(
            2,
            Some("Sig"),
            Some("sig_field"),
            None,
            None,
            None,
        );

        let fields = vec![
            PdfObject::Ref(text_ref),
            PdfObject::Ref(sig_ref),
        ];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(text_ref, text);
        resolver.cache_object(sig_ref, sig);

        let all_fields = walk_acroform_fields(&resolver, &catalog);

        assert_eq!(all_fields.len(), 2);

        // Verify field types are preserved
        let text_field = all_fields.iter().find(|f| f.full_name == "text_field").unwrap();
        assert_eq!(text_field.field_type.as_deref(), Some("Tx"));

        let sig_field = all_fields.iter().find(|f| f.full_name == "sig_field").unwrap();
        assert_eq!(sig_field.field_type.as_deref(), Some("Sig"));
    }
}
