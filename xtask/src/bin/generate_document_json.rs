//! Generate .expected.json files for document model test fixtures.
//!
//! Run with: cargo run --bin generate_expected_json

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use pdftract_core::document::parse_pdf_file;
use pdftract_core::detection;
use serde_json::json;

fn main() {
    println!("Generating .expected.json files for document model fixtures...");

    let fixtures_dir = PathBuf::from("tests/document_model/fixtures");

    let fixtures = [
        ("encrypted_rc4_test", Some("test")),
        ("encrypted_aes128_test", Some("test")),
        ("encrypted_aes256_test", Some("test")),
        ("encrypted_empty_password", Some("")),
        ("encrypted_unknown_handler", None),
        ("tagged_3_level_outline", None),
        ("ocg_default_off", None),
        ("multi_revision_3", None),
        ("inheritance_grandparent_mediabox", None),
        ("missing_mediabox", None),
        ("partial_resource_override", None),
        ("js_in_openaction", None),
        ("xfa_form", None),
        ("pdfa_1b_conformance", None),
        ("page_labels_roman_arabic", None),
    ];

    for (name, password) in fixtures.iter() {
        let pdf_path = fixtures_dir.join(format!("{}.pdf", name));
        let expected_path = fixtures_dir.join(format!("{}.expected.json", name));

        if !pdf_path.exists() {
            eprintln!("Warning: PDF fixture not found: {}", pdf_path.display());
            continue;
        }

        println!("Processing {}...", name);

        match generate_expected_json(&pdf_path, name, *password) {
            Ok(json_str) => {
                fs::write(&expected_path, &json_str)
                    .expect(&format!("Failed to write {}", expected_path.display()));
                println!("  Created {}", expected_path.display());
            }
            Err(e) => {
                eprintln!("  Error generating JSON for {}: {}", name, e);
                // Generate a fallback JSON with error info
                let fallback = json!({
                    "fixture": name,
                    "error": e.to_string(),
                    "page_count": 0,
                    "is_encrypted": false,
                    "is_tagged": false,
                    "ocg_present": false,
                    "contains_javascript": false,
                    "contains_xfa": false,
                    "pages": []
                });
                fs::write(&expected_path, &serde_json::to_string_pretty(&fallback).unwrap())
                    .expect(&format!("Failed to write {}", expected_path.display()));
                println!("  Created fallback {}", expected_path.display());
            }
        }
    }

    println!("\nAll .expected.json files generated!");
}

fn generate_expected_json(pdf_path: &Path, name: &str, _password: Option<&str>) -> Result<String, String> {
    // Parse the PDF - for now we use the unencrypted parse since the test
    // infrastructure doesn't support password-protected files yet
    let (_fingerprint, catalog, pages, resolver) = parse_pdf_file(pdf_path)
        .map_err(|e| format!("Failed to parse PDF: {}", e))?;

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
    let contains_javascript = detection::detect_javascript(&catalog, &pages, &acroform, &resolver);
    let contains_xfa = detection::detect_xfa(&acroform);

    // Get OCG information
    let ocg_present = catalog.oc_properties.as_ref().map(|p| p.present).unwrap_or(false);
    let ocg_base_state = catalog.oc_properties.as_ref()
        .map(|p| format!("{:?}", p.base_state));

    // Get page labels
    let page_labels: Vec<serde_json::Value> = if let Some(ref labels_tree) = catalog.page_labels {
        labels_tree.labels().iter()
            .map(|(idx, label)| {
                json!({
                    "index": idx,
                    "style": format!("{:?}", label.style),
                    "prefix": label.prefix,
                    "start": label.start,
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    // Build document metadata
    let mut doc = json!({
        "fixture": name,
        "page_count": pages.len(),
        "is_encrypted": is_encrypted,
        "is_tagged": catalog.mark_info.is_tagged,
        "ocg_present": ocg_present,
        "contains_javascript": contains_javascript,
        "contains_xfa": contains_xfa,
    });

    // Add encryption status if present
    if let Some(status) = encryption_status {
        doc.as_object_mut().unwrap().insert("encryption_status".to_string(), json!(status));
    }

    // Add OCG base state if present
    if let Some(base_state) = ocg_base_state {
        doc.as_object_mut().unwrap().insert("ocg_base_state".to_string(), json!(base_state));
    }

    // Add page labels if present
    if !page_labels.is_empty() {
        doc.as_object_mut().unwrap().insert("page_labels".to_string(), json!(page_labels));
    }

    // Add page-level information
    let pages_array: Vec<serde_json::Value> = pages.iter().enumerate().map(|(i, page)| {
        let mut page_obj = json!({
            "page_index": i,
            "media_box": page.media_box,
            "rotate": page.rotate,
        });

        // Add crop_box if present
        if let Some(crop_box) = page.crop_box {
            page_obj.as_object_mut().unwrap().insert("crop_box".to_string(), json!(crop_box));
        } else {
            page_obj.as_object_mut().unwrap().insert("crop_box".to_string(), json!(page.media_box));
        }

        // Track inheritance - add font info if present
        if !page.resources.fonts.is_empty() {
            let fonts: HashMap<_, _> = page.resources.fonts.iter()
                .map(|(name, _)| (name.clone(), "present".to_string()))
                .collect();
            page_obj.as_object_mut().unwrap().insert("fonts".to_string(), json!(fonts));
        }

        page_obj
    }).collect();

    doc.as_object_mut()
        .unwrap()
        .insert("pages".to_string(), json!(pages_array));

    Ok(serde_json::to_string_pretty(&doc).unwrap())
}
