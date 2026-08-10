//! Integration tests for catalog emptiness checks.
//!
//! This module tests the three catalog emptiness detection scenarios:
//! 1. Empty catalog.dictionary (no keys at all)
//! 2. None catalog.dictionary (not a dictionary at all)
//! 3. Missing essential keys (/Type or /Pages)
//!
//! Per acceptance criteria:
//! - At least 3 new test cases covering the three emptiness scenarios
//! - Test verifies error message includes source identifier
//! - Test verifies valid catalog passes through normally
//! - All tests pass with no hangs or orphaned processes

use pdftract_core::document::{validate_pages_structure, DocumentError};
use pdftract_core::parser::catalog::{Catalog, is_catalog_dict_empty, is_catalog_dict_none, catalog_dict_missing_essential_keys};
use pdftract_core::parser::object::{PdfObject, ObjRef};
use pdftract_core::parser::xref::XrefResolver;
use indexmap::indexmap;

/// Test 1: Empty catalog.dictionary triggers DocumentError::EmptyDocument
#[test]
fn test_empty_catalog_dict_triggers_empty_document_error() {
    // Create a catalog with an empty dictionary (no keys at all)
    let empty_dict = PdfObject::Dict(Box::new(indexmap::IndexMap::new()));
    let catalog = Catalog::new(ObjRef::new(1, 0), empty_dict.clone());

    // Verify the helper function detects emptiness
    assert!(is_catalog_dict_empty(&empty_dict),
        "is_catalog_dict_empty should return true for empty dictionary");

    // Create a resolver
    let resolver = XrefResolver::new();

    // Call validate_pages_structure - should return EmptyDocument error
    let result = validate_pages_structure(&catalog, &resolver, "test_empty.pdf");

    // Verify the error type and message
    match result {
        Err(DocumentError::EmptyDocument { source }) => {
            assert_eq!(source, "test_empty.pdf",
                "Error message should include source identifier 'test_empty.pdf'");
        }
        Ok(_) => {
            panic!("Expected EmptyDocument error for empty catalog dictionary, but got Ok");
        }
        Err(other_error) => {
            panic!("Expected EmptyDocument error, but got {:?}", other_error);
        }
    }
}

/// Test 2: None catalog.dictionary triggers DocumentError::EmptyDocument
#[test]
fn test_none_catalog_dict_triggers_empty_document_error() {
    // Create a catalog with None dictionary (not a dictionary at all - use Null)
    let none_dict = PdfObject::Null;
    let catalog = Catalog::new(ObjRef::new(1, 0), none_dict.clone());

    // Verify the helper function detects None
    assert!(is_catalog_dict_none(&none_dict),
        "is_catalog_dict_none should return true for Null object");

    // Create a resolver
    let resolver = XrefResolver::new();

    // Call validate_pages_structure - should return EmptyDocument error
    let result = validate_pages_structure(&catalog, &resolver, "test_none.pdf");

    // Verify the error type and message
    match result {
        Err(DocumentError::EmptyDocument { source }) => {
            assert_eq!(source, "test_none.pdf",
                "Error message should include source identifier 'test_none.pdf'");
        }
        Ok(_) => {
            panic!("Expected EmptyDocument error for None catalog dictionary, but got Ok");
        }
        Err(other_error) => {
            panic!("Expected EmptyDocument error, but got {:?}", other_error);
        }
    }
}

/// Test 3: Missing essential keys triggers DocumentError::EmptyDocument
#[test]
fn test_missing_essential_keys_triggers_empty_document_error() {
    // Test Case 3a: Dictionary missing /Type (has /Pages)
    {
        let mut dict = indexmap::IndexMap::new();
        dict.insert("Pages".into(), PdfObject::Ref(ObjRef::new(2, 0)));
        let missing_type = PdfObject::Dict(Box::new(dict));
        let catalog = Catalog::new(ObjRef::new(1, 0), missing_type);

        // Verify the helper function detects missing essential keys
        assert!(catalog_dict_missing_essential_keys(&catalog),
            "catalog_dict_missing_essential_keys should return true when /Type is missing");

        let resolver = XrefResolver::new();
        let result = validate_pages_structure(&catalog, &resolver, "test_missing_type.pdf");

        match result {
            Err(DocumentError::EmptyDocument { source }) => {
                assert_eq!(source, "test_missing_type.pdf");
            }
            _ => panic!("Expected EmptyDocument error for catalog missing /Type"),
        }
    }

    // Test Case 3b: Dictionary missing /Pages (has /Type)
    {
        let mut dict = indexmap::IndexMap::new();
        dict.insert("Type".into(), PdfObject::Name("Catalog".into()));
        let missing_pages = PdfObject::Dict(Box::new(dict));
        let catalog = Catalog::new(ObjRef::new(1, 0), missing_pages);

        assert!(catalog_dict_missing_essential_keys(&catalog),
            "catalog_dict_missing_essential_keys should return true when /Pages is missing");

        let resolver = XrefResolver::new();
        let result = validate_pages_structure(&catalog, &resolver, "test_missing_pages.pdf");

        match result {
            Err(DocumentError::EmptyDocument { source }) => {
                assert_eq!(source, "test_missing_pages.pdf");
            }
            _ => panic!("Expected EmptyDocument error for catalog missing /Pages"),
        }
    }

    // Test Case 3c: Dictionary missing both /Type and /Pages (empty dict case)
    {
        let empty_dict = PdfObject::Dict(Box::new(indexmap::IndexMap::new()));
        let catalog = Catalog::new(ObjRef::new(1, 0), empty_dict);

        assert!(catalog_dict_missing_essential_keys(&catalog),
            "catalog_dict_missing_essential_keys should return true for empty dictionary");

        let resolver = XrefResolver::new();
        let result = validate_pages_structure(&catalog, &resolver, "test_missing_both.pdf");

        match result {
            Err(DocumentError::EmptyDocument { source }) => {
                assert_eq!(source, "test_missing_both.pdf");
            }
            _ => panic!("Expected EmptyDocument error for catalog missing both essential keys"),
        }
    }
}

/// Test 4: Error message includes source identifier
#[test]
fn test_error_message_includes_source_identifier() {
    let test_sources = vec![
        "document.pdf",
        "https://example.com/doc.pdf",
        "/path/to/file.pdf",
        "C:\\Users\\Test\\file.pdf",
        "relative/path/doc.pdf",
    ];

    for source in test_sources {
        let empty_dict = PdfObject::Dict(Box::new(indexmap::IndexMap::new()));
        let catalog = Catalog::new(ObjRef::new(1, 0), empty_dict);
        let resolver = XrefResolver::new();

        let result = validate_pages_structure(&catalog, &resolver, source);

        match result {
            Err(DocumentError::EmptyDocument { source: error_source }) => {
                assert_eq!(error_source, source,
                    "Error source should match the provided source identifier for '{}'", source);
            }
            _ => panic!("Expected EmptyDocument error with source '{}'", source),
        }
    }
}

/// Test 5: Valid catalog passes through normally
#[test]
fn test_valid_catalog_passes_through_normally() {
    // Create a resolver with a minimal pages tree first
    let resolver = XrefResolver::new();

    // Create a minimal page dictionary (object 3)
    let mut page_dict = indexmap::IndexMap::new();
    page_dict.insert("Type".into(), PdfObject::Name("Page".into()));
    page_dict.insert("MediaBox".into(), PdfObject::Array(Box::new(vec![
        PdfObject::Real(0.0),
        PdfObject::Real(0.0),
        PdfObject::Real(612.0),
        PdfObject::Real(792.0),
    ])));
    resolver.cache_object(ObjRef::new(3, 0), PdfObject::Dict(Box::new(page_dict)));

    // Create a Pages dictionary with /Kids array (object 2)
    let mut pages_dict = indexmap::IndexMap::new();
    pages_dict.insert("Type".into(), PdfObject::Name("Pages".into()));
    pages_dict.insert("Kids".into(), PdfObject::Array(Box::new(vec![
        PdfObject::Ref(ObjRef::new(3, 0)),
    ])));
    pages_dict.insert("Count".into(), PdfObject::Integer(1));
    resolver.cache_object(ObjRef::new(2, 0), PdfObject::Dict(Box::new(pages_dict)));

    // Create a valid catalog with both essential keys and a valid pages reference
    let mut dict = indexmap::IndexMap::new();
    dict.insert("Type".into(), PdfObject::Name("Catalog".into()));
    dict.insert("Pages".into(), PdfObject::Ref(ObjRef::new(2, 0)));
    let valid_dict = PdfObject::Dict(Box::new(dict));

    // Note: Catalog::new(pages_ref, raw_dict) - pages_ref must match the /Pages entry
    let catalog = Catalog::new(ObjRef::new(2, 0), valid_dict);

    // Verify helper functions return false (not empty, not None, not missing keys)
    assert!(!is_catalog_dict_empty(&catalog.raw_dict),
        "Valid catalog should not be detected as empty");
    assert!(!is_catalog_dict_none(&catalog.raw_dict),
        "Valid catalog should not be detected as None");
    assert!(!catalog_dict_missing_essential_keys(&catalog),
        "Valid catalog should not be detected as missing essential keys");

    // Call validate_pages_structure - should succeed
    let result = validate_pages_structure(&catalog, &resolver, "valid_document.pdf");

    match result {
        Ok(_) => {
            // Success - valid catalog passes through
        }
        Err(e) => {
            panic!("Expected Ok for valid catalog, but got error: {}", e);
        }
    }
}

/// Test 6: Integration test with various non-dictionary catalog types
#[test]
fn test_various_none_catalog_types_trigger_empty_document() {
    let non_dict_types = vec![
        (PdfObject::Null, "Null"),
        (PdfObject::Bool(true), "Bool(true)"),
        (PdfObject::Bool(false), "Bool(false)"),
        (PdfObject::Integer(42), "Integer"),
        (PdfObject::Real(3.14), "Real"),
        (PdfObject::String(Box::new(b"test".to_vec())), "String"),
        (PdfObject::Name("Test".into()), "Name"),
        (PdfObject::Array(Box::new(vec![PdfObject::Integer(1)])), "Array"),
    ];

    for (obj, type_name) in non_dict_types {
        let catalog = Catalog::new(ObjRef::new(1, 0), obj.clone());
        let resolver = XrefResolver::new();

        let result = validate_pages_structure(&catalog, &resolver, &format!("test_{}.pdf", type_name));

        match result {
            Err(DocumentError::EmptyDocument { source }) => {
                assert!(source.contains(&format!("test_{}.pdf", type_name)),
                    "Error source should contain the filename");
            }
            Ok(_) => {
                panic!("Expected EmptyDocument error for {} catalog, but got Ok", type_name);
            }
            Err(other) => {
                panic!("Expected EmptyDocument error for {} catalog, but got {:?}", type_name, other);
            }
        }
    }
}

/// Test 7: Test that empty dict detection happens before None dict detection
#[test]
fn test_detection_order_empty_then_none() {
    // Empty dict should be caught first, even though it's also "None" in some sense
    let empty_dict = PdfObject::Dict(Box::new(indexmap::IndexMap::new()));
    let catalog = Catalog::new(ObjRef::new(1, 0), empty_dict);

    // Verify empty is detected
    assert!(is_catalog_dict_empty(&catalog.raw_dict));
    // But it's not None (it IS a dictionary, just empty)
    assert!(!is_catalog_dict_none(&catalog.raw_dict));

    let resolver = XrefResolver::new();
    let result = validate_pages_structure(&catalog, &resolver, "test_order.pdf");

    match result {
        Err(DocumentError::EmptyDocument { .. }) => {
            // Correct - caught as empty dict
        }
        _ => panic!("Expected EmptyDocument error"),
    }
}

/// Test 8: Catalog with only optional fields but missing essential keys
#[test]
fn test_catalog_with_optional_fields_missing_essential() {
    // Create a catalog with optional fields but missing essential /Type and /Pages
    let mut dict = indexmap::IndexMap::new();
    dict.insert("Outlines".into(), PdfObject::Ref(ObjRef::new(3, 0)));
    dict.insert("MarkInfo".into(), PdfObject::Dict(Box::new(indexmap::IndexMap::new())));
    dict.insert("Version".into(), PdfObject::Name("1.4".into()));

    let catalog = Catalog::new(ObjRef::new(1, 0), PdfObject::Dict(Box::new(dict)));

    // Should detect missing essential keys despite having optional fields
    assert!(catalog_dict_missing_essential_keys(&catalog));

    let resolver = XrefResolver::new();
    let result = validate_pages_structure(&catalog, &resolver, "test_optional_only.pdf");

    match result {
        Err(DocumentError::EmptyDocument { source }) => {
            assert_eq!(source, "test_optional_only.pdf");
        }
        _ => panic!("Expected EmptyDocument error for catalog with only optional fields"),
    }
}

/// Test 9: Verify no panic or hang on empty catalog validation
#[test]
fn test_no_panic_or_hang_on_empty_catalog() {
    // This test ensures the validation doesn't hang or panic, per acceptance criteria
    use std::time::Instant;

    let empty_dict = PdfObject::Dict(Box::new(indexmap::IndexMap::new()));
    let catalog = Catalog::new(ObjRef::new(1, 0), empty_dict);
    let resolver = XrefResolver::new();

    let start = Instant::now();
    let result = validate_pages_structure(&catalog, &resolver, "test_hang.pdf");
    let elapsed = start.elapsed();

    // Should complete quickly (well under 1 second)
    assert!(elapsed.as_secs() < 1, "Validation should complete quickly, took {:?}", elapsed);

    // Should return error, not panic
    match result {
        Err(DocumentError::EmptyDocument { .. }) => {
            // Success - completed without panic
        }
        _ => panic!("Expected EmptyDocument error"),
    }
}
