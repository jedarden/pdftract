//! End-to-end contract tests for diagnostic arrays on JSON and NDJSON output.
//!
//! These tests assemble results at the same typed boundary used by the
//! extraction pipeline, then exercise the public output serializers.  This
//! keeps the assertions deterministic while covering the complete wire path:
//! legacy metadata strings, structured diagnostics, full JSON `errors`, and
//! NDJSON page/footer frames.

use pdftract_core::diagnostics::{DiagCode, Diagnostic, ObjRef};
use pdftract_core::diagnostics_compat::to_legacy_strings;
use pdftract_core::extract::{result_to_json, ExtractionMetadata, ExtractionResult, PageResult};
use pdftract_core::options::ReceiptsMode;
use pdftract_core::output::json::result_to_output;
use pdftract_core::output::ndjson::{
    footer_errors, write_frame, FooterFrame, HeaderFrame, NdjsonFrame, PageFrame,
};
use pdftract_core::schema::{DiagnosticJson, ExtractionQuality};
use serde_json::{json, Value};

fn page(index: usize, error: Option<&str>) -> PageResult {
    PageResult {
        index,
        page_number: (index + 1) as u32,
        page_label: None,
        width: None,
        height: None,
        rotation: None,
        page_type: None,
        spans: vec![],
        blocks: vec![],
        tables: vec![],
        annotations: vec![],
        error: error.map(str::to_owned),
    }
}

fn result_with(diagnostics: &[Diagnostic], pages: Vec<PageResult>) -> ExtractionResult {
    let error_count = pages.iter().filter(|page| page.error.is_some()).count();
    let page_count = pages.len();

    ExtractionResult {
        fingerprint: "diagnostics-array-contract".to_string(),
        pages,
        metadata: ExtractionMetadata {
            page_count,
            receipts_mode: ReceiptsMode::Off,
            span_count: 0,
            block_count: 0,
            cache_status: None,
            cache_age_seconds: None,
            error_count,
            reading_order_algorithm: None,
            diagnostics: to_legacy_strings(diagnostics),
            diagnostics_detailed: diagnostics.iter().map(DiagnosticJson::from).collect(),
            profile_name: None,
            profile_version: None,
            profile_fields: None,
        },
        signatures: vec![],
        form_fields: vec![],
        links: vec![],
        attachments: vec![],
        threads: vec![],
        javascript_actions: vec![],
    }
}

fn page_frame(page: &PageResult) -> PageFrame {
    let page_type = if page.spans.is_empty() && page.blocks.is_empty() {
        "blank"
    } else {
        "content"
    };
    let frame = PageFrame::new(
        page.index,
        page_type.to_string(),
        page.spans.clone(),
        page.blocks.clone(),
        page.tables.clone(),
    );

    match &page.error {
        Some(error) => frame.with_errors(vec![json!({
            "code": "page_extraction_error",
            "severity": "error",
            "message": error,
        })]),
        None => frame,
    }
}

fn serialized_frames(frames: &[NdjsonFrame]) -> Vec<Value> {
    let mut bytes = Vec::new();
    for frame in frames {
        write_frame(&mut bytes, frame).expect("NDJSON frame must serialize");
    }

    let lines: Vec<&[u8]> = bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .collect();
    assert_eq!(
        lines.len(),
        frames.len(),
        "one JSON line must be emitted per frame"
    );
    lines
        .into_iter()
        .map(|line| serde_json::from_slice(line).expect("NDJSON line must be valid JSON"))
        .collect()
}

fn serialized_footer(result: &ExtractionResult) -> Value {
    serialized_frames(&[NdjsonFrame::Footer(FooterFrame::new(
        ExtractionQuality::new(),
        footer_errors(result).expect("footer diagnostics must serialize"),
    ))])[0]
        .clone()
}

fn assert_legacy_structured_byte_parity(result: &ExtractionResult) {
    let legacy = &result.metadata.diagnostics;
    let structured = &result.metadata.diagnostics_detailed;
    assert_eq!(legacy.len(), structured.len());

    for (index, (legacy, structured)) in legacy.iter().zip(structured).enumerate() {
        assert_eq!(
            legacy.as_bytes(),
            structured.message.as_bytes(),
            "diagnostic {index} changed bytes between legacy and structured messages"
        );
    }
}

#[test]
fn empty_diagnostics_omit_metadata_arrays_but_keep_stable_error_arrays() {
    let result = result_with(&[], vec![]);

    let compact = result_to_json(&result);
    for field in ["diagnostics", "diagnostics_detailed"] {
        assert!(
            compact["metadata"].get(field).is_none(),
            "empty metadata.{field} must be omitted from compact JSON"
        );
    }
    let metadata = serde_json::to_value(&result.metadata).expect("metadata must serialize");
    for field in ["diagnostics", "diagnostics_detailed"] {
        assert!(
            metadata.get(field).is_none(),
            "empty metadata.{field} must be omitted when metadata is serialized directly"
        );
    }

    let full = serde_json::to_value(result_to_output(&result)).expect("full JSON must serialize");
    assert_eq!(full["errors"], json!([]));

    let footer = serialized_footer(&result);
    assert_eq!(footer["frame"], "footer");
    assert_eq!(footer["errors"], json!([]));
}

#[test]
fn document_diagnostics_reach_full_json_and_ndjson_in_emission_order() {
    let diagnostics = vec![
        Diagnostic::with_static_no_offset(DiagCode::XrefRepaired, "xref repaired"),
        Diagnostic::with_static_no_offset(
            DiagCode::LayoutTaggedPdfDeferred,
            "tagged document deferred",
        ),
    ];
    let result = result_with(&diagnostics, vec![]);
    assert_legacy_structured_byte_parity(&result);

    let compact = result_to_json(&result);
    assert_eq!(
        compact["metadata"]["diagnostics"],
        json!(["xref repaired", "tagged document deferred"])
    );
    assert_eq!(
        compact["metadata"]["diagnostics_detailed"],
        serde_json::to_value(&result.metadata.diagnostics_detailed).unwrap()
    );

    let full = serde_json::to_value(result_to_output(&result)).expect("full JSON must serialize");
    assert_eq!(
        full["errors"],
        serde_json::to_value(&result.metadata.diagnostics_detailed).unwrap()
    );

    let footer = serialized_footer(&result);
    assert_eq!(footer["frame"], "footer");
    assert_eq!(footer["errors"], full["errors"]);
    assert_eq!(
        footer["errors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["code"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["XREF_REPAIRED", "TAGGED_PDF_STRUCT_TREE_DEFERRED"]
    );
}

#[test]
fn duplicate_messages_preserve_order_and_bytes_on_every_surface() {
    let diagnostics = vec![
        Diagnostic::with_static_no_offset(DiagCode::XrefRepaired, "same diagnostic bytes"),
        Diagnostic::with_static(DiagCode::StreamDecodeError, 17, "same diagnostic bytes")
            .with_object_ref(ObjRef::new(12, 3))
            .with_page_index(4),
    ];
    let result = result_with(&diagnostics, vec![]);
    assert_legacy_structured_byte_parity(&result);
    assert_eq!(
        result.metadata.diagnostics,
        vec!["same diagnostic bytes", "same diagnostic bytes"]
    );
    assert_ne!(
        result.metadata.diagnostics_detailed[0].code, result.metadata.diagnostics_detailed[1].code,
        "the structured array must not deduplicate equal legacy messages"
    );

    let full = serde_json::to_value(result_to_output(&result)).expect("full JSON must serialize");
    assert_eq!(full["errors"].as_array().unwrap().len(), 2);
    assert_eq!(full["errors"][0]["message"], full["errors"][1]["message"]);
    assert_eq!(full["errors"][1]["page_index"], 4);
    assert_eq!(full["errors"][1]["location"]["object_number"], 12);

    let footer = serialized_footer(&result);
    assert_eq!(footer["errors"], full["errors"]);
    assert_legacy_structured_byte_parity(&result);
}

#[test]
fn page_failures_emit_synthetic_ndjson_records_before_document_diagnostics() {
    let diagnostics = vec![Diagnostic::with_static_no_offset(
        DiagCode::XrefRepaired,
        "document diagnostic",
    )];
    let pages = vec![
        page(0, None),
        page(1, Some("page one failed")),
        page(2, Some("page two failed")),
    ];
    let result = result_with(&diagnostics, pages.clone());

    // Full JSON deliberately carries document diagnostics only; page failures
    // are represented by the NDJSON synthetic records below.
    let full = serde_json::to_value(result_to_output(&result)).expect("full JSON must serialize");
    assert_eq!(full["errors"].as_array().unwrap().len(), 1);
    assert_eq!(full["errors"][0]["code"], "XREF_REPAIRED");
    assert!(!full["errors"].to_string().contains("page_extraction_error"));

    let header = HeaderFrame::new(
        "1.0".to_string(),
        json!({"page_count": pages.len()}),
        None,
        pages.len(),
    );
    let footer = FooterFrame::new(
        ExtractionQuality::new(),
        footer_errors(&result).expect("footer diagnostics must serialize"),
    );
    let frames = vec![
        NdjsonFrame::Header(header),
        NdjsonFrame::Page(page_frame(&pages[0])),
        NdjsonFrame::Page(page_frame(&pages[1])),
        NdjsonFrame::Page(page_frame(&pages[2])),
        NdjsonFrame::Footer(footer),
    ];
    let lines = serialized_frames(&frames);

    assert_eq!(
        lines
            .iter()
            .map(|line| line["frame"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["header", "page", "page", "page", "footer"]
    );
    assert!(lines[1].get("errors").is_none());
    assert_eq!(
        lines[2]["errors"][0],
        json!({
            "code": "page_extraction_error",
            "severity": "error",
            "message": "page one failed",
        })
    );
    assert_eq!(lines[3]["errors"][0]["message"], "page two failed");

    let footer_errors = lines.last().unwrap()["errors"].as_array().unwrap();
    assert_eq!(footer_errors.len(), 3);
    assert_eq!(footer_errors[0]["code"], "page_extraction_error");
    assert_eq!(footer_errors[0]["message"], "page one failed");
    assert_eq!(footer_errors[1]["code"], "page_extraction_error");
    assert_eq!(footer_errors[1]["message"], "page two failed");
    assert_eq!(footer_errors[2], full["errors"][0]);
}
