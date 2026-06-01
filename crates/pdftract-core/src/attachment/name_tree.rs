//! /EmbeddedFiles name tree walker (PDF 1.7).
//!
//! This module implements the name tree walker for the /Catalog /Names /EmbeddedFiles
//! dictionary. Name trees are similar to number trees but use PdfString keys instead
//! of integer keys.
//!
//! Per PDF 1.7 spec §7.9.6 "Name Trees":
//! - Name trees map string keys to values (in this case, Filespec references)
//! - Structure is recursive: root node with /Kids or leaf node with /Names
//! - Each node has /Limits [min max] for the range of keys in that subtree
//! - Leaf nodes have /Names as alternating [key value key value ...] array
//! - Intermediate nodes have /Kids pointing to child nodes
//!
//! # Name Tree Structure
//!
//! ```text
//! Root node (dict)
//! ├── /Kids [ref1, ref2, ...]  (intermediate nodes)
//! └── /Names [key1, val1, key2, val2, ...]  (leaf entries)
//! ```
//!
//! Each node dict may have:
//! - `/Limits` [min_key max_key] - inclusive range of keys in this node's subtree
//! - `/Kids` [ref1, ref2, ...] - array of references to child nodes (intermediate only)
//! - `/Names` [key1, val1, ...] - array of alternating key-value pairs (leaf only)
//!
//! # Examples
//!
//! Walk the /EmbeddedFiles name tree:
//!
//! ```ignore
//! use pdftract_core::attachment::name_tree::walk_embedded_files;
//!
//! // names_ref is from catalog.names_ref
//! let entries = walk_embedded_files(&resolver, names_ref)?;
//!
//! for (name, filespec_ref) in entries {
//!     println!("Attachment: {} -> {}", name, filespec_ref);
//! }
//! ```

use crate::diagnostics::{DiagCode, Diagnostic};
use crate::parser::object::ObjRef;
use crate::parser::xref::XrefResolver;

/// Result type for name tree parsing.
pub type Result<T> = std::result::Result<T, Vec<Diagnostic>>;

/// A single entry from the /EmbeddedFiles name tree.
///
/// Contains the name (string key) and the Filespec reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedFileEntry {
    /// The name from the name tree (UTF-8 decoded)
    pub name: String,
    /// Reference to the Filespec dictionary
    pub filespec_ref: ObjRef,
}

impl EmbeddedFileEntry {
    /// Create a new embedded file entry.
    pub fn new(name: String, filespec_ref: ObjRef) -> Self {
        Self {
            name,
            filespec_ref,
        }
    }
}

/// Walk the /EmbeddedFiles name tree from the /Names dictionary.
///
/// # Arguments
/// * `resolver` - The xref resolver for resolving indirect references
/// * `names_ref` - Reference to the /Names dictionary from catalog
///
/// # Returns
///
/// A `Result<Vec<EmbeddedFileEntry>>` containing the list of embedded files.
/// Returns an empty Vec if /EmbeddedFiles is absent (not an error).
///
/// # Behavior
///
/// - If /Names is absent → returns Ok(vec![])
/// - If /Names resolution fails → returns Err with diagnostics
/// - If /EmbeddedFiles is absent → returns Ok(vec![])
/// - If name tree is malformed → emits diagnostics, continues with partial results
/// - Walks the tree depth-first, collecting all leaf entries
/// - Sorts entries by name for deterministic output
///
/// # Name Tree Walking
///
/// Per PDF 1.7 spec §7.9.6:
/// 1. Start at root /EmbeddedFiles dict
/// 2. If /Names present (leaf) → parse alternating key-value pairs
/// 3. If /Kids present (intermediate) → recursively walk each child
/// 4. Each node may have /Limits [min max] (not used for walking, only for optimization)
/// 5. Collect all entries and sort by key string
///
/// # Example
///
/// ```ignore
/// use pdftract_core::attachment::name_tree::walk_embedded_files;
///
/// // catalog.names_ref is the reference to /Names dictionary
/// let entries = walk_embedded_files(&resolver, catalog.names_ref)?;
///
/// for entry in entries {
///     println!("{}: filespec {}", entry.name, entry.filespec_ref);
/// }
/// ```
pub fn walk_embedded_files(
    resolver: &XrefResolver,
    names_ref: ObjRef,
) -> Result<Vec<EmbeddedFileEntry>> {
    let mut entries = Vec::new();
    let mut diagnostics = Vec::new();

    // Resolve the /Names dictionary
    let names_obj = match resolver.resolve(names_ref) {
        Ok(obj) => obj,
        Err(e) => {
            return Err(vec![Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!("Failed to resolve /Names {}: {}", names_ref, e),
            )]);
        }
    };

    let names_dict = match names_obj.as_dict() {
        Some(d) => d,
        None => {
            return Err(vec![Diagnostic::with_dynamic_no_offset(
                DiagCode::StructInvalidType,
                format!(
                    "/Names {} is not a dictionary (type: {})",
                    names_ref,
                    names_obj.type_name()
                ),
            )]);
        }
    };

    // Get /EmbeddedFiles from /Names (optional)
    let embedded_files_obj = match names_dict.get("/EmbeddedFiles") {
        Some(obj) => obj,
        None => {
            // /EmbeddedFiles is absent - this is normal for PDFs without attachments
            return Ok(entries);
        }
    };

    // /EmbeddedFiles must be a dict (the root of the name tree)
    let tree_root = match embedded_files_obj.as_ref() {
        Some(ref_) => match resolver.resolve(ref_) {
            Ok(obj) => obj,
            Err(e) => {
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructUnexpectedEof,
                    format!("Failed to resolve /EmbeddedFiles {}: {}", ref_, e),
                ));
                return Err(diagnostics);
            }
        },
        None => embedded_files_obj.clone(),
    };

    let tree_root_dict = match tree_root.as_dict() {
        Some(d) => d,
        None => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructInvalidType,
                format!(
                    "/EmbeddedFiles root is not a dictionary (type: {})",
                    tree_root.type_name()
                ),
            ));
            return Err(diagnostics);
        }
    };

    // Walk the tree recursively
    walk_tree_node(resolver, tree_root_dict, &mut entries, &mut diagnostics)?;

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    // Sort entries by name for deterministic output
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(entries)
}

/// Walk a single name tree node (either leaf or intermediate).
///
/// Recursively processes:
/// - Leaf nodes: parse /Names array for key-value pairs
/// - Intermediate nodes: recursively walk each /Kids entry
fn walk_tree_node(
    resolver: &XrefResolver,
    node_dict: &crate::parser::object::PdfDict,
    entries: &mut Vec<EmbeddedFileEntry>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<()> {
    // Check for /Names (leaf node) - alternating [key value key value ...]
    if let Some(names_array) = node_dict.get("/Names").and_then(|o| o.as_array()) {
        parse_names_array(names_array, entries, diagnostics)?;
    }

    // Check for /Kids (intermediate node) - array of child node references
    if let Some(kids_array) = node_dict.get("/Kids").and_then(|o| o.as_array()) {
        for (idx, kid_obj) in kids_array.iter().enumerate() {
            let kid_ref = match kid_obj.as_ref() {
                Some(r) => r,
                None => {
                    diagnostics.push(Diagnostic::with_dynamic_no_offset(
                        DiagCode::StructInvalidType,
                        format!(
                            "/Kids[{}] is not a reference (type: {})",
                            idx,
                            kid_obj.type_name()
                        ),
                    ));
                    continue;
                }
            };

            let kid_obj = match resolver.resolve(kid_ref) {
                Ok(obj) => obj,
                Err(e) => {
                    diagnostics.push(Diagnostic::with_dynamic_no_offset(
                        DiagCode::StructUnexpectedEof,
                        format!("Failed to resolve /Kids[{}] {}: {}", idx, kid_ref, e),
                    ));
                    continue;
                }
            };

            let kid_dict = match kid_obj.as_dict() {
                Some(d) => d,
                None => {
                    diagnostics.push(Diagnostic::with_dynamic_no_offset(
                        DiagCode::StructInvalidType,
                        format!(
                            "/Kids[{}] {} is not a dictionary (type: {})",
                            idx,
                            kid_ref,
                            kid_obj.type_name()
                        ),
                    ));
                    continue;
                }
            };

            // Recursively walk the child node
            walk_tree_node(resolver, kid_dict, entries, diagnostics)?;
        }
    }

    // Node may have /Limits [min max] - not used for walking, only for search optimization
    // We ignore /Limits since we're doing a full tree walk

    Ok(())
}

/// Parse a /Names array (alternating key-value pairs at leaves).
///
/// The /Names array has the structure:
/// ```text
/// [key1 value1 key2 value2 key3 value3 ...]
/// ```
///
/// Where:
/// - key is a PdfString (the attachment name)
/// - value is a Ref to a Filespec dictionary
fn parse_names_array(
    names: &[crate::parser::object::PdfObject],
    entries: &mut Vec<EmbeddedFileEntry>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<()> {
    for chunk in names.chunks(2) {
        if chunk.len() != 2 {
            // Odd number of elements - skip the last one
            continue;
        }

        // Key is a PdfString (attachment name)
        let key_bytes = match chunk[0].as_string() {
            Some(bytes) => bytes,
            None => {
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructInvalidType,
                    format!(
                        "/Names key is not a string (type: {})",
                        chunk[0].type_name()
                    ),
                ));
                continue;
            }
        };

        // Decode the key string (UTF-16BE BOM or PDFDocEncoding)
        let name = decode_name_key(key_bytes);
        if name.is_empty() {
            diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::StructInvalidType,
                "/Names key decoded to empty string",
            ));
            continue;
        }

        // Value is a Ref to Filespec
        let filespec_ref = match chunk[1].as_ref() {
            Some(r) => r,
            None => {
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructInvalidType,
                    format!(
                        "/Names value for key '{}' is not a reference (type: {})",
                        name,
                        chunk[1].type_name()
                    ),
                ));
                continue;
            }
        };

        entries.push(EmbeddedFileEntry::new(name, filespec_ref));
    }

    Ok(())
}

/// Decode a name tree key string to UTF-8.
///
/// Per PDF 1.7 spec §7.9.2 "Name Trees":
/// - Keys are PdfString objects (byte strings)
/// - PDF 1.7 uses PDFDocEncoding or UTF-16BE with BOM
/// - PDF 2.0 may use any UTF-8 string
///
/// This function tries:
/// 1. UTF-16BE BOM (0xFE 0xFF prefix) → UTF-8
/// 2. UTF-16BE without BOM heuristic → UTF-8 (most high bytes are 0x00)
/// 3. PDFDocEncoding fallback → Latin-1
fn decode_name_key(bytes: &[u8]) -> String {
    // Check for UTF-16BE BOM
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        return decode_utf16be_bom(&bytes[2..]);
    }

    // Check for UTF-16BE without BOM (heuristic)
    if looks_like_utf16be(bytes) {
        if let Ok(s) = decode_utf16be_raw(bytes) {
            return s;
        }
    }

    // Fall back to PDFDocEncoding (treat as Latin-1)
    decode_pdfdocencoding(bytes)
}

/// Decode UTF-16BE string with BOM (bytes after 0xFE 0xFF).
fn decode_utf16be_bom(bytes: &[u8]) -> String {
    if bytes.len() % 2 != 0 {
        return decode_pdfdocencoding(bytes);
    }

    let utf16_chars: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect();

    String::from_utf16(&utf16_chars).unwrap_or_default()
}

/// Decode raw UTF-16BE (without BOM).
fn decode_utf16be_raw(bytes: &[u8]) -> std::result::Result<String, ()> {
    if bytes.len() % 2 != 0 {
        return Err(());
    }

    let utf16_chars: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect();

    String::from_utf16(&utf16_chars).map_err(|_| ())
}

/// Heuristic check if bytes look like UTF-16BE.
///
/// Returns true if:
/// - Length is even
/// - Most high bytes (first byte of each pair) are 0x00
fn looks_like_utf16be(bytes: &[u8]) -> bool {
    if bytes.len() < 2 || bytes.len() % 2 != 0 {
        return false;
    }

    let mut zero_high_bytes = 0;
    let total_pairs = bytes.len() / 2;

    for chunk in bytes.chunks_exact(2) {
        if chunk[0] == 0x00 {
            zero_high_bytes += 1;
        }
    }

    zero_high_bytes >= total_pairs * 3 / 4
}

/// Decode PDFDocEncoding (treat as Latin-1 for basic use).
///
/// PDFDocEncoding is a superset of ISO-8859-1 (Latin-1) with some characters
/// remapped. For attachment names, treating as Latin-1 is sufficient.
fn decode_pdfdocencoding(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::{intern, PdfDict, PdfObject};
    use indexmap::IndexMap;

    /// Helper to create a test /Names dictionary with /EmbeddedFiles.
    fn make_names_dict(resolver: &XrefResolver, names_ref: ObjRef, tree_ref: ObjRef) {
        let mut dict = IndexMap::new();
        dict.insert(intern("/EmbeddedFiles"), PdfObject::Ref(tree_ref));
        resolver.cache_object(names_ref, PdfObject::Dict(Box::new(dict)));
    }

    /// Helper to create a name tree root with /Names (leaf).
    fn make_leaf_node(resolver: &XrefResolver, node_ref: ObjRef, entries: &[(Vec<u8>, ObjRef)]) {
        let mut names_array = Vec::new();
        for (key_bytes, filespec_ref) in entries {
            names_array.push(PdfObject::String(Box::new(key_bytes.clone())));
            names_array.push(PdfObject::Ref(*filespec_ref));
        }

        let mut dict = IndexMap::new();
        dict.insert(intern("/Names"), PdfObject::Array(Box::new(names_array)));
        resolver.cache_object(node_ref, PdfObject::Dict(Box::new(dict)));
    }

    /// Helper to create an intermediate node with /Kids.
    fn make_intermediate_node(
        resolver: &XrefResolver,
        node_ref: ObjRef,
        kids: &[ObjRef],
    ) {
        let kids_array: Vec<PdfObject> = kids.iter().map(|&r| PdfObject::Ref(r)).collect();
        let mut dict = IndexMap::new();
        dict.insert(intern("/Kids"), PdfObject::Array(Box::new(kids_array)));
        resolver.cache_object(node_ref, PdfObject::Dict(Box::new(dict)));
    }

    /// Helper to create a test Filespec (minimal).
    fn make_filespec(resolver: &XrefResolver, filespec_ref: ObjRef, filename: &str) {
        let mut dict = IndexMap::new();
        dict.insert(intern("/Type"), PdfObject::Name(intern("Filespec")));
        dict.insert(intern("/F"), PdfObject::String(Box::new(filename.as_bytes().to_vec())));

        let mut ef_dict = IndexMap::new();
        ef_dict.insert(intern("/F"), PdfObject::Ref(ObjRef::new(999, 0))); // Dummy stream ref
        dict.insert(intern("/EF"), PdfObject::Dict(Box::new(ef_dict)));

        resolver.cache_object(filespec_ref, PdfObject::Dict(Box::new(dict)));
    }

    #[test]
    fn test_walk_embedded_files_empty() {
        let resolver = XrefResolver::new();
        let names_ref = ObjRef::new(10, 0);

        // Create /Names without /EmbeddedFiles
        let mut names_dict = IndexMap::new();
        resolver.cache_object(names_ref, PdfObject::Dict(Box::new(names_dict)));

        let result = walk_embedded_files(&resolver, names_ref);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_walk_embedded_files_single_entry() {
        let resolver = XrefResolver::new();
        let names_ref = ObjRef::new(10, 0);
        let tree_ref = ObjRef::new(11, 0);
        let filespec_ref = ObjRef::new(12, 0);

        make_filespec(&resolver, filespec_ref, "test.pdf");
        make_leaf_node(&resolver, tree_ref, &[(b"test.pdf".to_vec(), filespec_ref)]);
        make_names_dict(&resolver, names_ref, tree_ref);

        let result = walk_embedded_files(&resolver, names_ref);
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "test.pdf");
        assert_eq!(entries[0].filespec_ref, filespec_ref);
    }

    #[test]
    fn test_walk_embedded_files_multiple_entries() {
        let resolver = XrefResolver::new();
        let names_ref = ObjRef::new(10, 0);
        let tree_ref = ObjRef::new(11, 0);

        let fs1 = ObjRef::new(20, 0);
        let fs2 = ObjRef::new(21, 0);
        let fs3 = ObjRef::new(22, 0);

        make_filespec(&resolver, fs1, "alpha.txt");
        make_filespec(&resolver, fs2, "beta.txt");
        make_filespec(&resolver, fs3, "gamma.txt");

        let entries = vec![
            (b"gamma.txt".to_vec(), fs3),
            (b"alpha.txt".to_vec(), fs1),
            (b"beta.txt".to_vec(), fs2),
        ];

        make_leaf_node(&resolver, tree_ref, &entries);
        make_names_dict(&resolver, names_ref, tree_ref);

        let result = walk_embedded_files(&resolver, names_ref);
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert_eq!(entries.len(), 3);

        // Verify sorting by name
        assert_eq!(entries[0].name, "alpha.txt");
        assert_eq!(entries[1].name, "beta.txt");
        assert_eq!(entries[2].name, "gamma.txt");

        // Verify refs are correct
        assert_eq!(entries[0].filespec_ref, fs1);
        assert_eq!(entries[1].filespec_ref, fs2);
        assert_eq!(entries[2].filespec_ref, fs3);
    }

    #[test]
    fn test_walk_embedded_files_with_kids() {
        let resolver = XrefResolver::new();
        let names_ref = ObjRef::new(10, 0);
        let root_ref = ObjRef::new(11, 0);
        let kid1_ref = ObjRef::new(12, 0);
        let kid2_ref = ObjRef::new(13, 0);

        let fs1 = ObjRef::new(20, 0);
        let fs2 = ObjRef::new(21, 0);
        let fs3 = ObjRef::new(22, 0);
        let fs4 = ObjRef::new(23, 0);
        let fs5 = ObjRef::new(24, 0);

        make_filespec(&resolver, fs1, "delta.txt");
        make_filespec(&resolver, fs2, "alpha.txt");
        make_filespec(&resolver, fs3, "epsilon.txt");
        make_filespec(&resolver, fs4, "beta.txt");
        make_filespec(&resolver, fs5, "gamma.txt");

        // First kid has 2 entries
        make_leaf_node(&resolver, kid1_ref, &[(b"delta.txt".to_vec(), fs1), (b"alpha.txt".to_vec(), fs2)]);

        // Second kid has 3 entries
        make_leaf_node(
            &resolver,
            kid2_ref,
            &[(b"epsilon.txt".to_vec(), fs3), (b"beta.txt".to_vec(), fs4), (b"gamma.txt".to_vec(), fs5)],
        );

        // Root has /Kids pointing to both leaves
        make_intermediate_node(&resolver, root_ref, &[kid1_ref, kid2_ref]);
        make_names_dict(&resolver, names_ref, root_ref);

        let result = walk_embedded_files(&resolver, names_ref);
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert_eq!(entries.len(), 5);

        // Verify sorted order
        assert_eq!(entries[0].name, "alpha.txt");
        assert_eq!(entries[1].name, "beta.txt");
        assert_eq!(entries[2].name, "delta.txt");
        assert_eq!(entries[3].name, "epsilon.txt");
        assert_eq!(entries[4].name, "gamma.txt");
    }

    #[test]
    fn test_walk_embedded_files_deep_tree() {
        let resolver = XrefResolver::new();
        let names_ref = ObjRef::new(10, 0);
        let root_ref = ObjRef::new(11, 0);
        let mid_ref = ObjRef::new(12, 0);
        let leaf1_ref = ObjRef::new(13, 0);
        let leaf2_ref = ObjRef::new(14, 0);

        let fs1 = ObjRef::new(30, 0);
        let fs2 = ObjRef::new(31, 0);
        let fs3 = ObjRef::new(32, 0);

        make_filespec(&resolver, fs1, "charlie.txt");
        make_filespec(&resolver, fs2, "alpha.txt");
        make_filespec(&resolver, fs3, "bravo.txt");

        // Level 2 leaves
        make_leaf_node(&resolver, leaf1_ref, &[(b"charlie.txt".to_vec(), fs1)]);
        make_leaf_node(&resolver, leaf2_ref, &[(b"alpha.txt".to_vec(), fs2), (b"bravo.txt".to_vec(), fs3)]);

        // Level 1 intermediate node
        make_intermediate_node(&resolver, mid_ref, &[leaf1_ref, leaf2_ref]);

        // Root with one kid
        make_intermediate_node(&resolver, root_ref, &[mid_ref]);
        make_names_dict(&resolver, names_ref, root_ref);

        let result = walk_embedded_files(&resolver, names_ref);
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert_eq!(entries.len(), 3);

        // Verify sorted order
        assert_eq!(entries[0].name, "alpha.txt");
        assert_eq!(entries[1].name, "bravo.txt");
        assert_eq!(entries[2].name, "charlie.txt");
    }

    #[test]
    fn test_decode_name_key_ascii() {
        let bytes: &[u8] = b"test.pdf";
        let decoded = decode_name_key(bytes);
        assert_eq!(decoded, "test.pdf");
    }

    #[test]
    fn test_decode_name_key_utf16be_bom() {
        // UTF-16BE BOM (0xFE 0xFF) + "test.pdf"
        let mut bytes = vec![0xFE, 0xFF];
        bytes.extend_from_slice(b"\x00t\x00e\x00s\x00t\x00.\x00p\x00d\x00f");
        let decoded = decode_name_key(&bytes);
        assert_eq!(decoded, "test.pdf");
    }

    #[test]
    fn test_decode_name_key_utf16be_no_bom() {
        // UTF-16BE without BOM (high bytes are 0x00)
        let bytes: &[u8] = b"\x00t\x00e\x00s\x00t";
        let decoded = decode_name_key(bytes);
        assert_eq!(decoded, "test");
    }

    #[test]
    fn test_decode_name_key_latin1() {
        // Latin-1 encoded (é = 0xE9)
        let bytes: &[u8] = b"\x74\xE9\x73\x74"; // "tést"
        let decoded = decode_name_key(bytes);
        assert_eq!(decoded, "t\u{00E9}st"); // t + é + s + t
    }

    #[test]
    fn test_embedded_file_entry_new() {
        let entry = EmbeddedFileEntry::new("example.txt".to_string(), ObjRef::new(42, 0));
        assert_eq!(entry.name, "example.txt");
        assert_eq!(entry.filespec_ref, ObjRef::new(42, 0));
    }

    #[test]
    fn test_walk_embedded_files_non_string_key() {
        let resolver = XrefResolver::new();
        let names_ref = ObjRef::new(10, 0);
        let tree_ref = ObjRef::new(11, 0);
        let filespec_ref = ObjRef::new(12, 0);

        make_filespec(&resolver, filespec_ref, "test.pdf");

        // Create a leaf with a non-string key (should emit diagnostic)
        let mut names_array = Vec::new();
        names_array.push(PdfObject::Name(intern("invalid"))); // Name instead of String
        names_array.push(PdfObject::Ref(filespec_ref));

        let mut dict = IndexMap::new();
        dict.insert(intern("/Names"), PdfObject::Array(Box::new(names_array)));
        resolver.cache_object(tree_ref, PdfObject::Dict(Box::new(dict)));

        make_names_dict(&resolver, names_ref, tree_ref);

        let result = walk_embedded_files(&resolver, names_ref);
        assert!(result.is_err());

        let diagnostics = result.unwrap_err();
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("not a string")));
    }

    #[test]
    fn test_walk_embedded_files_non_ref_value() {
        let resolver = XrefResolver::new();
        let names_ref = ObjRef::new(10, 0);
        let tree_ref = ObjRef::new(11, 0);

        // Create a leaf with a non-Ref value (should emit diagnostic)
        let mut names_array = Vec::new();
        names_array.push(PdfObject::String(Box::new(b"test.pdf".to_vec())));
        names_array.push(PdfObject::Name(intern("invalid"))); // Name instead of Ref

        let mut dict = IndexMap::new();
        dict.insert(intern("/Names"), PdfObject::Array(Box::new(names_array)));
        resolver.cache_object(tree_ref, PdfObject::Dict(Box::new(dict)));

        make_names_dict(&resolver, names_ref, tree_ref);

        let result = walk_embedded_files(&resolver, names_ref);
        assert!(result.is_err());

        let diagnostics = result.unwrap_err();
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("not a reference")));
    }

    #[test]
    fn test_walk_embedded_files_odd_names_array() {
        let resolver = XrefResolver::new();
        let names_ref = ObjRef::new(10, 0);
        let tree_ref = ObjRef::new(11, 0);
        let filespec_ref = ObjRef::new(12, 0);

        make_filespec(&resolver, filespec_ref, "test.pdf");

        // Create a leaf with odd number of elements (last key should be ignored)
        let mut names_array = Vec::new();
        names_array.push(PdfObject::String(Box::new(b"test.pdf".to_vec())));
        names_array.push(PdfObject::Ref(filespec_ref));
        names_array.push(PdfObject::String(Box::new(b"orphan".to_vec()))); // No value

        let mut dict = IndexMap::new();
        dict.insert(intern("/Names"), PdfObject::Array(Box::new(names_array)));
        resolver.cache_object(tree_ref, PdfObject::Dict(Box::new(dict)));

        make_names_dict(&resolver, names_ref, tree_ref);

        let result = walk_embedded_files(&resolver, names_ref);
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert_eq!(entries.len(), 1); // Only one complete pair
        assert_eq!(entries[0].name, "test.pdf");
    }

    #[test]
    fn test_decode_name_key_empty() {
        let bytes: &[u8] = b"";
        let decoded = decode_name_key(bytes);
        assert_eq!(decoded, "");
    }

    #[test]
    fn test_looks_like_utf16be() {
        // UTF-16BE pattern (high bytes are 0x00)
        assert!(looks_like_utf16be(b"\x00t\x00e\x00s\x00t"));

        // Not UTF-16BE (mixed high bytes)
        assert!(!looks_like_utf16be(b"test"));

        // Too short
        assert!(!looks_like_utf16be(b"\x00"));

        // Odd length (5 bytes - should return false)
        assert!(!looks_like_utf16be(b"\x00t\x00e\x00s\x00"));
    }

    #[test]
    fn test_decode_utf16be_bom() {
        // Valid UTF-16BE with BOM (10 bytes = 5 chars)
        let bytes = b"\x00H\x00e\x00l\x00l\x00o";
        let decoded = decode_utf16be_bom(bytes);
        assert_eq!(decoded, "Hello");

        // Odd length (7 bytes) - fallback to PDFDocEncoding (treat each byte as char)
        let bytes = b"\x00H\x00e\x00l\x00";  // 7 bytes (odd)
        let decoded = decode_utf16be_bom(bytes);
        assert_eq!(decoded, "\u{0}H\u{0}e\u{0}l\u{0}"); // Each 0x00 becomes null char
    }

    #[test]
    fn test_decode_utf16be_raw() {
        // Valid UTF-16BE
        let bytes = b"\x00W\x00o\x00r\x00l\x00d";
        let decoded = decode_utf16be_raw(bytes).unwrap();
        assert_eq!(decoded, "World");

        // Odd length (3 bytes, not 4)
        let bytes = b"\x00W\x00o\x00";
        assert!(decode_utf16be_raw(bytes).is_err());

        // Valid surrogate pair for U+10000
        let bytes = b"\xD8\x00\xDC\x00"; // High surrogate 0xD800, Low surrogate 0xDC00
        let decoded = decode_utf16be_raw(bytes).unwrap();
        assert_eq!(decoded.chars().count(), 1); // Single code point
        assert_eq!(decoded, "\u{10000}");
    }

    #[test]
    fn test_decode_pdfdocencoding() {
        // ASCII
        assert_eq!(decode_pdfdocencoding(b"hello"), "hello");

        // Latin-1 extended
        let bytes = b"\xE9\xE0\xEE"; // é à î
        let decoded = decode_pdfdocencoding(bytes);
        assert_eq!(decoded.chars().count(), 3); // Check character count, not byte length
        assert_eq!(decoded, "éàî");
    }
}
