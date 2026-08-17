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

use crate::diagnostics::{DiagCode, Diagnostic};
use crate::parser::catalog::{MarkInfo, ReadingOrderAlgorithm};
use crate::parser::marked_content::CoverageResult;
use crate::parser::object::{ObjRef, PdfObject};
use crate::parser::xref::XrefResolver;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
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
    /// Document - root of the structure hierarchy
    Document,
    /// Part - major division of a document
    Part,
    /// Art - self-contained region of content
    Art,
    /// Sect - section of a document
    Sect,
    /// Div - generic grouping element
    Div,
    /// BlockQuote - block quotation
    BlockQuote,
    /// Caption - caption for table or figure
    Caption,
    /// Toc - table of contents
    Toc,
    /// Toci - table of contents item
    Toci,
    /// Index - index section
    Index,
    /// NonStruct - non-structural element
    NonStruct,
    /// Private - private use
    Private,

    // Block-level elements
    /// P - paragraph
    P,
    /// H - heading (level unspecified)
    H,
    /// H1 - level 1 heading
    H1,
    /// H2 - level 2 heading
    H2,
    /// H3 - level 3 heading
    H3,
    /// H4 - level 4 heading
    H4,
    /// H5 - level 5 heading
    H5,
    /// H6 - level 6 heading
    H6,
    /// L - list
    L,
    /// LI - list item
    LI,
    /// Lbl - label for list item
    Lbl,
    /// LBody - list item body
    LBody,
    /// Table - table
    Table,
    /// TR - table row
    TR,
    /// TH - table header cell
    TH,
    /// TD - table data cell
    TD,
    /// THead - table header section
    THead,
    /// TBody - table body section
    TBody,
    /// TFoot - table footer section
    TFoot,

    // Inline elements
    /// Span - inline span
    Span,
    /// Quote - inline quotation
    Quote,
    /// Note - footnote or endnote
    Note,
    /// Reference - bibliographic reference
    Reference,
    /// BibEntry - bibliography entry
    BibEntry,
    /// Code - code fragment
    Code,
    /// Link - hyperlink
    Link,
    /// Annot - annotation
    Annot,
    /// Ruby - ruby annotation container
    Ruby,
    /// RB - ruby base text
    RB,
    /// RT - ruby text
    RT,
    /// RP - ruby parenthesis
    RP,
    /// Warichu - warichu annotation container
    Warichu,
    /// WT - warichu text
    WT,
    /// WP - warichu parenthesis
    WP,

    // Illustration/media
    /// Figure - figure/illustration
    Figure,
    /// Formula - mathematical formula
    Formula,
    /// Form - interactive form
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
        matches!(
            self,
            StructureType::H
                | StructureType::H1
                | StructureType::H2
                | StructureType::H3
                | StructureType::H4
                | StructureType::H5
                | StructureType::H6
        )
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
    /// A marked content reference (MCID on a specific page).
    Mcr {
        /// Page object reference containing the marked content.
        page: ObjRef,
        /// Marked content identifier on that page.
        mcid: u32,
    },
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

/// ParentTree entry for a page or annotation.
///
/// The ParentTree is a number tree where each key is a /StructParents value
/// and the value is either:
/// - An array of StructElem refs (for pages, indexed by MCID)
/// - A single StructElem ref (for annotations with /StructParent)
#[derive(Debug, Clone)]
pub enum ParentTreeEntry {
    /// Array of StructElem refs indexed by MCID (for pages)
    Array(Vec<ObjRef>),
    /// Single StructElem ref (for annotations)
    Single(ObjRef),
}

/// ParentTree resolver.
///
/// Caches the resolved ParentTree and provides per-page MCID-to-StructElem mapping.
#[derive(Debug, Clone)]
pub struct ParentTreeResolver {
    /// Map from /StructParents key to ParentTree entry
    entries: HashMap<i32, ParentTreeEntry>,
    /// Diagnostics emitted during parsing
    diagnostics: Vec<Diagnostic>,
    /// Map from object reference to parsed StructElem node
    /// Set after struct tree parsing is complete
    struct_elems: HashMap<ObjRef, Rc<StructElemNode>>,
}

impl ParentTreeResolver {
    /// Create a new empty ParentTreeResolver.
    pub fn new() -> Self {
        ParentTreeResolver {
            entries: HashMap::new(),
            diagnostics: Vec::new(),
            struct_elems: HashMap::new(),
        }
    }

    /// Set the struct_elems map after parsing is complete.
    pub(crate) fn set_struct_elems(&mut self, struct_elems: HashMap<ObjRef, Rc<StructElemNode>>) {
        self.struct_elems = struct_elems;
    }

    /// Parse a ParentTree from a StructTreeRoot dictionary.
    ///
    /// # Arguments
    ///
    /// * `resolver` - The xref resolver
    /// * `struct_tree_root` - The StructTreeRoot dictionary (must contain /ParentTree)
    ///
    /// # Returns
    ///
    /// A `ParentTreeResolver` with all entries parsed from the number tree.
    pub fn parse(resolver: &XrefResolver, struct_tree_root: &PdfObject) -> Self {
        let mut resolver_impl = Self::new();

        // Get the /ParentTree entry (may be indirect reference)
        let parent_tree_obj = match struct_tree_root.as_dict() {
            Some(dict) => dict.get("ParentTree"),
            None => {
                resolver_impl
                    .diagnostics
                    .push(Diagnostic::with_dynamic_no_offset(
                        DiagCode::StructMissingKey,
                        "StructTreeRoot is not a dictionary".to_string(),
                    ));
                return resolver_impl;
            }
        };

        let parent_tree_obj = match parent_tree_obj {
            Some(obj) => obj,
            None => {
                // No ParentTree is valid - just return empty resolver
                return resolver_impl;
            }
        };

        // Resolve if it's an indirect reference
        let tree_obj = match parent_tree_obj.as_ref() {
            Some(ref_obj) => match resolver.resolve(ref_obj) {
                Ok(obj) => obj,
                Err(e) => {
                    resolver_impl
                        .diagnostics
                        .push(Diagnostic::with_dynamic_no_offset(
                            DiagCode::StructUnexpectedEof,
                            format!("Failed to resolve ParentTree reference {}: {}", ref_obj, e),
                        ));
                    return resolver_impl;
                }
            },
            None => parent_tree_obj.clone(),
        };

        // Walk the number tree
        walk_number_tree(resolver, &tree_obj, &mut resolver_impl);

        resolver_impl
    }

    /// Resolve MCIDs for a page to their owning StructElem nodes.
    ///
    /// # Arguments
    ///
    /// * `struct_parents` - The /StructParents value from the page dictionary
    ///
    /// # Returns
    ///
    /// A map from MCID to StructElem node, plus a set of orphan MCIDs (those present
    /// in content but not claimed by any StructElem).
    pub fn resolve_page(
        &self,
        struct_parents: Option<i32>,
    ) -> (HashMap<u32, Rc<StructElemNode>>, Vec<u32>) {
        let struct_parents = match struct_parents {
            Some(sp) => sp,
            None => {
                // No /StructParents - no MCIDs can be resolved
                return (HashMap::new(), Vec::new());
            }
        };

        let entry = match self.entries.get(&struct_parents) {
            Some(e) => e,
            None => {
                // /StructParents key not found in ParentTree - all MCIDs are orphans
                return (HashMap::new(), Vec::new());
            }
        };

        match entry {
            ParentTreeEntry::Array(refs) => {
                let mut map = HashMap::new();
                let mut orphans = Vec::new();

                for (mcid, elem_ref) in refs.iter().enumerate() {
                    // Check if this is a "null" object reference (object = 0)
                    if elem_ref.object == 0 {
                        // Null entry means this MCID is an orphan
                        orphans.push(mcid as u32);
                    } else {
                        // Look up the StructElem node from the struct_elems map
                        if let Some(node) = self.struct_elems.get(elem_ref) {
                            map.insert(mcid as u32, Rc::clone(node));
                        } else {
                            // Reference not found in struct_elems - treat as orphan
                            orphans.push(mcid as u32);
                        }
                    }
                }

                (map, orphans)
            }
            ParentTreeEntry::Single(ref_obj) => {
                // Single entry - treat as if MCID 0 maps to this ref
                let mut map = HashMap::new();
                if let Some(node) = self.struct_elems.get(ref_obj) {
                    map.insert(0, Rc::clone(node));
                } else {
                    // Reference not found - MCID 0 is orphan
                    return (HashMap::new(), vec![0]);
                }
                (map, Vec::new())
            }
        }
    }

    /// Resolve an annotation's /StructParent to its owning StructElem ref.
    ///
    /// # Arguments
    ///
    /// * `struct_parent` - The /StructParent value from the annotation dictionary
    ///
    /// # Returns
    ///
    /// The StructElem ref if found, None otherwise.
    pub fn resolve_annotation(&self, struct_parent: Option<i32>) -> Option<ObjRef> {
        let struct_parent = struct_parent?;

        let entry = self.entries.get(&struct_parent)?;

        match entry {
            ParentTreeEntry::Single(ref_obj) => Some(*ref_obj),
            ParentTreeEntry::Array(refs) => {
                // Annotations should always map to Single, but if we get an Array,
                // use the first entry as a fallback
                if refs.is_empty() {
                    None
                } else {
                    Some(refs[0])
                }
            }
        }
    }

    /// Get all diagnostics emitted during parsing.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Compute StructTree coverage for a page.
    ///
    /// This method calculates the coverage ratio for the Suspects fallback check:
    /// - claimed_mcids: MCIDs that resolve to a non-Artifact StructElem
    /// - total_mcids: Total MCIDs emitted in marked-content sequences
    ///
    /// # Arguments
    ///
    /// * `page_index` - The page index (0-based)
    /// * `struct_parents` - The /StructParents value from the page dictionary
    /// * `all_mcids` - All MCIDs seen in marked-content sequences on this page
    ///
    /// # Returns
    ///
    /// A `CoverageResult` containing the coverage ratio and fallback decision.
    ///
    /// # Coverage Calculation
    ///
    /// Coverage = claimed_mcids / total_mcids
    ///
    /// Where:
    /// - claimed_mcids = MCIDs that resolved to a StructElem (non-null ParentTree entries)
    /// - total_mcids = All MCIDs from marked-content sequences (from MCID tracker)
    ///
    /// If total_mcids == 0 (no marked content), coverage is 0.0 and fallback is recommended.
    /// The fallback threshold is hard-coded at 0.80 (80%) per the plan.
    pub fn compute_coverage(
        &self,
        page_index: usize,
        struct_parents: Option<i32>,
        all_mcids: &std::collections::HashSet<u32>,
    ) -> crate::parser::marked_content::CoverageResult {
        use crate::parser::marked_content::compute_coverage_from_sets;

        // Resolve MCIDs to StructElems
        let (claimed_map, _orphans) = self.resolve_page(struct_parents);

        // Build set of claimed MCIDs
        let claimed_mcids: std::collections::HashSet<u32> = claimed_map.keys().cloned().collect();

        // Compute coverage using the sets
        compute_coverage_from_sets(page_index, all_mcids, &claimed_mcids)
    }
}

impl Default for ParentTreeResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-page coverage check result for Phase 7.1.4 Suspects fallback.
///
/// Contains the coverage result for each page and the overall reading order algorithm.
#[derive(Debug, Clone)]
pub struct CoverageCheckResult {
    /// Per-page coverage results
    pub page_results: Vec<CoverageResult>,
    /// The reading order algorithm to use for the document
    pub reading_order_algorithm: ReadingOrderAlgorithm,
    /// Diagnostics emitted during coverage check
    pub diagnostics: Vec<Diagnostic>,
}

impl CoverageCheckResult {
    /// Create a new coverage check result.
    fn new() -> Self {
        CoverageCheckResult {
            page_results: Vec::new(),
            reading_order_algorithm: ReadingOrderAlgorithm::StructTree,
            diagnostics: Vec::new(),
        }
    }
}

/// Check StructTree coverage for all pages and determine reading order algorithm.
///
/// This function implements Phase 7.1.4: if /MarkInfo /Suspects is true,
/// compute per-page coverage and fall back to XY-cut for pages with coverage < 80%.
///
/// # Arguments
///
/// * `struct_tree` - The parsed structure tree with ParentTree resolver
/// * `mark_info` - The MarkInfo from catalog (checked for /Suspects flag)
/// * `pages_with_mcids` - Slice of (page_index, struct_parents, mcid_count) tuples
///
/// # Returns
///
/// A `CoverageCheckResult` containing per-page coverage results and the overall
/// reading order algorithm to use.
///
/// # Reading Order Algorithm Selection
///
/// - If /Suspects is false: use StructTree for all pages
/// - If /Suspects is true:
///   - Compute coverage for each page: claimed_mcids / total_mcids
///   - If coverage < 80% on any page: use XY-cut for the entire document
///   - Otherwise: use StructTree
///
/// # Coverage Calculation
///
/// Coverage = claimed_mcids / total_mcids
///
/// Where:
/// - claimed_mcids: MCIDs that resolve to a non-Artifact StructElem via ParentTree
/// - total_mcids: All MCIDs emitted in marked-content sequences on this page
///
/// If total_mcids == 0 (no marked content), coverage is 0.0 and the page
/// triggers fallback if /Suspects is true.
pub fn check_coverage_for_pages(
    struct_tree: &StructTreeRoot,
    mark_info: &MarkInfo,
    pages_with_mcids: &[(usize, Option<i32>, std::collections::HashSet<u32>)],
) -> CoverageCheckResult {
    use crate::parser::catalog::ReadingOrderAlgorithm;

    let mut result = CoverageCheckResult::new();

    // Always compute coverage for each page (needed for diagnostics and transparency)
    // But only apply fallback logic when /Suspects is true
    let suspects_mode = mark_info.requires_coverage_check();
    let mut any_fallback = false;

    for (page_index, struct_parents, all_mcids) in pages_with_mcids {
        // Compute coverage using ParentTreeResolver
        let coverage_result =
            struct_tree
                .parent_tree
                .compute_coverage(*page_index, *struct_parents, &all_mcids);

        // Apply Suspects mode to determine actual fallback behavior
        let coverage_result = coverage_result.with_suspects_mode(suspects_mode);

        // Track if any page should fall back (only matters in Suspects mode)
        if coverage_result.should_fallback {
            any_fallback = true;
        }

        result.page_results.push(coverage_result);
    }

    // Determine reading order algorithm
    // If /Suspects is false, always use StructTree
    // If /Suspects is true and any page falls back, use XY-cut for the entire document
    result.reading_order_algorithm = if !suspects_mode {
        ReadingOrderAlgorithm::StructTree
    } else if any_fallback {
        ReadingOrderAlgorithm::XyCut
    } else {
        ReadingOrderAlgorithm::StructTree
    };

    // Emit diagnostics for pages that triggered fallback (only in Suspects mode)
    if suspects_mode {
        for page_result in &result.page_results {
            if let Some(diag_message) = page_result.fallback_diagnostic() {
                result.diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructIncompleteCoverage,
                    diag_message,
                ));
            }
        }
    }

    result
}

/// Walk a number tree and extract all key-value pairs.
///
/// Number trees use the same structure as name trees (ISO 32000-2 §7.9.6):
/// - Root node has either /Nums (leaf) or /Kids (intermediate) + /Limits
/// - Intermediate nodes have /Kids + /Limits
/// - Leaf nodes have /Nums array: [key1, value1, key2, value2, ...]
///
/// # Arguments
///
/// * `resolver` - The xref resolver
/// * `node_obj` - The root node of the number tree
/// * `parent_resolver` - The ParentTreeResolver to populate
fn walk_number_tree(
    resolver: &XrefResolver,
    node_obj: &PdfObject,
    parent_resolver: &mut ParentTreeResolver,
) {
    let dict = match node_obj.as_dict() {
        Some(d) => d,
        None => {
            parent_resolver
                .diagnostics
                .push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructInvalidType,
                    format!(
                        "Number tree node is not a dictionary (type: {})",
                        node_obj.type_name()
                    ),
                ));
            return;
        }
    };

    // Check if this is a leaf node (has /Nums) or intermediate node (has /Kids)
    let nums = dict.get("Nums");
    let kids = dict.get("Kids");

    if let Some(nums_array) = nums {
        // Leaf node - process /Nums array
        process_nums_array(nums_array, parent_resolver);
    } else if let Some(kids_array) = kids {
        // Intermediate node - recurse into /Kids
        if let Some(arr) = kids_array.as_array() {
            for kid_obj in arr.as_ref() {
                if let Some(kid_ref) = kid_obj.as_ref() {
                    match resolver.resolve(kid_ref) {
                        Ok(kid_node) => walk_number_tree(resolver, &kid_node, parent_resolver),
                        Err(e) => {
                            parent_resolver
                                .diagnostics
                                .push(Diagnostic::with_dynamic_no_offset(
                                    DiagCode::StructUnexpectedEof,
                                    format!("Failed to resolve number tree kid {}: {}", kid_ref, e),
                                ));
                        }
                    }
                } else {
                    walk_number_tree(resolver, kid_obj, parent_resolver);
                }
            }
        }
    } else {
        // Neither /Nums nor /Kids - invalid number tree node
        parent_resolver
            .diagnostics
            .push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructMissingKey,
                "Number tree node has neither /Nums nor /Kids".to_string(),
            ));
    }
}

/// Process a /Nums array from a number tree leaf node.
///
/// The /Nums array contains alternating key-value pairs: [key1, value1, key2, value2, ...]
/// where keys are integers and values are either arrays (for pages) or single refs (for annotations).
fn process_nums_array(nums_obj: &PdfObject, parent_resolver: &mut ParentTreeResolver) {
    let nums = match nums_obj.as_array() {
        Some(arr) => arr.as_ref(),
        None => {
            parent_resolver
                .diagnostics
                .push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructInvalidType,
                    format!("/Nums is not an array (type: {})", nums_obj.type_name()),
                ));
            return;
        }
    };

    // Process pairs: [key1, value1, key2, value2, ...]
    let mut chunks = nums.chunks_exact(2);
    for chunk in &mut chunks {
        let key_obj = &chunk[0];
        let value_obj = &chunk[1];

        // Extract the key (must be an integer)
        let key = match key_obj.as_int() {
            Some(k) => k as i32, // Convert i64 to i32 for the HashMap key
            None => {
                parent_resolver
                    .diagnostics
                    .push(Diagnostic::with_dynamic_no_offset(
                        DiagCode::StructInvalidType,
                        format!(
                            "Number tree key is not an integer (type: {})",
                            key_obj.type_name()
                        ),
                    ));
                continue;
            }
        };

        // Extract the value
        let entry = match value_obj {
            PdfObject::Array(arr) => {
                // Array of refs (for pages)
                // Null entries are preserved as ObjRef { object: 0 } to mark orphan MCIDs
                let refs: Vec<ObjRef> = arr
                    .as_ref()
                    .iter()
                    .map(|o| match o {
                        PdfObject::Ref(r) => *r,
                        PdfObject::Null => ObjRef {
                            object: 0,
                            generation: 0,
                        },
                        _ => ObjRef {
                            object: 0,
                            generation: 0,
                        }, // Invalid ref treated as null
                    })
                    .collect();
                ParentTreeEntry::Array(refs)
            }
            PdfObject::Ref(ref_obj) => {
                // Single ref (for annotations)
                ParentTreeEntry::Single(*ref_obj)
            }
            PdfObject::Null => {
                // Null entry - treat as empty array
                ParentTreeEntry::Array(Vec::new())
            }
            _ => {
                parent_resolver
                    .diagnostics
                    .push(Diagnostic::with_dynamic_no_offset(
                        DiagCode::StructInvalidType,
                        format!(
                            "Number tree value has unsupported type: {}",
                            value_obj.type_name()
                        ),
                    ));
                continue;
            }
        };

        parent_resolver.entries.insert(key, entry);
    }

    // Check for trailing element (odd-length array)
    if !chunks.remainder().is_empty() {
        parent_resolver
            .diagnostics
            .push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructInvalidType,
                "Number tree /Nums array has odd length (trailing element without value)"
                    .to_string(),
            ));
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
    /// ParentTree resolver for MCID-to-StructElem mapping
    pub parent_tree: ParentTreeResolver,
    /// Diagnostics emitted during parsing
    pub diagnostics: Vec<Diagnostic>,
    /// Map from object reference to parsed StructElem node
    /// Used by ParentTreeResolver to resolve MCIDs to actual nodes
    pub(crate) struct_elems: HashMap<ObjRef, Rc<StructElemNode>>,
}

impl StructTreeRoot {
    /// Create a new empty StructTreeRoot.
    pub fn new() -> Self {
        StructTreeRoot {
            kids: Vec::new(),
            role_map: RoleMap::new(),
            parent_tree: ParentTreeResolver::new(),
            diagnostics: Vec::new(),
            struct_elems: HashMap::new(),
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
    fn resolve(
        &self,
        type_name: &str,
        diagnostics: &mut Vec<Diagnostic>,
        visited: &mut HashSet<String>,
    ) -> StructureType {
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
pub fn parse_struct_tree(
    resolver: &XrefResolver,
    struct_tree_root_ref: ObjRef,
) -> Result<StructTreeRoot> {
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
                format!(
                    "StructTreeRoot is not a dictionary (type: {})",
                    root_obj.type_name()
                ),
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
                        format!(
                            "Failed to resolve RoleMap reference {}: {}",
                            role_map_ref, e
                        ),
                    ));
                    // Use empty RoleMap (already initialized in new())
                }
            }
        } else {
            root.role_map = RoleMap::parse(role_map_obj);
        }
    }

    // Parse the ParentTree
    root.parent_tree = ParentTreeResolver::parse(resolver, &root_obj);
    diagnostics.extend(root.parent_tree.diagnostics().iter().cloned());

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
    let mut struct_elems = HashMap::new();
    root.kids = walk_kids(
        resolver,
        kids_array,
        &root.role_map,
        &mut diagnostics,
        &mut visited,
        &mut struct_elems,
        None, // No parent lang at root
        None, // No parent actual_text at root
    );

    // Store the struct_elems map and set it on the ParentTreeResolver
    root.struct_elems = struct_elems;
    root.parent_tree.set_struct_elems(root.struct_elems.clone());

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
/// * `struct_elems` - Map to populate with ObjRef -> StructElemNode
/// * `parent_lang` - Inherited language from parent
/// * `parent_actual_text` - Inherited actual_text from parent
fn walk_kids(
    resolver: &XrefResolver,
    kids_obj: &PdfObject,
    role_map: &RoleMap,
    diagnostics: &mut Vec<Diagnostic>,
    visited: &mut HashSet<ObjRef>,
    struct_elems: &mut HashMap<ObjRef, Rc<StructElemNode>>,
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
            struct_elems,
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
    struct_elems: &mut HashMap<ObjRef, Rc<StructElemNode>>,
    parent_lang: Option<&str>,
    parent_actual_text: Option<&str>,
) -> Option<Kid> {
    match entry {
        // Integer MCID
        PdfObject::Integer(mcid) if *mcid >= 0 => Some(Kid::Mcid(*mcid as u32)),

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
                            return Some(Kid::Mcr {
                                page,
                                mcid: mcid as u32,
                            });
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
                struct_elems,
                parent_lang,
                parent_actual_text,
                Some(*obj_ref),
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
                        return Some(Kid::Mcr {
                            page,
                            mcid: mcid as u32,
                        });
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

            // Otherwise, treat as a StructElem (no object ref available for direct dict)
            let elem_node = parse_struct_elem(
                resolver,
                entry,
                role_map,
                diagnostics,
                visited,
                struct_elems,
                parent_lang,
                parent_actual_text,
                None, // No ObjRef for direct dict
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
    struct_elems: &mut HashMap<ObjRef, Rc<StructElemNode>>,
    parent_lang: Option<&str>,
    parent_actual_text: Option<&str>,
    obj_ref: Option<ObjRef>,
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
    let actual_text = dict
        .get("ActualText")
        .and_then(|a| a.as_string())
        .and_then(|bytes| std::str::from_utf8(bytes).ok().map(|s| s.to_string()));

    // Use parent's actual_text if we don't have our own
    node.actual_text = actual_text.or_else(|| parent_actual_text.map(|s| s.to_string()));

    // Extract /Lang (language tag, inherits from parent)
    let lang = dict
        .get("Lang")
        .and_then(|l| l.as_string())
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
            struct_elems,
            inherited_lang,
            inherited_actual_text,
        );
    }

    // Store the node in the struct_elems map if we have an object reference
    if let Some(ref obj_ref) = obj_ref {
        struct_elems.insert(*obj_ref, Rc::new(node.clone()));
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
    Heading {
        /// Heading level (1 = highest, 6 = lowest)
        level: u8,
    },
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
        !matches!(
            self,
            BlockKind::Inline | BlockKind::StructuralContainer | BlockKind::Artifact
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
        StructureType::TR => BlockKind::StructuralContainer, // Table row - container
        StructureType::TH => BlockKind::StructuralContainer, // Table header cell
        StructureType::TD => BlockKind::StructuralContainer, // Table data cell
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
pub fn is_artifact(_std_type: StructureType) -> bool {
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
                "Unknown structure type after RoleMap resolution, falling back to paragraph"
                    .to_string(),
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
        assert_eq!(
            StructureType::from_name("UnknownType"),
            StructureType::Unknown
        );
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
        assert_eq!(
            role_map.resolve("P", &mut diagnostics, &mut visited),
            StructureType::P
        );

        // Mapped type resolves through RoleMap
        assert_eq!(
            role_map.resolve("Heading1", &mut diagnostics, &mut visited),
            StructureType::H1
        );

        // Unknown type returns Unknown
        assert_eq!(
            role_map.resolve("FooBar", &mut diagnostics, &mut visited),
            StructureType::Unknown
        );
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
        assert_eq!(
            role_map.resolve("CustomA", &mut diagnostics, &mut visited),
            StructureType::H1
        );
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
        assert_eq!(
            role_map.resolve("CustomA", &mut diagnostics, &mut visited),
            StructureType::NonStruct
        );
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
        child_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(0), // MCID
            ])),
        );
        let child_ref = ObjRef::new(11, 0);
        resolver.cache_object(child_ref, PdfObject::Dict(Box::new(child_dict)));

        // Create StructTreeRoot
        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(child_ref)])),
        );
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
        parent_dict.insert(
            intern("Lang"),
            PdfObject::String(Box::new(b"en-US".to_vec())),
        );
        let parent_ref = ObjRef::new(11, 0);
        resolver.cache_object(parent_ref, PdfObject::Dict(Box::new(parent_dict)));

        // Child without /Lang (should inherit)
        let mut child_dict = PdfDict::new();
        child_dict.insert(intern("S"), PdfObject::Name(intern("P")));
        child_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Integer(0)])),
        );
        let child_ref = ObjRef::new(12, 0);
        resolver.cache_object(child_ref, PdfObject::Dict(Box::new(child_dict)));

        // Create parent's /K with child
        let mut parent_with_k = PdfDict::new();
        parent_with_k.insert(intern("S"), PdfObject::Name(intern("Div")));
        parent_with_k.insert(
            intern("Lang"),
            PdfObject::String(Box::new(b"en-US".to_vec())),
        );
        parent_with_k.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(child_ref)])),
        );
        resolver.cache_object(parent_ref, PdfObject::Dict(Box::new(parent_with_k)));

        // Create StructTreeRoot
        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(parent_ref)])),
        );
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
        parent_dict.insert(
            intern("ActualText"),
            PdfObject::String(Box::new(b"Parent text".to_vec())),
        );
        let parent_ref = ObjRef::new(11, 0);

        // Child without /ActualText (should inherit parent's)
        let mut child_dict = PdfDict::new();
        child_dict.insert(intern("S"), PdfObject::Name(intern("P")));
        child_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Integer(0)])),
        );
        let child_ref = ObjRef::new(12, 0);
        resolver.cache_object(child_ref, PdfObject::Dict(Box::new(child_dict)));

        // Create parent's /K with child
        let mut parent_with_k = PdfDict::new();
        parent_with_k.insert(intern("S"), PdfObject::Name(intern("Div")));
        parent_with_k.insert(
            intern("ActualText"),
            PdfObject::String(Box::new(b"Parent text".to_vec())),
        );
        parent_with_k.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(child_ref)])),
        );
        resolver.cache_object(parent_ref, PdfObject::Dict(Box::new(parent_with_k)));

        // Create StructTreeRoot
        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(parent_ref)])),
        );
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
        root_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(mcr_ref)])),
        );
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
        root_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(objr_ref)])),
        );
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
        root_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Integer(123)])),
        );
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
        assert_eq!(
            structure_type_to_block_kind(StructureType::H1),
            BlockKind::Heading { level: 1 }
        );
        assert_eq!(
            structure_type_to_block_kind(StructureType::H2),
            BlockKind::Heading { level: 2 }
        );
        assert_eq!(
            structure_type_to_block_kind(StructureType::H3),
            BlockKind::Heading { level: 3 }
        );
        assert_eq!(
            structure_type_to_block_kind(StructureType::H4),
            BlockKind::Heading { level: 4 }
        );
        assert_eq!(
            structure_type_to_block_kind(StructureType::H5),
            BlockKind::Heading { level: 5 }
        );
        assert_eq!(
            structure_type_to_block_kind(StructureType::H6),
            BlockKind::Heading { level: 6 }
        );
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
        assert!(result
            .diagnostic
            .unwrap()
            .message
            .contains("Unknown structure type"));
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
            assert!(
                !kind.is_emitted(),
                "Type {:?} should not be emitted",
                std_type
            );
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

    // ParentTree number tree tests (Phase 7.1.3)

    #[test]
    fn test_parent_tree_resolver_new() {
        let resolver = ParentTreeResolver::new();
        assert!(resolver.entries.is_empty());
        assert!(resolver.diagnostics.is_empty());
    }

    #[test]
    fn test_parent_tree_resolver_default() {
        let resolver = ParentTreeResolver::default();
        assert!(resolver.entries.is_empty());
    }

    #[test]
    fn test_parent_tree_leaf_nums() {
        // Test parsing a simple leaf number tree with /Nums array
        let resolver = XrefResolver::new();

        // Create /Nums array: [0, [ref1, ref2], 1, [ref3]]
        let struct_elem1_ref = ObjRef::new(10, 0);
        let struct_elem2_ref = ObjRef::new(11, 0);
        let struct_elem3_ref = ObjRef::new(12, 0);

        let nums_array = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Array(Box::new(vec![
                PdfObject::Ref(struct_elem1_ref),
                PdfObject::Ref(struct_elem2_ref),
            ])),
            PdfObject::Integer(1),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(struct_elem3_ref)])),
        ]));

        // Wrap in a StructTreeRoot-like structure with /ParentTree
        let mut parent_tree_dict = PdfDict::new();
        parent_tree_dict.insert(intern("Nums"), nums_array);
        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(parent_tree_dict)),
        );
        let root_obj = PdfObject::Dict(Box::new(root_dict));

        // Parse
        let parent_resolver = ParentTreeResolver::parse(&resolver, &root_obj);

        // Verify entries
        assert_eq!(parent_resolver.entries.len(), 2);

        // Key 0 should map to array with 2 refs
        match parent_resolver.entries.get(&0) {
            Some(ParentTreeEntry::Array(refs)) => {
                assert_eq!(refs.len(), 2);
                assert_eq!(refs[0], struct_elem1_ref);
                assert_eq!(refs[1], struct_elem2_ref);
            }
            _ => panic!("Expected Array entry for key 0"),
        }

        // Key 1 should map to array with 1 ref
        match parent_resolver.entries.get(&1) {
            Some(ParentTreeEntry::Array(refs)) => {
                assert_eq!(refs.len(), 1);
                assert_eq!(refs[0], struct_elem3_ref);
            }
            _ => panic!("Expected Array entry for key 1"),
        }
    }

    #[test]
    fn test_parent_tree_single_ref() {
        // Test parsing a number tree with single refs (for annotations)
        let resolver = XrefResolver::new();

        let annot_ref = ObjRef::new(20, 0);

        let nums_array = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(5),
            PdfObject::Ref(annot_ref),
        ]));

        // Wrap in a StructTreeRoot-like structure with /ParentTree
        let mut parent_tree_dict = PdfDict::new();
        parent_tree_dict.insert(intern("Nums"), nums_array);
        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(parent_tree_dict)),
        );
        let root_obj = PdfObject::Dict(Box::new(root_dict));

        // Parse
        let parent_resolver = ParentTreeResolver::parse(&resolver, &root_obj);

        // Verify entry
        match parent_resolver.entries.get(&5) {
            Some(ParentTreeEntry::Single(r)) => {
                assert_eq!(*r, annot_ref);
            }
            _ => panic!("Expected Single entry for key 5"),
        }
    }

    #[test]
    fn test_parent_tree_null_entry() {
        // Test that null entries in arrays are handled
        let resolver = XrefResolver::new();

        let struct_elem_ref = ObjRef::new(10, 0);

        let nums_array = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Array(Box::new(vec![
                PdfObject::Ref(struct_elem_ref),
                PdfObject::Null, // Null entry (orphan MCID)
                PdfObject::Ref(struct_elem_ref),
            ])),
        ]));

        // Wrap in a StructTreeRoot-like structure with /ParentTree
        let mut parent_tree_dict = PdfDict::new();
        parent_tree_dict.insert(intern("Nums"), nums_array);
        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(parent_tree_dict)),
        );
        let root_obj = PdfObject::Dict(Box::new(root_dict));

        // Parse
        let mut parent_resolver = ParentTreeResolver::parse(&resolver, &root_obj);

        // Populate struct_elems map with mock nodes
        let mock_node = Rc::new(StructElemNode::new("P".to_string(), StructureType::P));
        parent_resolver
            .struct_elems
            .insert(struct_elem_ref, mock_node);

        // Resolve page and check orphans
        let (mcid_map, orphans) = parent_resolver.resolve_page(Some(0));

        // Should have 2 valid MCIDs
        assert_eq!(mcid_map.len(), 2);
        assert!(mcid_map.get(&0).is_some());
        assert!(mcid_map.get(&2).is_some());

        // MCID 1 should be orphan
        assert_eq!(orphans, vec![1]);
    }

    #[test]
    fn test_parent_tree_intermediate_kids() {
        // Test parsing a number tree with intermediate nodes (/Kids + /Limits)
        let resolver = XrefResolver::new();

        // Create leaf node 1
        let leaf1_ref = ObjRef::new(100, 0);
        let struct_elem1_ref = ObjRef::new(10, 0);
        let mut leaf1_with_limits = PdfDict::new();
        leaf1_with_limits.insert(
            intern("Nums"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(0),
                PdfObject::Array(Box::new(vec![PdfObject::Ref(struct_elem1_ref)])),
            ])),
        );
        leaf1_with_limits.insert(
            intern("Limits"),
            PdfObject::Array(Box::new(vec![PdfObject::Integer(0), PdfObject::Integer(0)])),
        );
        resolver.cache_object(leaf1_ref, PdfObject::Dict(Box::new(leaf1_with_limits)));

        // Create leaf node 2
        let leaf2_ref = ObjRef::new(101, 0);
        let struct_elem2_ref = ObjRef::new(11, 0);
        let mut leaf2_with_limits = PdfDict::new();
        leaf2_with_limits.insert(
            intern("Nums"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(10),
                PdfObject::Array(Box::new(vec![PdfObject::Ref(struct_elem2_ref)])),
            ])),
        );
        leaf2_with_limits.insert(
            intern("Limits"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(10),
                PdfObject::Integer(10),
            ])),
        );
        resolver.cache_object(leaf2_ref, PdfObject::Dict(Box::new(leaf2_with_limits)));

        // Create ParentTree root node with /Kids
        let mut parent_tree_dict = PdfDict::new();
        parent_tree_dict.insert(
            intern("Kids"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Ref(leaf1_ref),
                PdfObject::Ref(leaf2_ref),
            ])),
        );

        // Wrap in a StructTreeRoot-like structure with /ParentTree
        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(parent_tree_dict)),
        );
        let root_obj = PdfObject::Dict(Box::new(root_dict));

        // Parse
        let parent_resolver = ParentTreeResolver::parse(&resolver, &root_obj);

        // Verify both leaf nodes were processed
        assert_eq!(parent_resolver.entries.len(), 2);
        assert!(parent_resolver.entries.contains_key(&0));
        assert!(parent_resolver.entries.contains_key(&10));
    }

    #[test]
    fn test_parent_tree_missing_key() {
        // Test resolve_page when /StructParents key is not in tree
        let resolver = ParentTreeResolver::new();

        let (mcid_map, orphans) = resolver.resolve_page(Some(999));

        assert!(mcid_map.is_empty());
        assert!(orphans.is_empty()); // No orphans because no entry found
    }

    #[test]
    fn test_parent_tree_no_struct_parents() {
        // Test resolve_page when page has no /StructParents
        let resolver = ParentTreeResolver::new();

        let (mcid_map, orphans) = resolver.resolve_page(None);

        assert!(mcid_map.is_empty());
        assert!(orphans.is_empty());
    }

    #[test]
    fn test_parent_tree_annotation_resolution() {
        // Test resolving annotation /StructParent
        let mut resolver_impl = ParentTreeResolver::new();
        let struct_elem_ref = ObjRef::new(50, 0);

        // Insert a single ref entry (for annotations)
        resolver_impl
            .entries
            .insert(7, ParentTreeEntry::Single(struct_elem_ref));

        // Resolve annotation
        let result = resolver_impl.resolve_annotation(Some(7));
        assert_eq!(result, Some(struct_elem_ref));

        // Non-existent key
        let result = resolver_impl.resolve_annotation(Some(999));
        assert_eq!(result, None);

        // No key
        let result = resolver_impl.resolve_annotation(None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parent_tree_annotation_from_array() {
        // Test that annotations incorrectly mapped to arrays still work
        let mut resolver_impl = ParentTreeResolver::new();
        let struct_elem_ref = ObjRef::new(60, 0);

        // Insert an array entry (should be for pages, but test fallback)
        resolver_impl
            .entries
            .insert(8, ParentTreeEntry::Array(vec![struct_elem_ref]));

        // Resolve annotation - should use first array element
        let result = resolver_impl.resolve_annotation(Some(8));
        assert_eq!(result, Some(struct_elem_ref));

        // Empty array
        resolver_impl
            .entries
            .insert(9, ParentTreeEntry::Array(vec![]));
        let result = resolver_impl.resolve_annotation(Some(9));
        assert_eq!(result, None);
    }

    #[test]
    fn test_parent_tree_malformed_nums_non_integer_key() {
        // Test diagnostic when key is not an integer
        let resolver = XrefResolver::new();

        let nums_array = PdfObject::Array(Box::new(vec![
            PdfObject::Name(intern("invalid")), // Non-integer key
            PdfObject::Array(Box::new(vec![])),
        ]));

        // Wrap in a StructTreeRoot-like structure with /ParentTree
        let mut parent_tree_dict = PdfDict::new();
        parent_tree_dict.insert(intern("Nums"), nums_array);
        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(parent_tree_dict)),
        );
        let root_obj = PdfObject::Dict(Box::new(root_dict));

        // Parse
        let parent_resolver = ParentTreeResolver::parse(&resolver, &root_obj);

        // Should have diagnostic
        assert!(!parent_resolver.diagnostics.is_empty());
        assert!(parent_resolver
            .diagnostics
            .iter()
            .any(|d| d.message.contains("not an integer")));
    }

    #[test]
    fn test_parent_tree_malformed_nums_odd_length() {
        // Test diagnostic when /Nums has odd length
        let resolver = XrefResolver::new();

        let nums_array = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Array(Box::new(vec![])),
            PdfObject::Integer(1), // Trailing element without value
        ]));

        // Wrap in a StructTreeRoot-like structure with /ParentTree
        let mut parent_tree_dict = PdfDict::new();
        parent_tree_dict.insert(intern("Nums"), nums_array);
        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(parent_tree_dict)),
        );
        let root_obj = PdfObject::Dict(Box::new(root_dict));

        // Parse
        let parent_resolver = ParentTreeResolver::parse(&resolver, &root_obj);

        // Should have diagnostic
        assert!(!parent_resolver.diagnostics.is_empty());
        assert!(parent_resolver
            .diagnostics
            .iter()
            .any(|d| d.message.contains("odd length")));
    }

    #[test]
    fn test_parent_tree_malformed_unsupported_value_type() {
        // Test diagnostic when value has unsupported type
        let resolver = XrefResolver::new();

        let nums_array = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Bool(true), // Unsupported value type
        ]));

        // Wrap in a StructTreeRoot-like structure with /ParentTree
        let mut parent_tree_dict = PdfDict::new();
        parent_tree_dict.insert(intern("Nums"), nums_array);
        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(parent_tree_dict)),
        );
        let root_obj = PdfObject::Dict(Box::new(root_dict));

        // Parse
        let parent_resolver = ParentTreeResolver::parse(&resolver, &root_obj);

        // Should have diagnostic
        assert!(!parent_resolver.diagnostics.is_empty());
        assert!(parent_resolver
            .diagnostics
            .iter()
            .any(|d| d.message.contains("unsupported type")));
    }

    #[test]
    fn test_parent_tree_no_parent_tree_entry() {
        // Test parsing StructTreeRoot without /ParentTree
        let resolver = XrefResolver::new();

        let mut dict = PdfDict::new();
        dict.insert(intern("K"), PdfObject::Array(Box::new(vec![])));
        let root_obj = PdfObject::Dict(Box::new(dict));

        // Parse
        let parent_resolver = ParentTreeResolver::parse(&resolver, &root_obj);

        // Should have empty entries (no error - missing ParentTree is valid)
        assert!(parent_resolver.entries.is_empty());
        assert!(parent_resolver.diagnostics.is_empty());
    }

    #[test]
    fn test_parent_tree_invalid_node_type() {
        // Test diagnostic when node is not a dictionary
        let resolver = XrefResolver::new();

        let root_obj = PdfObject::Integer(42); // Not a dict

        // Parse
        let parent_resolver = ParentTreeResolver::parse(&resolver, &root_obj);

        // Should have diagnostic
        assert!(!parent_resolver.diagnostics.is_empty());
        assert!(parent_resolver
            .diagnostics
            .iter()
            .any(|d| d.message.contains("not a dictionary")));
    }

    #[test]
    fn test_parent_tree_empty_struct_tree_root() {
        // Test integration with parse_struct_tree
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Create StructTreeRoot with ParentTree
        let struct_elem_ref = ObjRef::new(10, 0);
        let parent_tree_nums = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(struct_elem_ref)])),
        ]));

        // ParentTree must be a dictionary with /Nums, not an array directly
        let mut parent_tree_dict = PdfDict::new();
        parent_tree_dict.insert(intern("Nums"), parent_tree_nums);

        let mut root_dict = PdfDict::new();
        root_dict.insert(intern("K"), PdfObject::Array(Box::new(vec![])));
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(parent_tree_dict)),
        );
        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));

        // Parse struct tree
        let result = parse_struct_tree(&resolver, root_ref);
        assert!(result.is_ok());

        let tree = result.unwrap();

        // Verify ParentTree was parsed - MCID 0 should be an orphan since
        // there's no StructElem with that ref in the tree
        let (mcid_map, orphans) = tree.parent_tree.resolve_page(Some(0));
        assert!(mcid_map.is_empty()); // No struct_elems with that ref
        assert_eq!(orphans, vec![0]); // MCID 0 is an orphan
    }

    #[test]
    fn test_parent_tree_annotation_with_struct_parent() {
        // Integration test: tagged PDF with annotation /StructParent linking to body StructElem
        // This test verifies that an annotation's /StructParent correctly resolves to
        // a StructElem in the structure tree, as required by PDF 1.7 spec 14.7.4.4
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Create body paragraph StructElem that the annotation will reference
        let mut body_dict = PdfDict::new();
        body_dict.insert(intern("S"), PdfObject::Name(intern("P")));
        body_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Integer(0)])),
        );
        let body_ref = ObjRef::new(10, 0);
        resolver.cache_object(body_ref, PdfObject::Dict(Box::new(body_dict)));

        // Create ParentTree with:
        // - Key 0: array for page with 2 MCIDs (one null entry for orphan)
        // - Key 100: single ref for annotation /StructParent
        let parent_tree_nums = PdfObject::Array(Box::new(vec![
            // Page 0's ParentTree entry (array of StructElem refs)
            PdfObject::Integer(0),
            PdfObject::Array(Box::new(vec![
                PdfObject::Ref(body_ref), // MCID 0 -> body paragraph
                PdfObject::Null,          // MCID 1 -> orphan (null entry)
            ])),
            // Annotation's ParentTree entry (single StructElem ref)
            PdfObject::Integer(100),
            PdfObject::Ref(body_ref), // Annotation /StructParent=100 -> body paragraph
        ]));

        let mut parent_tree_dict = PdfDict::new();
        parent_tree_dict.insert(intern("Nums"), parent_tree_nums);

        // Create StructTreeRoot
        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(body_ref)])),
        );
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(parent_tree_dict)),
        );
        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));

        // Parse struct tree
        let result = parse_struct_tree(&resolver, root_ref);
        assert!(result.is_ok());

        let tree = result.unwrap();

        // Verify page MCID resolution
        let (mcid_map, orphans) = tree.parent_tree.resolve_page(Some(0));

        // MCID 0 should map to the body paragraph
        assert_eq!(mcid_map.len(), 1);
        let mcid0_node = mcid_map.get(&0).unwrap();
        assert_eq!(mcid0_node.std_type, StructureType::P);

        // MCID 1 should be an orphan (null entry)
        assert_eq!(orphans, vec![1]);

        // Verify annotation /StructParent resolution
        let annot_struct_ref = tree.parent_tree.resolve_annotation(Some(100));
        assert_eq!(annot_struct_ref, Some(body_ref));

        // Verify the referenced StructElem is actually in the tree
        assert!(tree.struct_elems.contains_key(&body_ref));
        assert_eq!(
            tree.struct_elems.get(&body_ref).unwrap().std_type,
            StructureType::P
        );
    }

    #[test]
    fn test_parent_tree_off_by_one_missing_entries() {
        // Test that malformed ParentTree with off-by-one indexing or missing entries
        // doesn't crash and records orphans appropriately
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Create two StructElems with /K arrays containing MCIDs
        let mut elem1_dict = PdfDict::new();
        elem1_dict.insert(intern("S"), PdfObject::Name(intern("P")));
        elem1_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Integer(0)])),
        );
        let elem1_ref = ObjRef::new(10, 0);
        resolver.cache_object(elem1_ref, PdfObject::Dict(Box::new(elem1_dict)));

        let mut elem2_dict = PdfDict::new();
        elem2_dict.insert(intern("S"), PdfObject::Name(intern("H1")));
        elem2_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Integer(2)])),
        );
        let elem2_ref = ObjRef::new(11, 0);
        resolver.cache_object(elem2_ref, PdfObject::Dict(Box::new(elem2_dict)));

        // Create ParentTree with sparse array (missing entries)
        // Only 3 entries for what might be more MCIDs on the page
        let parent_tree_nums = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Array(Box::new(vec![
                PdfObject::Ref(elem1_ref),
                PdfObject::Null,
                PdfObject::Ref(elem2_ref),
            ])),
        ]));

        let mut parent_tree_dict = PdfDict::new();
        parent_tree_dict.insert(intern("Nums"), parent_tree_nums);

        // Add StructElems to /K array so they get parsed into struct_elems
        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Ref(elem1_ref),
                PdfObject::Ref(elem2_ref),
            ])),
        );
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(parent_tree_dict)),
        );
        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));

        // Parse struct tree
        let result = parse_struct_tree(&resolver, root_ref);
        assert!(result.is_ok());

        let tree = result.unwrap();

        // Resolve page - should only map the 2 non-null entries
        let (mcid_map, orphans) = tree.parent_tree.resolve_page(Some(0));
        assert_eq!(mcid_map.len(), 2);
        assert!(mcid_map.get(&0).is_some());
        assert!(mcid_map.get(&2).is_some());
        assert_eq!(orphans, vec![1]); // MCID 1 is null

        // If the page has MCIDs beyond the array length, they'd be orphans too
        // (This would be detected in Phase 7.1.4 coverage check)
    }

    // Phase 7.1.4 Coverage Check Tests

    #[test]
    fn test_compute_coverage_full_coverage() {
        // Test 100% coverage: all MCIDs claimed by StructTree
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Create a StructElem
        let mut elem_dict = PdfDict::new();
        elem_dict.insert(intern("S"), PdfObject::Name(intern("P")));
        elem_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(0),
                PdfObject::Integer(1),
                PdfObject::Integer(2),
            ])),
        );
        let elem_ref = ObjRef::new(10, 0);
        resolver.cache_object(elem_ref, PdfObject::Dict(Box::new(elem_dict)));

        // Create ParentTree with 3 MCIDs all claimed
        let parent_tree_nums = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Array(Box::new(vec![
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
            ])),
        ]));

        let mut parent_tree_dict = PdfDict::new();
        parent_tree_dict.insert(intern("Nums"), parent_tree_nums);

        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(elem_ref)])),
        );
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(parent_tree_dict)),
        );
        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));

        // Parse struct tree
        let result = parse_struct_tree(&resolver, root_ref);
        assert!(result.is_ok());
        let tree = result.unwrap();

        // All MCIDs present on page
        let mut all_mcids = std::collections::HashSet::new();
        all_mcids.insert(0);
        all_mcids.insert(1);
        all_mcids.insert(2);

        // Compute coverage
        let coverage = tree.parent_tree.compute_coverage(0, Some(0), &all_mcids);

        assert_eq!(coverage.page_index, 0);
        assert_eq!(coverage.total_mcids, 3);
        assert_eq!(coverage.claimed_mcids, 3);
        assert!((coverage.coverage - 1.0).abs() < f64::EPSILON);
        assert!(!coverage.should_fallback); // 100% >= 80%
    }

    #[test]
    fn test_compute_coverage_below_threshold() {
        // Test coverage below 80% threshold: should trigger fallback
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Create a StructElem
        let mut elem_dict = PdfDict::new();
        elem_dict.insert(intern("S"), PdfObject::Name(intern("P")));
        elem_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Integer(0)])),
        );
        let elem_ref = ObjRef::new(10, 0);
        resolver.cache_object(elem_ref, PdfObject::Dict(Box::new(elem_dict)));

        // Create ParentTree with 10 MCIDs but only 6 claimed (60% coverage)
        let parent_tree_nums = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Array(Box::new(vec![
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Null, // MCID 6 is orphan
                PdfObject::Null, // MCID 7 is orphan
                PdfObject::Null, // MCID 8 is orphan
                PdfObject::Null, // MCID 9 is orphan
            ])),
        ]));

        let mut parent_tree_dict = PdfDict::new();
        parent_tree_dict.insert(intern("Nums"), parent_tree_nums);

        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(elem_ref)])),
        );
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(parent_tree_dict)),
        );
        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));

        // Parse struct tree
        let result = parse_struct_tree(&resolver, root_ref);
        assert!(result.is_ok());
        let tree = result.unwrap();

        // All MCIDs present on page (0-9)
        let mut all_mcids = std::collections::HashSet::new();
        for i in 0..10 {
            all_mcids.insert(i);
        }

        // Compute coverage
        let coverage = tree.parent_tree.compute_coverage(0, Some(0), &all_mcids);

        assert_eq!(coverage.total_mcids, 10);
        assert_eq!(coverage.claimed_mcids, 6);
        assert!((coverage.coverage - 0.60).abs() < f64::EPSILON);
        assert!(coverage.should_fallback); // 60% < 80%
        assert!(coverage.fallback_diagnostic().unwrap().contains("60.0%"));
    }

    #[test]
    fn test_compute_coverage_above_threshold() {
        // Test coverage above 80% threshold: should NOT trigger fallback
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Create a StructElem
        let mut elem_dict = PdfDict::new();
        elem_dict.insert(intern("S"), PdfObject::Name(intern("P")));
        elem_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Integer(0)])),
        );
        let elem_ref = ObjRef::new(10, 0);
        resolver.cache_object(elem_ref, PdfObject::Dict(Box::new(elem_dict)));

        // Create ParentTree with 10 MCIDs, 9 claimed (90% coverage)
        let parent_tree_nums = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Array(Box::new(vec![
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Null, // Only MCID 9 is orphan
            ])),
        ]));

        let mut parent_tree_dict = PdfDict::new();
        parent_tree_dict.insert(intern("Nums"), parent_tree_nums);

        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(elem_ref)])),
        );
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(parent_tree_dict)),
        );
        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));

        // Parse struct tree
        let result = parse_struct_tree(&resolver, root_ref);
        assert!(result.is_ok());
        let tree = result.unwrap();

        // All MCIDs present on page (0-9)
        let mut all_mcids = std::collections::HashSet::new();
        for i in 0..10 {
            all_mcids.insert(i);
        }

        // Compute coverage
        let coverage = tree.parent_tree.compute_coverage(0, Some(0), &all_mcids);

        assert_eq!(coverage.total_mcids, 10);
        assert_eq!(coverage.claimed_mcids, 9);
        assert!((coverage.coverage - 0.90).abs() < f64::EPSILON);
        assert!(!coverage.should_fallback); // 90% >= 80%
    }

    #[test]
    fn test_compute_coverage_no_mcids() {
        // Test page with no marked content (no MCIDs)
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Empty StructTreeRoot
        let mut root_dict = PdfDict::new();
        root_dict.insert(intern("K"), PdfObject::Array(Box::new(vec![])));
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(PdfDict::new())),
        );
        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));

        // Parse struct tree
        let result = parse_struct_tree(&resolver, root_ref);
        assert!(result.is_ok());
        let tree = result.unwrap();

        // No MCIDs on page
        let all_mcids = std::collections::HashSet::new();

        // Compute coverage
        let coverage = tree.parent_tree.compute_coverage(0, None, &all_mcids);

        assert_eq!(coverage.total_mcids, 0);
        assert_eq!(coverage.claimed_mcids, 0);
        assert_eq!(coverage.coverage, 0.0);
        assert!(coverage.should_fallback); // No MCIDs = fallback
        assert!(coverage
            .fallback_diagnostic()
            .unwrap()
            .contains("no marked-content sequences"));
    }

    #[test]
    fn test_compute_coverage_threshold_edge_case() {
        // Test exactly 80% coverage (threshold boundary)
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Create a StructElem
        let mut elem_dict = PdfDict::new();
        elem_dict.insert(intern("S"), PdfObject::Name(intern("P")));
        elem_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Integer(0)])),
        );
        let elem_ref = ObjRef::new(10, 0);
        resolver.cache_object(elem_ref, PdfObject::Dict(Box::new(elem_dict)));

        // Create ParentTree with 10 MCIDs, 8 claimed (80% coverage)
        let parent_tree_nums = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Array(Box::new(vec![
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Null, // MCID 8 is orphan
                PdfObject::Null, // MCID 9 is orphan
            ])),
        ]));

        let mut parent_tree_dict = PdfDict::new();
        parent_tree_dict.insert(intern("Nums"), parent_tree_nums);

        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(elem_ref)])),
        );
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(parent_tree_dict)),
        );
        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));

        // Parse struct tree
        let result = parse_struct_tree(&resolver, root_ref);
        assert!(result.is_ok());
        let tree = result.unwrap();

        // All MCIDs present on page (0-9)
        let mut all_mcids = std::collections::HashSet::new();
        for i in 0..10 {
            all_mcids.insert(i);
        }

        // Compute coverage
        let coverage = tree.parent_tree.compute_coverage(0, Some(0), &all_mcids);

        assert_eq!(coverage.total_mcids, 10);
        assert_eq!(coverage.claimed_mcids, 8);
        assert!((coverage.coverage - 0.80).abs() < f64::EPSILON);
        assert!(!coverage.should_fallback); // 80% >= 80% (not less than)
    }

    #[test]
    fn test_compute_coverage_with_orphan_mcids() {
        // Test that MCIDs not in the ParentTree are correctly counted as orphans
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Create a StructElem
        let mut elem_dict = PdfDict::new();
        elem_dict.insert(intern("S"), PdfObject::Name(intern("P")));
        elem_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Integer(0)])),
        );
        let elem_ref = ObjRef::new(10, 0);
        resolver.cache_object(elem_ref, PdfObject::Dict(Box::new(elem_dict)));

        // ParentTree only has 3 entries, but page has 5 MCIDs
        // MCIDs 3 and 4 are orphans (not in ParentTree)
        let parent_tree_nums = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Array(Box::new(vec![
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Null, // MCID 2 is null (orphan)
                                 // MCIDs 3 and 4 don't exist in ParentTree at all
            ])),
        ]));

        let mut parent_tree_dict = PdfDict::new();
        parent_tree_dict.insert(intern("Nums"), parent_tree_nums);

        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(elem_ref)])),
        );
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(parent_tree_dict)),
        );
        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));

        // Parse struct tree
        let result = parse_struct_tree(&resolver, root_ref);
        assert!(result.is_ok());
        let tree = result.unwrap();

        // Page has 5 MCIDs (0-4)
        let mut all_mcids = std::collections::HashSet::new();
        for i in 0..5 {
            all_mcids.insert(i);
        }

        // Compute coverage
        let coverage = tree.parent_tree.compute_coverage(0, Some(0), &all_mcids);

        // Only MCIDs 0 and 1 are claimed (2/5 = 40%)
        assert_eq!(coverage.total_mcids, 5);
        assert_eq!(coverage.claimed_mcids, 2);
        assert!((coverage.coverage - 0.40).abs() < f64::EPSILON);
        assert!(coverage.should_fallback); // 40% < 80%
    }

    // Tests for check_coverage_for_pages with MarkInfo Suspects flag

    #[test]
    fn test_check_coverage_suspects_false_low_coverage() {
        // Suspects false + 50% coverage -> no fallback (trust tree)
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Create a StructElem
        let mut elem_dict = PdfDict::new();
        elem_dict.insert(intern("S"), PdfObject::Name(intern("P")));
        elem_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Integer(0)])),
        );
        let elem_ref = ObjRef::new(10, 0);
        resolver.cache_object(elem_ref, PdfObject::Dict(Box::new(elem_dict)));

        // ParentTree with 10 MCIDs, 5 claimed (50% coverage)
        let parent_tree_nums = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Array(Box::new(vec![
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Null,
                PdfObject::Null,
                PdfObject::Null,
                PdfObject::Null,
                PdfObject::Null,
            ])),
        ]));

        let mut parent_tree_dict = PdfDict::new();
        parent_tree_dict.insert(intern("Nums"), parent_tree_nums);

        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(elem_ref)])),
        );
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(parent_tree_dict)),
        );
        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));

        // Parse struct tree
        let result = parse_struct_tree(&resolver, root_ref);
        assert!(result.is_ok());
        let tree = result.unwrap();

        // MarkInfo with Suspects false
        let mark_info = MarkInfo {
            is_tagged: true,
            user_properties: false,
            suspects: false,
        };

        // Pages with MCID data: (page_index, struct_parents, mcid_set)
        let pages_with_mcids: Vec<(usize, Option<i32>, std::collections::HashSet<u32>)> = vec![(
            0,
            Some(0),
            (0..10u32).collect::<std::collections::HashSet<_>>(),
        )];

        // Check coverage
        let coverage_result = check_coverage_for_pages(&tree, &mark_info, &pages_with_mcids);

        // Suspects false means we trust the tree regardless of coverage
        assert_eq!(
            coverage_result.reading_order_algorithm,
            ReadingOrderAlgorithm::StructTree
        );
        assert!(coverage_result.diagnostics.is_empty()); // No diagnostics when Suspects false
        assert_eq!(coverage_result.page_results.len(), 1);
        assert!((coverage_result.page_results[0].coverage - 0.50).abs() < f64::EPSILON);
        assert!(!coverage_result.page_results[0].should_fallback); // No fallback when Suspects false
    }

    #[test]
    fn test_check_coverage_suspects_true_high_coverage() {
        // Suspects true + 95% coverage -> no fallback
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Create a StructElem
        let mut elem_dict = PdfDict::new();
        elem_dict.insert(intern("S"), PdfObject::Name(intern("P")));
        elem_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Integer(0)])),
        );
        let elem_ref = ObjRef::new(10, 0);
        resolver.cache_object(elem_ref, PdfObject::Dict(Box::new(elem_dict)));

        // ParentTree with 20 MCIDs, 19 claimed (95% coverage)
        let mut refs = vec![PdfObject::Ref(elem_ref); 19];
        refs.push(PdfObject::Null); // MCID 19 is orphan

        let parent_tree_nums = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Array(Box::new(refs)),
        ]));

        let mut parent_tree_dict = PdfDict::new();
        parent_tree_dict.insert(intern("Nums"), parent_tree_nums);

        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(elem_ref)])),
        );
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(parent_tree_dict)),
        );
        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));

        // Parse struct tree
        let result = parse_struct_tree(&resolver, root_ref);
        assert!(result.is_ok());
        let tree = result.unwrap();

        // MarkInfo with Suspects true
        let mark_info = MarkInfo {
            is_tagged: true,
            user_properties: false,
            suspects: true,
        };

        // Pages with MCID data: (page_index, struct_parents, mcid_set)
        let pages_with_mcids = vec![(
            0,
            Some(0),
            (0..20u32).collect::<std::collections::HashSet<_>>(),
        )];

        // Check coverage
        let coverage_result = check_coverage_for_pages(&tree, &mark_info, &pages_with_mcids);

        // 95% >= 80%, so use StructTree
        assert_eq!(
            coverage_result.reading_order_algorithm,
            ReadingOrderAlgorithm::StructTree
        );
        assert!(coverage_result.diagnostics.is_empty()); // No diagnostics when above threshold
        assert_eq!(coverage_result.page_results.len(), 1);
        assert!((coverage_result.page_results[0].coverage - 0.95).abs() < f64::EPSILON);
        assert!(!coverage_result.page_results[0].should_fallback); // No fallback at 95%
    }

    #[test]
    fn test_check_coverage_suspects_true_low_coverage() {
        // Suspects true + 60% coverage -> fallback to XY-cut
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Create a StructElem
        let mut elem_dict = PdfDict::new();
        elem_dict.insert(intern("S"), PdfObject::Name(intern("P")));
        elem_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Integer(0)])),
        );
        let elem_ref = ObjRef::new(10, 0);
        resolver.cache_object(elem_ref, PdfObject::Dict(Box::new(elem_dict)));

        // ParentTree with 10 MCIDs, 6 claimed (60% coverage)
        let parent_tree_nums = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Array(Box::new(vec![
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Ref(elem_ref),
                PdfObject::Null,
                PdfObject::Null,
                PdfObject::Null,
                PdfObject::Null,
            ])),
        ]));

        let mut parent_tree_dict = PdfDict::new();
        parent_tree_dict.insert(intern("Nums"), parent_tree_nums);

        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(elem_ref)])),
        );
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(parent_tree_dict)),
        );
        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));

        // Parse struct tree
        let result = parse_struct_tree(&resolver, root_ref);
        assert!(result.is_ok());
        let tree = result.unwrap();

        // MarkInfo with Suspects true
        let mark_info = MarkInfo {
            is_tagged: true,
            user_properties: false,
            suspects: true,
        };

        // Pages with MCID data: (page_index, struct_parents, mcid_set)
        let pages_with_mcids: Vec<(usize, Option<i32>, std::collections::HashSet<u32>)> = vec![(
            0,
            Some(0),
            (0..10u32).collect::<std::collections::HashSet<_>>(),
        )];

        // Check coverage
        let coverage_result = check_coverage_for_pages(&tree, &mark_info, &pages_with_mcids);

        // 60% < 80%, so fall back to XY-cut
        assert_eq!(
            coverage_result.reading_order_algorithm,
            ReadingOrderAlgorithm::XyCut
        );
        assert!(!coverage_result.diagnostics.is_empty()); // Diagnostic emitted for fallback
        assert_eq!(coverage_result.diagnostics.len(), 1);
        assert_eq!(
            coverage_result.diagnostics[0].code,
            DiagCode::StructIncompleteCoverage
        );
        assert!(coverage_result.diagnostics[0].message.contains("Page 0"));
        assert!(coverage_result.diagnostics[0].message.contains("60.0%"));
        assert!(coverage_result.diagnostics[0].message.contains("6/10"));
        assert!(coverage_result.diagnostics[0]
            .message
            .contains("falling back to XY-cut"));

        assert_eq!(coverage_result.page_results.len(), 1);
        assert!((coverage_result.page_results[0].coverage - 0.60).abs() < f64::EPSILON);
        assert!(coverage_result.page_results[0].should_fallback); // Fallback at 60%
        assert!(coverage_result.page_results[0]
            .fallback_diagnostic()
            .is_some());
    }

    #[test]
    fn test_check_coverage_multi_page_one_fallback() {
        // Test that if any page falls back, the whole document uses XY-cut
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Create a StructElem
        let mut elem_dict = PdfDict::new();
        elem_dict.insert(intern("S"), PdfObject::Name(intern("P")));
        elem_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Integer(0)])),
        );
        let elem_ref = ObjRef::new(10, 0);
        resolver.cache_object(elem_ref, PdfObject::Dict(Box::new(elem_dict)));

        // ParentTree for struct_parents=0 (high coverage: 90%)
        let high_refs = vec![PdfObject::Ref(elem_ref); 9];
        let mut high_refs_with_null = high_refs;
        high_refs_with_null.push(PdfObject::Null);

        // ParentTree for struct_parents=1 (low coverage: 60%)
        let low_refs = vec![PdfObject::Ref(elem_ref); 6];
        let mut low_refs_with_null = low_refs;
        for _ in 0..4 {
            low_refs_with_null.push(PdfObject::Null);
        }

        let parent_tree_nums = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Array(Box::new(high_refs_with_null)),
            PdfObject::Integer(1),
            PdfObject::Array(Box::new(low_refs_with_null)),
        ]));

        let mut parent_tree_dict = PdfDict::new();
        parent_tree_dict.insert(intern("Nums"), parent_tree_nums);

        let mut root_dict = PdfDict::new();
        root_dict.insert(
            intern("K"),
            PdfObject::Array(Box::new(vec![PdfObject::Ref(elem_ref)])),
        );
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(parent_tree_dict)),
        );
        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));

        // Parse struct tree
        let result = parse_struct_tree(&resolver, root_ref);
        assert!(result.is_ok());
        let tree = result.unwrap();

        // MarkInfo with Suspects true
        let mark_info = MarkInfo {
            is_tagged: true,
            user_properties: false,
            suspects: true,
        };

        // Two pages: page 0 has 90% coverage, page 1 has 60% coverage
        let pages_with_mcids = vec![
            (
                0,
                Some(0),
                (0..10u32).collect::<std::collections::HashSet<_>>(),
            ), // 90% coverage
            (
                1,
                Some(1),
                (0..10u32).collect::<std::collections::HashSet<_>>(),
            ), // 60% coverage (triggers fallback)
        ];

        // Check coverage
        let coverage_result = check_coverage_for_pages(&tree, &mark_info, &pages_with_mcids);

        // One page triggers fallback, so whole document uses XY-cut
        assert_eq!(
            coverage_result.reading_order_algorithm,
            ReadingOrderAlgorithm::XyCut
        );
        assert_eq!(coverage_result.diagnostics.len(), 1); // One diagnostic for page 1
        assert!(coverage_result.diagnostics[0].message.contains("Page 1"));

        assert_eq!(coverage_result.page_results.len(), 2);
        assert!((coverage_result.page_results[0].coverage - 0.90).abs() < f64::EPSILON);
        assert!(!coverage_result.page_results[0].should_fallback); // Page 0 OK

        assert!((coverage_result.page_results[1].coverage - 0.60).abs() < f64::EPSILON);
        assert!(coverage_result.page_results[1].should_fallback); // Page 1 triggers fallback
    }

    #[test]
    fn test_check_coverage_no_marked_content() {
        // Test page with no marked content (mcid_count = 0)
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Empty StructTreeRoot
        let mut root_dict = PdfDict::new();
        root_dict.insert(intern("K"), PdfObject::Array(Box::new(vec![])));
        root_dict.insert(
            intern("ParentTree"),
            PdfObject::Dict(Box::new(PdfDict::new())),
        );
        resolver.cache_object(root_ref, PdfObject::Dict(Box::new(root_dict)));

        // Parse struct tree
        let result = parse_struct_tree(&resolver, root_ref);
        assert!(result.is_ok());
        let tree = result.unwrap();

        // MarkInfo with Suspects true
        let mark_info = MarkInfo {
            is_tagged: true,
            user_properties: false,
            suspects: true,
        };

        // Page with no marked content
        let pages_with_mcids = vec![(0, None, std::collections::HashSet::new())];

        // Check coverage
        let coverage_result = check_coverage_for_pages(&tree, &mark_info, &pages_with_mcids);

        // No marked content = fallback to XY-cut
        assert_eq!(
            coverage_result.reading_order_algorithm,
            ReadingOrderAlgorithm::XyCut
        );
        assert_eq!(coverage_result.diagnostics.len(), 1);
        assert!(coverage_result.diagnostics[0]
            .message
            .contains("no marked-content sequences"));

        assert_eq!(coverage_result.page_results.len(), 1);
        assert_eq!(coverage_result.page_results[0].coverage, 0.0);
        assert!(coverage_result.page_results[0].should_fallback);
    }
}
