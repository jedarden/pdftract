//! End-to-end contract coverage for diagnostic JSON and NDJSON surfaces.
//!
//! The diagnostics contract has two compatibility layers: the legacy
//! message-only metadata array and the structured `DiagnosticJson` array. The
//! full JSON output and NDJSON footer expose the structured entries, while an
//! NDJSON footer prepends one synthetic entry for every failed page. Keep all
//! of those relationships pinned together here.

use pdftract_core::diagnostics::{DiagCode, Diagnostic, ObjRef};
use pdftract_core::diagnostics_compat::to_legacy_strings;
use pdftract_core::extract::{ExtractionMetadata, ExtractionResult, PageResult};
use pdftract_core::options::ReceiptsMode;
use pdftract_core::output::json::result_to_output;
use pdftract_core::output::ndjson::footer_errors;
use pdftract_core::output::ndjson::frames::{FooterFrame, NdjsonFrame, PageFrame};
use pdftract_core::schema::DiagnosticJson;

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

fn populated_result() -> ExtractionResult {
    // The final three entries deliberately share message bytes. Their
    // structured codes and contexts remain distinct, so this catches both
    // accidental deduplication and accidental reordering of either surface.
    let diagnostics = vec![
        Diagnostic::with_static_no_offset(DiagCode::XrefRepaired, "xref repaired"),
        Diagnostic::with_static(DiagCode::StreamDecodeError, 1234, "duplicate message")
            .with_object_ref(ObjRef::new(42, 7))
            .with_page_index(1),
        Diagnostic::with_static_no_offset(DiagCode::StructInvalidName, "duplicate message"),
        Diagnostic::with_static_no_offset(DiagCode::StreamBomb, "duplicate message"),
    ];
    let diagnostics_detailed: Vec<DiagnosticJson> = diagnostics.iter().map(Into::into).collect();

    ExtractionResult {
        fingerprint: "diagnostics-json-ndjson-contract".to_owned(),
        pages: vec![
            page(0, None),
            page(1, Some("first page failed")),
            page(2, Some("second page failed")),
        ],
        metadata: ExtractionMetadata {
            page_count: 3,
            receipts_mode: ReceiptsMode::Off,
            span_count: 0,
            block_count: 0,
            cache_status: None,
            cache_age_seconds: None,
            error_count: 2,
            reading_order_algorithm: None,
            diagnostics: to_legacy_strings(&diagnostics),
            diagnostics_detailed,
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

fn empty_result() -> ExtractionResult {
    let mut result = populated_result();
    result.pages.clear();
    result.metadata.page_count = 0;
    result.metadata.error_count = 0;
    result.metadata.diagnostics.clear();
    result.metadata.diagnostics_detailed.clear();
    result
}

fn assert_no_nulls(value: &serde_json::Value, context: &str) {
    match value {
        serde_json::Value::Null => panic!("{context} contains an unexpected null"),
        serde_json::Value::Array(values) => {
            for value in values {
                assert_no_nulls(value, context);
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values() {
                assert_no_nulls(value, context);
            }
        }
        _ => {}
    }
}

#[test]
fn populated_diagnostics_keep_legacy_and_structured_order_across_outputs() {
    let result = populated_result();
    let expected_detailed: Vec<serde_json::Value> = result
        .metadata
        .diagnostics_detailed
        .iter()
        .map(|diagnostic| serde_json::to_value(diagnostic).unwrap())
        .collect();

    // Compact JSON carries both metadata arrays. They must be one-to-one,
    // message-verbatim, and preserve duplicate entries.
    let compact = pdftract_core::extract::result_to_json(&result);
    let metadata = compact["metadata"].as_object().expect("metadata object");
    assert_eq!(
        metadata["diagnostics"],
        serde_json::json!([
            "xref repaired",
            "duplicate message",
            "duplicate message",
            "duplicate message"
        ])
    );
    assert_eq!(
        metadata["diagnostics_detailed"],
        serde_json::Value::Array(expected_detailed.clone())
    );
    for (legacy, detailed) in result
        .metadata
        .diagnostics
        .iter()
        .zip(&result.metadata.diagnostics_detailed)
    {
        assert_eq!(legacy, &detailed.message);
    }
    assert_eq!(
        result
            .metadata
            .diagnostics_detailed
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        vec![
            "XREF_REPAIRED",
            "STREAM_DECODE_ERROR",
            "STRUCT_INVALID_NAME",
            "STREAM_BOMB",
        ]
    );

    // Full JSON exposes exactly the same structured sequence at top level.
    let full = serde_json::to_value(result_to_output(&result)).expect("full JSON serializes");
    assert_eq!(
        full["errors"],
        serde_json::Value::Array(expected_detailed.clone())
    );

    // The NDJSON footer is a union: failed-page records first, then the
    // unchanged document-level structured diagnostics in emission order.
    let footer_values = footer_errors(&result).expect("footer errors assemble");
    assert_eq!(footer_values.len(), 2 + expected_detailed.len());
    for (entry, message) in footer_values[..2]
        .iter()
        .zip(["first page failed", "second page failed"])
    {
        assert_eq!(entry["code"], "page_extraction_error");
        assert_eq!(entry["severity"], "error");
        assert_eq!(entry["message"], message);
        assert!(entry.get("page_index").is_none());
        assert!(entry.get("location").is_none());
        assert!(entry.get("hint").is_none());
    }
    assert_eq!(&footer_values[2..], expected_detailed.as_slice());

    let footer = FooterFrame::new(
        pdftract_core::schema::ExtractionQuality::new().with_quality("medium"),
        footer_values.clone(),
    );
    let mut footer_line =
        serde_json::to_string(&NdjsonFrame::Footer(footer)).expect("footer frame serializes");
    footer_line.push('\n');
    assert!(footer_line.ends_with('\n'));
    let decoded: NdjsonFrame = serde_json::from_str(footer_line.trim_end())
        .expect("serialized footer is a valid NDJSON frame");
    let NdjsonFrame::Footer(decoded) = decoded else {
        panic!("serialized union must decode as a footer frame");
    };
    assert_eq!(decoded.errors, footer_values);
}

#[test]
fn diagnostic_field_presence_and_omission_rules_hold_on_json_and_ndjson() {
    let result = populated_result();
    let compact = pdftract_core::extract::result_to_json(&result);
    let full = serde_json::to_value(result_to_output(&result)).expect("full JSON serializes");
    let footer = footer_errors(&result).expect("footer errors assemble");

    // A context-rich diagnostic retains every optional field on all
    // structured output surfaces, while byte_offset remains in-process-only.
    for (surface, value) in [
        ("compact", &compact["metadata"]["diagnostics_detailed"][1]),
        ("full", &full["errors"][1]),
        ("footer", &footer[3]),
    ] {
        assert_no_nulls(value, surface);
        assert_eq!(value["code"], "STREAM_DECODE_ERROR");
        assert_eq!(value["message"], "duplicate message");
        assert_eq!(value["severity"], "warning");
        assert_eq!(value["page_index"], 1);
        assert_eq!(value["location"]["object_number"], 42);
        assert_eq!(value["location"]["generation_number"], 7);
        assert!(value.get("hint").is_some());
        assert!(value.get("byte_offset").is_none());
    }

    // A document-level diagnostic omits absent optional fields instead of
    // serializing them as null. Required fields remain present.
    let bare = DiagnosticJson {
        code: "XREF_REPAIRED".to_owned(),
        message: "xref repaired".to_owned(),
        severity: "info".to_owned(),
        page_index: None,
        location: None,
        hint: None,
    };
    let bare_value = serde_json::to_value(&bare).expect("bare diagnostic serializes");
    assert_no_nulls(&bare_value, "bare diagnostic");
    for field in ["code", "message", "severity"] {
        assert!(
            bare_value.get(field).is_some(),
            "required field {field} missing"
        );
    }
    for field in ["page_index", "location", "hint"] {
        assert!(
            bare_value.get(field).is_none(),
            "optional field {field} was not omitted"
        );
    }
}

#[test]
fn clean_outputs_omit_metadata_arrays_but_keep_stable_error_arrays() {
    let result = empty_result();

    let compact = pdftract_core::extract::result_to_json(&result);
    let metadata = compact["metadata"].as_object().expect("metadata object");
    assert!(metadata.get("diagnostics").is_none());
    assert!(metadata.get("diagnostics_detailed").is_none());

    let serialized_metadata = serde_json::to_value(&result.metadata).unwrap();
    assert!(serialized_metadata.get("diagnostics").is_none());
    assert!(serialized_metadata.get("diagnostics_detailed").is_none());

    let full = serde_json::to_value(result_to_output(&result)).expect("full JSON serializes");
    assert_eq!(full.get("errors"), Some(&serde_json::json!([])));

    let footer_values = footer_errors(&result).expect("clean footer errors assemble");
    assert!(footer_values.is_empty());
    let footer = FooterFrame::new(
        pdftract_core::schema::ExtractionQuality::new().with_quality("high"),
        footer_values,
    );
    let footer_value = serde_json::to_value(&footer).expect("clean footer serializes");
    assert_eq!(footer_value.get("errors"), Some(&serde_json::json!([])));

    let page = PageFrame::new(0, "blank".to_owned(), vec![], vec![], vec![]);
    let page_value = serde_json::to_value(page).expect("clean page serializes");
    assert!(
        page_value.get("errors").is_none(),
        "successful page frames omit errors rather than emitting an empty array"
    );
}

#[test]
fn synthetic_page_error_is_present_only_on_failed_ndjson_page_frames() {
    let result = populated_result();
    let footer = footer_errors(&result).expect("footer errors assemble");
    let synthetic = footer[0].clone();

    let failed = PageFrame::new(1, "blank".to_owned(), vec![], vec![], vec![])
        .with_errors(vec![synthetic.clone()]);
    let failed_value = serde_json::to_value(&failed).expect("failed page serializes");
    assert_eq!(failed_value["errors"], serde_json::json!([synthetic]));

    let succeeded = PageFrame::new(0, "blank".to_owned(), vec![], vec![], vec![]);
    let succeeded_value = serde_json::to_value(&succeeded).expect("successful page serializes");
    assert!(succeeded_value.get("errors").is_none());
}
