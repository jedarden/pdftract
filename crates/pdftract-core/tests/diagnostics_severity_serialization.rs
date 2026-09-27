//! Focused wire-contract coverage for structured diagnostics.
//!
//! The compact extraction metadata keeps its legacy message array, while the
//! full JSON output and NDJSON footer carry the same structured objects. This
//! test exercises every documented severity and both complete and absent
//! optional context through all three public serialization paths.

use pdftract_core::diagnostics::{DiagCode, Diagnostic, ObjRef};
use pdftract_core::diagnostics_compat::to_legacy_string;
use pdftract_core::extract::{result_to_json, ExtractionResult};
use pdftract_core::output::json::result_to_output;
use pdftract_core::output::ndjson::footer_errors;
use pdftract_core::schema::DiagnosticJson;

fn empty_result() -> ExtractionResult {
    serde_json::from_value(serde_json::json!({
        "fingerprint": "diagnostics-severity-serialization",
        "pages": [],
        "metadata": {
            "page_count": 0,
            "receipts_mode": "off",
            "span_count": 0,
            "block_count": 0,
            "cache_status": null,
            "cache_age_seconds": null,
            "error_count": 0,
            "reading_order_algorithm": null,
            "diagnostics": [],
            "diagnostics_detailed": [],
            "profile_name": null,
            "profile_version": null,
            "profile_fields": null
        },
        "signatures": [],
        "form_fields": [],
        "links": [],
        "attachments": [],
        "threads": [],
        "javascript_actions": []
    }))
    .expect("test fixture must deserialize into ExtractionResult")
}

fn representative_diagnostics() -> Vec<Diagnostic> {
    vec![
        // Document-level context is intentionally absent for info and fatal.
        Diagnostic::with_static_no_offset(DiagCode::XrefRepaired, "xref repaired"),
        Diagnostic::with_static_no_offset(DiagCode::StructInvalidName, "invalid name")
            .with_page_index(2),
        Diagnostic::with_dynamic_no_offset(
            DiagCode::PageOutOfRange,
            "page 8 is outside the document".to_string(),
        )
        .with_page_index(7),
        Diagnostic::with_static_no_offset(
            DiagCode::EncryptionUnsupported,
            "encrypted document cannot be opened",
        ),
    ]
}

#[test]
fn all_severities_round_trip_and_preserve_optional_context() {
    let typed = representative_diagnostics();
    let expected = [
        ("XREF_REPAIRED", "info"),
        ("STRUCT_INVALID_NAME", "warning"),
        ("PAGE_OUT_OF_RANGE", "error"),
        ("ENCRYPTION_UNSUPPORTED", "fatal"),
    ];

    for (diagnostic, (code, severity)) in typed.iter().zip(expected) {
        let structured = DiagnosticJson::from(diagnostic);
        assert_eq!(structured.code, code);
        assert_eq!(structured.severity, severity);

        let wire = serde_json::to_value(&structured).expect("diagnostic must serialize");
        let decoded: DiagnosticJson =
            serde_json::from_value(wire.clone()).expect("diagnostic must deserialize");
        assert_eq!(decoded, structured, "{code} lost fields during round trip");

        assert_eq!(to_legacy_string(diagnostic), diagnostic.message.as_ref());
        assert!(wire.get("code").is_some());
        assert!(wire.get("message").is_some());
        assert!(wire.get("severity").is_some());
        if diagnostic.page_index.is_none() && diagnostic.object_ref.is_none() {
            assert!(wire.get("page_index").is_none());
            assert!(wire.get("location").is_none());
        }
    }

    let complete = Diagnostic::with_dynamic(
        DiagCode::StreamDecodeError,
        4096,
        "stream truncated".to_string(),
    )
    .with_object_ref(ObjRef::new(12, 3))
    .with_page_index(4);
    let complete_wire = serde_json::to_value(DiagnosticJson::from(&complete)).unwrap();
    assert_eq!(complete_wire["page_index"], 4);
    assert_eq!(complete_wire["location"]["object_number"], 12);
    assert_eq!(complete_wire["location"]["generation_number"], 3);
}

#[test]
fn structured_diagnostics_are_mirrored_in_compact_full_and_ndjson_outputs() {
    let typed = representative_diagnostics();
    let structured: Vec<DiagnosticJson> = typed.iter().map(DiagnosticJson::from).collect();
    let legacy: Vec<String> = typed.iter().map(to_legacy_string).collect();

    let mut result = empty_result();
    result.metadata.diagnostics = legacy.clone();
    result.metadata.diagnostics_detailed = structured.clone();

    let compact = result_to_json(&result);
    assert_eq!(
        compact["metadata"]["diagnostics"],
        serde_json::json!(legacy)
    );
    assert_eq!(
        compact["metadata"]["diagnostics_detailed"],
        serde_json::to_value(&structured).unwrap()
    );

    let full = serde_json::to_value(result_to_output(&result)).unwrap();
    assert_eq!(full["errors"], serde_json::to_value(&structured).unwrap());

    let footer = footer_errors(&result).expect("NDJSON footer diagnostics must serialize");
    assert_eq!(
        serde_json::to_value(&footer).unwrap(),
        serde_json::to_value(&structured).unwrap()
    );
}
