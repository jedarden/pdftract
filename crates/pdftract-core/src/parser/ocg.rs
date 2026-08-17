//! Optional Content Groups (OCG) parser.
//!
//! This module handles parsing of `/OCProperties` from the document catalog,
//! including OCG groups, default visibility resolution, and optional content
//! membership dictionaries (OCMD).
//!
//! PDF 2.0 spec reference: ISO 32000-2 §8.11 (Optional Content)

use std::collections::HashMap;

use crate::parser::object::{ObjRef, PdfDict, PdfObject};
use crate::parser::xref::XrefResolver;
use crate::parser::{DiagCode, Diagnostic};

/// Base state for OCG visibility in the default configuration.
///
/// Represents the `/BaseState` entry in the default configuration dictionary `/D`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseState {
    /// All OCGs are ON by default
    On,
    /// All OCGs are OFF by default
    Off,
    /// Unchanged state (treat as ON for default config)
    Unchanged,
}

impl BaseState {
    /// Parse a BaseState from a name object.
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "ON" => Some(BaseState::On),
            "OFF" => Some(BaseState::Off),
            "Unchanged" => Some(BaseState::Unchanged),
            _ => None,
        }
    }

    /// Get the boolean visibility value for this base state.
    ///
    /// Per spec, `Unchanged` is treated as `ON` for the default configuration.
    fn as_bool(self) -> bool {
        match self {
            BaseState::On => true,
            BaseState::Off => false,
            BaseState::Unchanged => true,
        }
    }
}

/// Policy for an Optional Content Membership Dictionary (OCMD).
///
/// OCMDs express boolean combinations of OCG states. This enum represents
/// the `/P` entry in an OCMD dictionary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcmdPolicy {
    /// Visible iff all listed OCGs are ON
    AllOn,
    /// Visible iff all listed OCGs are OFF
    AllOff,
    /// Visible iff any listed OCG is ON
    AnyOn,
    /// Visible iff any listed OCG is OFF
    AnyOff,
}

impl OcmdPolicy {
    /// Parse a policy from a name object.
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "AllOn" => Some(OcmdPolicy::AllOn),
            "AllOff" => Some(OcmdPolicy::AllOff),
            "AnyOn" => Some(OcmdPolicy::AnyOn),
            "AnyOff" => Some(OcmdPolicy::AnyOff),
            _ => None,
        }
    }
}

/// An Optional Content Membership Dictionary (OCMD).
///
/// OCMDs express boolean combinations of OCG states. They are referenced
/// from content streams via the `/OC` property in marked content sequences.
#[derive(Debug, Clone)]
pub struct Ocmd {
    /// The OCGs referenced by this OCMD
    pub ocgs: Vec<ObjRef>,
    /// The visibility policy
    pub policy: OcmdPolicy,
}

impl Ocmd {
    /// Create a new OCMD.
    pub fn new(ocgs: Vec<ObjRef>, policy: OcmdPolicy) -> Self {
        Ocmd { ocgs, policy }
    }

    /// Parse an OCMD from a PdfObject.
    fn parse(obj: &PdfObject) -> Option<Self> {
        let dict = obj.as_dict()?;

        // Parse /OCGs (can be a single ref or an array)
        let ocgs = match dict.get("OCGs") {
            Some(PdfObject::Ref(ref_)) => vec![*ref_],
            Some(PdfObject::Array(arr)) => arr.iter().filter_map(|o| o.as_ref()).collect(),
            _ => return None,
        };

        // Parse /P (policy; defaults to AnyOn if absent per spec)
        let policy = dict
            .get("P")
            .and_then(|o| o.as_name())
            .and_then(OcmdPolicy::from_name)
            .unwrap_or(OcmdPolicy::AnyOn);

        Some(Ocmd::new(ocgs, policy))
    }
}

/// An Optional Content Group (OCG).
///
/// OCGs are named, independently togglable layers in a PDF document.
#[derive(Debug, Clone)]
pub struct OcGroup {
    /// Human-readable name from /Name
    pub name: Option<String>,
    /// Intent(s) from /Intent (e.g., "View", "Design")
    pub intent: Vec<String>,
    /// Usage dictionary from /Usage (informational)
    pub usage: Option<PdfDict>,
}

impl OcGroup {
    /// Create a new OcGroup.
    pub fn new() -> Self {
        OcGroup {
            name: None,
            intent: Vec::new(),
            usage: None,
        }
    }

    /// Parse an OcGroup from a PdfObject.
    fn parse(obj: &PdfObject, _diagnostics: &mut Vec<Diagnostic>) -> Self {
        let mut group = OcGroup::new();

        let dict = match obj.as_dict() {
            Some(d) => d,
            None => return group,
        };

        // Parse /Name (required per spec, but we handle missing)
        if let Some(name_obj) = dict.get("Name") {
            group.name = name_obj
                .as_string()
                .or_else(|| name_obj.as_name().map(|s| s.as_bytes()))
                .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());
        }

        // Parse /Intent (optional; can be a name or array)
        if let Some(intent_obj) = dict.get("Intent") {
            group.intent = match intent_obj {
                PdfObject::Name(name) => vec![name.to_string()],
                PdfObject::Array(arr) => arr
                    .iter()
                    .filter_map(|o| o.as_name().map(|s| s.to_string()))
                    .collect(),
                _ => Vec::new(),
            };
        }

        // Parse /Usage (optional; keep as dict for informational purposes)
        if let Some(PdfObject::Dict(usage_dict)) = dict.get("Usage") {
            group.usage = Some((**usage_dict).clone());
        }

        group
    }
}

impl Default for OcGroup {
    fn default() -> Self {
        Self::new()
    }
}

/// Optional Content Properties from the document catalog.
///
/// This struct contains all OCG-related information from `/OCProperties`,
/// including the default visibility map for all OCGs.
#[derive(Debug, Clone)]
pub struct OcProperties {
    /// True if /OCProperties was present in the catalog
    pub present: bool,
    /// All OCGs in the document, keyed by their object reference
    pub groups: HashMap<ObjRef, OcGroup>,
    /// Default visibility state for each OCG
    pub default_visibility: HashMap<ObjRef, bool>,
    /// Overall base state (ON/OFF/Unchanged)
    pub base_state: BaseState,
    /// Optional Content Membership Dictionaries (OCMDs) indexed by their ref
    pub ocmds: HashMap<ObjRef, Ocmd>,
    /// Diagnostics emitted during parsing
    pub diagnostics: Vec<Diagnostic>,
}

impl OcProperties {
    /// Create a new OcProperties with present=false (no /OCProperties in catalog).
    pub fn not_present() -> Self {
        OcProperties {
            present: false,
            groups: HashMap::new(),
            default_visibility: HashMap::new(),
            base_state: BaseState::On,
            ocmds: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Check if an OCG is visible by default.
    ///
    /// Returns true if the OCG is ON in the default configuration,
    /// false if OFF. If the OCG is not in the visibility map, returns
    /// the base state (treats unknown OCGs as visible per spec).
    pub fn is_visible(&self, ocg_ref: ObjRef) -> bool {
        self.default_visibility
            .get(&ocg_ref)
            .copied()
            .unwrap_or_else(|| self.base_state.as_bool())
    }

    /// Get the set of OCG refs that are OFF by default.
    ///
    /// Returns a HashSet containing all OCG object references that are
    /// OFF in the default configuration. Used by content stream parsing
    /// to determine visibility of marked content sequences.
    pub fn off_ocg_set(&self) -> std::collections::HashSet<ObjRef> {
        self.default_visibility
            .iter()
            .filter_map(|(&ref_, &visible)| if !visible { Some(ref_) } else { None })
            .collect()
    }

    /// Check if an OCMD is visible by default.
    ///
    /// Evaluates the OCMD's policy against the current visibility states.
    /// Returns true if visible, false if not.
    pub fn is_ocmd_visible(&self, ocmd_ref: ObjRef) -> bool {
        let ocmd = match self.ocmds.get(&ocmd_ref) {
            Some(o) => o,
            None => return true, // Unknown OCMD treated as visible
        };

        self.evaluate_ocmd_policy(ocmd)
    }

    /// Evaluate an OCMD policy against current OCG states.
    fn evaluate_ocmd_policy(&self, ocmd: &Ocmd) -> bool {
        let ocg_states: Vec<bool> = ocmd
            .ocgs
            .iter()
            .map(|&ref_| self.is_visible(ref_))
            .collect();

        match ocmd.policy {
            OcmdPolicy::AllOn => ocg_states.iter().all(|&v| v),
            OcmdPolicy::AllOff => ocg_states.iter().all(|&v| !v),
            OcmdPolicy::AnyOn => ocg_states.iter().any(|&v| v),
            OcmdPolicy::AnyOff => ocg_states.iter().any(|&v| !v),
        }
    }

    /// Get the name of an OCG by its reference.
    pub fn ocg_name(&self, ocg_ref: ObjRef) -> Option<&str> {
        self.groups.get(&ocg_ref)?.name.as_deref()
    }
}

impl Default for OcProperties {
    fn default() -> Self {
        Self::not_present()
    }
}

/// Parse `/OCProperties` from the catalog.
///
/// # Arguments
/// * `resolver` - The xref resolver for resolving indirect references
/// * `oc_props_ref` - The object reference to /OCProperties (None if not present)
///
/// # Returns
/// An `OcProperties` struct containing the parsed OCG information.
/// If `oc_props_ref` is None, returns `OcProperties::not_present()`.
pub fn parse_oc_properties(resolver: &XrefResolver, oc_props_ref: Option<ObjRef>) -> OcProperties {
    let oc_props_ref = match oc_props_ref {
        Some(r) => r,
        None => return OcProperties::not_present(),
    };

    let mut diagnostics = Vec::new();
    let mut oc_properties = OcProperties {
        present: true,
        groups: HashMap::new(),
        default_visibility: HashMap::new(),
        base_state: BaseState::On,
        ocmds: HashMap::new(),
        diagnostics: Vec::new(),
    };

    // Resolve the /OCProperties dictionary
    let oc_props_obj = match resolver.resolve(oc_props_ref) {
        Ok(obj) => obj,
        Err(e) => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!("Failed to resolve /OCProperties: {}", e),
            ));
            oc_properties.diagnostics = diagnostics;
            return oc_properties;
        }
    };

    let oc_props_dict = match oc_props_obj.as_dict() {
        Some(d) => d,
        None => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!(
                    "/OCProperties is not a dictionary (type: {})",
                    oc_props_obj.type_name()
                ),
            ));
            oc_properties.diagnostics = diagnostics;
            return oc_properties;
        }
    };

    // Parse /OCGs array (required per spec)
    let ocg_refs: Vec<ObjRef> = match oc_props_dict.get("OCGs") {
        Some(PdfObject::Array(arr)) => arr.iter().filter_map(|o| o.as_ref()).collect(),
        Some(other) => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!("/OCGs is not an array (type: {})", other.type_name()),
            ));
            oc_properties.diagnostics = diagnostics;
            return oc_properties;
        }
        None => {
            diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::StructMissingKey,
                "/OCGs key missing from /OCProperties",
            ));
            oc_properties.diagnostics = diagnostics;
            return oc_properties;
        }
    };

    // Parse each OCG dictionary
    for &ocg_ref in &ocg_refs {
        match resolver.resolve(ocg_ref) {
            Ok(ocg_obj) => {
                let group = OcGroup::parse(&ocg_obj, &mut diagnostics);
                oc_properties.groups.insert(ocg_ref, group);
            }
            Err(e) => {
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructUnexpectedEof,
                    format!("Failed to resolve OCG ref {}: {}", ocg_ref, e),
                ));
            }
        }
    }

    // Parse /D (default configuration; required per spec)
    let default_config = match oc_props_dict.get("D") {
        Some(PdfObject::Dict(d)) => &**d,
        Some(other) => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!("/D is not a dictionary (type: {})", other.type_name()),
            ));
            oc_properties.diagnostics = diagnostics;
            return oc_properties;
        }
        None => {
            diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::StructMissingKey,
                "/D key missing from /OCProperties",
            ));
            oc_properties.diagnostics = diagnostics;
            return oc_properties;
        }
    };

    // Parse /BaseState (defaults to ON if absent)
    oc_properties.base_state = default_config
        .get("BaseState")
        .and_then(|o| o.as_name())
        .and_then(BaseState::from_name)
        .unwrap_or(BaseState::On);

    // Initialize all OCGs to base state
    for &ocg_ref in &ocg_refs {
        oc_properties
            .default_visibility
            .insert(ocg_ref, oc_properties.base_state.as_bool());
    }

    // Apply /ON array (overrides BaseState for these OCGs)
    if let Some(PdfObject::Array(on_arr)) = default_config.get("ON") {
        for obj in on_arr.iter() {
            if let Some(ocg_ref) = obj.as_ref() {
                oc_properties.default_visibility.insert(ocg_ref, true);
            }
        }
    }

    // Apply /OFF array (overrides BaseState and /ON for these OCGs)
    if let Some(PdfObject::Array(off_arr)) = default_config.get("OFF") {
        for obj in off_arr.iter() {
            if let Some(ocg_ref) = obj.as_ref() {
                oc_properties.default_visibility.insert(ocg_ref, false);
            }
        }
    }

    // Parse /Configs (optional array of alternate configurations)
    // For now, we only store the default config (/D)
    // Full support for alternate configs is deferred to Phase 7 per plan

    oc_properties.diagnostics = diagnostics;
    oc_properties
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::intern;

    fn make_test_resolver() -> XrefResolver {
        XrefResolver::new()
    }

    fn make_test_ocg(obj_ref: ObjRef, name: &str, intent: Option<&str>) -> PdfObject {
        let mut dict = PdfDict::new();
        dict.insert(intern("Type"), PdfObject::Name(intern("OCG")));
        dict.insert(
            intern("Name"),
            PdfObject::String(Box::new(name.as_bytes().to_vec())),
        );
        if let Some(i) = intent {
            dict.insert(intern("Intent"), PdfObject::Name(intern(i)));
        }
        PdfObject::Dict(Box::new(dict))
    }

    #[test]
    fn test_base_state_from_name() {
        assert_eq!(BaseState::from_name("ON"), Some(BaseState::On));
        assert_eq!(BaseState::from_name("OFF"), Some(BaseState::Off));
        assert_eq!(
            BaseState::from_name("Unchanged"),
            Some(BaseState::Unchanged)
        );
        assert_eq!(BaseState::from_name("Invalid"), None);
    }

    #[test]
    fn test_base_state_as_bool() {
        assert_eq!(BaseState::On.as_bool(), true);
        assert_eq!(BaseState::Off.as_bool(), false);
        assert_eq!(BaseState::Unchanged.as_bool(), true);
    }

    #[test]
    fn test_ocmd_policy_from_name() {
        assert_eq!(OcmdPolicy::from_name("AllOn"), Some(OcmdPolicy::AllOn));
        assert_eq!(OcmdPolicy::from_name("AllOff"), Some(OcmdPolicy::AllOff));
        assert_eq!(OcmdPolicy::from_name("AnyOn"), Some(OcmdPolicy::AnyOn));
        assert_eq!(OcmdPolicy::from_name("AnyOff"), Some(OcmdPolicy::AnyOff));
        assert_eq!(OcmdPolicy::from_name("Invalid"), None);
    }

    #[test]
    fn test_ocg_name_none() {
        let resolver = make_test_resolver();
        let oc_props = parse_oc_properties(&resolver, None);
        assert!(!oc_props.present);
        assert_eq!(oc_props.ocg_name(ObjRef::new(1, 0)), None);
    }

    #[test]
    fn test_oc_properties_not_present() {
        let resolver = make_test_resolver();
        let oc_props = parse_oc_properties(&resolver, None);
        assert!(!oc_props.present);
        assert!(oc_props.groups.is_empty());
        assert!(oc_props.default_visibility.is_empty());
        assert_eq!(oc_props.base_state, BaseState::On);
    }

    #[test]
    fn test_parse_oc_properties_simple() {
        let mut resolver = make_test_resolver();

        // Create test OCGs
        let ocg1_ref = ObjRef::new(10, 0);
        let ocg2_ref = ObjRef::new(11, 0);

        resolver.cache_object(ocg1_ref, make_test_ocg(ocg1_ref, "Layer1", Some("View")));
        resolver.cache_object(ocg2_ref, make_test_ocg(ocg2_ref, "Layer2", Some("Design")));

        // Create /OCProperties dict
        let mut oc_props_dict = PdfDict::new();
        oc_props_dict.insert(
            intern("OCGs"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Ref(ocg1_ref),
                PdfObject::Ref(ocg2_ref),
            ])),
        );

        let mut default_config = PdfDict::new();
        default_config.insert(intern("BaseState"), PdfObject::Name(intern("ON")));
        oc_props_dict.insert(intern("D"), PdfObject::Dict(Box::new(default_config)));

        let oc_props_ref = ObjRef::new(1, 0);
        resolver.cache_object(oc_props_ref, PdfObject::Dict(Box::new(oc_props_dict)));

        let oc_props = parse_oc_properties(&resolver, Some(oc_props_ref));

        assert!(oc_props.present);
        assert_eq!(oc_props.groups.len(), 2);
        assert_eq!(oc_props.base_state, BaseState::On);
        assert_eq!(oc_props.is_visible(ocg1_ref), true);
        assert_eq!(oc_props.is_visible(ocg2_ref), true);
    }

    #[test]
    fn test_parse_oc_properties_base_state_off() {
        let mut resolver = make_test_resolver();

        let ocg1_ref = ObjRef::new(10, 0);
        let ocg2_ref = ObjRef::new(11, 0);

        resolver.cache_object(ocg1_ref, make_test_ocg(ocg1_ref, "Layer1", None));
        resolver.cache_object(ocg2_ref, make_test_ocg(ocg2_ref, "Layer2", None));

        let mut oc_props_dict = PdfDict::new();
        oc_props_dict.insert(
            intern("OCGs"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Ref(ocg1_ref),
                PdfObject::Ref(ocg2_ref),
            ])),
        );

        let mut default_config = PdfDict::new();
        default_config.insert(intern("BaseState"), PdfObject::Name(intern("OFF")));
        oc_props_dict.insert(intern("D"), PdfObject::Dict(Box::new(default_config)));

        let oc_props_ref = ObjRef::new(1, 0);
        resolver.cache_object(oc_props_ref, PdfObject::Dict(Box::new(oc_props_dict)));

        let oc_props = parse_oc_properties(&resolver, Some(oc_props_ref));

        assert_eq!(oc_props.base_state, BaseState::Off);
        assert_eq!(oc_props.is_visible(ocg1_ref), false);
        assert_eq!(oc_props.is_visible(ocg2_ref), false);
    }

    #[test]
    fn test_parse_oc_properties_with_on_array() {
        let mut resolver = make_test_resolver();

        let ocg1_ref = ObjRef::new(10, 0);
        let ocg2_ref = ObjRef::new(11, 0);
        let ocg3_ref = ObjRef::new(12, 0);

        resolver.cache_object(ocg1_ref, make_test_ocg(ocg1_ref, "Layer1", None));
        resolver.cache_object(ocg2_ref, make_test_ocg(ocg2_ref, "Layer2", None));
        resolver.cache_object(ocg3_ref, make_test_ocg(ocg3_ref, "Layer3", None));

        let mut oc_props_dict = PdfDict::new();
        oc_props_dict.insert(
            intern("OCGs"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Ref(ocg1_ref),
                PdfObject::Ref(ocg2_ref),
                PdfObject::Ref(ocg3_ref),
            ])),
        );

        let mut default_config = PdfDict::new();
        default_config.insert(intern("BaseState"), PdfObject::Name(intern("OFF")));
        default_config.insert(
            intern("ON"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Ref(ocg1_ref),
                PdfObject::Ref(ocg2_ref),
            ])),
        );
        oc_props_dict.insert(intern("D"), PdfObject::Dict(Box::new(default_config)));

        let oc_props_ref = ObjRef::new(1, 0);
        resolver.cache_object(oc_props_ref, PdfObject::Dict(Box::new(oc_props_dict)));

        let oc_props = parse_oc_properties(&resolver, Some(oc_props_ref));

        // BaseState OFF, but ocg1 and ocg2 are in /ON array
        assert_eq!(oc_props.is_visible(ocg1_ref), true);
        assert_eq!(oc_props.is_visible(ocg2_ref), true);
        assert_eq!(oc_props.is_visible(ocg3_ref), false);
    }

    #[test]
    fn test_parse_oc_properties_with_off_array() {
        let mut resolver = make_test_resolver();

        let ocg1_ref = ObjRef::new(10, 0);
        let ocg2_ref = ObjRef::new(11, 0);

        resolver.cache_object(ocg1_ref, make_test_ocg(ocg1_ref, "Layer1", None));
        resolver.cache_object(ocg2_ref, make_test_ocg(ocg2_ref, "Layer2", None));

        let mut oc_props_dict = PdfDict::new();
        oc_props_dict.insert(
            intern("OCGs"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Ref(ocg1_ref),
                PdfObject::Ref(ocg2_ref),
            ])),
        );

        let mut default_config = PdfDict::new();
        default_config.insert(intern("BaseState"), PdfObject::Name(intern("ON")));
        default_config.insert(
            intern("OFF"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(ocg2_ref)])),
        );
        oc_props_dict.insert(intern("D"), PdfObject::Dict(Box::new(default_config)));

        let oc_props_ref = ObjRef::new(1, 0);
        resolver.cache_object(oc_props_ref, PdfObject::Dict(Box::new(oc_props_dict)));

        let oc_props = parse_oc_properties(&resolver, Some(oc_props_ref));

        // BaseState ON, but ocg2 is in /OFF array
        assert_eq!(oc_props.is_visible(ocg1_ref), true);
        assert_eq!(oc_props.is_visible(ocg2_ref), false);
    }

    #[test]
    fn test_parse_oc_properties_off_overrides_on() {
        let mut resolver = make_test_resolver();

        let ocg1_ref = ObjRef::new(10, 0);

        resolver.cache_object(ocg1_ref, make_test_ocg(ocg1_ref, "Layer1", None));

        let mut oc_props_dict = PdfDict::new();
        oc_props_dict.insert(
            intern("OCGs"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(ocg1_ref)])),
        );

        let mut default_config = PdfDict::new();
        default_config.insert(intern("BaseState"), PdfObject::Name(intern("OFF")));
        // OCG in both /ON and /OFF: /OFF wins per spec
        default_config.insert(
            intern("ON"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(ocg1_ref)])),
        );
        default_config.insert(
            intern("OFF"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(ocg1_ref)])),
        );
        oc_props_dict.insert(intern("D"), PdfObject::Dict(Box::new(default_config)));

        let oc_props_ref = ObjRef::new(1, 0);
        resolver.cache_object(oc_props_ref, PdfObject::Dict(Box::new(oc_props_dict)));

        let oc_props = parse_oc_properties(&resolver, Some(oc_props_ref));

        // /OFF should override /ON
        assert_eq!(oc_props.is_visible(ocg1_ref), false);
    }

    #[test]
    fn test_ocg_name_retrieval() {
        let mut resolver = make_test_resolver();

        let ocg1_ref = ObjRef::new(10, 0);
        resolver.cache_object(ocg1_ref, make_test_ocg(ocg1_ref, "TestLayer", None));

        let mut oc_props_dict = PdfDict::new();
        oc_props_dict.insert(
            intern("OCGs"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(ocg1_ref)])),
        );

        let mut default_config = PdfDict::new();
        default_config.insert(intern("BaseState"), PdfObject::Name(intern("ON")));
        oc_props_dict.insert(intern("D"), PdfObject::Dict(Box::new(default_config)));

        let oc_props_ref = ObjRef::new(1, 0);
        resolver.cache_object(oc_props_ref, PdfObject::Dict(Box::new(oc_props_dict)));

        let oc_props = parse_oc_properties(&resolver, Some(oc_props_ref));

        assert_eq!(oc_props.ocg_name(ocg1_ref), Some("TestLayer"));
        assert_eq!(oc_props.ocg_name(ObjRef::new(99, 0)), None);
    }

    #[test]
    fn test_unknown_ocg_treated_as_visible() {
        let resolver = make_test_resolver();

        let oc_props = OcProperties {
            present: true,
            groups: HashMap::new(),
            default_visibility: HashMap::new(),
            base_state: BaseState::Off,
            ocmds: HashMap::new(),
            diagnostics: Vec::new(),
        };

        // Unknown OCG should be treated as base state (OFF in this case)
        assert_eq!(oc_props.is_visible(ObjRef::new(99, 0)), false);
    }

    #[test]
    fn test_ocmd_parse() {
        let ocg1_ref = ObjRef::new(10, 0);
        let ocg2_ref = ObjRef::new(11, 0);

        let mut ocmd_dict = PdfDict::new();
        ocmd_dict.insert(intern("Type"), PdfObject::Name(intern("OCMD")));
        ocmd_dict.insert(
            intern("OCGs"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Ref(ocg1_ref),
                PdfObject::Ref(ocg2_ref),
            ])),
        );
        ocmd_dict.insert(intern("P"), PdfObject::Name(intern("AllOn")));

        let ocmd = Ocmd::parse(&PdfObject::Dict(Box::new(ocmd_dict)));

        assert!(ocmd.is_some());
        let ocmd = ocmd.unwrap();
        assert_eq!(ocmd.policy, OcmdPolicy::AllOn);
        assert_eq!(ocmd.ocgs.len(), 2);
        assert!(ocmd.ocgs.contains(&ocg1_ref));
        assert!(ocmd.ocgs.contains(&ocg2_ref));
    }

    #[test]
    fn test_ocmd_parse_single_ref() {
        let ocg1_ref = ObjRef::new(10, 0);

        let mut ocmd_dict = PdfDict::new();
        ocmd_dict.insert(intern("Type"), PdfObject::Name(intern("OCMD")));
        ocmd_dict.insert(intern("OCGs"), PdfObject::Ref(ocg1_ref));
        // No /P means default AnyOn

        let ocmd = Ocmd::parse(&PdfObject::Dict(Box::new(ocmd_dict)));

        assert!(ocmd.is_some());
        let ocmd = ocmd.unwrap();
        assert_eq!(ocmd.policy, OcmdPolicy::AnyOn); // Default
        assert_eq!(ocmd.ocgs.len(), 1);
        assert_eq!(ocmd.ocgs[0], ocg1_ref);
    }

    #[test]
    fn test_ocmd_evaluation_all_on() {
        let ocg1_ref = ObjRef::new(10, 0);
        let ocg2_ref = ObjRef::new(11, 0);

        let mut oc_props = OcProperties {
            present: true,
            groups: HashMap::new(),
            default_visibility: HashMap::new(),
            base_state: BaseState::On,
            ocmds: HashMap::new(),
            diagnostics: Vec::new(),
        };

        // Both ON
        oc_props.default_visibility.insert(ocg1_ref, true);
        oc_props.default_visibility.insert(ocg2_ref, true);

        let ocmd = Ocmd::new(vec![ocg1_ref, ocg2_ref], OcmdPolicy::AllOn);
        assert!(oc_props.evaluate_ocmd_policy(&ocmd));

        // One OFF
        oc_props.default_visibility.insert(ocg2_ref, false);
        assert!(!oc_props.evaluate_ocmd_policy(&ocmd));
    }

    #[test]
    fn test_ocmd_evaluation_any_on() {
        let ocg1_ref = ObjRef::new(10, 0);
        let ocg2_ref = ObjRef::new(11, 0);

        let mut oc_props = OcProperties {
            present: true,
            groups: HashMap::new(),
            default_visibility: HashMap::new(),
            base_state: BaseState::On,
            ocmds: HashMap::new(),
            diagnostics: Vec::new(),
        };

        // Both OFF
        oc_props.default_visibility.insert(ocg1_ref, false);
        oc_props.default_visibility.insert(ocg2_ref, false);

        let ocmd = Ocmd::new(vec![ocg1_ref, ocg2_ref], OcmdPolicy::AnyOn);
        assert!(!oc_props.evaluate_ocmd_policy(&ocmd));

        // One ON
        oc_props.default_visibility.insert(ocg1_ref, true);
        assert!(oc_props.evaluate_ocmd_policy(&ocmd));
    }

    #[test]
    fn test_ocg_group_parse() {
        let mut ocg_dict = PdfDict::new();
        ocg_dict.insert(intern("Type"), PdfObject::Name(intern("OCG")));
        ocg_dict.insert(
            intern("Name"),
            PdfObject::String(Box::new(b"TestLayer".to_vec())),
        );
        ocg_dict.insert(
            intern("Intent"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Name(intern("View")),
                PdfObject::Name(intern("Design")),
            ])),
        );

        let group = OcGroup::parse(&PdfObject::Dict(Box::new(ocg_dict)), &mut Vec::new());

        assert_eq!(group.name, Some("TestLayer".to_string()));
        assert_eq!(group.intent.len(), 2);
        assert!(group.intent.contains(&"View".to_string()));
        assert!(group.intent.contains(&"Design".to_string()));
    }

    // Proptests for INV-8 compliance
    #[cfg(test)]
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// Test that parse_oc_properties never panics on arbitrary input (INV-8).
            #[test]
            fn fuzz_parse_oc_properties_no_panics(
                ocg_count in 0..10usize,
                base_state_name in "[A-Za-z]{0,10}",
                has_on_array in proptest::bool::ANY,
                has_off_array in proptest::bool::ANY,
            ) {
                let mut resolver = make_test_resolver();
                let mut ocg_refs = Vec::new();

                // Create random OCGs
                for i in 0..ocg_count {
                    let ocg_ref = ObjRef::new(10 + i as u32, 0);
                    ocg_refs.push(ocg_ref);
                    resolver.cache_object(ocg_ref, make_test_ocg(ocg_ref, &format!("Layer{}", i), None));
                }

                // Create /OCProperties dict
                let mut oc_props_dict = PdfDict::new();
                oc_props_dict.insert(intern("OCGs"), PdfObject::Array(Box::new(
                    ocg_refs.iter().map(|&r| PdfObject::Ref(r)).collect()
                )));

                let mut default_config = PdfDict::new();
                // Use potentially invalid base state name
                default_config.insert(intern("BaseState"), PdfObject::Name(intern(&base_state_name)));

                if has_on_array && !ocg_refs.is_empty() {
                    default_config.insert(intern("ON"), PdfObject::Array(Box::new(
                        ocg_refs.iter().map(|&r| PdfObject::Ref(r)).collect()
                    )));
                }

                if has_off_array && !ocg_refs.is_empty() {
                    default_config.insert(intern("OFF"), PdfObject::Array(Box::new(
                        ocg_refs.iter().map(|&r| PdfObject::Ref(r)).collect()
                    )));
                }

                oc_props_dict.insert(intern("D"), PdfObject::Dict(Box::new(default_config)));

                let oc_props_ref = ObjRef::new(1, 0);
                resolver.cache_object(oc_props_ref, PdfObject::Dict(Box::new(oc_props_dict)));

                // This should never panic
                let oc_props = parse_oc_properties(&resolver, Some(oc_props_ref));

                // Verify structural invariants
                prop_assert!(oc_props.groups.len() <= ocg_count);
                prop_assert!(oc_props.default_visibility.len() <= ocg_count);
            }

            /// Test that OcgGroup::parse never panics.
            #[test]
            fn fuzz_ocg_group_parse_no_panics(
                name in "[a-zA-Z0-9]{0,50}",
                intent in "[a-zA-Z0-9]{0,20}",
            ) {
                let mut dict = PdfDict::new();
                dict.insert(intern("Type"), PdfObject::Name(intern("OCG")));
                dict.insert(intern("Name"), PdfObject::String(Box::new(name.as_bytes().to_vec())));
                dict.insert(intern("Intent"), PdfObject::Name(intern(&intent)));

                let obj = PdfObject::Dict(Box::new(dict));
                let _ = OcGroup::parse(&obj, &mut Vec::new());
            }

            /// Test that Ocmd::parse never panics.
            #[test]
            fn fuzz_ocmd_parse_no_panics(
                policy in "[a-zA-Z0-9]{0,20}",
                num_refs in 0..5usize,
            ) {
                let mut dict = PdfDict::new();
                dict.insert(intern("Type"), PdfObject::Name(intern("OCMD")));

                if num_refs == 0 {
                    // Single ref
                    dict.insert(intern("OCGs"), PdfObject::Ref(ObjRef::new(10, 0)));
                } else {
                    // Array of refs
                    let refs: Vec<PdfObject> = (0..num_refs)
                        .map(|i| PdfObject::Ref(ObjRef::new(10 + i as u32, 0)))
                        .collect();
                    dict.insert(intern("OCGs"), PdfObject::Array(Box::new(refs)));
                }

                dict.insert(intern("P"), PdfObject::Name(intern(&policy)));

                let obj = PdfObject::Dict(Box::new(dict));
                let _ = Ocmd::parse(&obj);
            }
        }
    }
}
