//! AcroForm Ch (choice) field value extraction.
//!
//! This module implements Phase 7.4.2 Ch variant: extract choice field values
//! from /V, /DV, and /Opt entries with proper PDFDocEncoding/UTF-16BE BOM decoding.
//! Surfaces combo box vs list box via /Ff bit 18 (Combo) and multi-select via
//! /Ff bit 21 (MultiSelect).

use crate::forms::value_text::decode_pdf_string;
use crate::parser::object::PdfObject;
use std::fmt::{self, Display};

/// Choice kind classification.
///
/// Distinguishes between combo boxes and list boxes in choice fields.
/// Determined by the /Ff (field flags) entry in the field dictionary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChoiceKind {
    /// Combo box - dropdown with optional editable field.
    /// Identified by /Ff bit 18 (1 << 17 = 0x20000).
    Combo,

    /// List box - scrollable list of options.
    /// Default when Combo bit is not set.
    List,
}

impl Display for ChoiceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChoiceKind::Combo => write!(f, "combo"),
            ChoiceKind::List => write!(f, "list"),
        }
    }
}

/// Choice field value representation.
///
/// Represents either a single selected option or multiple selected options
/// (for multi-select list boxes).
#[derive(Debug, Clone, PartialEq)]
pub enum ChoiceValue {
    /// Single selected option.
    Single(Option<String>),
    /// Multiple selected options.
    Multiple(Vec<String>),
}

impl ChoiceValue {
    /// Check if this choice value is empty (no selection).
    ///
    /// # Examples
    ///
    /// ```
    /// use pdftract_core::forms::value_choice::ChoiceValue;
    ///
    /// assert!(ChoiceValue::Single(None).is_empty());
    /// assert!(ChoiceValue::Single(Some("".to_string())).is_empty());
    /// assert!(!ChoiceValue::Single(Some("text".to_string())).is_empty());
    /// assert!(ChoiceValue::Multiple(vec![]).is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        match self {
            ChoiceValue::Single(None) => true,
            ChoiceValue::Single(Some(s)) => s.is_empty(),
            ChoiceValue::Multiple(v) => v.is_empty(),
        }
    }

    /// Check if this choice value is truly empty (no value at all, not even empty string).
    ///
    /// This is different from `is_empty()` in that `Single(Some(""))` is NOT truly empty
    /// - an explicit empty string is a valid value (e.g., for /DV defaults).
    pub fn is_truly_empty(&self) -> bool {
        match self {
            ChoiceValue::Single(None) => true,
            ChoiceValue::Single(Some(_)) => false, // Even empty string is a value
            ChoiceValue::Multiple(v) => v.is_empty(),
        }
    }

    /// Get the first selected value as a string (for multi-select, returns comma-joined).
    pub fn as_display_string(&self) -> String {
        match self {
            ChoiceValue::Single(None) => String::new(),
            ChoiceValue::Single(Some(s)) => s.clone(),
            ChoiceValue::Multiple(v) => v.join(","),
        }
    }
}

/// Extracted choice field value.
///
/// Represents the complete state of a choice field, including its kind,
/// selected value(s), default value(s), all available options, and multi-select flag.
#[derive(Debug, Clone, PartialEq)]
pub struct ChoiceValueData {
    /// Choice kind (combo or list).
    pub kind: ChoiceKind,

    /// Current selected value(s).
    /// - Single-select: `Option<String>` (None if no selection)
    /// - Multi-select: `Vec<String>` (empty if no selection)
    pub selected: ChoiceValue,

    /// Default value(s) from /DV entry.
    pub default: Option<ChoiceValue>,

    /// All available options from /Opt array.
    /// Each tuple is (export_value, display_text).
    /// For simple choice fields without explicit export values, both entries are the same.
    pub options: Vec<(String, String)>,

    /// Multi-select flag (from /Ff bit 21, 1<<20 = 0x100000).
    pub multi_select: bool,
}

impl ChoiceValueData {
    /// Create a new ChoiceValueData.
    pub fn new(
        kind: ChoiceKind,
        selected: ChoiceValue,
        default: Option<ChoiceValue>,
        options: Vec<(String, String)>,
        multi_select: bool,
    ) -> Self {
        Self {
            kind,
            selected,
            default,
            options,
            multi_select,
        }
    }

    /// Create a combo box value.
    pub fn combo(
        selected: ChoiceValue,
        default: Option<ChoiceValue>,
        options: Vec<(String, String)>,
    ) -> Self {
        Self {
            kind: ChoiceKind::Combo,
            selected,
            default,
            options,
            multi_select: false,
        }
    }

    /// Create a list box value.
    pub fn list(
        selected: ChoiceValue,
        default: Option<ChoiceValue>,
        options: Vec<(String, String)>,
        multi_select: bool,
    ) -> Self {
        Self {
            kind: ChoiceKind::List,
            selected,
            default,
            options,
            multi_select,
        }
    }

    /// Check if this is a combo box.
    pub fn is_combo(&self) -> bool {
        self.kind == ChoiceKind::Combo
    }

    /// Check if this is a list box.
    pub fn is_list(&self) -> bool {
        self.kind == ChoiceKind::List
    }
}

/// Extract choice field value from raw PDF objects.
///
/// Parses the /V (value), /DV (default value), /Opt (options), and /Ff (flags)
/// entries from a choice field dictionary to extract the complete choice field state.
///
/// # Arguments
///
/// * `value` - The /V entry from the field dictionary (String, Name, or Array of Strings)
/// * `default` - The /DV entry from the field dictionary (same types as /V)
/// * `options` - The /Opt array from the field dictionary (if present)
/// * `flags` - The /Ff entry from the field dictionary (u32 bitfield)
///
/// # Returns
///
/// A `ChoiceValueData` containing the extracted choice field state.
///
/// # Behavior
///
/// - /Ff bit 18 (1 << 17 = 0x20000) → Combo (dropdown)
/// - /Ff bit 21 (1 << 20 = 0x100000) → MultiSelect (list only)
/// - /V as String or Name → single selection, decoded via PDFDocEncoding/UTF-16BE
/// - /V as Array → multi-select, each element decoded as String
/// - /V absent or null → selected: ChoiceValue::Single(None)
/// - /Opt parsed as Vec<(export_value, display_text)> pairs
/// - /DV decoded the same way as /V
pub fn extract_choice_value(
    value: Option<&PdfObject>,
    default: Option<&PdfObject>,
    options: Option<&PdfObject>,
    flags: u32,
) -> ChoiceValueData {
    const COMBO_FLAG: u32 = 1 << 17; // Bit 18 (1-indexed) = 0x20000
    const MULTI_SELECT_FLAG: u32 = 1 << 20; // Bit 21 (1-indexed) = 0x100000

    let is_combo = (flags & COMBO_FLAG) != 0;
    let is_multi_select = (flags & MULTI_SELECT_FLAG) != 0;

    // Determine kind
    let kind = if is_combo {
        ChoiceKind::Combo
    } else {
        ChoiceKind::List
    };

    // Extract current value from /V
    // Combo boxes are always single-select (multi-select flag is ignored for combo)
    let selected = extract_selected_value(value, is_multi_select && !is_combo);

    // Extract default value from /DV
    // Combo boxes are always single-select (multi-select flag is ignored for combo)
    let default_val =
        default.map(|dv| extract_selected_value(Some(dv), is_multi_select && !is_combo));

    // Extract options from /Opt
    let options = extract_options(options);

    ChoiceValueData {
        kind,
        selected,
        default: default_val.filter(|v| !v.is_truly_empty()),
        options,
        multi_select: is_multi_select,
    }
}

/// Extract selected value from /V entry.
///
/// # Arguments
///
/// * `value` - The /V entry (String, Name, Array, or absent)
/// * `is_multi_select` - Whether this is a multi-select field (affects array interpretation)
///
/// # Returns
///
/// A `ChoiceValue` containing the extracted selection.
fn extract_selected_value(value: Option<&PdfObject>, is_multi_select: bool) -> ChoiceValue {
    match value {
        Some(PdfObject::String(bytes)) => {
            // Single selection from string
            let decoded = decode_pdf_string(bytes)
                .unwrap_or_else(|_| String::from_utf8_lossy(bytes).to_string());
            ChoiceValue::Single(Some(decoded))
        }
        Some(PdfObject::Name(name)) => {
            // Single selection from name (rare, but valid per spec)
            ChoiceValue::Single(Some(name.as_ref().to_string()))
        }
        Some(PdfObject::Array(arr)) => {
            // Multi-select: array of strings
            let values: Vec<String> = arr
                .iter()
                .filter_map(|v| extract_string_from_object(v))
                .collect();

            if is_multi_select {
                // Multi-select list box: return Multiple
                ChoiceValue::Multiple(values)
            } else {
                // Single-select combo with array (malformed): take first or empty
                ChoiceValue::Single(values.into_iter().next())
            }
        }
        Some(_) => ChoiceValue::Single(None), // Unrecognized type
        None => ChoiceValue::Single(None),    // No value
    }
}

/// Extract a decoded string from a PDF object (String or Name).
fn extract_string_from_object(obj: &PdfObject) -> Option<String> {
    match obj {
        PdfObject::String(bytes) => Some(
            decode_pdf_string(bytes).unwrap_or_else(|_| String::from_utf8_lossy(bytes).to_string()),
        ),
        PdfObject::Name(name) => Some(name.as_ref().to_string()),
        _ => None,
    }
}

/// Extract options from /Opt array.
///
/// The /Opt array contains option values for choice fields. Each element can be:
/// - A string (both export value and display name are the same)
/// - An array of two strings [export_value, display_name]
///
/// # Arguments
///
/// * `options` - The /Opt entry (Array or absent)
///
/// # Returns
///
/// A `Vec<(String, String)>` where each tuple is (export_value, display_text).
/// Returns empty vec if /Opt is absent or malformed.
fn extract_options(options: Option<&PdfObject>) -> Vec<(String, String)> {
    let opt_array = match options {
        Some(PdfObject::Array(arr)) => arr,
        _ => return Vec::new(),
    };

    let mut result = Vec::new();
    for opt_obj in opt_array.iter() {
        match opt_obj {
            PdfObject::String(bytes) => {
                // Single string: export and display are the same
                if let Ok(s) = decode_pdf_string(bytes) {
                    result.push((s.clone(), s));
                } else if let Ok(s) = String::from_utf8(bytes.to_vec()) {
                    result.push((s.clone(), s));
                }
            }
            PdfObject::Array(arr) => {
                // Array of [export_value, display_name]
                let export = arr.get(0).and_then(|o| extract_string_from_object(o));
                let display = arr.get(1).and_then(|o| extract_string_from_object(o));

                if let (Some(export_val), Some(display_val)) = (export, display) {
                    result.push((export_val, display_val));
                }
            }
            _ => {} // Skip malformed entries
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::intern;

    #[test]
    fn test_choice_kind_display() {
        assert_eq!(ChoiceKind::Combo.to_string(), "combo");
        assert_eq!(ChoiceKind::List.to_string(), "list");
    }

    #[test]
    fn test_extract_combo_single_selection() {
        // Combo box (bit 18 set) with single selection
        let flags = 1 << 17; // Combo flag
        let value = PdfObject::String(Box::new(b"Option B".to_vec()));

        let result = extract_choice_value(Some(&value), None, None, flags);

        assert_eq!(result.kind, ChoiceKind::Combo);
        assert!(!result.multi_select);
        assert_eq!(
            result.selected,
            ChoiceValue::Single(Some("Option B".to_string()))
        );
        assert!(result.default.is_none());
        assert!(result.options.is_empty());
    }

    #[test]
    fn test_extract_list_single_selection() {
        // List box (no combo bit) with single selection
        let flags = 0;
        let value = PdfObject::String(Box::new(b"Item 1".to_vec()));

        let result = extract_choice_value(Some(&value), None, None, flags);

        assert_eq!(result.kind, ChoiceKind::List);
        assert!(!result.multi_select);
        assert_eq!(
            result.selected,
            ChoiceValue::Single(Some("Item 1".to_string()))
        );
    }

    #[test]
    fn test_extract_list_multi_select() {
        // Multi-select list box (bit 21 set)
        let flags = 1 << 20; // MultiSelect flag
        let value = PdfObject::Array(Box::new(vec![
            PdfObject::String(Box::new(b"A".to_vec())),
            PdfObject::String(Box::new(b"B".to_vec())),
            PdfObject::String(Box::new(b"C".to_vec())),
        ]));

        let result = extract_choice_value(Some(&value), None, None, flags);

        assert_eq!(result.kind, ChoiceKind::List);
        assert!(result.multi_select);
        assert_eq!(
            result.selected,
            ChoiceValue::Multiple(vec!["A".to_string(), "B".to_string(), "C".to_string()])
        );
    }

    #[test]
    fn test_extract_choice_no_value() {
        // No /V means no selection
        let flags = 0;
        let result = extract_choice_value(None, None, None, flags);

        assert_eq!(result.selected, ChoiceValue::Single(None));
    }

    #[test]
    fn test_extract_choice_with_default() {
        let flags = 0;
        let value = PdfObject::String(Box::new(b"Current".to_vec()));
        let default = PdfObject::String(Box::new(b"Default".to_vec()));

        let result = extract_choice_value(Some(&value), Some(&default), None, flags);

        assert_eq!(
            result.selected,
            ChoiceValue::Single(Some("Current".to_string()))
        );
        assert_eq!(
            result.default,
            Some(ChoiceValue::Single(Some("Default".to_string())))
        );
    }

    #[test]
    fn test_extract_options_simple_strings() {
        // /Opt array with simple strings (export = display)
        let flags = 0;
        let options = PdfObject::Array(Box::new(vec![
            PdfObject::String(Box::new(b"Option 1".to_vec())),
            PdfObject::String(Box::new(b"Option 2".to_vec())),
            PdfObject::String(Box::new(b"Option 3".to_vec())),
        ]));

        let result = extract_choice_value(None, None, Some(&options), flags);

        assert_eq!(result.options.len(), 3);
        assert_eq!(
            result.options[0],
            ("Option 1".to_string(), "Option 1".to_string())
        );
        assert_eq!(
            result.options[1],
            ("Option 2".to_string(), "Option 2".to_string())
        );
        assert_eq!(
            result.options[2],
            ("Option 3".to_string(), "Option 3".to_string())
        );
    }

    #[test]
    fn test_extract_options_with_export_values() {
        // /Opt array with [export_value, display_name] pairs
        let flags = 0;
        let options = PdfObject::Array(Box::new(vec![
            PdfObject::Array(Box::new(vec![
                PdfObject::String(Box::new(b"v1".to_vec())),
                PdfObject::String(Box::new(b"Display 1".to_vec())),
            ])),
            PdfObject::Array(Box::new(vec![
                PdfObject::String(Box::new(b"v2".to_vec())),
                PdfObject::String(Box::new(b"Display 2".to_vec())),
            ])),
        ]));

        let result = extract_choice_value(None, None, Some(&options), flags);

        assert_eq!(result.options.len(), 2);
        assert_eq!(
            result.options[0],
            ("v1".to_string(), "Display 1".to_string())
        );
        assert_eq!(
            result.options[1],
            ("v2".to_string(), "Display 2".to_string())
        );
    }

    #[test]
    fn test_extract_options_mixed() {
        // /Opt array with mixed simple strings and pairs
        let flags = 0;
        let options = PdfObject::Array(Box::new(vec![
            PdfObject::String(Box::new(b"Simple".to_vec())),
            PdfObject::Array(Box::new(vec![
                PdfObject::String(Box::new(b"export".to_vec())),
                PdfObject::String(Box::new(b"Display".to_vec())),
            ])),
        ]));

        let result = extract_choice_value(None, None, Some(&options), flags);

        assert_eq!(result.options.len(), 2);
        assert_eq!(
            result.options[0],
            ("Simple".to_string(), "Simple".to_string())
        );
        assert_eq!(
            result.options[1],
            ("export".to_string(), "Display".to_string())
        );
    }

    #[test]
    fn test_extract_choice_from_name() {
        // /V as Name (rare but valid)
        let flags = 0;
        let value = PdfObject::Name(intern("SelectedOption"));

        let result = extract_choice_value(Some(&value), None, None, flags);

        assert_eq!(
            result.selected,
            ChoiceValue::Single(Some("SelectedOption".to_string()))
        );
    }

    #[test]
    fn test_extract_combo_with_multi_select_flag() {
        // Combo boxes don't support multi-select (flag is ignored for combo)
        let flags = (1 << 17) | (1 << 20); // Both Combo and MultiSelect flags
        let value = PdfObject::Array(Box::new(vec![
            PdfObject::String(Box::new(b"A".to_vec())),
            PdfObject::String(Box::new(b"B".to_vec())),
        ]));

        let result = extract_choice_value(Some(&value), None, None, flags);

        // Combo kind takes precedence, multi-select is stored but values interpreted as array
        assert_eq!(result.kind, ChoiceKind::Combo);
        assert!(result.multi_select); // Flag is set even for combo
                                      // For combo with array, we take first value (malformed but handled)
        match result.selected {
            ChoiceValue::Single(Some(v)) => assert_eq!(v, "A"),
            _ => panic!("Expected Single(Some) for combo with array"),
        }
    }

    #[test]
    fn test_extract_empty_opt_array() {
        // Empty /Opt array
        let flags = 0;
        let options = PdfObject::Array(Box::new(vec![]));

        let result = extract_choice_value(None, None, Some(&options), flags);

        assert!(result.options.is_empty());
    }

    #[test]
    fn test_extract_no_opt() {
        // No /Opt entry
        let flags = 0;
        let result = extract_choice_value(None, None, None, flags);

        assert!(result.options.is_empty());
    }

    #[test]
    fn test_choice_value_is_empty() {
        assert!(ChoiceValue::Single(None).is_empty());
        assert!(ChoiceValue::Single(Some(String::new())).is_empty());
        assert!(!ChoiceValue::Single(Some("value".to_string())).is_empty());
        assert!(ChoiceValue::Multiple(Vec::new()).is_empty());
        assert!(!ChoiceValue::Multiple(vec!["a".to_string()]).is_empty());
    }

    #[test]
    fn test_choice_value_as_display_string() {
        assert_eq!(ChoiceValue::Single(None).as_display_string(), "");
        assert_eq!(
            ChoiceValue::Single(Some("test".to_string())).as_display_string(),
            "test"
        );
        assert_eq!(
            ChoiceValue::Multiple(vec!["a".to_string(), "b".to_string()]).as_display_string(),
            "a,b"
        );
    }

    #[test]
    fn test_choice_value_data_constructors() {
        let combo = ChoiceValueData::combo(
            ChoiceValue::Single(Some("opt".to_string())),
            None,
            vec![("opt".to_string(), "Option".to_string())],
        );

        assert!(combo.is_combo());
        assert!(!combo.is_list());
        assert!(!combo.multi_select);

        let list = ChoiceValueData::list(
            ChoiceValue::Multiple(vec!["a".to_string(), "b".to_string()]),
            None,
            vec![],
            true,
        );

        assert!(list.is_list());
        assert!(!list.is_combo());
        assert!(list.multi_select);
    }

    #[test]
    fn test_extract_with_other_flags_set() {
        // Test that other /Ff flags don't interfere with choice kind detection
        // ReadOnly (bit 1) + Required (bit 2) + Combo (bit 18)
        let flags = 1 | 2 | (1 << 17);
        let value = PdfObject::String(Box::new(b"test".to_vec()));

        let result = extract_choice_value(Some(&value), None, None, flags);

        assert_eq!(result.kind, ChoiceKind::Combo);
    }

    #[test]
    fn test_extract_malformed_opt_entry() {
        // /Opt with malformed entries should skip them
        let flags = 0;
        let options = PdfObject::Array(Box::new(vec![
            PdfObject::String(Box::new(b"Good".to_vec())),
            PdfObject::Integer(42), // Malformed: should be skipped
            PdfObject::Array(Box::new(vec![
                // Malformed: missing second element
                PdfObject::String(Box::new(b"partial".to_vec())),
            ])),
        ]));

        let result = extract_choice_value(None, None, Some(&options), flags);

        // Should only have the valid entry
        assert_eq!(result.options.len(), 1);
        assert_eq!(result.options[0], ("Good".to_string(), "Good".to_string()));
    }

    #[test]
    fn test_extract_array_with_non_string_elements() {
        // /V array with non-string elements (should skip them)
        let flags = 1 << 20; // MultiSelect
        let value = PdfObject::Array(Box::new(vec![
            PdfObject::String(Box::new(b"A".to_vec())),
            PdfObject::Integer(42), // Skip
            PdfObject::String(Box::new(b"B".to_vec())),
            PdfObject::Bool(true), // Skip
        ]));

        let result = extract_choice_value(Some(&value), None, None, flags);

        assert_eq!(
            result.selected,
            ChoiceValue::Multiple(vec!["A".to_string(), "B".to_string()])
        );
    }

    #[test]
    fn test_extract_default_multi_select() {
        // Multi-select default value
        let flags = 1 << 20; // MultiSelect
        let default = PdfObject::Array(Box::new(vec![
            PdfObject::String(Box::new(b"X".to_vec())),
            PdfObject::String(Box::new(b"Y".to_vec())),
        ]));

        let result = extract_choice_value(None, Some(&default), None, flags);

        assert_eq!(
            result.default,
            Some(ChoiceValue::Multiple(vec![
                "X".to_string(),
                "Y".to_string()
            ]))
        );
    }

    #[test]
    fn test_extract_default_none_becomes_none() {
        // Default that's None or empty should become None
        let flags = 0;
        let result1 = extract_choice_value(None, None, None, flags);
        assert!(result1.default.is_none());

        let default = PdfObject::String(Box::new(b"".to_vec()));
        let result2 = extract_choice_value(None, Some(&default), None, flags);
        // Empty default is still Some(ChoiceValue::Single(Some("")))
        // This is intentional - empty string is a valid default value
        assert!(result2.default.is_some());
    }

    #[test]
    fn test_choice_kind_equality() {
        assert_eq!(ChoiceKind::Combo, ChoiceKind::Combo);
        assert_eq!(ChoiceKind::List, ChoiceKind::List);
        assert_ne!(ChoiceKind::Combo, ChoiceKind::List);
    }

    #[test]
    fn test_choice_value_equality() {
        let v1 = ChoiceValue::Single(Some("test".to_string()));
        let v2 = ChoiceValue::Single(Some("test".to_string()));
        let v3 = ChoiceValue::Single(Some("other".to_string()));

        assert_eq!(v1, v2);
        assert_ne!(v1, v3);

        let m1 = ChoiceValue::Multiple(vec!["a".to_string(), "b".to_string()]);
        let m2 = ChoiceValue::Multiple(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(m1, m2);
    }

    #[test]
    fn test_choice_value_data_equality() {
        let v1 = ChoiceValueData::new(
            ChoiceKind::Combo,
            ChoiceValue::Single(Some("opt".to_string())),
            None,
            vec![("opt".to_string(), "Option".to_string())],
            false,
        );

        let v2 = ChoiceValueData::new(
            ChoiceKind::Combo,
            ChoiceValue::Single(Some("opt".to_string())),
            None,
            vec![("opt".to_string(), "Option".to_string())],
            false,
        );

        assert_eq!(v1, v2);
    }

    #[test]
    fn test_extract_string_from_object_variants() {
        // String object
        let s = PdfObject::String(Box::new(b"test".to_vec()));
        assert_eq!(extract_string_from_object(&s), Some("test".to_string()));

        // Name object
        let n = PdfObject::Name(intern("NameValue"));
        assert_eq!(
            extract_string_from_object(&n),
            Some("NameValue".to_string())
        );

        // Other types return None
        assert_eq!(extract_string_from_object(&PdfObject::Integer(42)), None);
        assert_eq!(extract_string_from_object(&PdfObject::Bool(true)), None);
    }
}
