//! AcroForm Btn (button) field value extraction.
//!
//! This module implements Phase 7.4.2 Btn variant: extract button field values
//! distinguishing pushbutton, checkbox, and radio button types via /Ff flags.
//! For checkbox/radio fields, extracts the selected state and appearance state
//! name (/Yes, /Off, or custom).

use crate::parser::object::PdfObject;
use std::fmt::{self, Display};

/// Button kind classification.
///
/// Distinguishes between the three types of button fields in PDF forms.
/// Determined by the /Ff (field flags) entry in the field dictionary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    /// Pushbutton - a clickable button with no persistent state.
    /// Identified by /Ff bit 26 (1 << 25 = 0x2000000).
    Pushbutton,

    /// Checkbox - a binary toggle field (checked/unchecked).
    /// The default when neither Pushbutton nor Radio bits are set.
    Checkbox,

    /// Radio button - one-of-N selection within a group.
    /// Identified by /Ff bit 25 (1 << 24 = 0x1000000).
    Radio,
}

impl Display for ButtonKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ButtonKind::Pushbutton => write!(f, "pushbutton"),
            ButtonKind::Checkbox => write!(f, "checkbox"),
            ButtonKind::Radio => write!(f, "radio"),
        }
    }
}

/// Extracted button field value.
///
/// Represents the complete state of a button field, including its kind,
/// selected state, and the appearance state name from the PDF.
#[derive(Debug, Clone, PartialEq)]
pub struct ButtonValue {
    /// Button kind (pushbutton, checkbox, or radio).
    pub kind: ButtonKind,

    /// Selected state.
    /// - Pushbutton: always false (no persistent state)
    /// - Checkbox: true if /V is not /Off, false otherwise
    /// - Radio: true if this radio button's /AS matches parent's /V
    pub selected: bool,

    /// Appearance state name from /V (or /AS for radio widgets).
    /// Common values: "Yes" (selected), "Off" (unselected), or custom names.
    /// None for pushbuttons (no /V).
    pub state_name: Option<String>,

    /// Pushbutton flag (from /Ff bit 26).
    pub pushbutton: bool,

    /// Radio button flag (from /Ff bit 25).
    pub radio: bool,
}

impl ButtonValue {
    /// Create a new ButtonValue.
    pub fn new(
        kind: ButtonKind,
        selected: bool,
        state_name: Option<String>,
        pushbutton: bool,
        radio: bool,
    ) -> Self {
        Self {
            kind,
            selected,
            state_name,
            pushbutton,
            radio,
        }
    }

    /// Create a pushbutton value.
    pub fn pushbutton() -> Self {
        Self {
            kind: ButtonKind::Pushbutton,
            selected: false,
            state_name: None,
            pushbutton: true,
            radio: false,
        }
    }

    /// Create a checkbox value.
    pub fn checkbox(selected: bool, state_name: Option<String>) -> Self {
        Self {
            kind: ButtonKind::Checkbox,
            selected,
            state_name,
            pushbutton: false,
            radio: false,
        }
    }

    /// Create a radio button value.
    pub fn radio(selected: bool, state_name: Option<String>) -> Self {
        Self {
            kind: ButtonKind::Radio,
            selected,
            state_name,
            pushbutton: false,
            radio: true,
        }
    }

    /// Check if this button is a pushbutton.
    pub fn is_pushbutton(&self) -> bool {
        self.kind == ButtonKind::Pushbutton
    }

    /// Check if this button is a checkbox.
    pub fn is_checkbox(&self) -> bool {
        self.kind == ButtonKind::Checkbox
    }

    /// Check if this button is a radio button.
    pub fn is_radio(&self) -> bool {
        self.kind == ButtonKind::Radio
    }
}

/// Extract button field value from raw PDF objects.
///
/// Parses the /V (value) entry and /Ff (flags) from a button field dictionary
/// to determine the button kind and selected state.
///
/// # Arguments
///
/// * `value` - The /V entry from the field dictionary (Name object or absent)
/// * `flags` - The /Ff entry from the field dictionary (u32 bitfield)
///
/// # Returns
///
/// A `ButtonValue` containing the extracted button state.
///
/// # Behavior
///
/// - /Ff bit 26 (1 << 25 = 0x2000000) → Pushbutton (no /V, selected: false)
/// - /Ff bit 25 (1 << 24 = 0x1000000) → Radio button
/// - Neither bit set → Checkbox (default)
/// - For checkbox/radio: /V is the appearance state name
///   - /V == /Off → selected: false, state_name: "Off"
///   - /V == /Yes or any other name → selected: true, state_name: the name
///   - /V absent → selected: false, state_name: None
pub fn extract_button_value(value: Option<&PdfObject>, flags: u32) -> ButtonValue {
    const PUSHBUTTON_FLAG: u32 = 1 << 25; // Bit 26 (1-indexed) = 0x2000000
    const RADIO_FLAG: u32 = 1 << 24; // Bit 25 (1-indexed) = 0x1000000

    let is_pushbutton = (flags & PUSHBUTTON_FLAG) != 0;
    let is_radio = (flags & RADIO_FLAG) != 0;

    // Determine kind
    let kind = if is_pushbutton {
        ButtonKind::Pushbutton
    } else if is_radio {
        ButtonKind::Radio
    } else {
        ButtonKind::Checkbox
    };

    match kind {
        ButtonKind::Pushbutton => {
            // Pushbuttons have no persistent state
            ButtonValue::pushbutton()
        }
        ButtonKind::Checkbox | ButtonKind::Radio => {
            // Extract state name from /V
            let (selected, state_name) = extract_state_from_value(value);

            if kind == ButtonKind::Radio {
                ButtonValue::radio(selected, state_name)
            } else {
                ButtonValue::checkbox(selected, state_name)
            }
        }
    }
}

/// Extract selected state and state name from the /V entry.
///
/// # Arguments
///
/// * `value` - The /V entry (Name object or absent)
///
/// # Returns
///
/// A tuple of (selected: bool, state_name: Option<String>).
///
/// # Behavior
///
/// - /V absent → (false, None)
/// - /V == /Off → (false, Some("Off"))
/// - /V == any other name → (true, Some(name))
fn extract_state_from_value(value: Option<&PdfObject>) -> (bool, Option<String>) {
    match value {
        Some(PdfObject::Name(name)) => {
            let state_name = name.as_ref().to_string();
            let selected = state_name != "Off";
            (selected, Some(state_name))
        }
        Some(_) => (false, None), // Non-Name /V is malformed
        None => (false, None),    // No /V means unchecked
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::intern;

    #[test]
    fn test_button_kind_display() {
        assert_eq!(ButtonKind::Pushbutton.to_string(), "pushbutton");
        assert_eq!(ButtonKind::Checkbox.to_string(), "checkbox");
        assert_eq!(ButtonKind::Radio.to_string(), "radio");
    }

    #[test]
    fn test_extract_pushbutton() {
        // Pushbutton flag is bit 26 (1 << 25)
        let flags = 1 << 25;
        let value = extract_button_value(None, flags);

        assert_eq!(value.kind, ButtonKind::Pushbutton);
        assert!(!value.selected);
        assert!(value.state_name.is_none());
        assert!(value.pushbutton);
        assert!(!value.radio);
    }

    #[test]
    fn test_extract_checkbox_selected_yes() {
        // No flags set → checkbox
        let flags = 0;
        let value = extract_button_value(Some(&PdfObject::Name(intern("Yes"))), flags);

        assert_eq!(value.kind, ButtonKind::Checkbox);
        assert!(value.selected);
        assert_eq!(value.state_name, Some("Yes".to_string()));
        assert!(!value.pushbutton);
        assert!(!value.radio);
    }

    #[test]
    fn test_extract_checkbox_unselected_off() {
        let flags = 0;
        let value = extract_button_value(Some(&PdfObject::Name(intern("Off"))), flags);

        assert_eq!(value.kind, ButtonKind::Checkbox);
        assert!(!value.selected);
        assert_eq!(value.state_name, Some("Off".to_string()));
        assert!(!value.pushbutton);
        assert!(!value.radio);
    }

    #[test]
    fn test_extract_checkbox_custom_state() {
        // Custom appearance state name
        let flags = 0;
        let value = extract_button_value(Some(&PdfObject::Name(intern("Selected"))), flags);

        assert_eq!(value.kind, ButtonKind::Checkbox);
        assert!(value.selected); // Anything other than "Off" is selected
        assert_eq!(value.state_name, Some("Selected".to_string()));
    }

    #[test]
    fn test_extract_checkbox_no_value() {
        // No /V means unchecked
        let flags = 0;
        let value = extract_button_value(None, flags);

        assert_eq!(value.kind, ButtonKind::Checkbox);
        assert!(!value.selected);
        assert!(value.state_name.is_none());
    }

    #[test]
    fn test_extract_radio_selected() {
        // Radio flag is bit 25 (1 << 24)
        let flags = 1 << 24;
        let value = extract_button_value(Some(&PdfObject::Name(intern("OptionA"))), flags);

        assert_eq!(value.kind, ButtonKind::Radio);
        assert!(value.selected);
        assert_eq!(value.state_name, Some("OptionA".to_string()));
        assert!(!value.pushbutton);
        assert!(value.radio);
    }

    #[test]
    fn test_extract_radio_unselected() {
        let flags = 1 << 24;
        let value = extract_button_value(Some(&PdfObject::Name(intern("Off"))), flags);

        assert_eq!(value.kind, ButtonKind::Radio);
        assert!(!value.selected);
        assert_eq!(value.state_name, Some("Off".to_string()));
        assert!(value.radio);
    }

    #[test]
    fn test_extract_radio_no_value() {
        let flags = 1 << 24;
        let value = extract_button_value(None, flags);

        assert_eq!(value.kind, ButtonKind::Radio);
        assert!(!value.selected);
        assert!(value.state_name.is_none());
        assert!(value.radio);
    }

    #[test]
    fn test_button_value_constructors() {
        let pushbutton = ButtonValue::pushbutton();
        assert!(pushbutton.is_pushbutton());
        assert!(!pushbutton.selected);

        let checkbox_checked = ButtonValue::checkbox(true, Some("Yes".to_string()));
        assert!(checkbox_checked.is_checkbox());
        assert!(checkbox_checked.selected);

        let radio_checked = ButtonValue::radio(true, Some("Option1".to_string()));
        assert!(radio_checked.is_radio());
        assert!(radio_checked.selected);
    }

    #[test]
    fn test_extract_with_other_flags_set() {
        // Test that other /Ff flags don't interfere with button kind detection
        // ReadOnly (bit 1) + Required (bit 2) + Radio (bit 25)
        let flags = 1 | 2 | (1 << 24);
        let value = extract_button_value(Some(&PdfObject::Name(intern("Opt1"))), flags);

        assert_eq!(value.kind, ButtonKind::Radio);
        assert!(value.selected);
        assert!(value.radio);
    }

    #[test]
    fn test_extract_state_from_value_malformed() {
        // Non-Name /V should be handled gracefully
        let (selected, state_name) = extract_state_from_value(Some(&PdfObject::Integer(42)));

        assert!(!selected);
        assert!(state_name.is_none());
    }

    #[test]
    fn test_button_kind_equality() {
        assert_eq!(ButtonKind::Pushbutton, ButtonKind::Pushbutton);
        assert_eq!(ButtonKind::Checkbox, ButtonKind::Checkbox);
        assert_eq!(ButtonKind::Radio, ButtonKind::Radio);

        assert_ne!(ButtonKind::Pushbutton, ButtonKind::Checkbox);
        assert_ne!(ButtonKind::Checkbox, ButtonKind::Radio);
    }

    #[test]
    fn test_button_value_equality() {
        let v1 = ButtonValue::checkbox(true, Some("Yes".to_string()));
        let v2 = ButtonValue::checkbox(true, Some("Yes".to_string()));
        let v3 = ButtonValue::checkbox(false, Some("Off".to_string()));

        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }

    #[test]
    fn test_pushbutton_takes_precedence() {
        // If both Pushbutton and Radio flags are set (malformed), Pushbutton wins
        let flags = (1 << 25) | (1 << 24);
        let value = extract_button_value(None, flags);

        assert_eq!(value.kind, ButtonKind::Pushbutton);
        assert!(value.pushbutton);
        // Note: radio flag is also true in flags, but kind is Pushbutton
    }
}
