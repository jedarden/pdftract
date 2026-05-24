//! AcroForm field extraction and walking.
//!
//! This module implements Phase 7.4.1: AcroForm field walker. It walks the
//! AcroForm `/Fields` array recursively, builds dot-joined field names, resolves
//! field type inheritance, and emits a flat `Vec<AcroFormField>` with complete
//! field metadata.
//!
//! ## Architecture
//!
//! - **Discovery** (7.4.1): Walk `/Fields` recursively, filter to all field types
//! - **Type-specific extraction** (7.4.2): Extract type-specific values (Tx, Btn, Ch)
//! - **XFA parsing** (7.4.3): Parse XFA XML streams for hybrid forms
//! - **Combiner** (7.4.4): Merge AcroForm and XFA fields with XFA-wins precedence
//!
//! ## Reuse
//!
//! The `walk_acroform_fields` function is designed for reuse by Phase 7.3 (signature
//! discovery), which filters its output to `/FT /Sig` fields only.

pub mod combiner;
pub mod value_button;
pub mod xfa;

pub use xfa::{extract_xfa_fields, XfaField};

pub use combiner::{combine, ChoiceValue, FormFieldValue};
pub use value_button::{extract_button_value, ButtonKind, ButtonValue};

/// Convert an AcroFormField to FormFieldValue.
///
/// This function implements Phase 7.4.2 type-specific extraction: it converts
/// the raw AcroFormField (from Phase 7.4.1) into a type-safe FormFieldValue
/// that can be combined with XFA fields and serialized to JSON.
///
/// # Arguments
///
/// * `field` - The AcroFormField to convert
///
/// # Returns
///
/// A `FormFieldValue` variant matching the field's type, with values extracted
/// from the field's /V, /DV, /Ff, and /Opt entries.
pub fn acro_field_to_value(field: &AcroFormField) -> FormFieldValue {
    match field.field_type {
        AcroFieldType::Tx => {
            // Text field: extract string value from /V
            let value = field
                .value
                .as_ref()
                .and_then(|v| v.as_string())
                .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());
            let default = field
                .default
                .as_ref()
                .and_then(|v| v.as_string())
                .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());
            let multiline = field.is_multi_line();

            // Extract /MaxLen if present (would need to be added to AcroFormField)
            let max_length = None; // TODO: extract from field dict if needed

            FormFieldValue::Text {
                value,
                default,
                multiline,
                max_length,
            }
        }

        AcroFieldType::Btn => {
            // Button field: use extract_button_value
            let button_value = extract_button_value(field.value.as_ref(), field.flags);

            // Extract default selected state from /DV
            let default_selected = field
                .default
                .as_ref()
                .and_then(|v| v.as_name())
                .map(|name| {
                    let name_str: &str = &name;
                    name_str != "Off"
                });

            FormFieldValue::Button {
                kind: button_value.kind,
                selected: button_value.selected,
                state_name: button_value.state_name,
                default_selected,
                pushbutton: button_value.pushbutton,
                radio: button_value.radio,
            }
        }

        AcroFieldType::Ch => {
            // Choice field: extract selected value(s) and options
            let (value, default) = extract_choice_values(&field.value, &field.default);
            let options = field.opt.clone().unwrap_or_default();

            // Extract combo and multi_select flags from /Ff
            const COMBO_FLAG: u32 = 1 << 17; // Bit 18
            const MULTI_SELECT_FLAG: u32 = 1 << 20; // Bit 21
            let is_combo = (field.flags & COMBO_FLAG) != 0;
            let is_multi_select = (field.flags & MULTI_SELECT_FLAG) != 0;

            FormFieldValue::Choice {
                value,
                default,
                options,
                is_combo,
                is_multi_select,
            }
        }

        AcroFieldType::Sig => {
            // Signature field: extract reference number
            let ref_num = field.value.as_ref().and_then(|v| match v {
                PdfObject::Ref(ref_) => Some(ref_.object),
                _ => None,
            });

            FormFieldValue::Signature {
                signature_ref: ref_num,
            }
        }

        AcroFieldType::Other => {
            // Unknown field type: treat as text
            let value = field
                .value
                .as_ref()
                .and_then(|v| v.as_string())
                .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());
            let default = field
                .default
                .as_ref()
                .and_then(|v| v.as_string())
                .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());

            FormFieldValue::Text {
                value,
                default,
                multiline: false,
                max_length: None,
            }
        }
    }
}

/// Extract choice field values from /V and /DV entries.
///
/// Choice fields can have either a single selected value or multiple
/// selected values (for multi-select list boxes).
fn extract_choice_values(
    value: &Option<PdfObject>,
    default: &Option<PdfObject>,
) -> (ChoiceValue, Option<ChoiceValue>) {
    // Extract current value
    let current = match value {
        Some(PdfObject::String(s)) => String::from_utf8(s.to_vec())
            .ok()
            .map(|v| ChoiceValue::Single(v))
            .unwrap_or_else(|| ChoiceValue::Single(String::new())),
        Some(PdfObject::Array(arr)) => {
            let values: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_string())
                .filter_map(|bytes| String::from_utf8(bytes.to_vec()).ok())
                .collect();
            if values.is_empty() {
                ChoiceValue::Single(String::new())
            } else if values.len() == 1 {
                ChoiceValue::Single(values.into_iter().next().unwrap())
            } else {
                ChoiceValue::Multiple(values)
            }
        }
        _ => ChoiceValue::Single(String::new()),
    };

    // Extract default value
    let default_val = match default {
        Some(PdfObject::String(s)) => String::from_utf8(s.to_vec())
            .ok()
            .map(|v| ChoiceValue::Single(v)),
        Some(PdfObject::Array(arr)) => {
            let values: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_string())
                .filter_map(|bytes| String::from_utf8(bytes.to_vec()).ok())
                .collect();
            if values.is_empty() {
                None
            } else if values.len() == 1 {
                Some(ChoiceValue::Single(values.into_iter().next().unwrap()))
            } else {
                Some(ChoiceValue::Multiple(values))
            }
        }
        _ => None,
    };

    (current, default_val)
}

use crate::diagnostics::{DiagCode, Diagnostic};
use crate::parser::catalog::Catalog;
use crate::parser::object::{intern, ObjRef, PdfDict, PdfObject};
use crate::parser::pages::PageDict;
use crate::parser::xref::XrefResolver;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Result type for form operations.
pub type Result<T> = std::result::Result<T, Vec<Diagnostic>>;

/// AcroForm field type (/FT entry).
///
/// Represents the type of an interactive form field. Per PDF 1.7 spec section 12.7.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcroFieldType {
    /// Text field (/FT /Tx) - single-line or multi-line text input
    Tx,
    /// Button field (/FT /Btn) - pushbuttons, checkboxes, radio buttons
    Btn,
    /// Choice field (/FT /Ch) - scrollable list boxes or drop-down combo boxes
    Ch,
    /// Signature field (/FT /Sig) - digital signature field
    Sig,
    /// Unknown or other field type
    Other,
}

impl AcroFieldType {
    /// Parse a field type from a name string.
    fn from_name(name: &str) -> Self {
        match name {
            "Tx" => AcroFieldType::Tx,
            "Btn" => AcroFieldType::Btn,
            "Ch" => AcroFieldType::Ch,
            "Sig" => AcroFieldType::Sig,
            _ => AcroFieldType::Other,
        }
    }

    /// Get the string representation of this field type.
    pub fn as_str(&self) -> &'static str {
        match self {
            AcroFieldType::Tx => "Tx",
            AcroFieldType::Btn => "Btn",
            AcroFieldType::Ch => "Ch",
            AcroFieldType::Sig => "Sig",
            AcroFieldType::Other => "Other",
        }
    }
}

/// An AcroForm field with complete metadata.
///
/// Represents a single field from the AcroForm `/Fields` hierarchy, including
/// its absolute (dot-joined) name, type, value, default value, flags, bounding
/// rectangle, page index, and choice options (for Ch fields).
#[derive(Debug, Clone, PartialEq)]
pub struct AcroFormField {
    /// Absolute (dot-joined) field name, e.g., "employer_signature" or "form.employee_sig"
    pub full_name: String,

    /// Field type (Tx, Btn, Ch, Sig, or Other)
    pub field_type: AcroFieldType,

    /// Current value (/V entry) - type varies by field_type
    ///
    /// - Tx: String or stream
    /// - Btn: Name (the selected appearance state)
    /// - Ch: String (selected option) or array of strings (multi-select)
    /// - Sig: Signature dictionary (indirect ref)
    pub value: Option<PdfObject>,

    /// Default value (/DV entry) - same type as /V
    pub default: Option<PdfObject>,

    /// Field flags (/Ff entry) - bitfield controlling field behavior
    ///
    /// Common flags (bit positions):
    /// - 1: ReadOnly (field cannot be modified)
    /// - 2: Required (field must have a value at submit time)
    /// - 3: NoExport (field must not be exported)
    /// - 13: MultiLine (Tx field can have multiple lines)
    /// - 14: Password (Tx field displays asterisks)
    /// - 15: FileSelect (Tx field is for file selection)
    /// - 16: DoNotSpellCheck (Tx field should not be spell-checked)
    /// - 17: DoNotScroll (Tx field does not scroll)
    /// - 18: Comb (Tx field comb formatting)
    /// - 19: RichText (Tx field uses rich text value)
    /// - 24: NoToggleToOff (Btn field radio button behavior)
    /// - 25: Radio (Btn field is a radio button)
    /// - 26: Pushbutton (Btn field is a pushbutton)
    pub flags: u32,

    /// Bounding rectangle for the field's widget annotation
    ///
    /// Format: [x0, y0, x1, y1] in PDF user-space points.
    /// None if the field has no visual appearance.
    pub rect: Option<[f32; 4]>,

    /// Index of the page containing this field's widget annotation
    ///
    /// None if the field has no widget on any page (form-only field).
    pub page_index: Option<usize>,

    /// Choice field options (/Opt array) - present only for Ch fields
    ///
    /// Each element is a (export_value, display_name) pair. For simple choice
    /// fields without explicit export values, both entries are the same string.
    pub opt: Option<Vec<(String, String)>>,
}

impl AcroFormField {
    /// Check if this field is read-only (bit 1 of flags).
    pub fn is_read_only(&self) -> bool {
        (self.flags & 1) != 0
    }

    /// Check if this field is required (bit 2 of flags).
    pub fn is_required(&self) -> bool {
        (self.flags & 2) != 0
    }

    /// Check if this field is multi-line (bit 13 of flags, Tx only).
    pub fn is_multi_line(&self) -> bool {
        (self.flags & (1 << 12)) != 0
    }

    /// Check if this field is a password field (bit 14 of flags, Tx only).
    pub fn is_password(&self) -> bool {
        (self.flags & (1 << 13)) != 0
    }

    /// Check if this field is a radio button (bit 25 of flags, Btn only).
    pub fn is_radio(&self) -> bool {
        (self.flags & (1 << 24)) != 0
    }

    /// Check if this field is a pushbutton (bit 26 of flags, Btn only).
    pub fn is_pushbutton(&self) -> bool {
        (self.flags & (1 << 25)) != 0
    }

    /// Get the is_checked state for a checkbox/radio button field.
    ///
    /// Returns Some(true) if checked, Some(false) if unchecked, None if not applicable.
    /// For Btn fields, the /V entry is the appearance state name; we compare it
    /// against the widget's /AP dictionary to determine checked state.
    pub fn is_checked(&self) -> Option<bool> {
        if self.field_type != AcroFieldType::Btn {
            return None;
        }

        // The value is a Name indicating the selected appearance state
        // Off/Yes are common states; "Off" means unchecked, anything else means checked
        match &self.value {
            Some(PdfObject::Name(name)) if name.as_ref() == "Off" => Some(false),
            Some(PdfObject::Name(_)) => Some(true),
            _ => None,
        }
    }
}

/// Walk the AcroForm `/Fields` array recursively and collect all fields.
///
/// This is the main entry point for Phase 7.4.1. It performs DFS traversal of
/// the `/Kids` hierarchy, resolves `/FT` inheritance, constructs absolute field
/// names by dot-joining partial names, and resolves widget annotations to page
/// indices.
///
/// # Arguments
///
/// * `resolver` - Xref resolver for dereferencing indirect objects
/// * `catalog` - Document catalog containing the AcroForm reference
/// * `pages` - Optional slice of PageDict for page_index resolution
///
/// # Returns
///
/// A `Vec<AcroFormField>` containing all discovered fields (all types, not just signatures).
/// Returns empty vec if the PDF has no AcroForm or no fields.
///
/// # Behavior
///
/// - If `/AcroForm` is absent, returns empty vec (not an error)
/// - If `/Fields` is absent or empty, returns empty vec
/// - Descends recursively via `/Kids` arrays
/// - Resolves `/FT`, `/V`, `/DV`, `/Ff` inheritance from parent to child fields
/// - Constructs absolute names by joining `/T` values with "."
/// - Skips empty `/T` segments (e.g., "" or absent)
/// - Resolves widget annotations to page indices when `pages` is provided
/// - Detects cycles in `/Kids` and emits diagnostics
/// - Handles name collisions by keeping the last field (with diagnostic)
/// - Emits diagnostics for malformed structures but continues
///
/// # Example
///
/// ```ignore
/// use pdftract_core::forms::{walk_acroform_fields, AcroFieldType};
///
/// let fields = walk_acroform_fields(&resolver, &catalog, Some(&pages));
/// for field in fields {
///     println!("Field: {} (type: {})", field.full_name, field.field_type.as_str());
///     if let Some(idx) = field.page_index {
///         println!("  Page: {}", idx);
///     }
/// }
/// ```
pub fn walk_acroform_fields(
    resolver: &XrefResolver,
    catalog: &Catalog,
    pages: Option<&[PageDict]>,
) -> Vec<AcroFormField> {
    let mut fields = Vec::new();
    let mut diagnostics = Vec::new();
    let mut visited = HashSet::new();

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
                format!(
                    "/AcroForm is not a dictionary (type: {})",
                    acroform.type_name()
                ),
            ));
            return fields;
        }
    };

    // /Fields is an array of indirect references to field dictionaries
    let fields_array = match acroform_dict.get("Fields").and_then(|o| o.as_array()) {
        Some(arr) => arr,
        None => return fields, // No /Fields means no form fields
    };

    // Build a map of field_ref -> page_index for widget resolution
    let page_map = if let Some(pages_slice) = pages {
        build_widget_page_map(resolver, pages_slice, &mut diagnostics)
    } else {
        HashMap::new()
    };

    // Track field names for collision detection
    let mut field_names = HashSet::new();

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
            None, // No parent /FT at root
            None, // No parent /V at root
            None, // No parent /DV at root
            0,    // No parent flags at root
            &page_map,
            &mut visited,
            &mut field_names,
            &mut diagnostics,
        );
    }

    fields
}

/// Build a map of field_ref -> page_index by searching page /Annots arrays.
///
/// For each page, walk its `/Annots` array and collect widget annotations
/// (those with `/Subtype /Widget`). Map the annotation's indirect reference
/// (or its `/Parent` field reference) to the page index.
fn build_widget_page_map(
    resolver: &XrefResolver,
    pages: &[PageDict],
    diagnostics: &mut Vec<Diagnostic>,
) -> HashMap<ObjRef, usize> {
    let mut page_map = HashMap::new();

    for (page_idx, page) in pages.iter().enumerate() {
        for annot_ref in &page.annots {
            // Resolve the annotation dictionary
            let annot_obj = match resolver.resolve(*annot_ref) {
                Ok(obj) => obj,
                Err(_) => continue,
            };

            let annot_dict = match annot_obj.as_dict() {
                Some(d) => d,
                None => continue,
            };

            // Check if this is a widget annotation
            let subtype = annot_dict.get("Subtype").and_then(|o| o.as_name());
            if subtype.map(|s| s.as_ref()) != Some("Widget") {
                continue;
            }

            // Widget annotations can reference the field via:
            // 1. Their own indirect reference (if they are the field dict)
            // 2. A /Parent entry pointing to the field dict
            page_map.insert(*annot_ref, page_idx);

            if let Some(PdfObject::Ref(parent_ref)) = annot_dict.get("Parent") {
                page_map.insert(*parent_ref, page_idx);
            }
        }
    }

    page_map
}

/// Recursively walk a field dictionary and its /Kids.
///
/// This helper function performs DFS traversal of the field hierarchy,
/// building absolute field names and tracking inheritance.
///
/// # Arguments
///
/// * `resolver` - Xref resolver
/// * `field_ref` - Indirect reference to the current field dictionary
/// * `fields` - Output accumulator for discovered fields
/// * `parent_name` - Accumulated absolute name from parent path
/// * `parent_ft` - Inherited field type from parent (/FT value)
/// * `parent_v` - Inherited value from parent (/V value)
/// * `parent_dv` - Inherited default value from parent (/DV value)
/// * `parent_ff` - Inherited flags from parent (/Ff value)
/// * `page_map` - Map of field_ref -> page_index for widget resolution
/// * `visited` - Set of visited field refs for cycle detection
/// * `field_names` - Set of emitted field names for collision detection
/// * `diagnostics` - Diagnostic accumulator
fn walk_field_recursive(
    resolver: &XrefResolver,
    field_ref: ObjRef,
    fields: &mut Vec<AcroFormField>,
    parent_name: String,
    parent_ft: Option<AcroFieldType>,
    parent_v: Option<&PdfObject>,
    parent_dv: Option<&PdfObject>,
    parent_ff: u32,
    page_map: &HashMap<ObjRef, usize>,
    visited: &mut HashSet<ObjRef>,
    field_names: &mut HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Cycle detection
    if !visited.insert(field_ref) {
        diagnostics.push(Diagnostic::with_dynamic_no_offset(
            DiagCode::StructUnexpectedEof,
            format!(
                "Cycle detected in /Kids: field {} already visited",
                field_ref
            ),
        ));
        return;
    }

    // Resolve the field dictionary
    let field_obj = match resolver.resolve(field_ref) {
        Ok(obj) => obj,
        Err(_) => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!("Failed to resolve field reference {}", field_ref),
            ));
            visited.remove(&field_ref);
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
            visited.remove(&field_ref);
            return;
        }
    };

    // Extract /T (partial name) for building absolute name
    let partial_name = field_dict
        .get("T")
        .and_then(|o| o.as_string())
        .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
        .filter(|s| !s.is_empty()); // Skip empty /T segments

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
    let field_type = field_dict
        .get("FT")
        .and_then(|o| o.as_name())
        .map(|n| AcroFieldType::from_name(n.as_ref()))
        .or(parent_ft);

    // Extract /V (current value) - inherits from parent if absent
    let value = field_dict.get("V").cloned().or_else(|| parent_v.cloned());

    // Extract /DV (default value) - inherits from parent if absent
    let default = field_dict.get("DV").cloned().or_else(|| parent_dv.cloned());

    // Extract /Ff (flags) - inherits from parent if absent (default 0)
    let flags = field_dict
        .get("Ff")
        .and_then(|o| o.as_int())
        .map(|i| i as u32)
        .unwrap_or(parent_ff);

    // Extract /Rect (bounding rectangle) if present
    let rect = field_dict
        .get("Rect")
        .and_then(|o| o.as_array())
        .and_then(|arr| {
            if arr.len() == 4 {
                let coords: Vec<Option<f64>> = arr
                    .iter()
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

    // Resolve page_index from the widget map
    let page_index = page_map.get(&field_ref).copied();

    // Extract /Opt (choice options) for Ch fields
    let opt = if field_type == Some(AcroFieldType::Ch) {
        extract_choice_options(field_dict)
    } else {
        None
    };

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
                field_type,       // Pass /FT to children for inheritance
                value.as_ref(),   // Pass /V to children for inheritance
                default.as_ref(), // Pass /DV to children for inheritance
                flags,            // Pass /Ff to children for inheritance
                page_map,
                visited,
                field_names,
                diagnostics,
            );
        }
    } else {
        // This is a leaf field - emit it
        // Check for name collision
        if !full_name.is_empty() && !field_names.insert(full_name.clone()) {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!(
                    "Field name collision: '{}' already emitted (keeping last)",
                    full_name
                ),
            ));
        }

        fields.push(AcroFormField {
            full_name,
            field_type: field_type.unwrap_or(AcroFieldType::Other),
            value,
            default,
            flags,
            rect,
            page_index,
            opt,
        });
    }

    visited.remove(&field_ref);
}

/// Extract choice field options from a field dictionary.
///
/// The /Opt array contains option values for choice fields. Each element can be:
/// - A string (both export value and display name are the same)
/// - An array of two strings [export_value, display_name]
fn extract_choice_options(field_dict: &PdfDict) -> Option<Vec<(String, String)>> {
    let opt_array = field_dict.get("Opt")?.as_array()?;

    let mut options = Vec::new();
    for opt_obj in opt_array.iter() {
        if let Some(str_bytes) = opt_obj.as_string() {
            // Single string: export and display are the same
            if let Ok(s) = String::from_utf8(str_bytes.to_vec()) {
                options.push((s.clone(), s));
            }
        } else if let Some(arr) = opt_obj.as_array() {
            // Array of [export_value, display_name]
            let export = arr
                .get(0)
                .and_then(|o| o.as_string())
                .and_then(|b| String::from_utf8(b.to_vec()).ok());
            let display = arr
                .get(1)
                .and_then(|o| o.as_string())
                .and_then(|b| String::from_utf8(b.to_vec()).ok());

            if let (Some(export_val), Some(display_val)) = (export, display) {
                options.push((export_val, display_val));
            }
        }
    }

    if options.is_empty() {
        None
    } else {
        Some(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::{intern, PdfObject};
    use std::sync::Arc;

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
        v: Option<PdfObject>,
        dv: Option<PdfObject>,
        ff: Option<u32>,
        rect: Option<[f32; 4]>,
        kids: Option<Vec<ObjRef>>,
        opt: Option<Vec<PdfObject>>,
    ) -> (ObjRef, PdfObject) {
        let mut dict = indexmap::IndexMap::new();

        if let Some(ft_val) = ft {
            dict.insert(intern("FT"), PdfObject::Name(intern(ft_val)));
        }

        if let Some(t_val) = t {
            dict.insert(
                intern("T"),
                PdfObject::String(Box::new(t_val.as_bytes().to_vec())),
            );
        }

        if let Some(v_val) = v {
            dict.insert(intern("V"), v_val);
        }

        if let Some(dv_val) = dv {
            dict.insert(intern("DV"), dv_val);
        }

        if let Some(ff_val) = ff {
            dict.insert(intern("Ff"), PdfObject::Integer(ff_val as i64));
        }

        if let Some(rect_val) = rect {
            let rect_array: Vec<PdfObject> = rect_val
                .iter()
                .map(|&c| PdfObject::Real(c as f64))
                .collect();
            dict.insert(intern("Rect"), PdfObject::Array(Box::new(rect_array)));
        }

        if let Some(kids_refs) = kids {
            let kids_array: Vec<PdfObject> = kids_refs.iter().map(|&r| PdfObject::Ref(r)).collect();
            dict.insert(intern("Kids"), PdfObject::Array(Box::new(kids_array)));
        }

        if let Some(opt_array) = opt {
            dict.insert(intern("Opt"), PdfObject::Array(Box::new(opt_array)));
        }

        let field_ref = ObjRef::new(100 + id, 0);
        (field_ref, PdfObject::Dict(Box::new(dict)))
    }

    #[test]
    fn test_walk_acroform_fields_no_acroform() {
        let catalog = Catalog::new(ObjRef::new(1, 0));
        let resolver = XrefResolver::new();

        let fields = walk_acroform_fields(&resolver, &catalog, None);

        assert!(fields.is_empty());
    }

    #[test]
    fn test_walk_acroform_fields_no_fields_array() {
        let mut resolver = XrefResolver::new();

        let acroform_ref = ObjRef::new(10, 0);
        let acroform_dict = indexmap::IndexMap::new();
        resolver.cache_object(acroform_ref, PdfObject::Dict(Box::new(acroform_dict)));

        let mut catalog = Catalog::new(ObjRef::new(1, 0));
        catalog.acroform_ref = Some(acroform_ref);

        let fields = walk_acroform_fields(&resolver, &catalog, None);

        assert!(fields.is_empty());
    }

    #[test]
    fn test_walk_acroform_fields_three_flat_fields() {
        let (field1_ref, field1) = make_field_dict_with_id(
            1,
            Some("Tx"),
            Some("employee_name"),
            Some(PdfObject::String(Box::new(b"John Doe".to_vec()))),
            None,
            Some(2), // Required flag
            None,
            None,
            None,
        );

        let (field2_ref, field2) = make_field_dict_with_id(
            2,
            Some("Tx"),
            Some("employee_title"),
            None,
            None,
            None,
            None,
            None,
            None,
        );

        let (field3_ref, field3) = make_field_dict_with_id(
            3,
            Some("Btn"),
            Some("is_manager"),
            Some(PdfObject::Name(intern("Yes"))),
            None,
            None,
            None,
            None,
            None,
        );

        let fields = vec![
            PdfObject::Ref(field1_ref),
            PdfObject::Ref(field2_ref),
            PdfObject::Ref(field3_ref),
        ];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(field1_ref, field1);
        resolver.cache_object(field2_ref, field2);
        resolver.cache_object(field3_ref, field3);

        let result = walk_acroform_fields(&resolver, &catalog, None);

        assert_eq!(result.len(), 3);

        let name_field = result
            .iter()
            .find(|f| f.full_name == "employee_name")
            .unwrap();
        assert_eq!(name_field.field_type, AcroFieldType::Tx);
        assert!(name_field.is_required());
        assert!(!name_field.is_read_only());

        let title_field = result
            .iter()
            .find(|f| f.full_name == "employee_title")
            .unwrap();
        assert_eq!(title_field.field_type, AcroFieldType::Tx);
        assert!(!title_field.is_required());

        let btn_field = result.iter().find(|f| f.full_name == "is_manager").unwrap();
        assert_eq!(btn_field.field_type, AcroFieldType::Btn);
        assert_eq!(btn_field.is_checked(), Some(true));
    }

    #[test]
    fn test_walk_acroform_fields_nested_two_levels() {
        // Create nested structure: parent.child.grandchild
        let (grandchild_ref, grandchild) = make_field_dict_with_id(
            3,
            None, // Inherits /FT from parent
            Some("grandchild"),
            Some(PdfObject::String(Box::new(b"value".to_vec()))),
            None,
            None,
            None,
            None,
            None,
        );

        let (child_ref, child) = make_field_dict_with_id(
            2,
            Some("Tx"),
            Some("child"),
            None,
            None,
            None,
            None,
            Some(vec![grandchild_ref]),
            None,
        );

        let (parent_ref, parent) = make_field_dict_with_id(
            1,
            Some("Tx"),
            Some("parent"),
            None,
            None,
            None,
            None,
            Some(vec![child_ref]),
            None,
        );

        let fields = vec![PdfObject::Ref(parent_ref)];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(parent_ref, parent);
        resolver.cache_object(child_ref, child);
        resolver.cache_object(grandchild_ref, grandchild);

        let result = walk_acroform_fields(&resolver, &catalog, None);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].full_name, "parent.child.grandchild");
        assert_eq!(result[0].field_type, AcroFieldType::Tx);
        assert_eq!(
            result[0].value,
            Some(PdfObject::String(Box::new(b"value".to_vec())))
        );
    }

    #[test]
    fn test_walk_acroform_fields_ft_inheritance() {
        // Parent with /FT /Tx, child without /FT inherits it
        let (child_ref, child) = make_field_dict_with_id(
            2,
            None, // No /FT - should inherit from parent
            Some("child"),
            Some(PdfObject::String(Box::new(b"inherited".to_vec()))),
            None,
            None,
            None,
            None,
            None,
        );

        let (parent_ref, parent) = make_field_dict_with_id(
            1,
            Some("Tx"), // Parent has /FT /Tx
            Some("parent"),
            None,
            None,
            None,
            None,
            Some(vec![child_ref]),
            None,
        );

        let fields = vec![PdfObject::Ref(parent_ref)];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(parent_ref, parent);
        resolver.cache_object(child_ref, child);

        let result = walk_acroform_fields(&resolver, &catalog, None);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].full_name, "parent.child");
        assert_eq!(result[0].field_type, AcroFieldType::Tx); // Inherited
    }

    #[test]
    fn test_walk_acroform_fields_child_overrides_ft() {
        // Parent with /FT /Tx, child overrides to /FT /Btn
        let (child_ref, child) = make_field_dict_with_id(
            2,
            Some("Btn"), // Overrides parent's /FT
            Some("checkbox"),
            Some(PdfObject::Name(intern("Yes"))),
            None,
            None,
            None,
            None,
            None,
        );

        let (parent_ref, parent) = make_field_dict_with_id(
            1,
            Some("Tx"), // Parent has /FT /Tx
            Some("parent"),
            None,
            None,
            None,
            None,
            Some(vec![child_ref]),
            None,
        );

        let fields = vec![PdfObject::Ref(parent_ref)];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(parent_ref, parent);
        resolver.cache_object(child_ref, child);

        let result = walk_acroform_fields(&resolver, &catalog, None);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].full_name, "parent.checkbox");
        assert_eq!(result[0].field_type, AcroFieldType::Btn); // Overridden
    }

    #[test]
    fn test_walk_acroform_fields_flags_inheritance() {
        // Parent with /Ff 3 (ReadOnly + Required), child inherits
        let (child_ref, child) = make_field_dict_with_id(
            2,
            Some("Tx"),
            Some("child"),
            None,
            None,
            None,
            None,
            None,
            None,
        );

        let (parent_ref, parent) = make_field_dict_with_id(
            1,
            Some("Tx"),
            Some("parent"),
            None,
            None,
            Some(3), // bit 0 (ReadOnly = 1) + bit 1 (Required = 2) = 3
            None,
            Some(vec![child_ref]),
            None,
        );

        let fields = vec![PdfObject::Ref(parent_ref)];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(parent_ref, parent);
        resolver.cache_object(child_ref, child);

        let result = walk_acroform_fields(&resolver, &catalog, None);

        assert_eq!(result.len(), 1);
        assert!(result[0].is_read_only()); // bit 0 set
        assert!(result[0].is_required()); // bit 1 set
    }

    #[test]
    fn test_walk_acroform_fields_empty_t_segment_skipped() {
        // Field with empty /T should not add extra dot
        let (child_ref, child) = make_field_dict_with_id(
            2,
            Some("Tx"),
            Some("child"),
            None,
            None,
            None,
            None,
            None,
            None,
        );

        let (parent_ref, parent) = make_field_dict_with_id(
            1,
            Some("Tx"),
            Some(""), // Empty /T
            None,
            None,
            None,
            None,
            Some(vec![child_ref]),
            None,
        );

        let fields = vec![PdfObject::Ref(parent_ref)];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(parent_ref, parent);
        resolver.cache_object(child_ref, child);

        let result = walk_acroform_fields(&resolver, &catalog, None);

        assert_eq!(result.len(), 1);
        // Should be "child" not ".child"
        assert_eq!(result[0].full_name, "child");
    }

    #[test]
    fn test_walk_acroform_fields_choice_field_options() {
        // Ch field with /Opt array
        let mut opt_array = Vec::new();
        // Simple string option
        opt_array.push(PdfObject::String(Box::new(b"Option1".to_vec())));
        // Array option [export, display]
        opt_array.push(PdfObject::Array(Box::new(vec![
            PdfObject::String(Box::new(b"value2".to_vec())),
            PdfObject::String(Box::new(b"Option 2".to_vec())),
        ])));

        let (field_ref, field) = make_field_dict_with_id(
            1,
            Some("Ch"),
            Some("dropdown"),
            Some(PdfObject::String(Box::new(b"Option1".to_vec()))),
            None,
            None,
            None,
            None,
            Some(opt_array),
        );

        let fields = vec![PdfObject::Ref(field_ref)];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(field_ref, field);

        let result = walk_acroform_fields(&resolver, &catalog, None);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].field_type, AcroFieldType::Ch);

        let opt = result[0].opt.as_ref().unwrap();
        assert_eq!(opt.len(), 2);
        assert_eq!(opt[0], ("Option1".to_string(), "Option1".to_string()));
        assert_eq!(opt[1], ("value2".to_string(), "Option 2".to_string()));
    }

    #[test]
    fn test_walk_acroform_fields_all_field_types() {
        // Create one of each field type
        let (tx_ref, tx) = make_field_dict_with_id(
            1,
            Some("Tx"),
            Some("text_field"),
            Some(PdfObject::String(Box::new(b"text".to_vec()))),
            None,
            None,
            None,
            None,
            None,
        );

        let (btn_ref, btn) = make_field_dict_with_id(
            2,
            Some("Btn"),
            Some("checkbox"),
            Some(PdfObject::Name(intern("Yes"))),
            None,
            None,
            None,
            None,
            None,
        );

        let (ch_ref, ch) = make_field_dict_with_id(
            3,
            Some("Ch"),
            Some("choice"),
            Some(PdfObject::String(Box::new(b"selection".to_vec()))),
            None,
            None,
            None,
            None,
            None,
        );

        let (sig_ref, sig) = make_field_dict_with_id(
            4,
            Some("Sig"),
            Some("signature"),
            None,
            None,
            None,
            None,
            None,
            None,
        );

        let fields = vec![
            PdfObject::Ref(tx_ref),
            PdfObject::Ref(btn_ref),
            PdfObject::Ref(ch_ref),
            PdfObject::Ref(sig_ref),
        ];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(tx_ref, tx);
        resolver.cache_object(btn_ref, btn);
        resolver.cache_object(ch_ref, ch);
        resolver.cache_object(sig_ref, sig);

        let result = walk_acroform_fields(&resolver, &catalog, None);

        assert_eq!(result.len(), 4);

        let types: Vec<_> = result.iter().map(|f| f.field_type).collect();
        assert!(types.contains(&AcroFieldType::Tx));
        assert!(types.contains(&AcroFieldType::Btn));
        assert!(types.contains(&AcroFieldType::Ch));
        assert!(types.contains(&AcroFieldType::Sig));
    }

    #[test]
    fn test_acro_field_type_from_name() {
        assert_eq!(AcroFieldType::from_name("Tx"), AcroFieldType::Tx);
        assert_eq!(AcroFieldType::from_name("Btn"), AcroFieldType::Btn);
        assert_eq!(AcroFieldType::from_name("Ch"), AcroFieldType::Ch);
        assert_eq!(AcroFieldType::from_name("Sig"), AcroFieldType::Sig);
        assert_eq!(AcroFieldType::from_name("Unknown"), AcroFieldType::Other);
    }

    #[test]
    fn test_acro_field_type_as_str() {
        assert_eq!(AcroFieldType::Tx.as_str(), "Tx");
        assert_eq!(AcroFieldType::Btn.as_str(), "Btn");
        assert_eq!(AcroFieldType::Ch.as_str(), "Ch");
        assert_eq!(AcroFieldType::Sig.as_str(), "Sig");
        assert_eq!(AcroFieldType::Other.as_str(), "Other");
    }

    #[test]
    fn test_acro_form_field_is_checked() {
        let mut field = AcroFormField {
            full_name: "checkbox".to_string(),
            field_type: AcroFieldType::Btn,
            value: Some(PdfObject::Name(intern("Yes"))),
            default: None,
            flags: 0,
            rect: None,
            page_index: None,
            opt: None,
        };

        assert_eq!(field.is_checked(), Some(true));

        field.value = Some(PdfObject::Name(intern("Off")));
        assert_eq!(field.is_checked(), Some(false));

        // Non-Btn field returns None
        field.field_type = AcroFieldType::Tx;
        assert_eq!(field.is_checked(), None);
    }

    #[test]
    fn test_acro_form_field_flag_accessors() {
        let mut field = AcroFormField {
            full_name: "test".to_string(),
            field_type: AcroFieldType::Tx,
            value: None,
            default: None,
            flags: 0,
            rect: None,
            page_index: None,
            opt: None,
        };

        assert!(!field.is_read_only());
        assert!(!field.is_required());
        assert!(!field.is_multi_line());
        assert!(!field.is_password());

        // Set ReadOnly (bit 1)
        field.flags |= 1;
        assert!(field.is_read_only());

        // Set Required (bit 2)
        field.flags |= 2;
        assert!(field.is_required());

        // Set MultiLine (bit 13)
        field.flags |= 1 << 12;
        assert!(field.is_multi_line());

        // Set Password (bit 14)
        field.flags |= 1 << 13;
        assert!(field.is_password());
    }

    #[test]
    fn test_acro_form_field_btn_flag_accessors() {
        let mut field = AcroFormField {
            full_name: "radio".to_string(),
            field_type: AcroFieldType::Btn,
            value: None,
            default: None,
            flags: 0,
            rect: None,
            page_index: None,
            opt: None,
        };

        assert!(!field.is_radio());
        assert!(!field.is_pushbutton());

        // Set Radio (bit 25)
        field.flags |= 1 << 24;
        assert!(field.is_radio());

        // Set Pushbutton (bit 26)
        field.flags |= 1 << 25;
        assert!(field.is_pushbutton());
    }
}
