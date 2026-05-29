//! AcroForm + XFA field combiner with XFA-wins precedence.
//!
//! This module implements Phase 7.4.4: merge AcroForm fields and XFA fields
//! into a unified representation. When field names collide, XFA values take
//! precedence (per PDF 1.7 spec convention and Adobe Reader behavior).

use crate::diagnostics::{DiagCode, Diagnostic};
use std::collections::HashMap;

/// Normalized form field value.
///
/// Represents a form field value in a type-safe way, abstracting away the
/// raw PDF object representation. This enum is the output type for the
/// `combine()` function and is suitable for JSON serialization and
/// downstream consumption.
#[derive(Debug, Clone, PartialEq)]
pub enum FormFieldValue {
    /// Text field value (/FT /Tx).
    Text {
        /// Current value (null if empty/absent).
        value: Option<String>,
        /// Default value (/DV entry).
        default: Option<String>,
        /// Multi-line flag (from /Ff bit 13).
        multiline: bool,
        /// Max length (from /MaxLen entry, if present).
        max_length: Option<u32>,
    },

    /// Button field value (/FT /Btn) - pushbutton, checkbox, or radio button.
    Button {
        /// Button kind (pushbutton, checkbox, or radio).
        kind: super::value_button::ButtonKind,
        /// Selected state (true = checked, false = unchecked).
        selected: bool,
        /// Appearance state name (e.g., "Yes", "Off", or custom).
        state_name: Option<String>,
        /// Default selected state (from /DV).
        default_selected: Option<bool>,
        /// Pushbutton flag (from /Ff bit 26).
        pushbutton: bool,
        /// Radio button flag (from /Ff bit 25).
        radio: bool,
    },

    /// Choice field value (/FT /Ch) - dropdown or list box.
    Choice {
        /// Selected option(s) (single string or array for multi-select).
        value: ChoiceValue,
        /// Default selected option(s).
        default: Option<ChoiceValue>,
        /// All available options (from /Opt array).
        options: Vec<(String, String)>,
        /// Combo box flag (editable dropdown, from /Ff bit 18).
        is_combo: bool,
        /// Multi-select flag (from /Ff bit 21).
        is_multi_select: bool,
    },

    /// Signature field value (/FT /Sig).
    Signature {
        /// Signature dictionary reference (indirect object number).
        signature_ref: Option<u32>,
    },
}

/// Choice field value representation.
///
/// Choice fields can have either a single selected value or multiple
/// selected values (for multi-select list boxes).
#[derive(Debug, Clone, PartialEq)]
pub enum ChoiceValue {
    /// Single selected option.
    Single(String),
    /// Multiple selected options.
    Multiple(Vec<String>),
}

impl FormFieldValue {
    /// Check if this field value is empty (no current value set).
    pub fn is_empty(&self) -> bool {
        match self {
            FormFieldValue::Text { value, .. } => value.as_ref().map_or(true, |v| v.is_empty()),
            FormFieldValue::Button { selected, .. } => !selected,
            FormFieldValue::Choice { value, .. } => match value {
                ChoiceValue::Single(s) => s.is_empty(),
                ChoiceValue::Multiple(v) => v.is_empty(),
            },
            FormFieldValue::Signature { .. } => true,
        }
    }
}

/// Source of a field value in the combined output.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Source {
    /// Value comes only from AcroForm.
    AcroForm,
    /// Value comes only from XFA.
    Xfa,
    /// Value present in both AcroForm and XFA (XFA won).
    Both,
}

/// Merge AcroForm fields and XFA fields into a unified map.
///
/// This function implements Phase 7.4.4: combine the AcroForm field map
/// (from Phase 7.4.2) and the XFA field map (from Phase 7.4.3) into a
/// single Vec<(String, FormFieldValue)>. On field-name collision, XFA
/// values win over AcroForm values.
///
/// # Arguments
///
/// * `acro_fields` - AcroForm fields as Vec<(full_name, FormFieldValue)>
/// * `xfa_fields` - XFA fields as Vec<(full_name, String)> (XFA values are always strings)
///
/// # Returns
///
/// A `Vec<(String, FormFieldValue)>` sorted alphabetically by field name,
/// plus a `Vec<Diagnostic>` containing any collision diagnostics.
///
/// # Behavior
///
/// - Insert AcroForm fields first
/// - Insert XFA fields second, overwriting AcroForm values on collision
/// - Track which fields came from both sources (emit diagnostic)
/// - Convert XFA boolean strings ("true"/"false"/"1"/"0") to Button::selected
/// - Preserve AcroForm type hints when XFA provides the value
/// - Empty XFA values overwrite non-empty AcroForm values (XFA is canonical)
/// - Sort output alphabetically by full_name for deterministic ordering
///
/// # Example
///
/// ```ignore
/// use pdftract_core::forms::{combine, FormFieldValue};
///
/// let acro_fields = vec![
///     ("name".to_string(), FormFieldValue::Text { ... }),
///     ("checked".to_string(), FormFieldValue::Button { ... }),
/// ];
///
/// let xfa_fields = vec![
///     ("name".to_string(), "Jane Doe".to_string()),  // Overwrites AcroForm
///     ("email".to_string(), "jane@example.com".to_string()),
/// ];
///
/// let (combined, diagnostics) = combine(acro_fields, xfa_fields);
/// // combined[0] == ("checked", FormFieldValue::Button { ... })  // AcroForm only
/// // combined[1] == ("email", FormFieldValue::Text { ... })     // XFA only
/// // combined[2] == ("name", FormFieldValue::Text { ... })      // XFA wins (collision)
/// ```
pub fn combine(
    mut acro_fields: Vec<(String, FormFieldValue)>,
    xfa_fields: Vec<(String, String)>,
) -> (Vec<(String, FormFieldValue)>, Vec<Diagnostic>) {
    let mut map: HashMap<String, (FormFieldValue, Source)> = HashMap::new();
    let mut diagnostics = Vec::new();

    // Insert AcroForm fields first
    for (name, value) in acro_fields.drain(..) {
        map.insert(name, (value, Source::AcroForm));
    }

    // Insert XFA fields second (overwrites on collision)
    for (name, xfa_value) in xfa_fields {
        let source = if map.contains_key(&name) {
            Source::Both
        } else {
            Source::Xfa
        };

        // Emit diagnostic for collisions (capture old value before overwrite)
        if source == Source::Both {
            if let Some((old_value, _)) = map.get(&name) {
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructUnexpectedEof,
                    format!(
                        "form_field_collision: name='{}', acro_value={:?}, xfa_value='{}'",
                        name, old_value, xfa_value
                    ),
                ));
            }
        }

        // Convert XFA string value to FormFieldValue
        // Preserve type hints from existing AcroForm value if present
        let converted_value = if let Some((acro_value, _)) = map.get(&name) {
            // Merge XFA value with existing AcroForm type
            merge_xfa_value_with_acro_type(acro_value, &xfa_value, &name, &mut diagnostics)
        } else {
            // New XFA-only field, infer type from value
            infer_xfa_field_type(&xfa_value)
        };

        map.insert(name, (converted_value, source));
    }

    // Convert to Vec and sort alphabetically
    let mut combined: Vec<(String, FormFieldValue)> = map
        .into_iter()
        .map(|(name, (value, _source))| (name, value))
        .collect();

    combined.sort_by(|a, b| a.0.cmp(&b.0));

    (combined, diagnostics)
}

/// Merge an XFA string value with an existing AcroForm type.
///
/// Preserves the AcroForm's type information while replacing the value
/// with the XFA-provided string.
fn merge_xfa_value_with_acro_type(
    acro_value: &FormFieldValue,
    xfa_value: &str,
    name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> FormFieldValue {
    match acro_value {
        FormFieldValue::Text {
            value: _,
            default,
            multiline,
            max_length,
        } => FormFieldValue::Text {
            value: Some(xfa_value.to_string()),
            default: default.clone(),
            multiline: *multiline,
            max_length: *max_length,
        },

        FormFieldValue::Button {
            kind,
            selected: _,
            state_name: _,
            default_selected,
            pushbutton,
            radio,
        } => {
            // Convert XFA boolean string to selected state
            let selected = parse_xfa_boolean(xfa_value).unwrap_or(false);
            FormFieldValue::Button {
                kind: *kind,
                selected,
                state_name: if selected {
                    Some(xfa_value.to_string())
                } else {
                    None
                },
                default_selected: *default_selected,
                pushbutton: *pushbutton,
                radio: *radio,
            }
        }

        FormFieldValue::Choice {
            value: _,
            default,
            options,
            is_combo,
            is_multi_select,
        } => {
            // XFA choice values are comma-separated for multi-select
            let value = if *is_multi_select {
                ChoiceValue::Multiple(
                    xfa_value
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                )
            } else {
                ChoiceValue::Single(xfa_value.to_string())
            };
            FormFieldValue::Choice {
                value,
                default: default.clone(),
                options: options.clone(),
                is_combo: *is_combo,
                is_multi_select: *is_multi_select,
            }
        }

        FormFieldValue::Signature { .. } => {
            // XFA doesn't provide signature values; keep AcroForm
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!(
                    "XFA value provided for signature field '{}': XFA cannot override signature (keeping AcroForm)",
                    name
                ),
            ));
            acro_value.clone()
        }
    }
}

/// Parse an XFA boolean string to a bool.
///
/// XFA forms represent boolean values as strings: "true", "false", "1", "0".
fn parse_xfa_boolean(s: &str) -> Option<bool> {
    match s.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

/// Infer field type from an XFA-only field value.
///
/// When a field exists only in XFA (not in AcroForm), we must infer its
/// type from the string value. Defaults to Text field.
fn infer_xfa_field_type(xfa_value: &str) -> FormFieldValue {
    // Check for boolean patterns
    if parse_xfa_boolean(xfa_value).is_some() {
        // Could be a button, but without AcroForm type hints we treat as text
        // to avoid misclassifying text fields that happen to contain "true"/"false"
    }

    // Default: treat XFA-only fields as text
    FormFieldValue::Text {
        value: Some(xfa_value.to_string()),
        default: None,
        multiline: false,
        max_length: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::value_button::ButtonKind;

    fn make_text_value(value: &str) -> FormFieldValue {
        FormFieldValue::Text {
            value: Some(value.to_string()),
            default: None,
            multiline: false,
            max_length: None,
        }
    }

    fn make_button_value(selected: bool) -> FormFieldValue {
        FormFieldValue::Button {
            kind: ButtonKind::Checkbox,
            selected,
            state_name: if selected {
                Some("Yes".to_string())
            } else {
                Some("Off".to_string())
            },
            default_selected: None,
            pushbutton: false,
            radio: false,
        }
    }

    #[test]
    fn test_combine_no_overlap() {
        // 3 AcroForm + 2 XFA, no overlap
        let acro_fields = vec![
            ("field1".to_string(), make_text_value("acro1")),
            ("field2".to_string(), make_text_value("acro2")),
            ("field3".to_string(), make_text_value("acro3")),
        ];

        let xfa_fields = vec![
            ("xfa1".to_string(), "xfa_value1".to_string()),
            ("xfa2".to_string(), "xfa_value2".to_string()),
        ];

        let (combined, diagnostics) = combine(acro_fields, xfa_fields);

        assert_eq!(combined.len(), 5);
        assert_eq!(combined[0].0, "field1");
        assert_eq!(combined[1].0, "field2");
        assert_eq!(combined[2].0, "field3");
        assert_eq!(combined[3].0, "xfa1");
        assert_eq!(combined[4].0, "xfa2");
        assert!(diagnostics.is_empty()); // No collisions
    }

    #[test]
    fn test_combine_both_overlapping() {
        // 3 AcroForm + 2 XFA, both overlapping on 2 fields
        let acro_fields = vec![
            ("field1".to_string(), make_text_value("acro1")),
            ("field2".to_string(), make_text_value("acro2")),
            ("field3".to_string(), make_text_value("acro3")),
        ];

        let xfa_fields = vec![
            ("field2".to_string(), "xfa_value2".to_string()), // Overwrites field2
            ("field3".to_string(), "xfa_value3".to_string()), // Overwrites field3
        ];

        let (combined, diagnostics) = combine(acro_fields, xfa_fields);

        assert_eq!(combined.len(), 3);
        assert_eq!(combined[0].0, "field1");
        assert_eq!(combined[1].0, "field2");
        assert_eq!(combined[2].0, "field3");

        // XFA values won
        if let FormFieldValue::Text { value, .. } = &combined[1].1 {
            assert_eq!(value.as_ref().unwrap(), "xfa_value2");
        } else {
            panic!("Expected Text field");
        }

        if let FormFieldValue::Text { value, .. } = &combined[2].1 {
            assert_eq!(value.as_ref().unwrap(), "xfa_value3");
        } else {
            panic!("Expected Text field");
        }

        // 2 collision diagnostics
        assert_eq!(diagnostics.len(), 2);
    }

    #[test]
    fn test_xfa_boolean_to_checkbox() {
        // XFA boolean string converts to Button selected state
        let acro_fields = vec![("checkbox".to_string(), make_button_value(false))];

        let xfa_fields = vec![("checkbox".to_string(), "true".to_string())];

        let (combined, diagnostics) = combine(acro_fields, xfa_fields);

        assert_eq!(combined.len(), 1);
        if let FormFieldValue::Button { selected, .. } = &combined[0].1 {
            assert!(selected);
        } else {
            panic!("Expected Button field");
        }

        // 1 collision diagnostic
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_empty_xfa_wins_over_nonempty_acro() {
        // Empty XFA value overwrites non-empty AcroForm value
        let acro_fields = vec![("field1".to_string(), make_text_value("acro_value"))];

        let xfa_fields = vec![("field1".to_string(), "".to_string())];

        let (combined, _diagnostics) = combine(acro_fields, xfa_fields);

        assert_eq!(combined.len(), 1);
        if let FormFieldValue::Text { value, .. } = &combined[0].1 {
            assert_eq!(value.as_ref().unwrap(), "");
        } else {
            panic!("Expected Text field");
        }
    }

    #[test]
    fn test_parse_xfa_boolean() {
        assert_eq!(parse_xfa_boolean("true"), Some(true));
        assert_eq!(parse_xfa_boolean("false"), Some(false));
        assert_eq!(parse_xfa_boolean("1"), Some(true));
        assert_eq!(parse_xfa_boolean("0"), Some(false));
        assert_eq!(parse_xfa_boolean("TRUE"), Some(true)); // Case insensitive
        assert_eq!(parse_xfa_boolean("yes"), Some(true));
        assert_eq!(parse_xfa_boolean("no"), Some(false));
        assert_eq!(parse_xfa_boolean("random"), None); // Not a boolean
    }

    #[test]
    fn test_sort_order_deterministic() {
        // Verify alphabetical sorting
        let acro_fields = vec![
            ("zebra".to_string(), make_text_value("z")),
            ("apple".to_string(), make_text_value("a")),
            ("banana".to_string(), make_text_value("b")),
        ];

        let (combined, _diagnostics) = combine(acro_fields, vec![]);

        assert_eq!(combined.len(), 3);
        assert_eq!(combined[0].0, "apple");
        assert_eq!(combined[1].0, "banana");
        assert_eq!(combined[2].0, "zebra");
    }

    #[test]
    fn test_choice_value_single() {
        let acro_fields = vec![(
            "dropdown".to_string(),
            FormFieldValue::Choice {
                value: ChoiceValue::Single("option1".to_string()),
                default: None,
                options: vec![
                    ("opt1".to_string(), "Option 1".to_string()),
                    ("opt2".to_string(), "Option 2".to_string()),
                ],
                is_combo: false,
                is_multi_select: false,
            },
        )];

        let xfa_fields = vec![("dropdown".to_string(), "opt2".to_string())];

        let (combined, _diagnostics) = combine(acro_fields, xfa_fields);

        assert_eq!(combined.len(), 1);
        if let FormFieldValue::Choice { value, .. } = &combined[0].1 {
            assert_eq!(value, &ChoiceValue::Single("opt2".to_string()));
        } else {
            panic!("Expected Choice field");
        }
    }

    #[test]
    fn test_choice_value_multi_select() {
        let acro_fields = vec![(
            "listbox".to_string(),
            FormFieldValue::Choice {
                value: ChoiceValue::Single("opt1".to_string()),
                default: None,
                options: vec![
                    ("opt1".to_string(), "Option 1".to_string()),
                    ("opt2".to_string(), "Option 2".to_string()),
                    ("opt3".to_string(), "Option 3".to_string()),
                ],
                is_combo: false,
                is_multi_select: true,
            },
        )];

        // XFA multi-select values are comma-separated
        let xfa_fields = vec![("listbox".to_string(), "opt1,opt3".to_string())];

        let (combined, _diagnostics) = combine(acro_fields, xfa_fields);

        assert_eq!(combined.len(), 1);
        if let FormFieldValue::Choice {
            value,
            is_multi_select,
            ..
        } = &combined[0].1
        {
            assert!(*is_multi_select);
            assert_eq!(
                value,
                &ChoiceValue::Multiple(vec!["opt1".to_string(), "opt3".to_string()])
            );
        } else {
            panic!("Expected Choice field");
        }
    }
}
