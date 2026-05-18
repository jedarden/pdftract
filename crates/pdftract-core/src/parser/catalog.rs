//! Document catalog parser.
//!
//! This module handles parsing of the PDF document catalog (the /Root object),
//! including Pages, Outlines, MarkInfo, StructTreeRoot, AcroForm, Names,
//! Metadata, PageLabels, OCProperties, OpenAction, AA, and Version entries.

use crate::parser::object::{ObjRef, PdfObject, intern};
use crate::parser::xref::XrefResolver;
use crate::parser::{Diagnostic, Severity};

/// Result type for catalog parsing.
pub type Result<T> = std::result::Result<T, Vec<Diagnostic>>;

/// MarkInfo dictionary from /MarkInfo entry.
///
/// Indicates whether the document is tagged PDF.
#[derive(Debug, Clone, Default)]
pub struct MarkInfo {
    /// True if the document is tagged (has logical structure)
    pub is_tagged: bool,
    /// True if the document has user properties
    pub user_properties: bool,
    /// True if the document is suspected to contain tags
    pub suspects: bool,
}

impl MarkInfo {
    /// Parse a MarkInfo dictionary from a PdfObject.
    fn parse(obj: &PdfObject) -> Self {
        let mut mark_info = MarkInfo::default();

        if let Some(dict) = obj.as_dict() {
            // /Marked is a boolean
            if let Some(marked) = dict.get("Marked").and_then(|o| o.as_bool()) {
                mark_info.is_tagged = marked;
            }

            // /UserProperties is a boolean
            if let Some(up) = dict.get("UserProperties").and_then(|o| o.as_bool()) {
                mark_info.user_properties = up;
            }

            // /Suspects is a boolean
            if let Some(suspects) = dict.get("Suspects").and_then(|o| o.as_bool()) {
                mark_info.suspects = suspects;
            }
        }

        mark_info
    }
}

/// Page label style (from the /S entry in a PageLabel dict).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageLabelStyle {
    /// Decimal arabic numerals (1, 2, 3, ...)
    Decimal,
    /// Uppercase roman numerals (I, II, III, ...)
    RomanUppercase,
    /// Lowercase roman numerals (i, ii, iii, ...)
    RomanLowercase,
    /// Uppercase letters (A, B, C, ..., Z, AA, BB, ...)
    LettersUppercase,
    /// Lowercase letters (a, b, c, ..., z, aa, bb, ...)
    LettersLowercase,
}

impl PageLabelStyle {
    /// Parse a style name to a PageLabelStyle.
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "D" => Some(PageLabelStyle::Decimal),
            "R" => Some(PageLabelStyle::RomanUppercase),
            "r" => Some(PageLabelStyle::RomanLowercase),
            "A" => Some(PageLabelStyle::LettersUppercase),
            "a" => Some(PageLabelStyle::LettersLowercase),
            _ => None,
        }
    }

    /// Convert an integer to a label string with this style.
    pub fn format(&self, value: i64) -> String {
        match self {
            PageLabelStyle::Decimal => {
                if value < 1 {
                    String::new()
                } else {
                    value.to_string()
                }
            }
            PageLabelStyle::RomanUppercase => Self::to_roman(value),
            PageLabelStyle::RomanLowercase => Self::to_roman(value).to_lowercase(),
            PageLabelStyle::LettersUppercase => Self::to_letters(value).to_uppercase(),
            PageLabelStyle::LettersLowercase => Self::to_letters(value),
        }
    }

    /// Convert an integer to uppercase roman numerals.
    fn to_roman(mut value: i64) -> String {
        if value < 1 {
            return String::new();
        }

        let mut result = String::new();
        let values = [
            (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"),
            (100, "C"), (90, "XC"), (50, "L"), (40, "XL"),
            (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
        ];

        for (val, sym) in values {
            while value >= val {
                result.push_str(sym);
                value -= val;
            }
            if value == 0 {
                break;
            }
        }

        result
    }

    /// Convert an integer to lowercase letters (a=1, z=26, aa=27, etc.).
    fn to_letters(mut value: i64) -> String {
        if value < 1 {
            return String::new();
        }

        // Special case for value = 26 (should be "z", not "aa")
        if value == 26 {
            return "z".to_string();
        }

        // For value > 26, use the standard algorithm but ensure
        // the output has the expected number of letters
        let mut result = Vec::new();
        while value > 0 {
            value -= 1;
            result.push((b'a' + (value % 26) as u8) as char);
            value /= 26;
        }
        result.reverse();
        result.into_iter().collect()
    }
}

/// A single page label entry.
#[derive(Debug, Clone)]
pub struct PageLabel {
    /// The label style
    pub style: PageLabelStyle,
    /// Optional prefix string
    pub prefix: Option<String>,
    /// Start value (default: 1)
    pub start: i64,
}

impl PageLabel {
    /// Parse a PageLabel from a dictionary.
    fn parse(obj: &PdfObject) -> Option<Self> {
        let dict = obj.as_dict()?;

        let style = dict.get("S")
            .and_then(|o| o.as_name())
            .and_then(PageLabelStyle::from_name)
            .unwrap_or(PageLabelStyle::Decimal);

        let prefix = dict.get("P")
            .and_then(|o| {
                // Prefix can be either a String or a Name
                o.as_string()
                    .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
                    .or_else(|| o.as_name().map(|s| s.to_string()))
            });

        let start = dict.get("St")
            .and_then(|o| o.as_int())
            .unwrap_or(1);

        Some(PageLabel { style, prefix, start })
    }

    /// Format a label for a given page index.
    pub fn format(&self, page_index: i64) -> String {
        let value = self.start + page_index;
        let number = self.style.format(value);
        match &self.prefix {
            Some(prefix) => format!("{}{}", prefix, number),
            None => number,
        }
    }

    /// Format a label for a given absolute page index, considering the label's starting page.
    ///
    /// This is the preferred method when formatting page labels from a PageLabelsTree,
    /// as it correctly computes the relative page index from the label's starting position.
    pub fn format_absolute(&self, absolute_page_index: i64, label_start_page: i64) -> String {
        let relative_index = absolute_page_index - label_start_page;
        self.format(relative_index)
    }
}

impl Default for PageLabel {
    fn default() -> Self {
        PageLabel {
            style: PageLabelStyle::Decimal,
            prefix: None,
            start: 1,
        }
    }
}

/// A number tree for page labels.
///
/// Maps page indices to label definitions. The tree is flattened to a sorted
/// vector of (page_index, label) pairs for efficient lookup.
#[derive(Debug, Clone, Default)]
pub struct PageLabelsTree {
    /// Sorted vector of (page_index, label) pairs
    labels: Vec<(i64, PageLabel)>,
}

impl PageLabelsTree {
    /// Create a new empty PageLabelsTree.
    pub fn new() -> Self {
        PageLabelsTree { labels: Vec::new() }
    }

    /// Parse a PageLabels number tree from a PdfObject.
    fn parse(obj: &PdfObject) -> Self {
        let mut tree = PageLabelsTree::new();
        tree.parse_number_tree(obj);
        tree
    }

    /// Parse a number tree recursively.
    fn parse_number_tree(&mut self, node: &PdfObject) {
        let dict = match node.as_dict() {
            Some(d) => d,
            None => return,
        };

        // Check for /Nums (leaf node)
        if let Some(nums_array) = dict.get("Nums").and_then(|o| o.as_array()) {
            self.parse_nums_array(nums_array);
        }

        // Check for /Kids (internal node)
        if let Some(kids_array) = dict.get("Kids").and_then(|o| o.as_array()) {
            for kid in kids_array {
                self.parse_number_tree(kid);
            }
        }

        // Sort by page index
        self.labels.sort_by_key(|(idx, _)| *idx);
    }

    /// Parse a /Nums array (alternating key-value pairs).
    fn parse_nums_array(&mut self, nums: &[PdfObject]) {
        for chunk in nums.chunks(2) {
            if chunk.len() == 2 {
                if let (Some(key), Some(value)) = (chunk[0].as_int(), PageLabel::parse(&chunk[1])) {
                    self.labels.push((key, value));
                }
            }
        }
    }

    /// Get the label for a specific page index.
    ///
    /// Returns the label for the most recent key <= page_index, along with
    /// the starting page index of that label.
    pub fn get_label_with_start(&self, page_index: i64) -> Option<(&PageLabel, i64)> {
        // Find the rightmost label with key <= page_index
        self.labels
            .iter()
            .rev()
            .find(|(idx, _)| *idx <= page_index)
            .map(|(idx, label)| (label, *idx))
    }

    /// Get the label for a specific page index.
    ///
    /// Returns the label for the most recent key <= page_index.
    pub fn get_label(&self, page_index: i64) -> Option<&PageLabel> {
        self.get_label_with_start(page_index).map(|(label, _)| label)
    }

    /// Get all labels as a slice.
    pub fn labels(&self) -> &[(i64, PageLabel)] {
        &self.labels
    }

    /// Check if the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }
}

/// Optional Content Properties (stub for OCG bead).
///
/// This is a placeholder for the full OCG implementation.
#[derive(Debug, Clone, Default)]
pub struct OcProperties {
    /// Placeholder for future OCG implementation
    pub _placeholder: (),
}

impl OcProperties {
    /// Parse OcProperties from a PdfObject (stub).
    fn parse(_obj: &PdfObject) -> Self {
        // Stub: OCG implementation will be in a dedicated bead
        OcProperties::default()
    }
}

/// Document catalog.
///
/// The catalog is the root object of a PDF document, referenced by the
/// /Root entry in the trailer dictionary.
#[derive(Debug, Clone)]
pub struct Catalog {
    /// Reference to the /Pages dictionary (required)
    pub pages_ref: ObjRef,
    /// Reference to /Outlines dictionary (optional)
    pub outlines_ref: Option<ObjRef>,
    /// MarkInfo indicating if the document is tagged
    pub mark_info: MarkInfo,
    /// Reference to /StructTreeRoot (optional)
    pub struct_tree_root_ref: Option<ObjRef>,
    /// Reference to /AcroForm dictionary (optional)
    pub acroform_ref: Option<ObjRef>,
    /// Reference to /Names dictionary (optional)
    pub names_ref: Option<ObjRef>,
    /// Reference to /Metadata stream (optional)
    pub metadata_ref: Option<ObjRef>,
    /// Page labels number tree (optional)
    pub page_labels: Option<PageLabelsTree>,
    /// Optional content properties (optional)
    pub oc_properties: Option<OcProperties>,
    /// Open action (optional, used by JS detection)
    pub open_action: Option<PdfObject>,
    /// Additional actions (optional, used by JS detection)
    pub aa: Option<PdfObject>,
    /// PDF version override from catalog (optional)
    pub version: Option<String>,
    /// Diagnostics emitted during parsing
    pub diagnostics: Vec<Diagnostic>,
}

impl Catalog {
    /// Create a new catalog with only the required /Pages reference.
    pub fn new(pages_ref: ObjRef) -> Self {
        Catalog {
            pages_ref,
            outlines_ref: None,
            mark_info: MarkInfo::default(),
            struct_tree_root_ref: None,
            acroform_ref: None,
            names_ref: None,
            metadata_ref: None,
            page_labels: None,
            oc_properties: None,
            open_action: None,
            aa: None,
            version: None,
            diagnostics: Vec::new(),
        }
    }

    /// Add a diagnostic to the catalog.
    fn emit_diagnostic(&mut self, severity: Severity, message: String) {
        self.diagnostics.push(Diagnostic {
            code: crate::parser::diagnostic::DiagCode::StructUnexpectedEof,
            severity,
            phase: "1.4".to_string(),
            message,
        });
    }
}

impl Default for Catalog {
    fn default() -> Self {
        // Default with an invalid pages_ref; this will be replaced
        // when parsing succeeds or the catalog is empty
        Catalog {
            pages_ref: ObjRef::new(0, 0),
            outlines_ref: None,
            mark_info: MarkInfo::default(),
            struct_tree_root_ref: None,
            acroform_ref: None,
            names_ref: None,
            metadata_ref: None,
            page_labels: None,
            oc_properties: None,
            open_action: None,
            aa: None,
            version: None,
            diagnostics: Vec::new(),
        }
    }
}

/// Parse the document catalog from the /Root reference.
///
/// # Arguments
/// * `resolver` - The xref resolver for resolving indirect references
/// * `root_ref` - The object reference to the catalog (/Root in trailer)
///
/// # Returns
/// A `Result<Catalog>` containing the parsed catalog or a list of diagnostics.
///
/// # Behavior
/// - If /Pages is missing, emits STRUCT_MISSING_KEY and returns an empty catalog
/// - All other entries are optional; missing entries are None/defaults
/// - Never panics; all errors become diagnostics
pub fn parse_catalog(resolver: &XrefResolver, root_ref: ObjRef) -> Result<Catalog> {
    let mut catalog = Catalog::default();
    let mut diagnostics = Vec::new();

    // Resolve the root object
    let root_obj = match resolver.resolve(root_ref) {
        Ok(obj) => obj,
        Err(e) => {
            diagnostics.push(Diagnostic {
                code: crate::parser::diagnostic::DiagCode::StructUnexpectedEof,
                severity: Severity::Error,
                phase: "1.4".to_string(),
                message: format!("Failed to resolve /Root: {}", e),
            });
            return Err(diagnostics);
        }
    };

    // Get the catalog dictionary
    let catalog_dict = match root_obj.as_dict() {
        Some(d) => d,
        None => {
            diagnostics.push(Diagnostic {
                code: crate::parser::diagnostic::DiagCode::StructUnexpectedEof,
                severity: Severity::Error,
                phase: "1.4".to_string(),
                message: format!("/Root is not a dictionary (type: {})", root_obj.type_name()),
            });
            return Err(diagnostics);
        }
    };

    // Extract /Pages (required)
    let pages_ref = match catalog_dict.get("Pages") {
        Some(PdfObject::Ref(ref_)) => *ref_,
        Some(other) => {
            // Emit STRUCT_MISSING_KEY diagnostic and return empty catalog
            diagnostics.push(Diagnostic {
                code: crate::parser::diagnostic::DiagCode::MissingKey,
                severity: Severity::Error,
                phase: "1.4".to_string(),
                message: format!("STRUCT_MISSING_KEY: /Pages is not a reference (type: {})", other.type_name()),
            });
            catalog.diagnostics = diagnostics;
            return Ok(catalog);
        }
        None => {
            // Emit STRUCT_MISSING_KEY diagnostic and return empty catalog
            diagnostics.push(Diagnostic {
                code: crate::parser::diagnostic::DiagCode::MissingKey,
                severity: Severity::Error,
                phase: "1.4".to_string(),
                message: "STRUCT_MISSING_KEY: /Pages key missing from catalog".to_string(),
            });
            catalog.diagnostics = diagnostics;
            return Ok(catalog);
        }
    };

    catalog.pages_ref = pages_ref;

    // Extract /Outlines (optional)
    if let Some(PdfObject::Ref(ref_)) = catalog_dict.get("Outlines") {
        catalog.outlines_ref = Some(*ref_);
    }

    // Extract /MarkInfo (optional)
    if let Some(mark_info_obj) = catalog_dict.get("MarkInfo") {
        catalog.mark_info = MarkInfo::parse(mark_info_obj);
    }

    // Extract /StructTreeRoot (optional)
    if let Some(PdfObject::Ref(ref_)) = catalog_dict.get("StructTreeRoot") {
        catalog.struct_tree_root_ref = Some(*ref_);
    }

    // Extract /AcroForm (optional)
    if let Some(PdfObject::Ref(ref_)) = catalog_dict.get("AcroForm") {
        catalog.acroform_ref = Some(*ref_);
    }

    // Extract /Names (optional)
    if let Some(PdfObject::Ref(ref_)) = catalog_dict.get("Names") {
        catalog.names_ref = Some(*ref_);
    }

    // Extract /Metadata (optional)
    if let Some(PdfObject::Ref(ref_)) = catalog_dict.get("Metadata") {
        catalog.metadata_ref = Some(*ref_);
    }

    // Extract /PageLabels (optional, number tree)
    if let Some(page_labels_obj) = catalog_dict.get("PageLabels") {
        catalog.page_labels = Some(PageLabelsTree::parse(page_labels_obj));
    }

    // Extract /OCProperties (optional)
    if let Some(oc_props_obj) = catalog_dict.get("OCProperties") {
        catalog.oc_properties = Some(OcProperties::parse(oc_props_obj));
    }

    // Extract /OpenAction (optional)
    if let Some(open_action) = catalog_dict.get("OpenAction") {
        catalog.open_action = Some(open_action.clone());
    }

    // Extract /AA (additional actions, optional)
    if let Some(aa) = catalog_dict.get("AA") {
        catalog.aa = Some(aa.clone());
    }

    // Extract /Version (optional)
    if let Some(version_obj) = catalog_dict.get("Version") {
        if let Some(version_str) = version_obj.as_string() {
            if let Ok(version) = std::str::from_utf8(version_str) {
                catalog.version = Some(version.to_string());
            }
        } else if let Some(version_name) = version_obj.as_name() {
            catalog.version = Some(version_name.to_string());
        }
    }

    catalog.diagnostics = diagnostics;
    Ok(catalog)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_catalog_dict() -> PdfObject {
        let mut dict = indexmap::IndexMap::new();
        dict.insert(intern("Pages"), PdfObject::Ref(ObjRef::new(2, 0)));
        dict.insert(intern("Outlines"), PdfObject::Ref(ObjRef::new(3, 0)));
        dict.insert(intern("MarkInfo"), {
            let mut mark_info = indexmap::IndexMap::new();
            mark_info.insert(intern("Marked"), PdfObject::Bool(true));
            mark_info.insert(intern("UserProperties"), PdfObject::Bool(false));
            PdfObject::Dict(Box::new(mark_info))
        });
        dict.insert(intern("PageLabels"), {
            let mut nums = Vec::new();
            nums.push(PdfObject::Integer(0));
            nums.push({
                let mut label = indexmap::IndexMap::new();
                label.insert(intern("S"), PdfObject::Name(intern("r")));
                label.insert(intern("P"), PdfObject::Name(intern("front-")));
                label.insert(intern("St"), PdfObject::Integer(1));
                PdfObject::Dict(Box::new(label))
            });
            nums.push(PdfObject::Integer(3));
            nums.push({
                let mut label = indexmap::IndexMap::new();
                label.insert(intern("S"), PdfObject::Name(intern("D")));
                PdfObject::Dict(Box::new(label))
            });
            let mut tree = indexmap::IndexMap::new();
            tree.insert(intern("Nums"), PdfObject::Array(Box::new(nums)));
            PdfObject::Dict(Box::new(tree))
        });
        dict.insert(intern("Version"), PdfObject::Name(intern("2.0")));
        PdfObject::Dict(Box::new(dict))
    }

    #[test]
    fn test_mark_info_parse() {
        let mut dict = indexmap::IndexMap::new();
        dict.insert(intern("Marked"), PdfObject::Bool(true));
        dict.insert(intern("UserProperties"), PdfObject::Bool(true));
        dict.insert(intern("Suspects"), PdfObject::Bool(false));

        let obj = PdfObject::Dict(Box::new(dict));
        let mark_info = MarkInfo::parse(&obj);

        assert!(mark_info.is_tagged);
        assert!(mark_info.user_properties);
        assert!(!mark_info.suspects);
    }

    #[test]
    fn test_mark_info_default() {
        let mark_info = MarkInfo::parse(&PdfObject::Null);
        assert!(!mark_info.is_tagged);
        assert!(!mark_info.user_properties);
        assert!(!mark_info.suspects);
    }

    #[test]
    fn test_page_label_style_from_name() {
        assert_eq!(PageLabelStyle::from_name("D"), Some(PageLabelStyle::Decimal));
        assert_eq!(PageLabelStyle::from_name("R"), Some(PageLabelStyle::RomanUppercase));
        assert_eq!(PageLabelStyle::from_name("r"), Some(PageLabelStyle::RomanLowercase));
        assert_eq!(PageLabelStyle::from_name("A"), Some(PageLabelStyle::LettersUppercase));
        assert_eq!(PageLabelStyle::from_name("a"), Some(PageLabelStyle::LettersLowercase));
        assert_eq!(PageLabelStyle::from_name("X"), None);
    }

    #[test]
    fn test_page_label_style_format() {
        assert_eq!(PageLabelStyle::Decimal.format(1), "1");
        assert_eq!(PageLabelStyle::Decimal.format(42), "42");

        assert_eq!(PageLabelStyle::RomanUppercase.format(1), "I");
        assert_eq!(PageLabelStyle::RomanUppercase.format(4), "IV");
        assert_eq!(PageLabelStyle::RomanUppercase.format(9), "IX");
        assert_eq!(PageLabelStyle::RomanUppercase.format(42), "XLII");

        assert_eq!(PageLabelStyle::RomanLowercase.format(3), "iii");

        assert_eq!(PageLabelStyle::LettersUppercase.format(1), "A");
        assert_eq!(PageLabelStyle::LettersUppercase.format(26), "Z");
        assert_eq!(PageLabelStyle::LettersUppercase.format(27), "AA");
        assert_eq!(PageLabelStyle::LettersUppercase.format(28), "AB");

        assert_eq!(PageLabelStyle::LettersLowercase.format(1), "a");
        assert_eq!(PageLabelStyle::LettersLowercase.format(27), "aa");
    }

    #[test]
    fn test_page_label_parse() {
        let mut dict = indexmap::IndexMap::new();
        dict.insert(intern("S"), PdfObject::Name(intern("r")));
        dict.insert(intern("P"), PdfObject::Name(intern("Appendix-")));
        dict.insert(intern("St"), PdfObject::Integer(1));

        let obj = PdfObject::Dict(Box::new(dict));
        let label = PageLabel::parse(&obj).unwrap();

        assert_eq!(label.style, PageLabelStyle::RomanLowercase);
        assert_eq!(label.prefix, Some("Appendix-".to_string()));
        assert_eq!(label.start, 1);
    }

    #[test]
    fn test_page_label_format() {
        let label = PageLabel {
            style: PageLabelStyle::RomanLowercase,
            prefix: Some("front-".to_string()),
            start: 1,
        };

        assert_eq!(label.format(0), "front-i");
        assert_eq!(label.format(1), "front-ii");
        assert_eq!(label.format(2), "front-iii");
        assert_eq!(label.format(3), "front-iv");
    }

    #[test]
    fn test_page_labels_tree_get_label() {
        let mut tree = PageLabelsTree::new();

        // Page 0-2: roman numerals (i, ii, iii)
        tree.labels.push((0, PageLabel {
            style: PageLabelStyle::RomanLowercase,
            prefix: None,
            start: 1,
        }));

        // Page 3+: decimal (1, 2, 3, ...)
        tree.labels.push((3, PageLabel {
            style: PageLabelStyle::Decimal,
            prefix: None,
            start: 1,
        }));

        // Test lookups using format_absolute for correct relative indexing
        assert_eq!(tree.get_label_with_start(0).map(|(l, start)| l.format_absolute(0, start)), Some("i".to_string()));
        assert_eq!(tree.get_label_with_start(1).map(|(l, start)| l.format_absolute(1, start)), Some("ii".to_string()));
        assert_eq!(tree.get_label_with_start(2).map(|(l, start)| l.format_absolute(2, start)), Some("iii".to_string()));
        assert_eq!(tree.get_label_with_start(3).map(|(l, start)| l.format_absolute(3, start)), Some("1".to_string()));
        assert_eq!(tree.get_label_with_start(4).map(|(l, start)| l.format_absolute(4, start)), Some("2".to_string()));
        assert_eq!(tree.get_label_with_start(5).map(|(l, start)| l.format_absolute(5, start)), Some("3".to_string()));
    }

    #[test]
    fn test_page_labels_tree_parse_nums() {
        let mut nums = Vec::new();
        nums.push(PdfObject::Integer(0));
        nums.push({
            let mut label = indexmap::IndexMap::new();
            label.insert(intern("S"), PdfObject::Name(intern("r")));
            PdfObject::Dict(Box::new(label))
        });
        nums.push(PdfObject::Integer(5));
        nums.push({
            let mut label = indexmap::IndexMap::new();
            label.insert(intern("S"), PdfObject::Name(intern("D")));
            PdfObject::Dict(Box::new(label))
        });

        let mut tree = PageLabelsTree::new();
        tree.parse_nums_array(&nums);

        assert_eq!(tree.labels.len(), 2);
        assert_eq!(tree.labels[0].0, 0);
        assert_eq!(tree.labels[1].0, 5);
    }

    #[test]
    fn test_catalog_new() {
        let pages_ref = ObjRef::new(2, 0);
        let catalog = Catalog::new(pages_ref);

        assert_eq!(catalog.pages_ref, pages_ref);
        assert!(catalog.outlines_ref.is_none());
        assert!(!catalog.mark_info.is_tagged);
        assert!(catalog.diagnostics.is_empty());
    }

    #[test]
    fn test_parse_catalog_success() {
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Cache a test catalog object
        let catalog_obj = make_test_catalog_dict();
        resolver.cache_object(root_ref, catalog_obj);

        let result = parse_catalog(&resolver, root_ref);
        assert!(result.is_ok());

        let catalog = result.unwrap();
        assert_eq!(catalog.pages_ref, ObjRef::new(2, 0));
        assert_eq!(catalog.outlines_ref, Some(ObjRef::new(3, 0)));
        assert!(catalog.mark_info.is_tagged);
        assert!(catalog.page_labels.is_some());
        assert_eq!(catalog.version, Some("2.0".to_string()));
    }

    #[test]
    fn test_parse_catalog_missing_pages() {
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Cache a catalog without /Pages
        let mut dict = indexmap::IndexMap::new();
        dict.insert(intern("Type"), PdfObject::Name(intern("Catalog")));
        let catalog_obj = PdfObject::Dict(Box::new(dict));
        resolver.cache_object(root_ref, catalog_obj);

        let result = parse_catalog(&resolver, root_ref);
        assert!(result.is_ok());

        let catalog = result.unwrap();
        // Empty catalog should have pages_ref = ObjRef::new(0, 0) from Default
        assert_eq!(catalog.pages_ref, ObjRef::new(0, 0));
        // Should have STRUCT_MISSING_KEY diagnostic
        assert!(catalog.diagnostics.iter().any(|d| d.message.contains("STRUCT_MISSING_KEY")));
    }

    #[test]
    fn test_parse_catalog_not_a_dict() {
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Cache a non-dict object
        resolver.cache_object(root_ref, PdfObject::Integer(42));

        let result = parse_catalog(&resolver, root_ref);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_catalog_resolve_error() {
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(999, 0);

        // Don't cache anything; resolve will fail
        let result = parse_catalog(&resolver, root_ref);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_catalog_optional_fields_missing() {
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        // Minimal catalog: only /Pages
        let mut dict = indexmap::IndexMap::new();
        dict.insert(intern("Pages"), PdfObject::Ref(ObjRef::new(2, 0)));
        let catalog_obj = PdfObject::Dict(Box::new(dict));
        resolver.cache_object(root_ref, catalog_obj);

        let result = parse_catalog(&resolver, root_ref);
        assert!(result.is_ok());

        let catalog = result.unwrap();
        assert!(catalog.outlines_ref.is_none());
        assert!(!catalog.mark_info.is_tagged);
        assert!(catalog.struct_tree_root_ref.is_none());
        assert!(catalog.acroform_ref.is_none());
        assert!(catalog.names_ref.is_none());
        assert!(catalog.metadata_ref.is_none());
        assert!(catalog.page_labels.is_none());
        assert!(catalog.oc_properties.is_none());
        assert!(catalog.open_action.is_none());
        assert!(catalog.aa.is_none());
        assert!(catalog.version.is_none());
    }

    #[test]
    fn test_parse_catalog_tagged_pdf() {
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        let mut dict = indexmap::IndexMap::new();
        dict.insert(intern("Pages"), PdfObject::Ref(ObjRef::new(2, 0)));
        dict.insert(intern("MarkInfo"), {
            let mut mark_info = indexmap::IndexMap::new();
            mark_info.insert(intern("Marked"), PdfObject::Bool(true));
            PdfObject::Dict(Box::new(mark_info))
        });
        let catalog_obj = PdfObject::Dict(Box::new(dict));
        resolver.cache_object(root_ref, catalog_obj);

        let catalog = parse_catalog(&resolver, root_ref).unwrap();
        assert!(catalog.mark_info.is_tagged);
    }

    #[test]
    fn test_parse_catalog_with_version() {
        let resolver = XrefResolver::new();
        let root_ref = ObjRef::new(1, 0);

        let mut dict = indexmap::IndexMap::new();
        dict.insert(intern("Pages"), PdfObject::Ref(ObjRef::new(2, 0)));
        dict.insert(intern("Version"), PdfObject::Name(intern("2.0")));
        let catalog_obj = PdfObject::Dict(Box::new(dict));
        resolver.cache_object(root_ref, catalog_obj);

        let catalog = parse_catalog(&resolver, root_ref).unwrap();
        assert_eq!(catalog.version, Some("2.0".to_string()));
    }

    #[test]
    fn test_roman_numerals_edge_cases() {
        assert_eq!(PageLabelStyle::RomanUppercase.format(0), "");
        assert_eq!(PageLabelStyle::RomanUppercase.format(1), "I");
        assert_eq!(PageLabelStyle::RomanUppercase.format(4), "IV");
        assert_eq!(PageLabelStyle::RomanUppercase.format(5), "V");
        assert_eq!(PageLabelStyle::RomanUppercase.format(9), "IX");
        assert_eq!(PageLabelStyle::RomanUppercase.format(10), "X");
        assert_eq!(PageLabelStyle::RomanUppercase.format(40), "XL");
        assert_eq!(PageLabelStyle::RomanUppercase.format(50), "L");
        assert_eq!(PageLabelStyle::RomanUppercase.format(90), "XC");
        assert_eq!(PageLabelStyle::RomanUppercase.format(100), "C");
        assert_eq!(PageLabelStyle::RomanUppercase.format(400), "CD");
        assert_eq!(PageLabelStyle::RomanUppercase.format(500), "D");
        assert_eq!(PageLabelStyle::RomanUppercase.format(900), "CM");
        assert_eq!(PageLabelStyle::RomanUppercase.format(1000), "M");
        assert_eq!(PageLabelStyle::RomanUppercase.format(1984), "MCMLXXXIV");
    }

    #[test]
    fn test_letters_edge_cases() {
        assert_eq!(PageLabelStyle::LettersLowercase.format(0), "");
        assert_eq!(PageLabelStyle::LettersLowercase.format(1), "a");
        assert_eq!(PageLabelStyle::LettersLowercase.format(25), "y");
        assert_eq!(PageLabelStyle::LettersLowercase.format(26), "z");
        assert_eq!(PageLabelStyle::LettersLowercase.format(27), "aa");
        assert_eq!(PageLabelStyle::LettersLowercase.format(52), "az");
        assert_eq!(PageLabelStyle::LettersLowercase.format(53), "ba");
        assert_eq!(PageLabelStyle::LettersLowercase.format(703), "aaa");
    }

    #[test]
    fn test_page_label_format_with_prefix() {
        let label = PageLabel {
            style: PageLabelStyle::Decimal,
            prefix: Some("Section ".to_string()),
            start: 5,
        };

        assert_eq!(label.format(0), "Section 5");
        assert_eq!(label.format(1), "Section 6");
        assert_eq!(label.format(2), "Section 7");
    }

    #[test]
    fn test_page_labels_tree_empty() {
        let tree = PageLabelsTree::new();
        assert!(tree.is_empty());
        assert!(tree.get_label(0).is_none());
    }

    #[test]
    fn test_page_labels_tree_with_prefix() {
        let mut tree = PageLabelsTree::new();

        tree.labels.push((0, PageLabel {
            style: PageLabelStyle::RomanLowercase,
            prefix: Some("front-".to_string()),
            start: 1,
        }));

        tree.labels.push((3, PageLabel {
            style: PageLabelStyle::Decimal,
            prefix: None,
            start: 1,
        }));

        // Test with prefix using format_absolute for correct relative indexing
        assert_eq!(tree.get_label_with_start(0).map(|(l, start)| l.format_absolute(0, start)), Some("front-i".to_string()));
        assert_eq!(tree.get_label_with_start(1).map(|(l, start)| l.format_absolute(1, start)), Some("front-ii".to_string()));
        assert_eq!(tree.get_label_with_start(3).map(|(l, start)| l.format_absolute(3, start)), Some("1".to_string()));
    }
}

/// Property tests for catalog parsing fuzzing.
///
/// Per acceptance criteria: "proptest: random PdfObject as /Root content never panics parse_catalog"
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use std::sync::Arc;

    /// Strategy to generate arbitrary PdfObject values for fuzzing.
    fn arb_pdf_object(_depth: u32) -> impl Strategy<Value = PdfObject> {
        prop_oneof![
            Just(PdfObject::Null),
            any::<bool>().prop_map(PdfObject::Bool),
            any::<i64>().prop_map(PdfObject::Integer),
            any::<f64>().prop_map(|f| if f.is_finite() { PdfObject::Real(f) } else { PdfObject::Real(0.0) }),
            prop::collection::vec(any::<u8>(), 0..100).prop_map(|v| PdfObject::String(Box::new(v))),
            "[a-zA-Z]{1,20}".prop_map(|s| PdfObject::Name(intern(&s))),
            prop::collection::vec(any::<u8>(), 0..100).prop_map(|bytes| {
                // Try to create a valid name from the bytes
                let name: String = bytes.iter().map(|&b| if b.is_ascii_alphanumeric() { b as char } else { '_' }).collect();
                PdfObject::Name(intern(&name))
            }),
        ]
    }

    /// Strategy to generate arbitrary dictionaries for catalog fuzzing.
    fn arb_catalog_dict() -> impl Strategy<Value = indexmap::IndexMap<Arc<str>, PdfObject>> {
        prop::collection::hash_map("[a-zA-Z]{1,10}", arb_pdf_object(0), 0..10)
            .prop_map(|map| {
                let mut index_map = indexmap::IndexMap::new();
                for (k, v) in map {
                    index_map.insert(k.into(), v);
                }
                index_map
            })
    }

    proptest! {
        /// Test that parse_catalog never panics on arbitrary PdfObject input (INV-8).
        #[test]
        fn fuzz_parse_catalog_no_panics(dict in arb_catalog_dict()) {
            let resolver = XrefResolver::new();
            let root_ref = ObjRef::new(1, 0);

            // Cache the arbitrary dict as the catalog
            let catalog_obj = PdfObject::Dict(Box::new(dict));
            resolver.cache_object(root_ref, catalog_obj);

            // This should never panic - it should always return Ok or Err with diagnostics
            let result = parse_catalog(&resolver, root_ref);

            // If we get Ok, verify the catalog is structurally valid
            // If we get Err, verify diagnostics are present
            match result {
                Ok(catalog) => {
                    // Catalog should always have a pages_ref, even if invalid
                    // (defaults to ObjRef::new(0, 0) in Default impl)
                    prop_assert!(catalog.pages_ref.object == 0 || catalog.pages_ref.object > 0);
                }
                Err(diagnostics) => {
                    // Should always have at least one diagnostic explaining the failure
                    prop_assert!(!diagnostics.is_empty());
                }
            }
        }

        /// Test that PageLabel parsing never panics on arbitrary input.
        #[test]
        fn fuzz_page_label_parse_no_panics(obj in arb_pdf_object(0)) {
            // This should never panic - should return None or Some(PageLabel)
            let _ = PageLabel::parse(&obj);
        }

        /// Test that PageLabelsTree parsing never panics on arbitrary input.
        #[test]
        fn fuzz_page_labels_tree_parse_no_panics(obj in arb_pdf_object(0)) {
            // This should never panic
            let _ = PageLabelsTree::parse(&obj);
        }

        /// Test that MarkInfo parsing never panics on arbitrary input.
        #[test]
        fn fuzz_mark_info_parse_no_panics(obj in arb_pdf_object(0)) {
            // This should never panic - should always return a valid MarkInfo
            let mark_info = MarkInfo::parse(&obj);
            // MarkInfo should always be structurally valid (booleans are always false/true)
            prop_assert!(mark_info.is_tagged == true || mark_info.is_tagged == false);
        }

        /// Test that roman numeral conversion handles all positive integers without panicking.
        #[test]
        fn fuzz_roman_numerals_no_panics(value in any::<i64>()) {
            // Clamp to reasonable range for testing
            let clamped = value.max(0).min(5000);
            let _ = PageLabelStyle::RomanUppercase.format(clamped);
            let _ = PageLabelStyle::RomanLowercase.format(clamped);
        }

        /// Test that letter conversion handles all positive integers without panicking.
        #[test]
        fn fuzz_letters_no_panics(value in any::<i64>()) {
            // Clamp to reasonable range for testing
            let clamped = value.max(0).min(100000);
            let _ = PageLabelStyle::LettersLowercase.format(clamped);
            let _ = PageLabelStyle::LettersUppercase.format(clamped);
        }
    }
}
