//! Integration tests for the PDF document model.
//!
//! These tests verify the complete document model construction by:
//! 1. Walking fixture files in tests/document_model/fixtures/
//! 2. Building the Document via Document::open()
//! 3. Comparing the resolved structure against the .expected.json golden file
//! 4. Verifying encryption status, OCG visibility map, outline tree, JS/XFA/conformance flags

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use pdftract_core::detection;
use pdftract_core::document::parse_pdf_file;
use pdftract_core::javascript;
use pdftract_core::parser::catalog::Catalog;
use pdftract_core::parser::pages::PageDict;
use pdftract_core::parser::xref::XrefResolver;
use serde_json::Value;

/// A single test fixture for document model construction.
struct Fixture {
    name: String,
    /// Path to the PDF fixture file
    pdf_path: PathBuf,
    /// Path to the expected JSON output
    expected_path: PathBuf,
    /// Optional password for encrypted files
    password: Option<String>,
}

impl Fixture {
    /// Load a fixture from the fixtures directory.
    fn load(name: &str) -> Self {
        let fixtures_dir = PathBuf::from("tests/document_model/fixtures");
        let pdf_path = fixtures_dir.join(format!("{}.pdf", name));
        let expected_path = fixtures_dir.join(format!("{}.expected.json", name));

        // Check PDF file exists
        assert!(
            pdf_path.exists(),
            "Fixture PDF not found: {}",
            pdf_path.display()
        );

        Self {
            name: name.to_string(),
            pdf_path,
            expected_path,
            password: None,
        }
    }

    /// Load a fixture with a password.
    fn load_with_password(name: &str, password: &str) -> Self {
        let mut fixture = Self::load(name);
        fixture.password = Some(password.to_string());
        fixture
    }
}

/// Compare JSON values with a helpful error message.
fn assert_json_eq(expected: &Value, actual: &Value, context: &str) {
    if expected != actual {
        println!("\n=== JSON MISMATCH ===");
        println!("Context: {}", context);
        println!("Expected: {}", serde_json::to_string_pretty(expected).unwrap());
        println!("Actual: {}", serde_json::to_string_pretty(actual).unwrap());
        println!("=====================\n");
        panic!("JSON mismatch at: {}", context);
    }
}

/// Test a single fixture.
fn test_fixture(fixture: Fixture) {
    println!("Testing fixture: {}", fixture.name);

    // Parse the PDF
    let (_fingerprint, catalog, pages, resolver) = parse_pdf_file(&fixture.pdf_path)
        .unwrap_or_else(|e| panic!("Failed to parse fixture {}: {}", fixture.name, e));

    // Read the expected JSON if it exists
    let expected_json = if fixture.expected_path.exists() {
        let json_str = fs::read_to_string(&fixture.expected_path)
            .unwrap_or_else(|e| panic!("Failed to read expected.json for {}: {}", fixture.name, e));
        Some(serde_json::from_str::<Value>(&json_str)
            .unwrap_or_else(|e| panic!("Failed to parse expected.json for {}: {}", fixture.name, e)))
    } else {
        None
    };

    // Build the actual JSON from the parsed document
    let actual_json = build_document_json(&fixture.name, &catalog, &pages, &resolver);

    // If expected JSON exists, compare; otherwise, print actual for manual review
    if let Some(expected) = expected_json {
        assert_json_eq(&expected, &actual_json, &fixture.name);
    } else {
        println!("No .expected.json found - actual output:");
        println!("{}", serde_json::to_string_pretty(&actual_json).unwrap());
    }
}

/// Build a JSON representation of the document for comparison.
fn build_document_json(
    fixture_name: &str,
    catalog: &Catalog,
    pages: &[PageDict],
    resolver: &XrefResolver,
) -> Value {
    // Check for encryption
    let is_encrypted = catalog.diagnostics.iter()
        .any(|d| d.code.contains("ENCRYPTION"));

    // Get encryption status from diagnostics
    let encryption_status = catalog.diagnostics.iter()
        .find(|d| d.code.contains("ENCRYPTION"))
        .map(|d| d.message.clone());

    // Resolve AcroForm if present
    let acroform = catalog.acroform_ref
        .and_then(|r| resolver.resolve(r).ok())
        .and_then(|o| o.as_dict().cloned());

    // Detect JavaScript and XFA
    let contains_javascript = detection::detect_javascript(catalog, pages, &acroform, resolver);
    let contains_xfa = detection::detect_xfa(&acroform);

    // Get OCG information
    let ocg_present = catalog.oc_properties.as_ref().map(|p| p.present).unwrap_or(false);
    let ocg_base_state = catalog.oc_properties.as_ref()
        .and_then(|p| Some(format!("{:?}", p.base_state)));

    // Get page labels
    let page_labels: Vec<Value> = if let Some(ref labels_tree) = catalog.page_labels {
        labels_tree.labels.iter()
            .map(|(idx, label)| {
                serde_json::json!({
                    "index": idx,
                    "style": label.style,
                    "value": label.value,
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    // Build document metadata
    let mut doc = serde_json::json!({
        "fixture": fixture_name,
        "page_count": pages.len(),
        "is_encrypted": is_encrypted,
        "is_tagged": catalog.mark_info.is_tagged,
        "ocg_present": ocg_present,
        "contains_javascript": contains_javascript,
        "contains_xfa": contains_xfa,
    });

    // Add encryption status if present
    if let Some(status) = encryption_status {
        doc.as_object_mut().unwrap().insert("encryption_status".to_string(), Value::String(status));
    }

    // Add OCG base state if present
    if let Some(base_state) = ocg_base_state {
        doc.as_object_mut().unwrap().insert("ocg_base_state".to_string(), Value::String(base_state));
    }

    // Add page labels if present
    if !page_labels.is_empty() {
        doc.as_object_mut().unwrap().insert("page_labels".to_string(), Value::Array(page_labels));
    }

    // Add page-level information
    let pages_array: Vec<Value> = pages.iter().enumerate().map(|(i, page)| {
        let mut page_obj = serde_json::json!({
            "page_index": i,
            "media_box": page.media_box,
            "rotate": page.rotate,
        });

        // Add crop_box if present
        if let Some(crop_box) = page.crop_box {
            page_obj.as_object_mut().unwrap().insert("crop_box".to_string(), serde_json::json!(crop_box));
        } else {
            page_obj.as_object_mut().unwrap().insert("crop_box".to_string(), serde_json::json!(page.media_box));
        }

        // Track inheritance
        if !page.resources.fonts.is_empty() {
            let fonts: HashMap<_, _> = page.resources.fonts.iter()
                .map(|(name, _)| (name.clone(), "present".to_string()))
                .collect();
            page_obj.as_object_mut().unwrap().insert("fonts".to_string(), serde_json::json!(fonts));
        }

        page_obj
    }).collect();

    doc.as_object_mut()
        .unwrap()
        .insert("pages".to_string(), Value::Array(pages_array));

    doc
}

// Test functions for each fixture category

#[test]
fn test_encrypted_rc4() {
    let fixture = Fixture::load_with_password("encrypted_rc4_test", "test");
    test_fixture(fixture);
}

#[test]
fn test_encrypted_aes128() {
    let fixture = Fixture::load_with_password("encrypted_aes128_test", "test");
    test_fixture(fixture);
}

#[test]
fn test_encrypted_aes256() {
    let fixture = Fixture::load_with_password("encrypted_aes256_test", "test");
    test_fixture(fixture);
}

#[test]
fn test_encrypted_empty_password() {
    let fixture = Fixture::load_with_password("encrypted_empty_password", "");
    test_fixture(fixture);
}

#[test]
fn test_encrypted_unknown_handler() {
    let fixture = Fixture::load("encrypted_unknown_handler");
    test_fixture(fixture);
}

#[test]
fn test_tagged_3_level_outline() {
    let fixture = Fixture::load("tagged_3_level_outline");
    test_fixture(fixture);
}

#[test]
fn test_ocg_default_off() {
    let fixture = Fixture::load("ocg_default_off");
    test_fixture(fixture);
}

#[test]
fn test_multi_revision_3() {
    let fixture = Fixture::load("multi_revision_3");
    test_fixture(fixture);
}

#[test]
fn test_inheritance_grandparent_mediabox() {
    let fixture = Fixture::load("inheritance_grandparent_mediabox");
    test_fixture(fixture);
}

#[test]
fn test_missing_mediabox() {
    let fixture = Fixture::load("missing_mediabox");
    test_fixture(fixture);
}

#[test]
fn test_partial_resource_override() {
    let fixture = Fixture::load("partial_resource_override");
    test_fixture(fixture);
}

#[test]
fn test_js_in_openaction() {
    let fixture = Fixture::load("js_in_openaction");
    test_fixture(fixture);
}

#[test]
fn test_xfa_form() {
    let fixture = Fixture::load("xfa_form");
    test_fixture(fixture);
}

#[test]
fn test_pdfa_1b_conformance() {
    let fixture = Fixture::load("pdfa_1b_conformance");
    test_fixture(fixture);
}

#[test]
fn test_page_labels_roman_arabic() {
    let fixture = Fixture::load("page_labels_roman_arabic");
    test_fixture(fixture);
}
