//! BMC/BDC/EMC operator parsers for marked content.
//!
//! This module implements the three marked-content operators that consume
//! operands from the operand stack and dispatch to the marked-content stack.
//!
//! Per PDF spec section 14.5:
//! - BMC /Tag: begin marked content with tag only
//! - BDC /Tag <<props>> or BDC /Tag /PropName: begin marked content with properties
//! - EMC: end marked content (pop top frame)

use crate::diagnostics::{DiagCode, Diagnostic};
use crate::parser::marked_content_stack::{MarkedContentFrame, MarkedContentStack};
use crate::parser::object::PdfObject;
use crate::parser::resources::ResourceDict;
use std::sync::Arc;

/// Parse BMC operator (begin marked content).
///
/// BMC consumes 1 operand from the operand stack: a Name (the tag).
/// Pushes a MarkedContentFrame with the tag and mcid=None.
///
/// # Arguments
///
/// * `stack` - The marked-content stack to push the frame onto
/// * `tag` - The tag name (e.g., "Span", "P", "Artifact")
///
/// # Returns
///
/// true if the frame was pushed, false if the stack depth limit was exceeded.
pub fn parse_bmc(stack: &mut MarkedContentStack, tag: Arc<str>) -> bool {
    stack.push_bmc(tag.to_string())
}

/// Parse BDC operator (begin marked content with properties).
///
/// BDC consumes 2 operands from the operand stack:
/// 1. A Name (the tag)
/// 2. Either a dictionary (inline properties) or a Name (property resource name)
///
/// If the second operand is a Name, it's resolved via ResourceDict::lookup_properties.
/// If the properties dict contains /MCID, the value is extracted; otherwise mcid=None.
///
/// Per bead pdftract-1q19p: If the tag is "OC" and the properties contain /OCG
/// referencing an Optional Content Group, check if the OCG is OFF by default.
/// If so, set is_hidden=true on the frame.
///
/// # Arguments
///
/// * `stack` - The marked-content stack to push the frame onto
/// * `tag` - The tag name (e.g., "Span", "P", "OC")
/// * `props` - The properties object (dict or name)
/// * `resources` - The page resource dictionary for property name resolution
/// * `default_off_ocgs` - Optional HashSet of OCG refs that are OFF by default
/// * `diagnostics` - Optional diagnostics vector to append errors to
///
/// # Returns
///
/// true if the frame was pushed, false if the stack depth limit was exceeded.
pub fn parse_bdc(
    stack: &mut MarkedContentStack,
    tag: Arc<str>,
    props: &PdfObject,
    resources: &ResourceDict,
    default_off_ocgs: Option<&std::collections::HashSet<crate::parser::object::ObjRef>>,
    diagnostics: Option<&mut Vec<Diagnostic>>,
) -> bool {
    let mcid = extract_mcid_from_props(props, resources, diagnostics);

    // Check for OCG /OC tag (bead pdftract-1q19p)
    let is_hidden = if tag.as_ref() == "OC" || tag.as_ref() == "/OC" {
        // Check if props dict has /OCG reference
        if let Some(ocg_ref) = extract_ocg_ref_from_props(props) {
            // Check if this OCG is in the OFF set
            default_off_ocgs
                .map(|off_set| off_set.contains(&ocg_ref))
                .unwrap_or(false)
        } else {
            // No /OCG property, not hidden
            false
        }
    } else {
        false
    };

    stack.push_bdc(tag.to_string(), mcid, is_hidden)
}

/// Parse EMC operator (end marked content).
///
/// EMC consumes 0 operands. Pops the top frame from the marked-content stack.
/// If the stack is empty, emits an EMC_WITHOUT_BMC diagnostic.
///
/// # Arguments
///
/// * `stack` - The marked-content stack to pop from
///
/// # Returns
///
/// Some(frame) if a frame was popped, None if the stack was empty.
pub fn parse_emc(stack: &mut MarkedContentStack) -> Option<MarkedContentFrame> {
    stack.pop_emc()
}

/// Extract MCID from a BDC properties object.
///
/// The properties object can be:
/// - A dictionary: read /MCID directly
/// - A name: look up in ResourceDict::/Properties, then read /MCID
/// - Anything else: emit diagnostic, return None
///
/// # Arguments
///
/// * `props` - The properties object (dict or name)
/// * `resources` - The page resource dictionary for property name resolution
/// * `diagnostics` - Optional diagnostics vector to append errors to
///
/// # Returns
///
/// Some(mcid) if found and valid, None otherwise.
fn extract_mcid_from_props(
    props: &PdfObject,
    resources: &ResourceDict,
    diagnostics: Option<&mut Vec<Diagnostic>>,
) -> Option<u32> {
    match props {
        PdfObject::Dict(dict) => {
            // Inline property dict - read /MCID directly
            extract_mcid_from_dict(dict)
        }
        PdfObject::Name(name) => {
            // Property resource name - look up in /Properties
            let name_str: &str = name.as_ref();
            let name_str = name_str.strip_prefix('/').unwrap_or(name_str);

            match resources.lookup_properties(name_str) {
                Some(_obj_ref) => {
                    // We have an ObjRef but can't resolve it here without the full resolver
                    // For now, return None and emit a diagnostic that we can't resolve indirect refs
                    // TODO: This would need to be resolved in the full executor context
                    None
                }
                None => {
                    // Unknown property name - emit diagnostic but continue
                    if let Some(diags) = diagnostics {
                        emit_unknown_property_name(diags, name_str);
                    }
                    None
                }
            }
        }
        _ => {
            // Invalid BDC operand - emit diagnostic via caller
            None
        }
    }
}

/// Extract MCID value from a property dictionary.
///
/// # Arguments
///
/// * `dict` - The property dictionary
///
/// # Returns
///
/// Some(mcid) if /MCID is present and valid, None otherwise.
fn extract_mcid_from_dict(dict: &indexmap::IndexMap<Arc<str>, PdfObject>) -> Option<u32> {
    match dict.get("/MCID") {
        Some(PdfObject::Integer(n)) if *n >= 0 => Some(*n as u32),
        Some(PdfObject::Integer(_)) => {
            // Negative MCID is invalid per spec ("non-negative integer")
            // Emit diagnostic and treat as missing
            None
        }
        Some(PdfObject::Real(f)) => {
            // MCID as real is non-standard but seen in the wild
            // Truncate to integer if it's a whole number
            let mcid = f.trunc() as i64;
            if mcid >= 0 && (f - mcid as f64).abs() < f64::EPSILON {
                Some(mcid as u32)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Extract OCG reference from a BDC properties object.
///
/// Per bead pdftract-1q19p: If the properties dict contains /OCG key
/// with an indirect reference value, return that reference.
///
/// # Arguments
///
/// * `props` - The properties object (dict or name)
///
/// # Returns
///
/// Some(ocg_ref) if /OCG is present and is an indirect reference, None otherwise.
fn extract_ocg_ref_from_props(props: &PdfObject) -> Option<crate::parser::object::ObjRef> {
    match props {
        PdfObject::Dict(dict) => {
            // Inline property dict - check for /OCG key
            dict.get("/OCG").and_then(|obj| obj.as_ref())
        }
        PdfObject::Name(_name) => {
            // Property resource name - would need to resolve via /Properties
            // For now, return None (property name resolution for OCG deferred)
            None
        }
        _ => None,
    }
}

/// Emit a diagnostic for an invalid BDC operand.
///
/// # Arguments
///
/// * `diagnostics` - The diagnostics vector to append to
/// * `operand` - The invalid operand
pub fn emit_invalid_bdc_operand(diagnostics: &mut Vec<Diagnostic>, operand: &PdfObject) {
    let type_name = match operand {
        PdfObject::Null => "null",
        PdfObject::Bool(_) => "boolean",
        PdfObject::Integer(_) => "integer",
        PdfObject::Real(_) => "real",
        PdfObject::String(_) => "string",
        PdfObject::Name(_) => "name",
        PdfObject::Array(_) => "array",
        PdfObject::Dict(_) => "dict",
        PdfObject::Ref(_) => "indirect reference",
        PdfObject::Stream(_) => "stream",
        PdfObject::Indirect(_) => "indirect object wrapper",
    };

    diagnostics.push(Diagnostic::with_dynamic_no_offset(
        DiagCode::StructInvalidBdcOperand,
        format!("BDC second operand is {}; expected dict or name", type_name),
    ));
}

/// Emit a diagnostic for an unknown marked-content property name.
///
/// # Arguments
///
/// * `diagnostics` - The diagnostics vector to append to
/// * `name` - The unknown property name
pub fn emit_unknown_property_name(diagnostics: &mut Vec<Diagnostic>, name: &str) {
    diagnostics.push(Diagnostic::with_dynamic_no_offset(
        DiagCode::UnknownMarkedContentProps,
        format!("BDC property name '{}' not found in /Properties", name),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::{intern, ObjRef};
    use indexmap::IndexMap;

    #[test]
    fn test_parse_bmc() {
        let mut stack = MarkedContentStack::new();
        assert!(parse_bmc(&mut stack, Arc::from("Span")));
        assert_eq!(stack.depth(), 1);
        assert_eq!(stack.innermost_frame().unwrap().tag, "Span");
        assert_eq!(stack.innermost_mcid(), None);
    }

    #[test]
    fn test_parse_bdc_with_inline_dict_mcid() {
        let mut stack = MarkedContentStack::new();
        let mut props = IndexMap::new();
        props.insert(intern("/MCID"), PdfObject::Integer(42));

        assert!(parse_bdc(
            &mut stack,
            Arc::from("P"),
            &PdfObject::Dict(Box::new(props)),
            &ResourceDict::new(),
            None,
            None
        ));
        assert_eq!(stack.depth(), 1);
        assert_eq!(stack.innermost_mcid(), Some(42));
    }

    #[test]
    fn test_parse_bdc_with_inline_dict_no_mcid() {
        let mut stack = MarkedContentStack::new();
        let props = IndexMap::new();

        assert!(parse_bdc(
            &mut stack,
            Arc::from("Artifact"),
            &PdfObject::Dict(Box::new(props)),
            &ResourceDict::new(),
            None,
            None
        ));
        assert_eq!(stack.depth(), 1);
        assert_eq!(stack.innermost_mcid(), None);
    }

    #[test]
    fn test_parse_bdc_with_property_name_found() {
        let mut stack = MarkedContentStack::new();
        let mut resources = ResourceDict::new();
        resources
            .properties
            .insert(Arc::from("MyProps"), ObjRef::new(10, 0));

        // Property name resolution requires full resolver, so this returns None
        assert!(parse_bdc(
            &mut stack,
            Arc::from("P"),
            &PdfObject::Name(Arc::from("MyProps")),
            &resources,
            None,
            None
        ));
        assert_eq!(stack.depth(), 1);
        assert_eq!(stack.innermost_mcid(), None); // Can't resolve without full resolver
    }

    #[test]
    fn test_parse_bdc_with_property_name_not_found() {
        let mut stack = MarkedContentStack::new();
        let resources = ResourceDict::new();
        let mut diagnostics = Vec::new();

        assert!(parse_bdc(
            &mut stack,
            Arc::from("P"),
            &PdfObject::Name(Arc::from("UnknownProps")),
            &resources,
            None,
            Some(&mut diagnostics)
        ));
        assert_eq!(stack.depth(), 1);
        assert_eq!(stack.innermost_mcid(), None);
        // Verify that the diagnostic was emitted
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics[0].code, DiagCode::UnknownMarkedContentProps);
    }

    #[test]
    fn test_parse_emc_success() {
        let mut stack = MarkedContentStack::new();
        parse_bmc(&mut stack, Arc::from("Span"));

        let frame = parse_emc(&mut stack).unwrap();
        assert_eq!(frame.tag, "Span");
        assert!(stack.is_empty());
    }

    #[test]
    fn test_parse_emc_underflow() {
        let mut stack = MarkedContentStack::new();
        let result = parse_emc(&mut stack);
        assert!(result.is_none());
        assert!(!stack.diagnostics().is_empty());
        assert_eq!(stack.diagnostics()[0].code, DiagCode::EmcWithoutBmc);
    }

    #[test]
    fn test_extract_mcid_from_dict_valid() {
        let mut dict = IndexMap::new();
        dict.insert(intern("/MCID"), PdfObject::Integer(123));

        assert_eq!(extract_mcid_from_dict(&dict), Some(123));
    }

    #[test]
    fn test_extract_mcid_from_dict_missing() {
        let dict = IndexMap::new();
        assert_eq!(extract_mcid_from_dict(&dict), None);
    }

    #[test]
    fn test_extract_mcid_from_dict_negative() {
        let mut dict = IndexMap::new();
        dict.insert(intern("/MCID"), PdfObject::Integer(-1));

        assert_eq!(extract_mcid_from_dict(&dict), None);
    }

    #[test]
    fn test_extract_mcid_from_dict_zero() {
        let mut dict = IndexMap::new();
        dict.insert(intern("/MCID"), PdfObject::Integer(0));

        assert_eq!(extract_mcid_from_dict(&dict), Some(0));
    }

    #[test]
    fn test_extract_mcid_from_dict_real_whole() {
        let mut dict = IndexMap::new();
        dict.insert(intern("/MCID"), PdfObject::Real(42.0));

        assert_eq!(extract_mcid_from_dict(&dict), Some(42));
    }

    #[test]
    fn test_extract_mcid_from_dict_real_fractional() {
        let mut dict = IndexMap::new();
        dict.insert(intern("/MCID"), PdfObject::Real(42.5));

        assert_eq!(extract_mcid_from_dict(&dict), None);
    }

    #[test]
    fn test_emit_invalid_bdc_operand() {
        let mut diagnostics = Vec::new();
        emit_invalid_bdc_operand(&mut diagnostics, &PdfObject::Integer(42));

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagCode::StructInvalidBdcOperand);
    }

    #[test]
    fn test_emit_unknown_property_name() {
        let mut diagnostics = Vec::new();
        emit_unknown_property_name(&mut diagnostics, "UnknownProps");

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagCode::UnknownMarkedContentProps);
    }

    #[test]
    fn test_nested_marked_content() {
        let mut stack = MarkedContentStack::new();

        // Outer BDC with MCID
        let mut props1 = IndexMap::new();
        props1.insert(intern("/MCID"), PdfObject::Integer(1));
        parse_bdc(
            &mut stack,
            Arc::from("P"),
            &PdfObject::Dict(Box::new(props1)),
            &ResourceDict::new(),
            None,
            None,
        );

        // Inner BMC
        parse_bmc(&mut stack, Arc::from("Span"));

        assert_eq!(stack.depth(), 2);
        assert_eq!(stack.innermost_mcid(), Some(1)); // Outer MCID still visible

        // Pop inner
        parse_emc(&mut stack);
        assert_eq!(stack.depth(), 1);
        assert_eq!(stack.innermost_mcid(), Some(1));

        // Pop outer
        parse_emc(&mut stack);
        assert!(stack.is_empty());
    }

    #[test]
    fn test_bmc_tag_leading_slash() {
        let mut stack = MarkedContentStack::new();
        parse_bmc(&mut stack, Arc::from("/Span"));

        assert_eq!(stack.depth(), 1);
        // The tag should include the leading slash as-is (caller's responsibility to strip)
        assert_eq!(stack.innermost_frame().unwrap().tag, "/Span");
    }

    #[test]
    fn test_bdc_tag_leading_slash() {
        let mut stack = MarkedContentStack::new();
        let mut props = IndexMap::new();
        props.insert(intern("/MCID"), PdfObject::Integer(5));

        parse_bdc(
            &mut stack,
            Arc::from("/P"),
            &PdfObject::Dict(Box::new(props)),
            &ResourceDict::new(),
            None,
            None,
        );

        assert_eq!(stack.depth(), 1);
        assert_eq!(stack.innermost_frame().unwrap().tag, "/P");
        assert_eq!(stack.innermost_mcid(), Some(5));
    }

    #[test]
    fn test_stack_depth_limit() {
        let mut stack = MarkedContentStack::new();

        // Fill to max depth
        for i in 0..64 {
            assert!(parse_bmc(&mut stack, Arc::from(format!("frame{}", i))));
        }

        // 65th should fail
        assert!(!parse_bmc(&mut stack, Arc::from("overflow")));
        assert_eq!(stack.depth(), 64);
    }

    #[test]
    fn test_parse_bdc_with_real_mcid_large() {
        let mut stack = MarkedContentStack::new();
        let mut props = IndexMap::new();
        props.insert(intern("/MCID"), PdfObject::Integer(10000));

        assert!(parse_bdc(
            &mut stack,
            Arc::from("P"),
            &PdfObject::Dict(Box::new(props)),
            &ResourceDict::new(),
            None,
            None
        ));
        assert_eq!(stack.innermost_mcid(), Some(10000));
    }

    #[test]
    fn test_parse_bdc_oc_tag_not_ocg() {
        let mut stack = MarkedContentStack::new();
        let mut props = IndexMap::new();
        props.insert(intern("/MCID"), PdfObject::Integer(5));

        // /OC tag without /OCG property should not be hidden
        assert!(parse_bdc(
            &mut stack,
            Arc::from("OC"),
            &PdfObject::Dict(Box::new(props)),
            &ResourceDict::new(),
            None,
            None
        ));
        assert_eq!(stack.depth(), 1);
        assert!(!stack.is_hidden()); // No /OCG, not hidden
    }

    #[test]
    fn test_parse_bdc_oc_tag_with_ocg_not_in_off_set() {
        let mut stack = MarkedContentStack::new();
        let mut props = IndexMap::new();
        let ocg_ref = ObjRef::new(10, 0);
        props.insert(intern("/OCG"), PdfObject::Ref(ocg_ref));

        // Create OFF set that doesn't include this OCG
        let mut off_set = std::collections::HashSet::new();
        off_set.insert(ObjRef::new(99, 0)); // Different OCG

        assert!(parse_bdc(
            &mut stack,
            Arc::from("OC"),
            &PdfObject::Dict(Box::new(props)),
            &ResourceDict::new(),
            Some(&off_set),
            None
        ));
        assert_eq!(stack.depth(), 1);
        assert!(!stack.is_hidden()); // OCG not in OFF set
    }

    #[test]
    fn test_parse_bdc_oc_tag_with_ocg_in_off_set() {
        let mut stack = MarkedContentStack::new();
        let mut props = IndexMap::new();
        let ocg_ref = ObjRef::new(10, 0);
        props.insert(intern("/OCG"), PdfObject::Ref(ocg_ref));

        // Create OFF set that includes this OCG
        let mut off_set = std::collections::HashSet::new();
        off_set.insert(ocg_ref);

        assert!(parse_bdc(
            &mut stack,
            Arc::from("OC"),
            &PdfObject::Dict(Box::new(props)),
            &ResourceDict::new(),
            Some(&off_set),
            None
        ));
        assert_eq!(stack.depth(), 1);
        assert!(stack.is_hidden()); // OCG in OFF set
    }

    #[test]
    fn test_parse_bdc_slash_oc_tag() {
        let mut stack = MarkedContentStack::new();
        let mut props = IndexMap::new();
        let ocg_ref = ObjRef::new(10, 0);
        props.insert(intern("/OCG"), PdfObject::Ref(ocg_ref));

        // Create OFF set that includes this OCG
        let mut off_set = std::collections::HashSet::new();
        off_set.insert(ocg_ref);

        // Test with /OC (leading slash)
        assert!(parse_bdc(
            &mut stack,
            Arc::from("/OC"),
            &PdfObject::Dict(Box::new(props)),
            &ResourceDict::new(),
            Some(&off_set),
            None,
        ));
        assert_eq!(stack.depth(), 1);
        assert!(stack.is_hidden()); // /OC with leading slash works
    }

    #[test]
    fn test_parse_bdc_non_oc_tag_ignores_ocg_property() {
        let mut stack = MarkedContentStack::new();
        let mut props = IndexMap::new();
        let ocg_ref = ObjRef::new(10, 0);
        props.insert(intern("/OCG"), PdfObject::Ref(ocg_ref));
        props.insert(intern("/MCID"), PdfObject::Integer(5));

        // Create OFF set that includes this OCG
        let mut off_set = std::collections::HashSet::new();
        off_set.insert(ocg_ref);

        // Non-OC tag should not check OCG
        assert!(parse_bdc(
            &mut stack,
            Arc::from("P"), // Not "OC" or "/OC"
            &PdfObject::Dict(Box::new(props)),
            &ResourceDict::new(),
            Some(&off_set),
            None,
        ));
        assert_eq!(stack.depth(), 1);
        assert!(!stack.is_hidden()); // Non-OC tag ignores OCG
        assert_eq!(stack.innermost_mcid(), Some(5)); // MCID still extracted
    }
}
