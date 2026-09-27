//! Exercise the NDJSON footer `errors` union through the public streaming API.
//!
//! The lower-level diagnostics contract tests build an `ExtractionResult`
//! directly. This test deliberately runs extraction so that page failures,
//! document diagnostics, and the footer are produced by the real pipeline.

use std::collections::BTreeSet;

use pdftract_core::extract::{extract_pdf, result_to_json};
use pdftract_core::options::ExtractionOptions;
use pdftract_core::output::ndjson::extract_streaming;
use serde_json::Value;

/// Build a small, valid PDF whose pages without `/Contents` fail during page
/// extraction. The marked document also emits the document-level tagged-PDF
/// diagnostic, giving the footer both sides of its union contract.
fn fixture_pdf(page_count: usize, failing_pages: &[usize], tagged: bool) -> Vec<u8> {
    assert!(page_count > 0);
    assert!(failing_pages.iter().all(|&page| page < page_count));

    let content_object = 3 + page_count;
    let kids = (0..page_count)
        .map(|page| format!("{} 0 R", 3 + page))
        .collect::<Vec<_>>()
        .join(" ");
    let mark_info = tagged
        .then_some("/MarkInfo << /Marked true >>")
        .unwrap_or("");

    let mut objects = vec![
        format!("<< /Type /Catalog /Pages 2 0 R {mark_info} >>").into_bytes(),
        format!("<< /Type /Pages /Kids [ {kids} ] /Count {page_count} >>").into_bytes(),
    ];

    for page in 0..page_count {
        let page_body = if failing_pages.contains(&page) {
            // No `/Contents` makes decode_page_content_streams report the
            // page extraction failure while leaving the page tree readable.
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> >>".to_owned()
        } else {
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
                 /Resources << >> /Contents {content_object} 0 R >>"
            )
        };
        objects.push(page_body.into_bytes());
    }

    // A non-empty but text-free content stream is enough for a successful
    // page, and keeps this fixture independent of font parsing.
    objects.push(b"<< /Length 3 >>\nstream\nq Q\nendstream".to_vec());

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }

    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

fn ndjson_frames(output: &[u8]) -> Vec<Value> {
    output
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).expect("stream output must be valid NDJSON"))
        .collect()
}

fn assert_synthetic_page_error(value: &Value, message: &str) {
    let object = value
        .as_object()
        .expect("synthetic error must be an object");
    let keys: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        BTreeSet::from(["code", "severity", "message"]),
        "synthetic page errors have exactly the documented fields"
    );
    assert_eq!(value["code"], "page_extraction_error");
    assert_eq!(value["severity"], "error");
    assert_eq!(value["message"], message);
}

#[test]
fn streaming_footer_puts_page_failures_before_document_diagnostics() {
    let temp_dir = tempfile::tempdir().expect("temporary directory");
    let pdf_path = temp_dir.path().join("mixed-page-errors.pdf");
    std::fs::write(&pdf_path, fixture_pdf(3, &[1, 2], true)).expect("write fixture PDF");

    let options = ExtractionOptions::default();
    let result = extract_pdf(&pdf_path, &options).expect("reference extraction succeeds");
    assert_eq!(result.metadata.error_count, 2);
    assert_eq!(
        result
            .pages
            .iter()
            .filter(|page| page.error.is_some())
            .count(),
        2
    );
    assert!(result
        .metadata
        .diagnostics_detailed
        .iter()
        .any(|diagnostic| diagnostic.code == "TAGGED_PDF_STRUCT_TREE_DEFERRED"));

    // Synthetic page failures belong only to the NDJSON page/footer surfaces,
    // never to either metadata diagnostics array.
    let metadata = serde_json::to_value(&result.metadata).expect("metadata serializes");
    for diagnostic in metadata["diagnostics"].as_array().into_iter().flatten() {
        assert!(!diagnostic
            .as_str()
            .unwrap()
            .contains("page_extraction_error"));
    }
    for diagnostic in metadata["diagnostics_detailed"]
        .as_array()
        .into_iter()
        .flatten()
    {
        assert_ne!(diagnostic["code"], "page_extraction_error");
    }
    let compact = result_to_json(&result);
    assert!(!compact.to_string().contains("page_extraction_error"));

    let mut output = Vec::new();
    extract_streaming(&pdf_path, &options, &mut output).expect("NDJSON extraction succeeds");
    let frames = ndjson_frames(&output);
    assert!(frames
        .first()
        .and_then(|frame| frame.get("schema_version"))
        .is_some());
    assert!(frames
        .last()
        .and_then(|frame| frame.get("extraction_quality"))
        .is_some());

    let page_frames: Vec<&Value> = frames
        .iter()
        .filter(|frame| frame.get("page_index").is_some())
        .collect();
    assert_eq!(page_frames.len(), result.pages.len());
    let failed_pages: Vec<&Value> = page_frames
        .iter()
        .copied()
        .filter(|frame| frame.get("errors").is_some())
        .collect();
    assert_eq!(failed_pages.len(), result.metadata.error_count);
    for (frame, page) in failed_pages
        .iter()
        .zip(result.pages.iter().filter(|page| page.error.is_some()))
    {
        let errors = frame["errors"]
            .as_array()
            .expect("failed page errors array");
        assert_eq!(errors.len(), 1);
        assert_synthetic_page_error(&errors[0], page.error.as_deref().unwrap());
    }

    let footer = frames.last().expect("footer frame");
    let footer_errors = footer["errors"].as_array().expect("footer errors array");
    assert_eq!(
        footer_errors
            .iter()
            .take_while(|error| error["code"] == "page_extraction_error")
            .count(),
        result.metadata.error_count,
        "footer synthetic count agrees with metadata.error_count"
    );
    for (error, page) in footer_errors
        .iter()
        .take(result.metadata.error_count)
        .zip(result.pages.iter().filter(|page| page.error.is_some()))
    {
        assert_synthetic_page_error(error, page.error.as_deref().unwrap());
    }

    let document_errors: Vec<Value> = result
        .metadata
        .diagnostics_detailed
        .iter()
        .map(|diagnostic| serde_json::to_value(diagnostic).expect("diagnostic serializes"))
        .collect();
    assert_eq!(
        &footer_errors[result.metadata.error_count..],
        document_errors.as_slice(),
        "document diagnostics follow all synthetic page failures in emission order"
    );
}

#[test]
fn streaming_footer_always_emits_an_empty_errors_array_when_clean() {
    let temp_dir = tempfile::tempdir().expect("temporary directory");
    let pdf_path = temp_dir.path().join("clean.pdf");
    std::fs::write(&pdf_path, fixture_pdf(1, &[], false)).expect("write fixture PDF");

    let mut output = Vec::new();
    extract_streaming(&pdf_path, &ExtractionOptions::default(), &mut output)
        .expect("clean NDJSON extraction succeeds");
    let frames = ndjson_frames(&output);
    let footer = frames.last().expect("clean extraction has a footer");
    assert!(footer.get("extraction_quality").is_some());
    assert!(footer.get("errors").is_some(), "footer errors is required");
    assert_eq!(footer["errors"], serde_json::json!([]));
}
