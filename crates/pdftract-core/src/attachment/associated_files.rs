//! /AF (Associated Files) array walker (PDF 2.0).
//!
//! This module implements the /AF array walker for PDF 2.0 documents.
//! /AF is the canonical location for embedded attachments in PDF 2.0,
//! superseding the legacy /EmbeddedFiles name tree.
//!
//! Per ISO 32000-2 §14.13:
//! - /AF is an array of Filespec dictionary references
//! - Each Filespec may have /AFRelationship indicating the file's role
//! - /AF can appear at document-level (/Catalog), page-level, or annotation-level
//!   (this module only handles document-level /Catalog /AF)
//!
//! # Relationship values
//!
//! Per PDF 2.0 spec, /AFRelationship can be:
//! - "Source": The file is the source for the content of the PDF
//! - "Data": The file contains data referenced by the PDF
//! - "Alternative": An alternative representation of the PDF
//! - "Supplement": Supplementary data for the PDF
//! - "EncryptedPayload": The file is an encrypted payload
//! - "Unspecified": No specific relationship (default)

use crate::diagnostics::{DiagCode, Diagnostic};
use crate::parser::object::ObjRef;
use crate::parser::xref::XrefResolver;

/// Result type for /AF parsing.
pub type Result<T> = std::result::Result<T, Vec<Diagnostic>>;

/// A single entry from the /AF array.
///
/// Contains the optional /AFRelationship string and the Filespec reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssociatedFileEntry {
    /// The /AFRelationship value (e.g., "Source", "Data", "Alternative")
    ///
    /// None if the Filespec does not specify a relationship.
    pub relationship: Option<String>,
    /// Reference to the Filespec dictionary
    pub filespec_ref: ObjRef,
}

impl AssociatedFileEntry {
    /// Create a new associated file entry.
    pub fn new(relationship: Option<String>, filespec_ref: ObjRef) -> Self {
        Self {
            relationship,
            filespec_ref,
        }
    }
}

/// Walk the /AF (Associated Files) array from the document catalog.
///
/// # Arguments
/// * `resolver` - The xref resolver for resolving indirect references
/// * `catalog_dict` - The catalog dictionary (already resolved)
///
/// # Returns
///
/// A `Result<Vec<AssociatedFileEntry>>` containing the list of associated files.
/// Returns an empty Vec if /AF is absent (not an error).
///
/// # Behavior
///
/// - If /AF is absent → returns Ok(vec![])
/// - If /AF is not an array → emits diagnostic, returns Ok(vec![])
/// - For each entry in /AF:
///   - Must be a Ref (Filespec reference)
///   - Resolves the Filespec to extract /AFRelationship
///   - Skips non-Ref entries with diagnostic
///
/// # Example
///
/// ```ignore
/// use pdftract_core::attachment::associated_files::{walk_af_array, AssociatedFileEntry};
///
/// // catalog_dict is the parsed /Catalog dictionary
/// let entries = walk_af_array(&resolver, &catalog_dict)?;
///
/// for entry in entries {
///     let relationship = entry.relationship.as_deref().unwrap_or("Unspecified");
///     println!("Filespec {}: relationship={}", entry.filespec_ref, relationship);
/// }
/// ```
pub fn walk_af_array(
    resolver: &XrefResolver,
    catalog_dict: &crate::parser::object::PdfDict,
) -> Result<Vec<AssociatedFileEntry>> {
    let mut entries = Vec::new();
    let mut diagnostics = Vec::new();

    // Get /AF from catalog (optional)
    let af_obj = match catalog_dict.get("/AF") {
        Some(obj) => obj,
        None => {
            // /AF is absent in PDF 1.7 documents - this is normal
            return Ok(entries);
        }
    };

    // /AF must be an array
    let af_array = match af_obj.as_array() {
        Some(arr) => arr,
        None => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructInvalidType,
                format!("/AF is not an array (type: {})", af_obj.type_name()),
            ));
            return Err(diagnostics);
        }
    };

    // Iterate through /AF array entries
    for (idx, entry_obj) in af_array.iter().enumerate() {
        // Each entry must be a Ref to a Filespec dictionary
        let filespec_ref = match entry_obj.as_ref() {
            Some(r) => r,
            None => {
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructInvalidType,
                    format!(
                        "/AF[{}] is not a reference (type: {})",
                        idx,
                        entry_obj.type_name()
                    ),
                ));
                continue;
            }
        };

        // Resolve the Filespec to extract /AFRelationship
        let relationship = match extract_af_relationship(resolver, filespec_ref) {
            Ok(rel) => rel,
            Err(mut errs) => {
                diagnostics.append(&mut errs);
                continue;
            }
        };

        entries.push(AssociatedFileEntry::new(relationship, filespec_ref));
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    Ok(entries)
}

/// Extract the /AFRelationship value from a Filespec dictionary.
///
/// # Arguments
/// * `resolver` - The xref resolver
/// * `filespec_ref` - Reference to the Filespec dictionary
///
/// # Returns
///
/// `Ok(Some(String))` if /AFRelationship is present,
/// `Ok(None)` if absent (valid; not all Filespecs have this),
/// `Err` if resolution fails.
fn extract_af_relationship(
    resolver: &XrefResolver,
    filespec_ref: ObjRef,
) -> Result<Option<String>> {
    let mut diagnostics = Vec::new();

    // Resolve the Filespec dictionary
    let filespec_obj = match resolver.resolve(filespec_ref) {
        Ok(obj) => obj,
        Err(e) => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!("Failed to resolve Filespec {}: {}", filespec_ref, e),
            ));
            return Err(diagnostics);
        }
    };

    // Get the Filespec dictionary
    let filespec_dict = match filespec_obj.as_dict() {
        Some(d) => d,
        None => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructInvalidType,
                format!(
                    "Filespec {} is not a dictionary (type: {})",
                    filespec_ref,
                    filespec_obj.type_name()
                ),
            ));
            return Err(diagnostics);
        }
    };

    // Extract /AFRelationship (optional)
    let relationship = filespec_dict.get("/AFRelationship").and_then(|obj| {
        // /AFRelationship is typically a Name object
        obj.as_name().map(|s| s.to_string())
    });

    Ok(relationship)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::{intern, PdfDict, PdfObject};
    use indexmap::IndexMap;

    /// Helper to create a test Filespec dictionary.
    fn make_filespec(resolver: &XrefResolver, obj_ref: ObjRef, relationship: Option<&str>) {
        let mut dict = IndexMap::new();
        dict.insert(intern("/Type"), PdfObject::Name(intern("Filespec")));
        dict.insert(intern("/F"), PdfObject::Name(intern("test.pdf")));
        dict.insert(intern("/UF"), PdfObject::Name(intern("test.pdf")));

        if let Some(rel) = relationship {
            dict.insert(intern("/AFRelationship"), PdfObject::Name(intern(rel)));
        }

        resolver.cache_object(obj_ref, PdfObject::Dict(Box::new(dict)));
    }

    /// Helper to create a test /AF array.
    fn make_af_array(refs: &[ObjRef]) -> PdfObject {
        let arr: Vec<PdfObject> = refs.iter().map(|&r| PdfObject::Ref(r)).collect();
        PdfObject::Array(Box::new(arr))
    }

    #[test]
    fn test_walk_af_array_empty() {
        let resolver = XrefResolver::new();
        let catalog_dict = PdfDict::new();

        let result = walk_af_array(&resolver, &catalog_dict);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_walk_af_array_single_entry() {
        let resolver = XrefResolver::new();

        // Create a Filespec with /AFRelationship
        let filespec_ref = ObjRef::new(10, 0);
        make_filespec(&resolver, filespec_ref, Some("Source"));

        // Create /AF array
        let af_array = make_af_array(&[filespec_ref]);

        // Create catalog with /AF
        let mut catalog_dict = IndexMap::new();
        catalog_dict.insert(intern("/AF"), af_array);

        let result = walk_af_array(&resolver, &catalog_dict);
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].relationship, Some("Source".to_string()));
        assert_eq!(entries[0].filespec_ref, filespec_ref);
    }

    #[test]
    fn test_walk_af_array_multiple_entries() {
        let resolver = XrefResolver::new();

        // Create three Filespecs with different relationships
        let fs1 = ObjRef::new(10, 0);
        make_filespec(&resolver, fs1, Some("Source"));

        let fs2 = ObjRef::new(11, 0);
        make_filespec(&resolver, fs2, Some("Data"));

        let fs3 = ObjRef::new(12, 0);
        make_filespec(&resolver, fs3, Some("Alternative"));

        // Create /AF array
        let af_array = make_af_array(&[fs1, fs2, fs3]);

        // Create catalog with /AF
        let mut catalog_dict = IndexMap::new();
        catalog_dict.insert(intern("/AF"), af_array);

        let result = walk_af_array(&resolver, &catalog_dict);
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].relationship, Some("Source".to_string()));
        assert_eq!(entries[1].relationship, Some("Data".to_string()));
        assert_eq!(entries[2].relationship, Some("Alternative".to_string()));
    }

    #[test]
    fn test_walk_af_array_no_relationship() {
        let resolver = XrefResolver::new();

        // Create a Filespec without /AFRelationship
        let filespec_ref = ObjRef::new(10, 0);
        make_filespec(&resolver, filespec_ref, None);

        // Create /AF array
        let af_array = make_af_array(&[filespec_ref]);

        // Create catalog with /AF
        let mut catalog_dict = IndexMap::new();
        catalog_dict.insert(intern("/AF"), af_array);

        let result = walk_af_array(&resolver, &catalog_dict);
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].relationship, None);
    }

    #[test]
    fn test_walk_af_array_not_an_array() {
        let resolver = XrefResolver::new();

        // Create catalog with /AF as a non-array
        let mut catalog_dict = IndexMap::new();
        catalog_dict.insert(intern("/AF"), PdfObject::Name(intern("invalid")));

        let result = walk_af_array(&resolver, &catalog_dict);
        assert!(result.is_err());

        let diagnostics = result.unwrap_err();
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("not an array")));
    }

    #[test]
    fn test_walk_af_array_non_ref_entry() {
        let resolver = XrefResolver::new();

        // Create a Filespec
        let filespec_ref = ObjRef::new(10, 0);
        make_filespec(&resolver, filespec_ref, Some("Source"));

        // Create /AF array with a non-Ref entry
        let mut arr = vec![PdfObject::Ref(filespec_ref)];
        arr.push(PdfObject::Name(intern("invalid")));
        let af_array = PdfObject::Array(Box::new(arr));

        // Create catalog with /AF
        let mut catalog_dict = IndexMap::new();
        catalog_dict.insert(intern("/AF"), af_array);

        let result = walk_af_array(&resolver, &catalog_dict);
        assert!(result.is_err());

        let diagnostics = result.unwrap_err();
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("not a reference")));
    }

    #[test]
    fn test_associated_file_entry_new() {
        let entry = AssociatedFileEntry::new(Some("Data".to_string()), ObjRef::new(42, 0));

        assert_eq!(entry.relationship, Some("Data".to_string()));
        assert_eq!(entry.filespec_ref, ObjRef::new(42, 0));
    }

    #[test]
    fn test_extract_af_relationship_present() {
        let resolver = XrefResolver::new();
        let filespec_ref = ObjRef::new(10, 0);
        make_filespec(&resolver, filespec_ref, Some("Supplement"));

        let result = extract_af_relationship(&resolver, filespec_ref);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("Supplement".to_string()));
    }

    #[test]
    fn test_extract_af_relationship_absent() {
        let resolver = XrefResolver::new();
        let filespec_ref = ObjRef::new(10, 0);
        make_filespec(&resolver, filespec_ref, None);

        let result = extract_af_relationship(&resolver, filespec_ref);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_extract_af_relationship_resolve_error() {
        let resolver = XrefResolver::new();
        let filespec_ref = ObjRef::new(999, 0); // Not cached

        let result = extract_af_relationship(&resolver, filespec_ref);
        assert!(result.is_err());
    }

    #[test]
    fn test_walk_af_array_preserves_order() {
        let resolver = XrefResolver::new();

        // Create Filespecs in a specific order
        let fs1 = ObjRef::new(30, 0);
        make_filespec(&resolver, fs1, Some("Unspecified"));

        let fs2 = ObjRef::new(10, 0);
        make_filespec(&resolver, fs2, Some("EncryptedPayload"));

        let fs3 = ObjRef::new(20, 0);
        make_filespec(&resolver, fs3, Some("Source"));

        // Create /AF array in insertion order
        let af_array = make_af_array(&[fs1, fs2, fs3]);

        // Create catalog with /AF
        let mut catalog_dict = IndexMap::new();
        catalog_dict.insert(intern("/AF"), af_array);

        let result = walk_af_array(&resolver, &catalog_dict);
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert_eq!(entries.len(), 3);

        // Verify order is preserved
        assert_eq!(entries[0].filespec_ref, fs1);
        assert_eq!(entries[1].filespec_ref, fs2);
        assert_eq!(entries[2].filespec_ref, fs3);

        assert_eq!(entries[0].relationship, Some("Unspecified".to_string()));
        assert_eq!(
            entries[1].relationship,
            Some("EncryptedPayload".to_string())
        );
        assert_eq!(entries[2].relationship, Some("Source".to_string()));
    }

    #[test]
    fn test_walk_af_array_all_relationship_types() {
        let resolver = XrefResolver::new();

        // Test all standard /AFRelationship values from PDF 2.0 spec
        let relationships = [
            "Source",
            "Data",
            "Alternative",
            "Supplement",
            "EncryptedPayload",
            "Unspecified",
        ];

        let mut refs = Vec::new();
        for (idx, rel) in relationships.iter().enumerate() {
            let fs_ref = ObjRef::new(10 + idx as u32, 0);
            make_filespec(&resolver, fs_ref, Some(rel));
            refs.push(fs_ref);
        }

        let af_array = make_af_array(&refs);

        let mut catalog_dict = IndexMap::new();
        catalog_dict.insert(intern("/AF"), af_array);

        let result = walk_af_array(&resolver, &catalog_dict);
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert_eq!(entries.len(), relationships.len());

        for (idx, entry) in entries.iter().enumerate() {
            assert_eq!(entry.relationship.as_deref(), Some(relationships[idx]));
        }
    }
}
