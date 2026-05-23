//! PDF structure tree parser (Tagged PDF).
//!
//! This module implements parsing of the PDF structure tree (StructTreeRoot)
//! as specified in ISO 32000-2 §14.7 (Tagged PDF). The structure tree provides
//! the logical reading order and semantic structure of a document, independent
//! of the visual content stream.
//!
//! # Key concepts
//!
//! - **StructTreeRoot**: The root of the structure tree, referenced from `/StructTreeRoot`
//!   in the document catalog.
//! - **StructElem**: A structure element representing a logical document element
//!   (paragraph, heading, table, etc.).
//! - **RoleMap**: A dictionary mapping non-standard structure type names to standard
//!   type names, allowing normalization of producer-specific tags.
//! - **MCID**: Marked Content Identifier, linking structure elements to content
//!   in the page's content stream.
//! - **MCR**: Marked Content Reference, a dictionary linking to an MCID on a specific page.
//! - **OBJR**: Object Reference, linking to an annotation or XObject.
//!
//! # Standard structure types
//!
//! Per PDF 1.7 §14.8.4:
//! - Grouping: Document, Part, Art, Sect, Div, BlockQuote, Caption, TOC, TOCI, Index, NonStruct, Private
//! - Block-level: P, H, H1..H6, L, LI, Lbl, LBody, Table, TR, TH, TD, THead, TBody, TFoot
//! - Inline: Span, Quote, Note, Reference, BibEntry, Code, Link, Annot, Ruby, RB, RT, RP, Warichu, WT, WP
//! - Illustration: Figure, Formula, Form

use crate::parser::object::{ObjRef, PdfObject};
use crate::parser::xref::XrefResolver;
use crate::diagnostics::{Diagnostic, DiagCode};
use std::collections::HashSet;
use std::sync::Arc;

/// Result type for structure tree parsing.
pub type Result<T> = std::result::Result<T, Vec<Diagnostic>>;

/// Standard structure type names per PDF 1.7 §14.8.4.
///
/// These are the canonical structure types defined by the PDF specification.
/// Non-standard types (e.g., "Heading1" from Microsoft Word) should be
/// resolved via /RoleMap to one of these standard types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureType {
    // Grouping elements
    Document,
    Part,
    Art,
    Sect,
    Div,
    BlockQuote,
    Caption,
    Toc,
    Toci,
    Index,
    NonStruct,
    Private,

    // Block-level elements
    P,
    H,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    L,
    LI,
    Lbl,
    LBody,
    Table,
    TR,
    TH,
    TD,
    THead,
    TBody,
    TFoot,

    // Inline elements
    Span,
    Quote,
    Note,
    Reference,
    BibEntry,
    Code,
    Link,
    Annot,
    Ruby,
    RB,
    RT,
    RP,
    Warichu,
    WT,
    WP,

    // Illustration/media
    Figure,
    Formula,
    Form,

    /// Unknown/non-standard type (not mapped by RoleMap)
    Unknown,
}

impl StructureType {
    /// Parse a structure type name to a StructureType.
    ///
    /// Returns `StructureType::Unknown` for non-standard names that should
    /// be resolved via RoleMap.
    pub fn from_name(name: &str) -> Self {
        match name {
            // Grouping elements
            "Document" => StructureType::Document,
            "Part" => StructureType::Part,
            "Art" => StructureType::Art,
            "Sect" => StructureType::Sect,
            "Div" => StructureType::Div,
            "BlockQuote" => StructureType::BlockQuote,
            "Caption" => StructureType::Caption,
            "TOC" => StructureType::Toc,
            "TOCI" => StructureType::Toci,
            "Index" => StructureType::Index,
            "NonStruct" => StructureType::NonStruct,
            "Private" => StructureType::Private,

            // Block-level elements
            "P" => StructureType::P,
            "H" => StructureType::H,
            "H1" => StructureType::H1,
            "H2" => StructureType::H2,
            "H3" => StructureType::H3,
            "H4" => StructureType::H4,
            "H5" => StructureType::H5,
            "H6" => StructureType::H6,
            "L" => StructureType::L,
            "LI" => StructureType::LI,
            "Lbl" => StructureType::Lbl,
            "LBody" => StructureType::LBody,
            "Table" => StructureType::Table,
            "TR" => StructureType::TR,
            "TH" => StructureType::TH,
            "TD" => StructureType::TD,
            "THead" => StructureType::THead,
            "TBody" => StructureType::TBody,
            "TFoot" => StructureType::TFoot,

            // Inline elements
            "Span" => StructureType::Span,
            "Quote" => StructureType::Quote,
            "Note" => StructureType::Note,
            "Reference" => StructureType::Reference,
            "BibEntry" => StructureType::BibEntry,
            "Code" => StructureType::Code,
            "Link" => StructureType::Link,
            "Annot" => StructureType::Annot,
            "Ruby" => StructureType::Ruby,
            "RB" => StructureType::RB,
            "RT" => StructureType::RT,
            "RP" => StructureType::RP,
            "Warichu" => StructureType::Warichu,
            "WT" => StructureType::WT,
            "WP" => StructureType::WP,

            // Illustration/media
            "Figure" => StructureType::Figure,
            "Formula" => StructureType::Formula,
            "Form" => StructureType::Form,

            _ => StructureType::Unknown,
        }
    }

    /// Get the string name for this structure type.
    pub fn as_str(&self) -> &'static str {
        match self {
            StructureType::Document => "Document",
            StructureType::Part => "Part",
            StructureType::Art => "Art",
            StructureType::Sect => "Sect",
            StructureType::Div => "Div",
            StructureType::BlockQuote => "BlockQuote",
            StructureType::Caption => "Caption",
            StructureType::Toc => "TOC",
            StructureType::Toci => "TOCI",
            StructureType::Index => "Index",
            StructureType::NonStruct => "NonStruct",
            StructureType::Private => "Private",
            StructureType::P => "P",
            StructureType::H => "H",
            StructureType::H1 => "H1",
            StructureType::H2 => "H2",
            StructureType::H3 => "H3",
            StructureType::H4 => "H4",
            StructureType::H5 => "H5",
            StructureType::H6 => "H6",
            StructureType::L => "L",
            StructureType::LI => "LI",
            StructureType::Lbl => "Lbl",
            StructureType::LBody => "LBody",
            StructureType::Table => "Table",
            StructureType::TR => "TR",
            StructureType::TH => "TH",
            StructureType::TD => "TD",
            StructureType::THead => "THead",
            StructureType::TBody => "TBody",
            StructureType::TFoot => "TFoot",
            StructureType::Span => "Span",
            StructureType::Quote => "Quote",
            StructureType::Note => "Note",
            StructureType::Reference => "Reference",
            StructureType::BibEntry => "BibEntry",
            StructureType::Code => "Code",
            StructureType::Link => "Link",
            StructureType::Annot => "Annot",
            StructureType::Ruby => "Ruby",
            StructureType::RB => "RB",
            StructureType::RT => "RT",
            StructureType::RP => "RP",
            StructureType::Warichu => "Warichu",
            StructureType::WT => "WT",
            StructureType::WP => "WP",
            StructureType::Figure => "Figure",
            StructureType::Formula => "Formula",
            StructureType::Form => "Form",
            StructureType::Unknown => "Unknown",
        }
    }

    /// Check if this is a heading type.
    pub fn is_heading(&self) -> bool {
        matches!(self, StructureType::H | StructureType::H1 | StructureType::H2 |
                 StructureType::H3 | StructureType::H4 | StructureType::H5 | StructureType::H6)
    }

    /// Get the heading level (1-6) for heading types.
    pub fn heading_level(&self) -> Option<u8> {
        match self {
            StructureType::H => Some(1),
            StructureType::H1 => Some(1),
            StructureType::H2 => Some(2),
            StructureType::H3 => Some(3),
            StructureType::H4 => Some(4),
            StructureType::H5 => Some(5),
            StructureType::H6 => Some(6),
            _ => None,
        }
    }
}

/// A kid in a StructElem's /K array.
///
/// The /K array can contain different types of entries:
/// - A child StructElem (dictionary)
/// - An integer MCID (direct reference to marked content)
/// - An MCR dictionary (marked content reference with explicit page)
/// - An OBJR dictionary (object reference to annotation/XObject)
#[derive(Debug, Clone)]
pub enum Kid {
    /// A child structure element
    Element(Box<StructElemNode>),
    /// A direct MCID integer (marked content identifier on the same page)
    Mcid(u32),
    /// A marked content reference (MCID on a specific page)
    Mcr { page: ObjRef, mcid: u32 },
    /// An object reference (annotation or XObject)
    ObjRef(ObjRef),
}

/// A node in the structure tree.
///
/// Represents a single StructElem with its resolved type, attributes,
/// and children. This is the primary output type for the structure tree walker.
#[derive(Debug, Clone)]
pub struct StructElemNode {
    /// Unique identifier (from /ID if present, otherwise generated)
    pub id: Option<String>,
    /// The raw structure type name from the /S entry
    pub raw_type: String,
    /// The resolved standard structure type (after RoleMap mapping)
    pub std_type: StructureType,
    /// Alternative text (for figures, formulas, etc.)
    pub alt: Option<String>,
    /// Actual text overriding extracted glyphs
    pub actual_text: Option<String>,
    /// BCP 47 language tag (inherited from parent if not present)
    pub lang: Option<String>,
    /// Page reference where this element's content lives
    pub page_ref: Option<ObjRef>,
    /// Children from the /K array
    pub kids: Vec<Kid>,
    /// Title (from /T entry)
    pub title: Option<String>,
    /// Abbreviation expansion (from /E entry)
    pub expansion: Option<String>,
}

impl StructElemNode {
    /// Create a new StructElemNode.
    fn new(raw_type: String, std_type: StructureType) -> Self {
        StructElemNode {
            id: None,
            raw_type,
            std_type,
            alt: None,
            actual_text: None,
            lang: None,
            page_ref: None,
            kids: Vec::new(),
            title: None,
            expansion: None,
        }
    }
}

/// The root of the structure tree.
///
/// Parsed from /StructTreeRoot in the document catalog.
#[derive(Debug, Clone)]
pub struct StructTreeRoot {
    /// Immediate children (from /K array)
    pub kids: Vec<Kid>,
    /// RoleMap mapping non-standard type names to standard types
    pub role_map: RoleMap,
    /// Diagnostics emitted during parsing
    pub diagnostics: Vec<Diagnostic>,
}

impl StructTreeRoot {
    /// Create a new empty StructTreeRoot.
    pub fn new() -> Self {
        StructTreeRoot {
            kids: Vec::new(),
            role_map: RoleMap::new(),
            diagnostics: Vec::new(),
        }
    }
}

impl Default for StructTreeRoot {
    fn default() -> Self {
        Self::new()
    }
}

/// RoleMap for resolving non-standard structure types.
///
/// The /RoleMap in StructTreeRoot maps producer-specific type names
/// to standard PDF structure types. For example, Microsoft Word uses
/// "Heading1" which should map to "H1".
#[derive(Debug, Clone)]
pub struct RoleMap {
    /// Map from non-standard name to target type name (may be non-standard itself for chaining)
    map: indexmap::IndexMap<Arc<str>, Arc<str>>,
}

impl RoleMap {
    /// Create a new empty RoleMap.
    pub fn new() -> Self {
        RoleMap {
            map: indexmap::IndexMap::new(),
        }
    }

    /// Parse a RoleMap from a dictionary object.
    fn parse(obj: &PdfObject) -> Self {
        let mut role_map = RoleMap::new();

        if let Some(dict) = obj.as_dict() {
            for (key, value) in dict.iter() {
                if let Some(target_name) = value.as_name() {
                    // Store the target name as a string, not the parsed type.
                    // This allows recursive resolution through the RoleMap
                    // (e.g., A -> B -> C -> H1).
                    role_map.map.insert(key.clone(), Arc::from(target_name));
                }
            }
        }

        role_map
    }

    /// Resolve a type name through the RoleMap, handling chains.
    ///
    /// Returns the final resolved type, or `StructureType::Unknown` if
    /// the type cannot be resolved to a standard type.
    ///
    /// # Cycle detection
    ///
    /// This method detects cycles in the RoleMap (e.g., A -> B -> A).
    /// If a cycle is detected, a warning diagnostic is emitted and
    /// `StructureType::NonStruct` is returned.
    fn resolve(&self, type_name: &str, diagnostics: &mut Vec<Diagnostic>, visited: &mut HashSet<String>) -> StructureType {
        // Check for cycles
        if visited.contains(type_name) {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructCircularRef,
                format!("RoleMap cycle detected: {}", type_name),
            ));
            return StructureType::NonStruct;
        }

        // If it's already a standard type, return it
        let std_type = StructureType::from_name(type_name);
        if std_type != StructureType::Unknown {
            return std_type;
        }

        // Look up in RoleMap
        if let Some(target_name) = self.map.get(type_name) {
            // Track visit for cycle detection
            visited.insert(type_name.to_string());

            // Recursively resolve the target name (may chain through multiple mappings)
            self.resolve(target_name, diagnostics, visited)
        } else {
            // Not in RoleMap and not a standard type
            StructureType::Unknown
        }
    }
}

impl Default for RoleMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse the structure tree from a StructTreeRoot reference.
///
/// # Arguments
///
/// * `resolver` - The xref resolver for resolving indirect references
/// * `struct_tree_root_ref` - Reference to the StructTreeRoot object
///
/// # Returns
///
/// A `Result<StructTreeRoot>` containing the parsed structure tree or diagnostics.
///
/// # Behavior
///
/// - If StructTreeRoot is missing or invalid, returns an empty tree with diagnostics
/// - Walks the /K array depth-first, resolving all structure elements
/// - Applies RoleMap normalization to all element types
/// - Tracks /Lang inheritance through the tree
/// - Extracts /ActualText, /Alt, and other attributes
pub fn parse_struct_tree(resolver: &XrefResolver, struct_tree_root_ref: ObjRef) -> Result<StructTreeRoot> {
    let mut diagnostics = Vec::new();
    let mut root = StructTreeRoot::new();

    // Resolve the StructTreeRoot object
    let root_obj = match resolver.resolve(struct_tree_root_ref) {
        Ok(obj) => obj,
        Err(e) => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!("Failed to resolve StructTreeRoot: {}", e),
            ));
            return Err(diagnostics);
        }
    };

    // Get the StructTreeRoot dictionary (may be a direct dict or array shorthand)
    let root_dict = match root_obj.as_dict() {
        Some(d) => d,
        None => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructInvalidType,
                format!("StructTreeRoot is not a dictionary (type: {})", root_obj.type_name()),
            ));
            return Err(diagnostics);
        }
    };

    // Parse the RoleMap if present (may be indirect reference)
    if let Some(role_map_obj) = root_dict.get("RoleMap") {
        // Resolve if it's an indirect reference
        if let Some(role_map_ref) = role_map_obj.as_ref() {
            match resolver.resolve(role_map_ref) {
                Ok(obj) => {
                    root.role_map = RoleMap::parse(&obj);
                }
                Err(e) => {
                    diagnostics.push(Diagnostic::with_dynamic_no_offset(
                        DiagCode::StructUnexpectedEof,
                        format!("Failed to resolve RoleMap reference {}: {}", role_map_ref, e),
                    ));
                    // Use empty RoleMap (already initialized in new())
                }
            }
        } else {
            root.role_map = RoleMap::parse(role_map_obj);
        }
    }

    // Get the /K array (kids)
    let kids_array = match root_dict.get("K") {
        Some(k) => k,
        None => {
            // Empty /K is valid
            root.diagnostics = diagnostics;
            return Ok(root);
        }
    };

    // Walk the /K array
    let mut visited = HashSet::new();
    root.kids = walk_kids(
        resolver,
        kids_array,
        &root.role_map,
        &mut diagnostics,
        &mut visited,
        None,  // No parent lang at root
        None,  // No parent actual_text at root
    );

    root.diagnostics = diagnostics;
    Ok(root)
}

/// Walk a /K array and return the parsed kids.
///
/// # Arguments
///
/// * `resolver` - The xref resolver
/// * `kids_obj` - The /K object (array or single entry)
/// * `role_map` - The RoleMap for type resolution
/// * `diagnostics` - Diagnostics accumulator
/// * `visited` - Set of visited object refs for cycle detection
/// * `parent_lang` - Inherited language from parent
/// * `parent_actual_text` - Inherited actual_text from parent
fn walk_kids(
    resolver: &XrefResolver,
    kids_obj: &PdfObject,
    role_map: &RoleMap,
    diagnostics: &mut Vec<Diagnostic>,
    visited: &mut HashSet<ObjRef>,
    parent_lang: Option<&str>,
    parent_actual_text: Option<&str>,
) -> Vec<Kid> {
    let mut kids = Vec::new();

    // /K can be an array or a single entry
    let entries = match kids_obj.as_array() {
        Some(arr) => arr.as_ref(),
        None => std::slice::from_ref(kids_obj),
    };

    for entry in entries {
        let kid = match parse_kid_entry(
            resolver,
            entry,
            role_map,
            diagnostics,
            visited,
            parent_lang,
            parent_actual_text,
        ) {
            Some(k) => k,
            None => continue,
        };
        kids.push(kid);
    }

    kids
}

/// Parse a single entry from a /K array.
fn parse_kid_entry(
    resolver: &XrefResolver,
    entry: &PdfObject,
    role_map: &RoleMap,
    diagnostics: &mut Vec<Diagnostic>,
    visited: &mut HashSet<ObjRef>,
    parent_lang: Option<&str>,
    parent_actual_text: Option<&str>,
) -> Option<Kid> {
    match entry {
        // Integer MCID
        PdfObject::Integer(mcid) if *mcid >= 0 => {
            Some(Kid::Mcid(*mcid as u32))
        }

        // Indirect reference to StructElem
        PdfObject::Ref(obj_ref) => {
            // Check for cycles
            if visited.contains(obj_ref) {
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructCircularRef,
                    format!("Cycle detected in structure tree at {}", obj_ref),
                ));
                return None;
            }

            // Resolve the referenced object
            let elem_obj = match resolver.resolve(*obj_ref) {
                Ok(obj) => obj,
                Err(e) => {
                    diagnostics.push(Diagnostic::with_dynamic_no_offset(
                        DiagCode::StructUnexpectedEof,
                        format!("Failed to resolve StructElem reference {}: {}", obj_ref, e),
                    ));
                    return None;
                }
            };

            // Check if the resolved object is an MCR or OBJR dictionary
            if let Some(dict) = elem_obj.as_dict() {
                if let Some(type_name) = dict.get("Type").and_then(|t| t.as_name()) {
                    if type_name == "MCR" {
                        // Parse MCR: /Type /MCR /Pg <page> /MCID <mcid>
                        let page = dict.get("Pg").and_then(|p| p.as_ref())?;
                        let mcid = dict.get("MCID").and_then(|m| m.as_int())?;
                        if mcid >= 0 {
                            return Some(Kid::Mcr { page, mcid: mcid as u32 });
                        }
                        return None;
                    }

                    if type_name == "OBJR" {
                        // Parse OBJR: /Type /OBJR /Obj <objref>
                        if let Some(obj_ref2) = dict.get("Obj").and_then(|o| o.as_ref()) {
                            return Some(Kid::ObjRef(obj_ref2));
                        }
                        return None;
                    }
                }
            }

            // Parse as StructElem
            let elem_node = parse_struct_elem(
                resolver,
                &elem_obj,
                role_map,
                diagnostics,
                visited,
                parent_lang,
                parent_actual_text,
            )?;

            Some(Kid::Element(Box::new(elem_node)))
        }

        // Dictionary - could be StructElem, MCR, or OBJR
        PdfObject::Dict(dict) => {
            // Check for MCR (marked content reference) first
            if let Some(type_name) = dict.get("Type").and_then(|t| t.as_name()) {
                if type_name == "MCR" {
                    // Parse MCR: /Type /MCR /Pg <page> /MCID <mcid>
                    let page = dict.get("Pg").and_then(|p| p.as_ref())?;
                    let mcid = dict.get("MCID").and_then(|m| m.as_int())?;
                    if mcid >= 0 {
                        return Some(Kid::Mcr { page, mcid: mcid as u32 });
                    }
                    return None;
                }

                if type_name == "OBJR" {
                    // Parse OBJR: /Type /OBJR /Obj <objref>
                    if let Some(obj_ref) = dict.get("Obj").and_then(|o| o.as_ref()) {
                        return Some(Kid::ObjRef(obj_ref));
                    }
                    return None;
                }
            }

            // Otherwise, treat as a StructElem
            let elem_node = parse_struct_elem(
                resolver,
                entry,
                role_map,
                diagnostics,
                visited,
                parent_lang,
                parent_actual_text,
            )?;
            Some(Kid::Element(Box::new(elem_node)))
        }

        // Unknown entry type - emit diagnostic and skip
        _ => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructInvalidType,
                format!("Unknown /K entry type: {}", entry.type_name()),
            ));
            None
        }
    }
}

/// Parse a StructElem dictionary.
fn parse_struct_elem(
    resolver: &XrefResolver,
    elem_obj: &PdfObject,
    role_map: &RoleMap,
    diagnostics: &mut Vec<Diagnostic>,
    visited: &mut HashSet<ObjRef>,
    parent_lang: Option<&str>,
    parent_actual_text: Option<&str>,
) -> Option<StructElemNode> {
    let dict = elem_obj.as_dict()?;

    // Get the structure type (/S is required)
    let raw_type = dict.get("S").and_then(|s| s.as_name())?;
    let mut std_type = StructureType::from_name(raw_type);

    // Resolve through RoleMap if not a standard type
    if std_type == StructureType::Unknown {
        let mut visited_types = HashSet::new();
        std_type = role_map.resolve(raw_type, diagnostics, &mut visited_types);
    }

    let mut node = StructElemNode::new(raw_type.to_string(), std_type);

    // Extract /ID (optional identifier)
    if let Some(id_bytes) = dict.get("ID").and_then(|i| i.as_string()) {
        if let Ok(id_str) = std::str::from_utf8(id_bytes) {
            node.id = Some(id_str.to_string());
        }
    }

    // Extract /Pg (page reference, optional)
    if let Some(page_ref) = dict.get("Pg").and_then(|p| p.as_ref()) {
        node.page_ref = Some(page_ref);
    }

    // Extract /T (title, optional)
    if let Some(title_bytes) = dict.get("T").and_then(|t| t.as_string()) {
        if let Ok(title_str) = std::str::from_utf8(title_bytes) {
            node.title = Some(title_str.to_string());
        }
    }

    // Extract /Alt (alternative text, optional)
    if let Some(alt_bytes) = dict.get("Alt").and_then(|a| a.as_string()) {
        if let Ok(alt_str) = std::str::from_utf8(alt_bytes) {
            node.alt = Some(alt_str.to_string());
        }
    }

    // Extract /ActualText (overrides glyph text, optional)
    let actual_text = dict.get("ActualText").and_then(|a| a.as_string())
        .and_then(|bytes| std::str::from_utf8(bytes).ok().map(|s| s.to_string()));

    // Use parent's actual_text if we don't have our own
    node.actual_text = actual_text.or_else(|| parent_actual_text.map(|s| s.to_string()));

    // Extract /Lang (language tag, inherits from parent)
    let lang = dict.get("Lang").and_then(|l| l.as_string())
        .and_then(|bytes| std::str::from_utf8(bytes).ok().map(|s| s.to_string()));

    // Use our own lang or inherit from parent
    node.lang = lang.or_else(|| parent_lang.map(|s| s.to_string()));

    // Extract /E (expansion, optional)
    if let Some(e_bytes) = dict.get("E").and_then(|e| e.as_string()) {
        if let Ok(e_str) = std::str::from_utf8(e_bytes) {
            node.expansion = Some(e_str.to_string());
        }
    }

    // Walk the /K array (kids)
    if let Some(kids_obj) = dict.get("K") {
        // For ActualText inheritance: if we have our own ActualText,
        // it applies to all descendants (overrides parent)
        let inherited_actual_text = node.actual_text.as_deref();

        // For Lang inheritance: pass our lang to children
        let inherited_lang = node.lang.as_deref();

        node.kids = walk_kids(
            resolver,
            kids_obj,
            role_map,
            diagnostics,
            visited,
            inherited_lang,
            inherited_actual_text,
        );
    }

    Some(node)
}

/// Block kind classification for Phase 4 output.
///
/// This enum represents the taxonomy of block kinds used in the extraction
/// output. It maps from PDF standard structure types to output block kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// Paragraph text
    Paragraph,
    /// Heading with level 1-6
    Heading { level: u8 },
    /// Table structure
    Table,
    /// List container
    List,
    /// List item
    ListItem,
    /// List label (e.g., bullet or number)
    ListLabel,
    /// List body content
    ListBody,
    /// Figure/image
    Figure,
    /// Caption (for figures, tables, etc.)
    Caption,
    /// Code block
    Code,
    /// Block quotation
    BlockQuote,
    /// Table of contents
    Toc,
    /// Formula/math
    Formula,
    /// Reference/citation
    Reference,
    /// Note/footnote
    Note,
    /// Form field structure
    FormFieldStruct,
    /// Inline element (no block emitted)
    Inline,
    /// Structural container (descend without emitting block)
    StructuralContainer,
    /// Artifact (suppressed - not emitted in output)
    Artifact,
    /// Unknown type (fallback to paragraph with diagnostic)
    Unknown,
}

impl BlockKind {
    /// Get the string representation of this block kind for JSON output.
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockKind::Paragraph => "paragraph",
            BlockKind::Heading { .. } => "heading",
            BlockKind::Table => "table",
            BlockKind::List => "list",
            BlockKind::ListItem => "list_item",
            BlockKind::ListLabel => "list_label",
            BlockKind::ListBody => "list_body",
            BlockKind::Figure => "figure",
            BlockKind::Caption => "caption",
            BlockKind::Code => "code",
            BlockKind::BlockQuote => "block_quote",
            BlockKind::Toc => "toc",
            BlockKind::Formula => "formula",
            BlockKind::Reference => "reference",
            BlockKind::Note => "note",
            BlockKind::FormFieldStruct => "form_field_struct",
            BlockKind::Inline => "inline",
            BlockKind::StructuralContainer => "structural_container",
            BlockKind::Artifact => "artifact",
            BlockKind::Unknown => "paragraph", // Unknown types fall back to paragraph in output
        }
    }

    /// Check if this block kind should be emitted in output.
    ///
    /// Returns `false` for inline elements, structural containers, and artifacts,
    /// which are handled specially (inline within parent blocks, descended without
    /// emitting, or suppressed entirely).
    pub fn is_emitted(&self) -> bool {
        !matches!(self,
            BlockKind::Inline
            | BlockKind::StructuralContainer
            | BlockKind::Artifact
        )
    }

    /// Get the heading level for heading block kinds.
    pub fn heading_level(&self) -> Option<u8> {
        match self {
            BlockKind::Heading { level } => Some(*level),
            _ => None,
        }
    }
}

/// Map a structure type to its corresponding block kind.
///
/// This function implements the element-type to block-kind mapping table
/// specified in Phase 7.1.2. It determines how each PDF standard structure
/// type should be represented in the extraction output.
///
/// # Mapping rules
///
/// - **Block-level elements** (P, H, H1..H6, Table, L, LI, Figure, etc.) map to
///   corresponding block kinds that are emitted in output.
///
/// - **Inline elements** (Span, Quote) map to `BlockKind::Inline`, indicating
///   they should be handled within their parent block's content, not as
///   separate blocks.
///
/// - **Structural containers** (Document, Part, Art, Sect, Div, NonStruct, Private)
///   map to `BlockKind::StructuralContainer`, indicating the walker should
///   descend into their children without emitting a block for the container itself.
///
/// - **Artifact** maps to `BlockKind::Artifact`, indicating suppression - neither
///   the element nor its content reaches output.
///
/// - **Unknown types** (after RoleMap resolution) map to `BlockKind::Unknown`,
///   which falls back to paragraph in output but emits a diagnostic.
///
/// # Arguments
///
/// * `std_type` - The resolved standard structure type
///
/// # Returns
///
/// The corresponding `BlockKind` for this structure type.
pub fn structure_type_to_block_kind(std_type: StructureType) -> BlockKind {
    match std_type {
        // Block-level elements
        StructureType::P => BlockKind::Paragraph,
        StructureType::H => BlockKind::Heading { level: 1 },
        StructureType::H1 => BlockKind::Heading { level: 1 },
        StructureType::H2 => BlockKind::Heading { level: 2 },
        StructureType::H3 => BlockKind::Heading { level: 3 },
        StructureType::H4 => BlockKind::Heading { level: 4 },
        StructureType::H5 => BlockKind::Heading { level: 5 },
        StructureType::H6 => BlockKind::Heading { level: 6 },
        StructureType::Table => BlockKind::Table,
        StructureType::L => BlockKind::List,
        StructureType::LI => BlockKind::ListItem,
        StructureType::Lbl => BlockKind::ListLabel,
        StructureType::LBody => BlockKind::ListBody,
        StructureType::Figure => BlockKind::Figure,
        StructureType::Caption => BlockKind::Caption,
        StructureType::Code => BlockKind::Code,
        StructureType::BlockQuote => BlockKind::BlockQuote,
        StructureType::Toc => BlockKind::Toc,
        StructureType::Toci => BlockKind::Toc,
        StructureType::Formula => BlockKind::Formula,
        StructureType::Reference => BlockKind::Reference,
        StructureType::Note => BlockKind::Note,
        StructureType::Form => BlockKind::FormFieldStruct,

        // Inline elements (no block emitted - handled within parent)
        StructureType::Span => BlockKind::Inline,
        StructureType::Quote => BlockKind::Inline,

        // Structural containers (descend without emitting block)
        StructureType::Document => BlockKind::StructuralContainer,
        StructureType::Part => BlockKind::StructuralContainer,
        StructureType::Art => BlockKind::StructuralContainer,
        StructureType::Sect => BlockKind::StructuralContainer,
        StructureType::Div => BlockKind::StructuralContainer,
        StructureType::NonStruct => BlockKind::StructuralContainer,
        StructureType::Private => BlockKind::StructuralContainer,
        StructureType::Index => BlockKind::StructuralContainer,
        StructureType::TR => BlockKind::StructuralContainer,   // Table row - container
        StructureType::TH => BlockKind::StructuralContainer,   // Table header cell
        StructureType::TD => BlockKind::StructuralContainer,   // Table data cell
        StructureType::THead => BlockKind::StructuralContainer, // Table head group
        StructureType::TBody => BlockKind::StructuralContainer, // Table body group
        StructureType::TFoot => BlockKind::StructuralContainer, // Table foot group

        // Other inline elements - treat as inline
        StructureType::BibEntry => BlockKind::Inline,
        StructureType::Link => BlockKind::Inline,
        StructureType::Annot => BlockKind::Inline,
        StructureType::Ruby => BlockKind::Inline,
        StructureType::RB => BlockKind::Inline,
        StructureType::RT => BlockKind::Inline,
        StructureType::RP => BlockKind::Inline,
        StructureType::Warichu => BlockKind::Inline,
        StructureType::WT => BlockKind::Inline,
        StructureType::WP => BlockKind::Inline,

        // Unknown type (after RoleMap resolution) - fall back to paragraph
        StructureType::Unknown => BlockKind::Unknown,
    }
}

/// Check if a structure type should be suppressed as an artifact.
///
/// This function handles both:
/// 1. Structure elements with type "Artifact"
/// 2. MCIDs inside Artifact marked-content sequences (from Phase 3.4)
///
/// # Arguments
///
/// * `std_type` - The resolved standard structure type
///
/// # Returns
///
/// `true` if this is an artifact that should be suppressed.
pub fn is_artifact(std_type: StructureType) -> bool {
    // Note: StructureType doesn't have an Artifact variant because Artifact
    // is handled as a marked-content tag, not a structure type.
    // This function is a placeholder for future Artifact marked-content integration.
    // When Phase 3.4 marked-content tagger is integrated, it will track
    // which MCIDs are inside Artifact sequences, and this function will
    // check that mapping.
    false
}

/// Mapping result for a structure element.
///
/// This type represents the result of mapping a structure element to
/// its block kind, including information about whether it should be
/// emitted and any diagnostic for unknown types.
#[derive(Debug, Clone)]
pub struct MappingResult {
    /// The block kind for this element
    pub block_kind: BlockKind,
    /// Whether this element should be emitted in output
    pub is_emitted: bool,
    /// Optional diagnostic for unknown types
    pub diagnostic: Option<Diagnostic>,
}

impl MappingResult {
    /// Create a new mapping result.
    fn new(block_kind: BlockKind) -> Self {
        let is_emitted = block_kind.is_emitted();
        let diagnostic = if matches!(block_kind, BlockKind::Unknown) {
            Some(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructInvalidType,
                "Unknown structure type after RoleMap resolution, falling back to paragraph".to_string(),
            ))
        } else {
            None
        };
        MappingResult {
            block_kind,
            is_emitted,
            diagnostic,
        }
    }

    /// Create a mapping result for an artifact (suppressed).
    fn artifact() -> Self {
        MappingResult {
            block_kind: BlockKind::Artifact,
            is_emitted: false,
            diagnostic: None,
        }
    }
}

/// Map a structure element node to its block kind with full context.
///
/// This is the primary mapping function used by the Phase 7.1 walker.
/// It takes a `StructElemNode` and returns a `MappingResult` indicating
/// how the element should be handled in the output.
///
/// # Arguments
///
/// * `node` - The structure element node to map
///
/// # Returns
///
/// A `MappingResult` containing the block kind, whether it should be emitted,
/// and an optional diagnostic for unknown types.
///
/// # Example
///
/// ```ignore
/// let result = map_element_to_block(&node);
/// if result.is_emitted {
///     // Emit a block with kind = result.block_kind.as_str()
///     if let Some(level) = result.block_kind.heading_level() {
///         // Include level in heading block
///     }
/// }
/// if let Some(diag) = result.diagnostic {
///     diagnostics.push(diag);
/// }
/// ```
pub fn map_element_to_block(node: &StructElemNode) -> MappingResult {
    // Check if this is an artifact (type "Artifact" or inside Artifact marked-content)
    if is_artifact(node.std_type) {
        return MappingResult::artifact();
    }

    // Map the structure type to a block kind
    let block_kind = structure_type_to_block_kind(node.std_type);
    MappingResult::new(block_kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::{intern, PdfDict};

    fn make_test_resolver() -> XrefResolver {
        XrefResolver::new()
    }

    #[test]
    fn test_structure_type_from_name() {
        assert_eq!(StructureType::from_name("P"), StructureType::P);
        assert_eq!(StructureType::from_name("H1"), StructureType::H1);
        assert_eq!(StructureType::from_name("Table"), StructureType::Table);
        assert_eq!(StructureType::from_name("Figure"), StructureType::Figure);
        assert_eq!(StructureType::from_name("UnknownType"), StructureType::Unknown);
    }

    #[test]
    fn test_structure_type_is_heading() {
        assert!(StructureType::H.is_heading());
        assert!(StructureType::H1.is_heading());
        assert!(StructureType::H6.is_heading());
        assert!(!StructureType::P.is_heading());
        assert!(!StructureType::Table.is_heading());
    }

    #[test]
    fn test_structure_type_heading_level() {
        assert_eq!(StructureType::H.heading_level(), Some(1));
        assert_eq!(StructureType::H1.heading_level(), Some(1));
        assert_eq!(StructureType::H2.heading_level(), Some(2));
        assert_eq!(StructureType::H6.heading_level(), Some(6));
        assert_eq!(StructureType::P.heading_level(), None);
    }

    #[test]
    fn test_role_map_parse() {
        let mut dict = PdfDict::new();
        dict.insert(intern("Heading1"), PdfObject::Name(intern("H1")));
        dict.insert(intern("Heading2"), PdfObject::Name(intern("H2")));
        dict.insert(intern("Normal"), PdfObject::Name(intern("P")));

        let obj = PdfObject::Dict(Box::new(dict));
        let role_map = RoleMap::parse(&obj);

        // RoleMap stores target names, not parsed types
        assert_eq!(role_map.map.get("Heading1"), Some(&Arc::from("H1")));
        assert_eq!(role_map.map.get("Heading2"), Some(&Arc::from("H2")));
        assert_eq!(role_map.map.get("Normal"), Some(&Arc::from("P")));
    }

    #[test]
    fn test_role_map_resolve() {
        let mut dict = PdfDict::new();
        dict.insert(intern("Heading1"), PdfObject::Name(intern("H1")));
        dict.insert(intern("CustomPara"), PdfObject::Name(intern("P")));

        let obj = PdfObject::Dict(Box::new(dict));
        let role_map = RoleMap::parse(&obj);

        let mut diagnostics = Vec::new();
        let mut visited = HashSet::new();

        // Standard type resolves directly
        assert_eq!(role_map.resolve("P", &mut diagnostics, &mut visited), StructureType::P);

        // Mapped type resolves through RoleMap
        assert_eq!(role_map.resolve("Heading1", &mut diagnostics, &mut visited), StructureType::H1);

        // Unknown type returns Unknown
        assert_eq!(role_map.resolve("FooBar", &mut diagnostics, &mut visited), StructureType::Unknown);
    }

    #[test]
    fn test_role_map_chaining() {
        // Test RoleMap with chaining: CustomA -> CustomB -> H1
        let mut dict = PdfDict::new();
        dict.insert(intern("CustomA"), PdfObject::Name(intern("CustomB")));
        dict.insert(intern("CustomB"), PdfObject::Name(intern("H1")));

        let obj = PdfObject::Dict(Box::new(dict));
        let role_map = RoleMap::parse(&obj);

        let mut diagnostics = Vec::new();
        let mut visited = HashSet::new();

        // CustomA should resolve to H1 through the chain
        assert_eq!(role_map.resolve("CustomA", &mut diagnostics, &mut visited), StructureType::H1);
        assert!(diagnostics.is_empty()); // No diagnostics for successful chain resolution
    }

    #[test]
    fn test_role_map_cycle_detection() {
        // Test RoleMap with a cycle: A -> B -> A
        let mut dict = PdfDict::new();
        dict.insert(intern("CustomA"), PdfObject::Name(intern("CustomB")));
        dict.insert(intern("CustomB"), PdfObject::Name(intern("CustomA")));

        let obj = PdfObject::Dict(Box::new(dict));
        let role_map = RoleMap::parse(&obj);

        let mut diagnostics = Vec::new();
        let mut visited = HashSet::new();

        // Should detect the cycle and return NonStruct
        assert_eq!(role_map.resolve("CustomA", &mut diagnostics, &mut visited), StructureType::NonStruct);
        assert!(!diagnostics.is_empty()); // Should have cycle diagnostic
        assert!(diagnostics.iter().any(|d| d.message.contains("cycle")));
    }

    #[test]
    fn test_role_map_self_mapping() {
        // Create a RoleMap with a self-referencing entry
        // (In real PDFs, this can happen when a producer maps a non-standard
        // type to itself, which is a cycle)
        let mut dict = PdfDict::new();
        // "Heading1" maps to "Heading1" - this is a cycle
        dict.insert(intern("Heading1"), PdfObject::Name(intern("Heading1")));

        let obj = PdfObject::Dict(Box::new(dict));
        let role_map = RoleMap::parse(&obj);

        let mut diagnostics = Vec::new();
        let mut visited = HashSet::new();

        // Should return NonStruct and emit a cycle diagnostic
        let result = role_map.resolve("Heading1", &mut diagnostics, &mut visited);
        assert_eq!(result, StructureType::NonStruct);
        assert!(!diagnostics.is_empty()); // Should have cycle diagnostic
        assert!(diagnostics.iter().any(|d| d.message.contains("cycle")));
    }

    #[test]
    fn test_struct_elem_node_new() {
        let node = StructElemNode::new("P".to_string(), StructureType::P);

        assert_eq!(node.raw_type, "P");
        assert_eq!(node.std_type, StructureType::P);
        assert!(node.id.is_none());
        assert!(node.alt.is_none());
        assert!(node.actual_text.is_none());
        assert!(node.lang.is_none());
        assert!(node.page_ref.is_none());
        assert!(node.kids.is_empty());
        assert!(node.title.is_none());
        assert!(node.expansion.is_none());
    }

    #[test]
    fn test_struct_tree_root_new() {
        let root = StructTreeRoot::new();

        assert!(root.kids.is_empty());
        assert!(root.role_map.map.is_empty());
        assert!(root.diagnostics.is_empty());
    }

    #[test]
    fn test_struct_tree_root_default() {
        let root = StructTreeRoot::default();

        assert!(root.kids.is_empty());
        assert!(root.role_map.map.is_empty());
    }

    #[test]
    fn test_struct_tree_word_rolemap_integration() {
        // Integration test: Word-generated PDF with RoleMap
        // RoleMap: Heading1 -> H1, Heading2 -> H2
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Create RoleMap
        let mut role_map_dict = PdfDict::new();
        role_map_dict.insert(intern("Heading1"), PdfObject::Name(intern("H1")));
        role_map_dict.insert(intern("Heading2"), PdfObject::Name(intern("H2")));
        let role_map_ref = ObjRef::new(10, 0);
        resolver.cache_object(role_map_ref, PdfObject::Dict(Box::new(role_map_dict)));

        // Create child StructElem with Word's "Heading1" type
        let mut child_dict = PdfDict::new();
        child_dict.insert(intern("S"), PdfObject::Name(intern("Heading1")));
        child_dict.insert(intern("K"), PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0), // MCID
        ])));
        let child_ref = ObjRef::new(11, 0);
        resolver.cache_object(child_ref, PdfObject::Dict(Box::new(child_dict)));

        // Create StructTreeRoot
        let mut root_dict = PdfDict::new();
        root_dict.insert(intern("K"), PdfObject::Array(Box::new(vec![
            PdfObject::Ref(child_ref),
        ])));
        root_dict.insert(intern("RoleMap"), PdfObject::Ref(role_map_ref));
        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));

        // Parse and verify
        let result = parse_struct_tree(&resolver, root_ref);
        assert!(result.is_ok());

        let tree = result.unwrap();
        assert_eq!(tree.kids.len(), 1);

        // Verify the Word "Heading1" was resolved to standard "H1"
        match &tree.kids[0] {
            Kid::Element(node) => {
                assert_eq!(node.raw_type, "Heading1");
                assert_eq!(node.std_type, StructureType::H1);
            }
            _ => panic!("Expected Element kid"),
        }
    }

    #[test]
    fn test_struct_tree_lang_inheritance() {
        // Test /Lang inheritance through the tree
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Parent with /Lang
        let mut parent_dict = PdfDict::new();
        parent_dict.insert(intern("S"), PdfObject::Name(intern("Div")));
        parent_dict.insert(intern("Lang"), PdfObject::String(Box::new(b"en-US".to_vec())));
        let parent_ref = ObjRef::new(11, 0);
        resolver.cache_object(parent_ref, PdfObject::Dict(Box::new(parent_dict)));

        // Child without /Lang (should inherit)
        let mut child_dict = PdfDict::new();
        child_dict.insert(intern("S"), PdfObject::Name(intern("P")));
        child_dict.insert(intern("K"), PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
        ])));
        let child_ref = ObjRef::new(12, 0);
        resolver.cache_object(child_ref, PdfObject::Dict(Box::new(child_dict)));

        // Create parent's /K with child
        let mut parent_with_k = PdfDict::new();
        parent_with_k.insert(intern("S"), PdfObject::Name(intern("Div")));
        parent_with_k.insert(intern("Lang"), PdfObject::String(Box::new(b"en-US".to_vec())));
        parent_with_k.insert(intern("K"), PdfObject::Array(Box::new(vec![
            PdfObject::Ref(child_ref),
        ])));
        resolver.cache_object(parent_ref, PdfObject::Dict(Box::new(parent_with_k)));

        // Create StructTreeRoot
        let mut root_dict = PdfDict::new();
        root_dict.insert(intern("K"), PdfObject::Array(Box::new(vec![
            PdfObject::Ref(parent_ref),
        ])));
        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));

        // Parse and verify
        let result = parse_struct_tree(&resolver, root_ref);
        assert!(result.is_ok());

        let tree = result.unwrap();
        match &tree.kids[0] {
            Kid::Element(parent) => {
                assert_eq!(parent.lang.as_ref().unwrap(), "en-US");
                // Child should inherit parent's lang
                match &parent.kids[0] {
                    Kid::Element(child) => {
                        assert_eq!(child.lang.as_ref().unwrap(), "en-US");
                    }
                    _ => panic!("Expected Element kid"),
                }
            }
            _ => panic!("Expected Element kid"),
        }
    }

    #[test]
    fn test_struct_tree_actual_text_scope() {
        // Test /ActualText scope: applies to all descendants
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Parent with /ActualText
        let mut parent_dict = PdfDict::new();
        parent_dict.insert(intern("S"), PdfObject::Name(intern("Div")));
        parent_dict.insert(intern("ActualText"), PdfObject::String(Box::new(b"Parent text".to_vec())));
        let parent_ref = ObjRef::new(11, 0);

        // Child without /ActualText (should inherit parent's)
        let mut child_dict = PdfDict::new();
        child_dict.insert(intern("S"), PdfObject::Name(intern("P")));
        child_dict.insert(intern("K"), PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
        ])));
        let child_ref = ObjRef::new(12, 0);
        resolver.cache_object(child_ref, PdfObject::Dict(Box::new(child_dict)));

        // Create parent's /K with child
        let mut parent_with_k = PdfDict::new();
        parent_with_k.insert(intern("S"), PdfObject::Name(intern("Div")));
        parent_with_k.insert(intern("ActualText"), PdfObject::String(Box::new(b"Parent text".to_vec())));
        parent_with_k.insert(intern("K"), PdfObject::Array(Box::new(vec![
            PdfObject::Ref(child_ref),
        ])));
        resolver.cache_object(parent_ref, PdfObject::Dict(Box::new(parent_with_k)));

        // Create StructTreeRoot
        let mut root_dict = PdfDict::new();
        root_dict.insert(intern("K"), PdfObject::Array(Box::new(vec![
            PdfObject::Ref(parent_ref),
        ])));
        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));

        // Parse and verify
        let result = parse_struct_tree(&resolver, root_ref);
        assert!(result.is_ok());

        let tree = result.unwrap();
        match &tree.kids[0] {
            Kid::Element(parent) => {
                assert_eq!(parent.actual_text.as_ref().unwrap(), "Parent text");
                // Child should inherit parent's actual_text
                match &parent.kids[0] {
                    Kid::Element(child) => {
                        assert_eq!(child.actual_text.as_ref().unwrap(), "Parent text");
                    }
                    _ => panic!("Expected Element kid"),
                }
            }
            _ => panic!("Expected Element kid"),
        }
    }

    #[test]
    fn test_struct_tree_mcr_kid() {
        // Test MCR (marked content reference) kid type
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Create MCR dictionary
        let mut mcr_dict = PdfDict::new();
        mcr_dict.insert(intern("Type"), PdfObject::Name(intern("MCR")));
        mcr_dict.insert(intern("Pg"), PdfObject::Ref(ObjRef::new(5, 0)));
        mcr_dict.insert(intern("MCID"), PdfObject::Integer(42));
        let mcr_ref = ObjRef::new(11, 0);
        resolver.cache_object(mcr_ref, PdfObject::Dict(Box::new(mcr_dict)));

        // Create StructTreeRoot with MCR kid
        let mut root_dict = PdfDict::new();
        root_dict.insert(intern("K"), PdfObject::Array(Box::new(vec![
            PdfObject::Ref(mcr_ref),
        ])));
        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));

        // Parse and verify
        let result = parse_struct_tree(&resolver, root_ref);
        assert!(result.is_ok());

        let tree = result.unwrap();
        assert_eq!(tree.kids.len(), 1);

        match &tree.kids[0] {
            Kid::Mcr { page, mcid } => {
                assert_eq!(*page, ObjRef::new(5, 0));
                assert_eq!(*mcid, 42);
            }
            _ => panic!("Expected Mcr kid"),
        }
    }

    #[test]
    fn test_struct_tree_objr_kid() {
        // Test OBJR (object reference) kid type
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Create OBJR dictionary
        let mut objr_dict = PdfDict::new();
        objr_dict.insert(intern("Type"), PdfObject::Name(intern("OBJR")));
        objr_dict.insert(intern("Obj"), PdfObject::Ref(ObjRef::new(7, 0)));
        let objr_ref = ObjRef::new(11, 0);
        resolver.cache_object(objr_ref, PdfObject::Dict(Box::new(objr_dict)));

        // Create StructTreeRoot with OBJR kid
        let mut root_dict = PdfDict::new();
        root_dict.insert(intern("K"), PdfObject::Array(Box::new(vec![
            PdfObject::Ref(objr_ref),
        ])));
        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));

        // Parse and verify
        let result = parse_struct_tree(&resolver, root_ref);
        assert!(result.is_ok());

        let tree = result.unwrap();
        assert_eq!(tree.kids.len(), 1);

        match &tree.kids[0] {
            Kid::ObjRef(obj_ref) => {
                assert_eq!(*obj_ref, ObjRef::new(7, 0));
            }
            _ => panic!("Expected ObjRef kid"),
        }
    }

    #[test]
    fn test_struct_tree_mcid_kid() {
        // Test direct MCID kid type
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Create StructTreeRoot with MCID kid
        let mut root_dict = PdfDict::new();
        root_dict.insert(intern("K"), PdfObject::Array(Box::new(vec![
            PdfObject::Integer(123),
        ])));
        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));

        // Parse and verify
        let result = parse_struct_tree(&resolver, root_ref);
        assert!(result.is_ok());

        let tree = result.unwrap();
        assert_eq!(tree.kids.len(), 1);

        match &tree.kids[0] {
            Kid::Mcid(mcid) => {
                assert_eq!(*mcid, 123);
            }
            _ => panic!("Expected Mcid kid"),
        }
    }

    // BlockKind mapping tests (Phase 7.1.2)

    #[test]
    fn test_block_kind_paragraph() {
        let kind = structure_type_to_block_kind(StructureType::P);
        assert_eq!(kind, BlockKind::Paragraph);
        assert_eq!(kind.as_str(), "paragraph");
        assert!(kind.is_emitted());
        assert!(kind.heading_level().is_none());
    }

    #[test]
    fn test_block_kind_heading_h() {
        // H (no explicit level) defaults to level 1
        let kind = structure_type_to_block_kind(StructureType::H);
        assert_eq!(kind, BlockKind::Heading { level: 1 });
        assert_eq!(kind.as_str(), "heading");
        assert!(kind.is_emitted());
        assert_eq!(kind.heading_level(), Some(1));
    }

    #[test]
    fn test_block_kind_heading_h1() {
        let kind = structure_type_to_block_kind(StructureType::H1);
        assert_eq!(kind, BlockKind::Heading { level: 1 });
        assert_eq!(kind.as_str(), "heading");
        assert_eq!(kind.heading_level(), Some(1));
    }

    #[test]
    fn test_block_kind_heading_h2() {
        let kind = structure_type_to_block_kind(StructureType::H2);
        assert_eq!(kind, BlockKind::Heading { level: 2 });
        assert_eq!(kind.as_str(), "heading");
        assert_eq!(kind.heading_level(), Some(2));
    }

    #[test]
    fn test_block_kind_heading_all_levels() {
        // Test all heading levels 1-6
        assert_eq!(structure_type_to_block_kind(StructureType::H1), BlockKind::Heading { level: 1 });
        assert_eq!(structure_type_to_block_kind(StructureType::H2), BlockKind::Heading { level: 2 });
        assert_eq!(structure_type_to_block_kind(StructureType::H3), BlockKind::Heading { level: 3 });
        assert_eq!(structure_type_to_block_kind(StructureType::H4), BlockKind::Heading { level: 4 });
        assert_eq!(structure_type_to_block_kind(StructureType::H5), BlockKind::Heading { level: 5 });
        assert_eq!(structure_type_to_block_kind(StructureType::H6), BlockKind::Heading { level: 6 });
    }

    #[test]
    fn test_block_kind_table() {
        let kind = structure_type_to_block_kind(StructureType::Table);
        assert_eq!(kind, BlockKind::Table);
        assert_eq!(kind.as_str(), "table");
        assert!(kind.is_emitted());
    }

    #[test]
    fn test_block_kind_list() {
        // L -> list
        let kind = structure_type_to_block_kind(StructureType::L);
        assert_eq!(kind, BlockKind::List);
        assert_eq!(kind.as_str(), "list");
        assert!(kind.is_emitted());
    }

    #[test]
    fn test_block_kind_list_item() {
        let kind = structure_type_to_block_kind(StructureType::LI);
        assert_eq!(kind, BlockKind::ListItem);
        assert_eq!(kind.as_str(), "list_item");
        assert!(kind.is_emitted());
    }

    #[test]
    fn test_block_kind_list_label() {
        let kind = structure_type_to_block_kind(StructureType::Lbl);
        assert_eq!(kind, BlockKind::ListLabel);
        assert_eq!(kind.as_str(), "list_label");
        assert!(kind.is_emitted());
    }

    #[test]
    fn test_block_kind_list_body() {
        let kind = structure_type_to_block_kind(StructureType::LBody);
        assert_eq!(kind, BlockKind::ListBody);
        assert_eq!(kind.as_str(), "list_body");
        assert!(kind.is_emitted());
    }

    #[test]
    fn test_block_kind_figure() {
        let kind = structure_type_to_block_kind(StructureType::Figure);
        assert_eq!(kind, BlockKind::Figure);
        assert_eq!(kind.as_str(), "figure");
        assert!(kind.is_emitted());
    }

    #[test]
    fn test_block_kind_caption() {
        let kind = structure_type_to_block_kind(StructureType::Caption);
        assert_eq!(kind, BlockKind::Caption);
        assert_eq!(kind.as_str(), "caption");
        assert!(kind.is_emitted());
    }

    #[test]
    fn test_block_kind_code() {
        let kind = structure_type_to_block_kind(StructureType::Code);
        assert_eq!(kind, BlockKind::Code);
        assert_eq!(kind.as_str(), "code");
        assert!(kind.is_emitted());
    }

    #[test]
    fn test_block_kind_block_quote() {
        let kind = structure_type_to_block_kind(StructureType::BlockQuote);
        assert_eq!(kind, BlockKind::BlockQuote);
        assert_eq!(kind.as_str(), "block_quote");
        assert!(kind.is_emitted());
    }

    #[test]
    fn test_block_kind_toc() {
        // TOC -> toc
        let kind = structure_type_to_block_kind(StructureType::Toc);
        assert_eq!(kind, BlockKind::Toc);
        assert_eq!(kind.as_str(), "toc");

        // TOCI also maps to toc
        let kind = structure_type_to_block_kind(StructureType::Toci);
        assert_eq!(kind, BlockKind::Toc);
    }

    #[test]
    fn test_block_kind_formula() {
        let kind = structure_type_to_block_kind(StructureType::Formula);
        assert_eq!(kind, BlockKind::Formula);
        assert_eq!(kind.as_str(), "formula");
        assert!(kind.is_emitted());
    }

    #[test]
    fn test_block_kind_reference() {
        let kind = structure_type_to_block_kind(StructureType::Reference);
        assert_eq!(kind, BlockKind::Reference);
        assert_eq!(kind.as_str(), "reference");
        assert!(kind.is_emitted());
    }

    #[test]
    fn test_block_kind_note() {
        let kind = structure_type_to_block_kind(StructureType::Note);
        assert_eq!(kind, BlockKind::Note);
        assert_eq!(kind.as_str(), "note");
        assert!(kind.is_emitted());
    }

    #[test]
    fn test_block_kind_form() {
        let kind = structure_type_to_block_kind(StructureType::Form);
        assert_eq!(kind, BlockKind::FormFieldStruct);
        assert_eq!(kind.as_str(), "form_field_struct");
        assert!(kind.is_emitted());
    }

    #[test]
    fn test_block_kind_inline_span() {
        let kind = structure_type_to_block_kind(StructureType::Span);
        assert_eq!(kind, BlockKind::Inline);
        assert_eq!(kind.as_str(), "inline");
        assert!(!kind.is_emitted()); // Inline elements are NOT emitted as blocks
    }

    #[test]
    fn test_block_kind_inline_quote() {
        let kind = structure_type_to_block_kind(StructureType::Quote);
        assert_eq!(kind, BlockKind::Inline);
        assert!(!kind.is_emitted());
    }

    #[test]
    fn test_block_kind_structural_container() {
        // Test all structural container types
        let containers = vec![
            StructureType::Document,
            StructureType::Part,
            StructureType::Art,
            StructureType::Sect,
            StructureType::Div,
            StructureType::NonStruct,
            StructureType::Private,
            StructureType::Index,
            StructureType::TR,
            StructureType::TH,
            StructureType::TD,
            StructureType::THead,
            StructureType::TBody,
            StructureType::TFoot,
        ];

        for std_type in containers {
            let kind = structure_type_to_block_kind(std_type);
            assert_eq!(kind, BlockKind::StructuralContainer);
            assert!(!kind.is_emitted()); // Structural containers are NOT emitted as blocks
        }
    }

    #[test]
    fn test_block_kind_unknown() {
        let kind = structure_type_to_block_kind(StructureType::Unknown);
        assert_eq!(kind, BlockKind::Unknown);
        assert_eq!(kind.as_str(), "paragraph"); // Unknown falls back to "paragraph" string
        assert!(kind.is_emitted()); // Unknown IS emitted (as paragraph fallback)
    }

    #[test]
    fn test_mapping_result_for_paragraph() {
        let node = StructElemNode::new("P".to_string(), StructureType::P);
        let result = map_element_to_block(&node);

        assert_eq!(result.block_kind, BlockKind::Paragraph);
        assert!(result.is_emitted);
        assert!(result.diagnostic.is_none()); // No diagnostic for known types
    }

    #[test]
    fn test_mapping_result_for_heading_with_level() {
        let node = StructElemNode::new("H2".to_string(), StructureType::H2);
        let result = map_element_to_block(&node);

        assert_eq!(result.block_kind, BlockKind::Heading { level: 2 });
        assert!(result.is_emitted);
        assert_eq!(result.block_kind.heading_level(), Some(2));
        assert!(result.diagnostic.is_none());
    }

    #[test]
    fn test_mapping_result_for_unknown_type() {
        let node = StructElemNode::new("CustomType".to_string(), StructureType::Unknown);
        let result = map_element_to_block(&node);

        assert_eq!(result.block_kind, BlockKind::Unknown);
        assert!(result.is_emitted); // Unknown types ARE emitted (as paragraph)
        assert!(result.diagnostic.is_some()); // Should have diagnostic
        assert!(result.diagnostic.unwrap().message.contains("Unknown structure type"));
    }

    #[test]
    fn test_mapping_result_for_inline_element() {
        let node = StructElemNode::new("Span".to_string(), StructureType::Span);
        let result = map_element_to_block(&node);

        assert_eq!(result.block_kind, BlockKind::Inline);
        assert!(!result.is_emitted); // Inline NOT emitted as separate block
        assert!(result.diagnostic.is_none());
    }

    #[test]
    fn test_mapping_result_for_structural_container() {
        let node = StructElemNode::new("Div".to_string(), StructureType::Div);
        let result = map_element_to_block(&node);

        assert_eq!(result.block_kind, BlockKind::StructuralContainer);
        assert!(!result.is_emitted); // Structural container NOT emitted as block
        assert!(result.diagnostic.is_none());
    }

    #[test]
    fn test_list_nesting_mapping() {
        // Test that list elements map correctly for nested structures
        let list_kind = structure_type_to_block_kind(StructureType::L);
        let item_kind = structure_type_to_block_kind(StructureType::LI);
        let label_kind = structure_type_to_block_kind(StructureType::Lbl);
        let body_kind = structure_type_to_block_kind(StructureType::LBody);

        assert_eq!(list_kind, BlockKind::List);
        assert_eq!(item_kind, BlockKind::ListItem);
        assert_eq!(label_kind, BlockKind::ListLabel);
        assert_eq!(body_kind, BlockKind::ListBody);

        // All should be emitted
        assert!(list_kind.is_emitted());
        assert!(item_kind.is_emitted());
        assert!(label_kind.is_emitted());
        assert!(body_kind.is_emitted());
    }

    #[test]
    fn test_table_grouping_mapping() {
        // Test that table row/cell types map to structural containers
        let tr_kind = structure_type_to_block_kind(StructureType::TR);
        let th_kind = structure_type_to_block_kind(StructureType::TH);
        let td_kind = structure_type_to_block_kind(StructureType::TD);
        let thead_kind = structure_type_to_block_kind(StructureType::THead);
        let tbody_kind = structure_type_to_block_kind(StructureType::TBody);
        let tfoot_kind = structure_type_to_block_kind(StructureType::TFoot);

        // All should map to structural container (descend without emitting block)
        assert_eq!(tr_kind, BlockKind::StructuralContainer);
        assert_eq!(th_kind, BlockKind::StructuralContainer);
        assert_eq!(td_kind, BlockKind::StructuralContainer);
        assert_eq!(thead_kind, BlockKind::StructuralContainer);
        assert_eq!(tbody_kind, BlockKind::StructuralContainer);
        assert_eq!(tfoot_kind, BlockKind::StructuralContainer);

        // None should be emitted
        assert!(!tr_kind.is_emitted());
        assert!(!th_kind.is_emitted());
        assert!(!td_kind.is_emitted());
        assert!(!thead_kind.is_emitted());
        assert!(!tbody_kind.is_emitted());
        assert!(!tfoot_kind.is_emitted());
    }

    #[test]
    fn test_span_passthrough() {
        // Test that inline elements like Span are not emitted as blocks
        let inline_types = vec![
            StructureType::Span,
            StructureType::Quote,
            StructureType::BibEntry,
            StructureType::Link,
            StructureType::Annot,
            StructureType::Ruby,
            StructureType::RB,
            StructureType::RT,
            StructureType::RP,
            StructureType::Warichu,
            StructureType::WT,
            StructureType::WP,
        ];

        for std_type in inline_types {
            let kind = structure_type_to_block_kind(std_type);
            assert!(!kind.is_emitted(), "Type {:?} should not be emitted", std_type);
        }
    }

    #[test]
    fn test_heading_level_not_auto_incremented() {
        // Test that nested H elements do NOT auto-increment level
        // (spec leaves this to the producer)
        let h_kind = structure_type_to_block_kind(StructureType::H);
        let h1_kind = structure_type_to_block_kind(StructureType::H1);

        // Both H and H1 have level 1 - no auto-increment
        assert_eq!(h_kind.heading_level(), Some(1));
        assert_eq!(h1_kind.heading_level(), Some(1));
    }
}
