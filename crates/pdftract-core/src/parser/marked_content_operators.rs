//! BMC/BDC/EMC operator parsers for marked content.
//!
//! This module implements the three marked-content operators that consume
//! operands from the operand stack and dispatch to the marked-content stack.
//!
//! Per PDF spec section 14.5:
//! - BMC /Tag: begin marked content with tag only
//! - BDC /Tag `<<props>>` or BDC /Tag /PropName: begin marked content with properties
//! - EMC: end marked content (pop top frame)

use crate::diagnostics::{DiagCode, Diagnostic};
use crate::parser::marked_content_stack::{MarkedContentFrame, MarkedContentStack};
use crate::parser::object::PdfObject;
use crate::parser::resources::ResourceDict;
use crate::parser::xref::XrefResolver;
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
    resolver: Option<&XrefResolver>,
) -> bool {
    let mcid = extract_mcid_from_props(props, resources, diagnostics, resolver);

    // Check for OCG /OC tag (bead pdftract-1q19p)
    let is_hidden = if tag.as_ref() == "OC" || tag.as_ref() == "/OC" {
        // Check if props dict has /OCG reference
        if let Some(ocg_ref) = extract_ocg_ref_from_props(props, resources, resolver) {
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
    resolver: Option<&XrefResolver>,
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
                Some(obj_ref) => {
                    // We have an ObjRef - resolve it if we have a resolver
                    if let Some(resolver) = resolver {
                        match resolver.resolve(obj_ref) {
                            Ok(resolved_obj) => {
                                // Extract MCID from the resolved object
                                match resolved_obj {
                                    PdfObject::Dict(dict) => extract_mcid_from_dict(&*dict),
                                    _ => {
                                        // Resolved object is not a dictionary
                                        if let Some(diags) = diagnostics {
                                            diags.push(Diagnostic::with_dynamic_no_offset(
                                                DiagCode::StructInvalidBdcOperand,
                                                format!(
                                                    "BDC property '{}' resolved to non-dict object",
                                                    name_str
                                                ),
                                            ));
                                        }
                                        None
                                    }
                                }
                            }
                            Err(e) => {
                                // Failed to resolve the reference
                                if let Some(diags) = diagnostics {
                                    diags.push(Diagnostic::with_dynamic_no_offset(
                                        DiagCode::StructInvalidBdcOperand,
                                        format!(
                                            "BDC property '{}' failed to resolve: {}",
                                            name_str, e
                                        ),
                                    ));
                                }
                                None
                            }
                        }
                    } else {
                        // No resolver available - emit diagnostic and return None
                        if let Some(diags) = diagnostics {
                            diags.push(Diagnostic::with_dynamic_no_offset(
                                DiagCode::StructInvalidBdcOperand,
                                format!(
                                    "BDC property '{}' is indirect reference but no resolver available",
                                    name_str
                                ),
                            ));
                        }
                        None
                    }
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
fn extract_ocg_ref_from_props(
    props: &PdfObject,
    resources: &ResourceDict,
    resolver: Option<&XrefResolver>,
) -> Option<crate::parser::object::ObjRef> {
    match props {
        PdfObject::Dict(dict) => {
            // Inline property dict - check for /OCG key
            dict.get("/OCG").and_then(|obj| obj.as_ref())
        }
        PdfObject::Name(name) => {
            // Property resource name - look up in /Properties
            let name_str: &str = name.as_ref();
            let name_str = name_str.strip_prefix('/').unwrap_or(name_str);

            if let Some(obj_ref) = resources.lookup_properties(name_str) {
                // Resolve the property dictionary to extract /OCG
                if let Some(resolver) = resolver {
                    match resolver.resolve(obj_ref) {
                        Ok(resolved_obj) => {
                            match resolved_obj {
                                PdfObject::Dict(dict) => {
                                    // Extract /OCG from the resolved dictionary
                                    dict.get("/OCG").and_then(|obj| obj.as_ref())
                                }
                                _ => None,
                            }
                        }
                        Err(_) => None,
                    }
                } else {
                    // No resolver available
                    None
                }
            } else {
                None
            }
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
    use crate::parser::ocg::parse_oc_properties;
    use crate::parser::xref::XrefEntry;
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
            None,
            None,
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
            None,
            None,
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
            None,
            None,
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
            Some(&mut diagnostics),
            None,
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
            None,
            None,
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
            None,
            None,
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
            None,
            None,
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
            None,
            None,
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
            None,
        ));
        assert_eq!(stack.depth(), 1);
        assert!(stack.is_hidden()); // /OC with leading slash works
    }

    // ------------------------------------------------------------------
    // OCG visibility integration: catalog → OcProperties → parse_bdc.
    //
    // These tests chain the two OCG fixes end-to-end: a real
    // /OCProperties dictionary is parsed via parse_oc_properties (the
    // path the catalog parser takes), its off_ocg_set() output is fed
    // into parse_bdc's default_off_ocgs parameter, and OC-marked frames
    // whose /OCG is OFF must come out hidden. Without the parse_bdc
    // default_off_ocgs wiring (pdftract-0af80cff) or the off_ocg_set
    // derivation from parsed state (pdftract-7ccf3d1e), the hidden
    // assertions below fail.
    // ------------------------------------------------------------------

    /// A resolver primed with two OCG objects and an /OCProperties dict.
    ///
    /// `on_ref` is default-visible (BaseState /ON, not in any OFF array);
    /// `off_ref` is listed in /D /OFF. The /OCProperties dict is cached
    /// under obj 1 gen 0, mirroring the fixture pattern used by the
    /// ocg.rs tests.
    fn ocg_catalog_fixture(on_ref: ObjRef, off_ref: ObjRef) -> XrefResolver {
        let resolver = XrefResolver::new();

        for (r, name) in [(on_ref, "VisibleLayer"), (off_ref, "HiddenLayer")] {
            let mut ocg_dict = IndexMap::new();
            ocg_dict.insert(intern("Type"), PdfObject::Name(intern("OCG")));
            ocg_dict.insert(
                intern("Name"),
                PdfObject::String(Box::new(name.as_bytes().to_vec())),
            );
            resolver.cache_object(r, PdfObject::Dict(Box::new(ocg_dict)));
        }

        let mut default_config = IndexMap::new();
        default_config.insert(intern("BaseState"), PdfObject::Name(intern("ON")));
        default_config.insert(
            intern("OFF"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(off_ref)])),
        );

        let mut oc_props_dict = IndexMap::new();
        oc_props_dict.insert(
            intern("OCGs"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Ref(on_ref),
                PdfObject::Ref(off_ref),
            ])),
        );
        oc_props_dict.insert(intern("D"), PdfObject::Dict(Box::new(default_config)));

        resolver.cache_object(ObjRef::new(1, 0), PdfObject::Dict(Box::new(oc_props_dict)));
        resolver
    }

    #[test]
    fn test_off_ocg_set_from_parsed_ocproperties_fixture() {
        let on_ref = ObjRef::new(10, 0);
        let off_ref = ObjRef::new(11, 0);
        let resolver = ocg_catalog_fixture(on_ref, off_ref);

        let oc_props = parse_oc_properties(&resolver, Some(ObjRef::new(1, 0)));
        assert!(oc_props.present);
        assert!(oc_props.diagnostics.is_empty());

        // off_ocg_set() must contain exactly the OFF-listed OCG
        let off_set = oc_props.off_ocg_set();
        assert_eq!(off_set.len(), 1);
        assert!(off_set.contains(&off_ref));
        assert!(!off_set.contains(&on_ref));

        // and agree with is_visible on both refs
        assert!(oc_props.is_visible(on_ref));
        assert!(!oc_props.is_visible(off_ref));
    }

    #[test]
    fn test_bdc_integration_ocg_visibility_from_parsed_catalog() {
        let on_ref = ObjRef::new(10, 0);
        let off_ref = ObjRef::new(11, 0);
        let resolver = ocg_catalog_fixture(on_ref, off_ref);

        // Catalog parse produces the OFF set that content parsing consumes
        let off_set = parse_oc_properties(&resolver, Some(ObjRef::new(1, 0))).off_ocg_set();
        assert_eq!(off_set.len(), 1);

        // OFF-marked content: hidden
        let mut stack = MarkedContentStack::new();
        assert!(parse_bdc(
            &mut stack,
            Arc::from("OC"),
            &ocg_props(off_ref),
            &ResourceDict::new(),
            Some(&off_set),
            None,
            None,
        ));
        assert!(stack.innermost_frame().unwrap().is_hidden);
        assert!(stack.is_hidden());

        // Content nested under the hidden frame inherits the hidden flag
        assert!(parse_bmc(&mut stack, Arc::from("Span")));
        assert!(stack.is_hidden());
        assert!(parse_emc(&mut stack).is_some()); // pop Span

        // Pop the hidden OC frame so the next frame stands alone
        assert!(parse_emc(&mut stack).is_some());
        assert!(!stack.is_hidden());

        // ON-marked content in the same stream: visible
        assert!(parse_bdc(
            &mut stack,
            Arc::from("OC"),
            &ocg_props(on_ref),
            &ResourceDict::new(),
            Some(&off_set),
            None,
            None,
        ));
        assert!(!stack.innermost_frame().unwrap().is_hidden);
        assert!(!stack.is_hidden());
    }

    #[test]
    fn test_bdc_oc_tag_without_ocg_property_never_hidden_even_with_off_set() {
        let mut stack = MarkedContentStack::new();

        // Non-empty OFF set, but the props carry no /OCG key at all
        let off_set: std::collections::HashSet<ObjRef> =
            (0..3).map(|i| ObjRef::new(i, 0)).collect();

        assert!(parse_bdc(
            &mut stack,
            Arc::from("OC"),
            &mcid_props(7),
            &ResourceDict::new(),
            Some(&off_set),
            None,
            None,
        ));
        assert!(!stack.innermost_frame().unwrap().is_hidden);
        assert!(!stack.is_hidden());
        assert_eq!(stack.innermost_mcid(), Some(7));
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
            None,
        ));
        assert_eq!(stack.depth(), 1);
        assert!(!stack.is_hidden()); // Non-OC tag ignores OCG
        assert_eq!(stack.innermost_mcid(), Some(5)); // MCID still extracted
    }

    // ------------------------------------------------------------------
    // Resolver-path coverage for BDC property resolution.
    //
    // `XrefResolver::resolve` consults the object cache first, so the
    // success paths prime it with `cache_object` (the pattern used by the
    // xref.rs tests). Beyond the cache, `resolve` is a stub: it returns
    // Null for a known InUse entry and NotFound when no entry exists. The
    // stub test pins that behaviour so a future implementation change in
    // resolution is loud rather than silent.
    //
    // Coverage map (bead pdftract-9ca0bb06) — every DiagCode::
    // StructInvalidBdcOperand branch of extract_mcid_from_props /
    // extract_ocg_ref_from_props is exercised:
    //   resolver + ObjRef -> Dict(/MCID)   test_mcid_resolver_resolves_cached_dict_with_mcid
    //   resolver + ObjRef -> non-dict      test_mcid_resolver_resolved_non_dict_emits_diagnostic
    //   resolver resolve() -> Err          test_mcid_resolver_error_emits_diagnostic
    //   no resolver, /Properties hit       test_mcid_no_resolver_emits_no_resolver_diagnostic
    //   /OCG mirror of the above           test_ocg_name_{resolves_cached_dict_with_ocg,
    //                                        resolved_dict_without_ocg, resolved_non_dict,
    //                                        resolver_error, no_resolver, lookup_miss}
    //   end-to-end parse_bdc wiring        test_parse_bdc_property_name_{resolves_mcid_via_resolver,
    //                                        no_resolver_emits_diagnostic}
    // ------------------------------------------------------------------

    /// A property dict containing `/MCID`.
    fn mcid_props(mcid: i64) -> PdfObject {
        let mut dict = IndexMap::new();
        dict.insert(intern("/MCID"), PdfObject::Integer(mcid));
        PdfObject::Dict(Box::new(dict))
    }

    /// A property dict containing `/OCG` pointing at an indirect reference.
    fn ocg_props(ocg: ObjRef) -> PdfObject {
        let mut dict = IndexMap::new();
        dict.insert(intern("/OCG"), PdfObject::Ref(ocg));
        PdfObject::Dict(Box::new(dict))
    }

    /// A resource dict whose /Properties maps `name` to `obj_ref`.
    fn resources_with(name: &str, obj_ref: ObjRef) -> ResourceDict {
        let mut resources = ResourceDict::new();
        resources.properties.insert(Arc::from(name), obj_ref);
        resources
    }

    #[test]
    fn test_mcid_resolver_resolves_cached_dict_with_mcid() {
        let resolver = XrefResolver::new();
        let obj_ref = ObjRef::new(10, 0);
        resolver.cache_object(obj_ref, mcid_props(42));

        let resources = resources_with("MyProps", obj_ref);
        let mut diagnostics = Vec::new();

        let mcid = extract_mcid_from_props(
            &PdfObject::Name(Arc::from("MyProps")),
            &resources,
            Some(&mut diagnostics),
            Some(&resolver),
        );

        assert_eq!(mcid, Some(42));
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_mcid_resolver_resolved_non_dict_emits_diagnostic() {
        let resolver = XrefResolver::new();
        let obj_ref = ObjRef::new(10, 0);
        resolver.cache_object(obj_ref, PdfObject::Integer(7));

        let resources = resources_with("MyProps", obj_ref);
        let mut diagnostics = Vec::new();

        let mcid = extract_mcid_from_props(
            &PdfObject::Name(Arc::from("MyProps")),
            &resources,
            Some(&mut diagnostics),
            Some(&resolver),
        );

        assert_eq!(mcid, None);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagCode::StructInvalidBdcOperand);
        assert!(diagnostics[0].message.contains("MyProps"));
        assert!(diagnostics[0].message.contains("non-dict"));
    }

    #[test]
    fn test_mcid_resolver_stub_returns_null_for_unprimed_entry() {
        // Pins the current `XrefResolver::resolve` stub: an InUse entry with
        // no cached object resolves to Null, which the BDC path reports as a
        // resolved-to-non-dict property. Revisit when resolve() learns to
        // read from a source.
        let mut resolver = XrefResolver::new();
        resolver.add_entry(
            10,
            XrefEntry::InUse {
                offset: 128,
                gen_nr: 0,
            },
        );

        let resources = resources_with("MyProps", ObjRef::new(10, 0));
        let mut diagnostics = Vec::new();

        let mcid = extract_mcid_from_props(
            &PdfObject::Name(Arc::from("MyProps")),
            &resources,
            Some(&mut diagnostics),
            Some(&resolver),
        );

        assert_eq!(mcid, None);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagCode::StructInvalidBdcOperand);
        assert!(diagnostics[0].message.contains("non-dict"));
    }

    #[test]
    fn test_mcid_resolver_error_emits_diagnostic() {
        // No cached object and no xref entry -> resolve() fails.
        let resolver = XrefResolver::new();
        let resources = resources_with("MyProps", ObjRef::new(10, 0));
        let mut diagnostics = Vec::new();

        let mcid = extract_mcid_from_props(
            &PdfObject::Name(Arc::from("MyProps")),
            &resources,
            Some(&mut diagnostics),
            Some(&resolver),
        );

        assert_eq!(mcid, None);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagCode::StructInvalidBdcOperand);
        assert!(diagnostics[0].message.contains("MyProps"));
        assert!(diagnostics[0].message.contains("failed to resolve"));
    }

    #[test]
    fn test_mcid_no_resolver_emits_no_resolver_diagnostic() {
        let resources = resources_with("MyProps", ObjRef::new(10, 0));
        let mut diagnostics = Vec::new();

        let mcid = extract_mcid_from_props(
            &PdfObject::Name(Arc::from("MyProps")),
            &resources,
            Some(&mut diagnostics),
            None,
        );

        assert_eq!(mcid, None);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagCode::StructInvalidBdcOperand);
        assert!(diagnostics[0].message.contains("MyProps"));
        assert!(diagnostics[0].message.contains("no resolver available"));
    }

    #[test]
    fn test_mcid_inline_dict_direct() {
        let resources = ResourceDict::new();

        assert_eq!(
            extract_mcid_from_props(&mcid_props(9), &resources, None, None),
            Some(9)
        );
    }

    #[test]
    fn test_ocg_inline_dict_returns_ref() {
        let ocg_ref = ObjRef::new(20, 0);
        let resources = ResourceDict::new();

        assert_eq!(
            extract_ocg_ref_from_props(&ocg_props(ocg_ref), &resources, None),
            Some(ocg_ref)
        );
    }

    #[test]
    fn test_ocg_name_resolves_cached_dict_with_ocg() {
        let resolver = XrefResolver::new();
        let props_ref = ObjRef::new(10, 0);
        let ocg_ref = ObjRef::new(20, 0);
        resolver.cache_object(props_ref, ocg_props(ocg_ref));

        let resources = resources_with("MyProps", props_ref);

        assert_eq!(
            extract_ocg_ref_from_props(
                &PdfObject::Name(Arc::from("MyProps")),
                &resources,
                Some(&resolver)
            ),
            Some(ocg_ref)
        );
    }

    #[test]
    fn test_ocg_name_resolved_dict_without_ocg() {
        let resolver = XrefResolver::new();
        let props_ref = ObjRef::new(10, 0);
        resolver.cache_object(props_ref, mcid_props(3));

        let resources = resources_with("MyProps", props_ref);

        assert_eq!(
            extract_ocg_ref_from_props(
                &PdfObject::Name(Arc::from("MyProps")),
                &resources,
                Some(&resolver)
            ),
            None
        );
    }

    #[test]
    fn test_ocg_name_resolved_non_dict() {
        let resolver = XrefResolver::new();
        let props_ref = ObjRef::new(10, 0);
        resolver.cache_object(props_ref, PdfObject::Integer(1));

        let resources = resources_with("MyProps", props_ref);

        assert_eq!(
            extract_ocg_ref_from_props(
                &PdfObject::Name(Arc::from("MyProps")),
                &resources,
                Some(&resolver)
            ),
            None
        );
    }

    #[test]
    fn test_ocg_name_resolver_error() {
        // No cached object and no xref entry -> resolve() fails.
        let resolver = XrefResolver::new();
        let resources = resources_with("MyProps", ObjRef::new(10, 0));

        assert_eq!(
            extract_ocg_ref_from_props(
                &PdfObject::Name(Arc::from("MyProps")),
                &resources,
                Some(&resolver)
            ),
            None
        );
    }

    #[test]
    fn test_ocg_name_no_resolver() {
        let resources = resources_with("MyProps", ObjRef::new(10, 0));

        assert_eq!(
            extract_ocg_ref_from_props(&PdfObject::Name(Arc::from("MyProps")), &resources, None),
            None
        );
    }

    #[test]
    fn test_ocg_name_lookup_miss() {
        let resolver = XrefResolver::new();
        let resources = ResourceDict::new();

        assert_eq!(
            extract_ocg_ref_from_props(
                &PdfObject::Name(Arc::from("Missing")),
                &resources,
                Some(&resolver)
            ),
            None
        );
    }

    #[test]
    fn test_parse_bdc_property_name_resolves_mcid_via_resolver() {
        let mut stack = MarkedContentStack::new();
        let resolver = XrefResolver::new();
        let obj_ref = ObjRef::new(10, 0);
        resolver.cache_object(obj_ref, mcid_props(42));
        let resources = resources_with("MyProps", obj_ref);

        assert!(parse_bdc(
            &mut stack,
            Arc::from("P"),
            &PdfObject::Name(Arc::from("MyProps")),
            &resources,
            None,
            None,
            Some(&resolver),
        ));
        assert_eq!(stack.depth(), 1);
        assert_eq!(stack.innermost_mcid(), Some(42));
    }

    #[test]
    fn test_parse_bdc_property_name_no_resolver_emits_diagnostic() {
        let mut stack = MarkedContentStack::new();
        let resources = resources_with("MyProps", ObjRef::new(10, 0));
        let mut diagnostics = Vec::new();

        assert!(parse_bdc(
            &mut stack,
            Arc::from("P"),
            &PdfObject::Name(Arc::from("MyProps")),
            &resources,
            None,
            Some(&mut diagnostics),
            None,
        ));
        assert_eq!(stack.depth(), 1);
        assert_eq!(stack.innermost_mcid(), None);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagCode::StructInvalidBdcOperand);
        assert!(diagnostics[0].message.contains("no resolver available"));
    }
}
