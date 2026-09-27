//! End-to-end coverage for document-level extraction result surfaces.
//!
//! The fixtures in `tests/fixtures/auxiliary` are intentionally small and
//! hand-shaped around one PDF feature each. Keeping these checks at the
//! extraction boundary catches regressions in discovery, conversion, and JSON
//! serialization together.

use std::path::PathBuf;

use pdftract_core::extract::{extract_pdf, result_to_json};
use pdftract_core::options::ExtractionOptions;
use serde_json::{json, Value};

const AUXILIARY_SURFACES: &[&str] = &[
    "signatures",
    "form_fields",
    "links",
    "attachments",
    "threads",
    "javascript_actions",
];

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/auxiliary")
        .join(name)
}

fn extract_json(name: &str) -> Value {
    let result = extract_pdf(&fixture(name), &ExtractionOptions::default())
        .unwrap_or_else(|error| panic!("failed to extract {name}: {error}"));
    let output = result_to_json(&result);

    // Exercise both the public result serializer and the output adapter. The
    // document-level arrays must agree in both representations.
    let serialized_result = serde_json::to_value(&result).expect("result serializes to JSON");
    for surface in AUXILIARY_SURFACES {
        assert_eq!(
            output.get(*surface),
            serialized_result.get(*surface),
            "{surface} differs between result and result_to_json"
        );
    }

    let encoded = serde_json::to_string(&output).expect("output serializes to JSON text");
    let decoded: Value = serde_json::from_str(&encoded).expect("serialized output is valid JSON");
    for surface in AUXILIARY_SURFACES {
        assert_eq!(
            decoded.get(*surface),
            output.get(*surface),
            "serialized {surface} surface must round-trip"
        );
    }
    output
}

fn assert_empty_surfaces(output: &Value) {
    for surface in AUXILIARY_SURFACES {
        let value = output
            .get(*surface)
            .unwrap_or_else(|| panic!("missing {surface} result surface"));
        assert!(value.is_array(), "{surface} must serialize as an array");
        assert!(
            value.as_array().unwrap().is_empty(),
            "{surface} must be empty"
        );
    }
}

#[test]
fn plain_pdf_has_empty_auxiliary_surfaces() {
    let output = extract_json("plain.pdf");
    assert_empty_surfaces(&output);
}

#[test]
fn form_and_javascript_fixture_preserves_existing_surfaces() {
    let output = extract_json("form-and-javascript.pdf");

    assert_eq!(
        output["form_fields"],
        json!([{
            "name": "customer_name",
            "type": "text",
            "value": "Ada Lovelace",
            "multiline": false,
            "required": false,
            "read_only": false
        }])
    );
    assert_eq!(
        output["javascript_actions"],
        json!([{
            "location": "catalog.openaction",
            "code_excerpt": "app.alert('fixture')"
        }])
    );
    for surface in ["signatures", "links", "attachments", "threads"] {
        assert!(output[surface].as_array().unwrap().is_empty(), "{surface}");
    }
}

#[test]
fn signature_fixture_extracts_and_serializes_metadata() {
    let output = extract_json("signature.pdf");
    let expected_coverage = 400.0
        / std::fs::metadata(fixture("signature.pdf"))
            .expect("signature fixture metadata")
            .len() as f64;

    assert_eq!(
        output["signatures"],
        json!([{
            "field_name": "approval_signature",
            "signer_name": "Ada Lovelace",
            "signing_date": "2026-01-02T11:22:33Z",
            "reason": "Approved",
            "location": "New York",
            "sub_filter": "adbe.pkcs7.detached",
            "byte_range": [0, 100, 200, 300],
            "coverage_fraction": expected_coverage,
            "validation_status": "not_checked"
        }])
    );
}

#[test]
fn links_fixture_extracts_uri_and_named_links() {
    let output = extract_json("links.pdf");

    assert_eq!(output["links"].as_array().unwrap().len(), 2);
    assert_eq!(
        output["links"][0],
        json!({
            "page_index": 0,
            "rect": [72.0, 700.0, 220.0, 720.0],
            "uri": "https://example.com/docs"
        })
    );
    assert_eq!(
        output["links"][1],
        json!({
            "page_index": 0,
            "rect": [72.0, 650.0, 220.0, 670.0],
            "dest": "chapter-one"
        })
    );
}

#[test]
fn attachment_fixture_extracts_and_base64_serializes_content() {
    let output = extract_json("attachment.pdf");

    assert_eq!(
        output["attachments"],
        json!([{
            "name": "notes.txt",
            "description": "Fixture notes",
            "mime_type": "text/plain",
            "size": 17,
            "data": "SGVsbG8gYXR0YWNobWVudCE=",
            "truncated": false
        }])
    );
}

#[test]
fn article_thread_fixture_extracts_bead_chain() {
    let output = extract_json("article-thread.pdf");

    assert_eq!(
        output["threads"],
        json!([{
            "title": "Fixture thread",
            "author": "Ada",
            "subject": "Reading order",
            "keywords": "pdf,thread",
            "beads": [
                { "page_index": 0, "rect": [50.0, 600.0, 250.0, 720.0] },
                { "page_index": 1, "rect": [300.0, 100.0, 550.0, 220.0] }
            ]
        }])
    );
}
