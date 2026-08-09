//! Document model integration tests.
//!
//! This test module loads curated PDF fixtures and verifies that the document
//! model correctly extracts and resolves all document-level information.

use pdftract_core::detection::{detect_javascript, detect_xfa};
use pdftract_core::document::parse_pdf_file;
use pdftract_core::parser::catalog::Catalog;
use pdftract_core::parser::pages::PageDict;
use pdftract_core::parser::xref::XrefResolver;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Golden file structure for document model verification.
///
/// This captures all the document-level information that should be
/// extracted and resolved by the document model integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DocumentModelGolden {
    /// Number of pages in the document
    page_count: usize,
    /// Encryption information (if applicable)
    encryption: Option<EncryptionInfo>,
    /// Optional content groups visibility (if present)
    ocg_visibility: Option<OcgVisibility>,
    /// Outline/bookmarks structure (if present)
    outlines: Option<OutlineNode>,
    /// JavaScript detection result
    contains_javascript: bool,
    /// XFA form detection result
    contains_xfa: bool,
    /// Page labels (if present)
    page_labels: Option<Vec<String>>,
    /// PDF/A conformance (if present in XMP metadata)
    pdfa_conformance: Option<String>,
    /// Diagnostics emitted during parsing
    diagnostics: Vec<DiagnosticInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptionInfo {
    is_encrypted: bool,
    handler: Option<String>,
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OcgVisibility {
    default_state: String,
    groups: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OutlineNode {
    title: String,
    dest_page: Option<usize>,
    children: Vec<OutlineNode>,
    is_expanded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiagnosticInfo {
    code: String,
    message: String,
}

/// Load a fixture PDF and extract its document model.
fn load_fixture(fixture_path: &Path) -> Result<DocumentModelGolden, Box<dyn std::error::Error>> {
    // Parse the PDF
    let (_fingerprint, catalog, pages, resolver) = parse_pdf_file(fixture_path)?;

    // Check encryption status
    let encryption_info = check_encryption(&resolver);

    // Extract OCG visibility
    let ocg_visibility = extract_ocg_visibility(&catalog);

    // Extract outlines (pass pages for destination resolution)
    let outlines = extract_outlines_with_pages(&catalog, &resolver, &pages);

    // Detect JavaScript and XFA
    let acroform = catalog.acroform_ref
        .and_then(|r| resolver.resolve(r).ok())
        .and_then(|o| o.as_dict().cloned());
    let contains_javascript = detect_javascript(&catalog, &pages, &acroform, &resolver);
    let contains_xfa = detect_xfa(&acroform);

    // Extract page labels
    let page_labels = extract_page_labels(&catalog, pages.len());

    // Extract PDF/A conformance
    let pdfa_conformance = extract_pdfa_conformance(&catalog, &resolver);

    // Collect diagnostics
    let diagnostics = collect_diagnostics(&catalog);

    Ok(DocumentModelGolden {
        page_count: pages.len(),
        encryption: encryption_info,
        ocg_visibility,
        outlines,
        contains_javascript,
        contains_xfa,
        page_labels,
        pdfa_conformance,
        diagnostics,
    })
}

/// Extract outline/bookmarks structure with pages for destination resolution.
fn extract_outlines_with_pages(
    catalog: &Catalog,
    resolver: &XrefResolver,
    pages: &[pdftract_core::parser::pages::PageDict],
) -> Option<OutlineNode> {
    let outlines_ref = catalog.outlines_ref?;
    let (outlines, _diagnostics) = pdftract_core::parser::outline::parse_outlines(
        resolver,
        Some(outlines_ref),
        pages,
    );

    if outlines.is_empty() {
        return None;
    }

    // Convert the first outline to our test structure
    // For now, just return the first outline at the root level
    Some(convert_outline_to_node(&outlines[0]))
}

/// Convert an Outline to our test's OutlineNode structure.
fn convert_outline_to_node(outline: &pdftract_core::parser::outline::Outline) -> OutlineNode {
    OutlineNode {
        title: outline.title.clone(),
        dest_page: outline.dest_page.map(|p| p as usize),
        children: outline.children.iter().map(convert_outline_to_node).collect(),
        is_expanded: outline.count > 0,
    }
}

/// Check if the document is encrypted.
///
/// This function attempts to detect encryption by parsing the trailer's
/// /Encrypt dictionary. Returns None for unencrypted documents.
fn check_encryption(resolver: &XrefResolver) -> Option<EncryptionInfo> {
    // Access the trailer from the resolver
    let trailer = &resolver.xref_section.trailer?;

    // Use the encryption detection module
    let mut diagnostics = Vec::new();
    let info = pdftract_core::encryption::detection::detect_encryption(
        trailer,
        resolver,
        &mut diagnostics,
    );

    // Map encryption::detection::EncryptionInfo to our test's EncryptionInfo
    info.map(|enc| EncryptionInfo {
        is_encrypted: true,
        handler: Some(format!("V={} R={}", enc.version, enc.revision)),
        status: format!("{}-bit", enc.key_length),
    })
}

/// Extract OCG visibility information.
fn extract_ocg_visibility(catalog: &Catalog) -> Option<OcgVisibility> {
    let oc_props = catalog.oc_properties.as_ref()?;
    let default_state = match oc_props.default_state {
        pdftract_core::parser::ocg::BaseState::On => "ON".to_string(),
        pdftract_core::parser::ocg::BaseState::Off => "OFF".to_string(),
        pdftract_core::parser::ocg::BaseState::Unchanged => "UNCHANGED".to_string(),
    };

    let groups: Vec<String> = oc_props.optional_content
        .iter()
        .map(|ocg| ocg.name.clone().unwrap_or_else(|| "Unnamed".to_string()))
        .collect();

    Some(OcgVisibility {
        default_state,
        groups,
    })
}

/// Extract outline/bookmarks structure.
fn extract_outlines(catalog: &Catalog, resolver: &XrefResolver) -> Option<OutlineNode> {
    let outlines_ref = catalog.outlines_ref?;
    // Note: parse_outlines needs the pages array, but we only have the resolver here.
    // For now, return None - this would require refactoring load_fixture to pass pages.
    None
}

/// Extract page labels for all pages.
fn extract_page_labels(catalog: &Catalog, page_count: usize) -> Option<Vec<String>> {
    let labels_tree = catalog.page_labels.as_ref()?;
    let mut labels = Vec::new();
    for i in 0..page_count as i64 {
        let label = labels_tree.get_label(i)?;
        let start = labels_tree.get_label_with_start(i)?.1;
        labels.push(label.format_absolute(i, start));
    }
    Some(labels)
}

/// Extract PDF/A conformance from XMP metadata.
fn extract_pdfa_conformance(catalog: &Catalog, resolver: &XrefResolver) -> Option<String> {
    let metadata_ref = catalog.metadata_ref?;
    let metadata_obj = resolver.resolve(metadata_ref).ok()?;
    let metadata_dict = metadata_obj.as_dict()?;
    let stream = metadata_dict.get("")?.as_stream()?;
    let metadata_bytes = stream.decoded_data.ok()?;
    let metadata_str = std::string::String::from_utf8(metadata_bytes).ok()?;

    // Simple check for PDF/A identifiers
    if metadata_str.contains("pdfaid:part") && metadata_str.contains("pdfaid:conformance") {
        // Extract part and conformance
        let part = metadata_str
            .split("pdfaid:part")
            .nth(1)?
            .split('>')
            .nth(1)?
            .split('<')
            .next()?;
        let conformance = metadata_str
            .split("pdfaid:conformance")
            .nth(1)?
            .split('>')
            .nth(1)?
            .split('<')
            .next()?;
        Some(format!("PDF/A-{}{}", part.trim(), conformance.trim()))
    } else {
        None
    }
}

/// Collect diagnostics emitted during parsing.
fn collect_diagnostics(catalog: &Catalog) -> Vec<DiagnosticInfo> {
    catalog
        .diagnostics
        .iter()
        .map(|d| DiagnosticInfo {
            code: d.code.to_string(),
            message: d.message.clone(),
        })
        .collect()
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::fs;

    fn run_fixture_test(fixture_name: &str) {
        let fixture_path = Path::new("tests/document_model/fixtures")
            .join(fixture_name)
            .with_extension("pdf");
        let expected_path = Path::new("tests/document_model/fixtures")
            .join(fixture_name)
            .with_extension("expected.json");

        // Load the fixture
        let actual = load_fixture(&fixture_path)
            .unwrap_or_else(|e| panic!("Failed to load fixture {:?}: {}", fixture_path, e));

        // Load or create the expected golden file
        let expected: DocumentModelGolden = if expected_path.exists() {
            serde_json::from_str(&fs::read_to_string(&expected_path).unwrap())
                .unwrap_or_else(|e| panic!("Failed to parse golden file {:?}: {}", expected_path, e))
        } else {
            // Create golden file if it doesn't exist
            let golden_json = serde_json::to_string_pretty(&actual).unwrap();
            fs::write(&expected_path, golden_json).unwrap();
            eprintln!("Created golden file: {:?}", expected_path);
            return; // Skip test assertion for newly created golden
        };

        // Compare with golden
        assert_eq!(
            actual, expected,
            "Fixture {} does not match golden file",
            fixture_name
        );
    }

    #[test]
    fn test_encrypted_rc4() {
        run_fixture_test("encrypted_rc4_test");
    }

    #[test]
    fn test_encrypted_aes128() {
        run_fixture_test("encrypted_aes128_test");
    }

    #[test]
    fn test_encrypted_aes256() {
        run_fixture_test("encrypted_aes256_test");
    }

    #[test]
    fn test_encrypted_empty_password() {
        run_fixture_test("encrypted_empty_password");
    }

    #[test]
    fn test_tagged_3_level_outline() {
        run_fixture_test("tagged_3_level_outline");
    }

    #[test]
    fn test_ocg_default_off() {
        run_fixture_test("ocg_default_off");
    }

    #[test]
    fn test_multi_revision_3() {
        run_fixture_test("multi_revision_3");
    }

    #[test]
    fn test_inheritance_grandparent_mediabox() {
        run_fixture_test("inheritance_grandparent_mediabox");
    }

    #[test]
    fn test_missing_mediabox() {
        run_fixture_test("missing_mediabox");
    }

    #[test]
    fn test_partial_resource_override() {
        run_fixture_test("partial_resource_override");
    }

    #[test]
    fn test_js_in_openaction() {
        run_fixture_test("js_in_openaction");
    }

    #[test]
    fn test_xfa_form() {
        run_fixture_test("xfa_form");
    }

    #[test]
    fn test_pdfa_1b_conformance() {
        run_fixture_test("pdfa_1b_conformance");
    }

    #[test]
    fn test_page_labels_roman_arabic() {
        run_fixture_test("page_labels_roman_arabic");
    }

    #[test]
    fn test_encrypted_unknown_handler() {
        run_fixture_test("encrypted_unknown_handler");
    }
}
