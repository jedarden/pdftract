//! End-to-end serialization contract for the structured diagnostic surfaces.
//!
//! This test deliberately builds an extraction result at the same typed
//! boundary used by the extraction pipeline. It verifies that a diagnostic's
//! code, severity, page, object location, and catalog hint survive compact
//! JSON, full JSON, and NDJSON footer serialization while the legacy string
//! metadata entry remains available.

use pdftract_core::diagnostics::{DiagCode, Diagnostic, ObjRef, DIAGNOSTIC_CATALOG};
use pdftract_core::diagnostics_compat::to_legacy_string;
use pdftract_core::extract::{result_to_json, ExtractionMetadata, ExtractionResult};
use pdftract_core::options::ReceiptsMode;
use pdftract_core::output::json::result_to_output;
use pdftract_core::output::ndjson::footer_errors;
use pdftract_core::schema::DiagnosticJson;

fn representative_result() -> ExtractionResult {
    let diagnostic = Diagnostic::with_static(
        DiagCode::StreamDecodeError,
        1234,
        "zlib stream truncated mid-inflation",
    )
    .with_object_ref(ObjRef::new(42, 7))
    .with_page_index(3);

    ExtractionResult {
        fingerprint: "diagnostics-output-contract".to_string(),
        pages: vec![],
        metadata: ExtractionMetadata {
            page_count: 1,
            receipts_mode: ReceiptsMode::Off,
            span_count: 0,
            block_count: 0,
            cache_status: None,
            cache_age_seconds: None,
            error_count: 0,
            reading_order_algorithm: None,
            diagnostics: vec![to_legacy_string(&diagnostic)],
            diagnostics_detailed: vec![DiagnosticJson::from(&diagnostic)],
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

fn assert_representative(value: &serde_json::Value) {
    assert_eq!(value["code"], "STREAM_DECODE_ERROR");
    assert_eq!(value["severity"], "warning");
    assert_eq!(value["page_index"], 3);
    assert_eq!(value["location"]["object_number"], 42);
    assert_eq!(value["location"]["generation_number"], 7);
    assert_eq!(
        value["hint"],
        "Partial output returned for this stream; consider re-saving the PDF through a normalising tool"
    );
}

#[test]
fn representative_diagnostic_survives_json_and_ndjson_contracts() {
    let result = representative_result();

    let compact = result_to_json(&result);
    assert_eq!(
        compact["metadata"]["diagnostics"][0], "zlib stream truncated mid-inflation",
        "the legacy Vec<String> metadata surface remains message-compatible"
    );
    assert_representative(&compact["metadata"]["diagnostics_detailed"][0]);

    let full = serde_json::to_value(result_to_output(&result)).unwrap();
    assert_representative(&full["errors"][0]);

    let footer = footer_errors(&result).unwrap();
    assert_eq!(footer.len(), 1);
    assert_representative(&footer[0]);
}

#[test]
fn absent_diagnostic_context_is_omitted_from_all_structured_json() {
    let bare = DiagnosticJson {
        code: "XREF_REPAIRED".to_string(),
        message: "Xref was reconstructed via forward scan".to_string(),
        severity: "info".to_string(),
        page_index: None,
        location: None,
        hint: None,
    };
    let value = serde_json::to_value(bare).unwrap();

    for field in ["page_index", "location", "hint"] {
        assert!(
            value.get(field).is_none(),
            "optional diagnostic field {field:?} must be omitted, not null: {value}"
        );
    }
}

#[test]
fn every_documented_code_round_trips_through_public_outputs() {
    let diagnostics: Vec<_> = DIAGNOSTIC_CATALOG
        .iter()
        .map(|info| {
            Diagnostic::with_dynamic(
                info.code,
                4096,
                format!("round-trip message for {}", info.code.name()),
            )
            .with_object_ref(ObjRef::new(12, 3))
            .with_page_index(7)
        })
        .collect();

    assert_eq!(diagnostics.len(), DiagCode::ALL.len());
    assert!(diagnostics
        .iter()
        .all(|diagnostic| diagnostic.byte_offset == Some(4096)));

    let result = ExtractionResult {
        fingerprint: "all-diagnostics-output-contract".to_string(),
        pages: vec![],
        metadata: ExtractionMetadata {
            page_count: 1,
            receipts_mode: ReceiptsMode::Off,
            span_count: 0,
            block_count: 0,
            cache_status: None,
            cache_age_seconds: None,
            error_count: 0,
            reading_order_algorithm: None,
            diagnostics: diagnostics.iter().map(to_legacy_string).collect(),
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
    };

    let compact = result_to_json(&result);
    let compact_structured = compact["metadata"]["diagnostics_detailed"]
        .as_array()
        .expect("compact structured diagnostics array");
    let full = serde_json::to_value(result_to_output(&result)).unwrap();
    let full_errors = full["errors"].as_array().expect("full errors array");
    let footer = footer_errors(&result).unwrap();

    assert_eq!(compact_structured.len(), diagnostics.len());
    assert_eq!(full_errors.len(), diagnostics.len());
    assert_eq!(footer.len(), diagnostics.len());
    assert_eq!(
        compact["metadata"]["diagnostics"],
        serde_json::json!(diagnostics.iter().map(to_legacy_string).collect::<Vec<_>>())
    );

    for (index, (info, diagnostic)) in DIAGNOSTIC_CATALOG
        .iter()
        .zip(diagnostics.iter())
        .enumerate()
    {
        for value in [
            &compact_structured[index],
            &full_errors[index],
            &footer[index],
        ] {
            assert_eq!(value["code"], info.code.name());
            assert_eq!(value["message"], diagnostic.message.as_ref());
            assert_eq!(value["severity"], info.severity.to_string());
            assert_eq!(value["page_index"], 7);
            assert_eq!(value["location"]["object_number"], 12);
            assert_eq!(value["location"]["generation_number"], 3);
            assert_eq!(value["hint"], info.suggested_action);

            let decoded: DiagnosticJson = serde_json::from_value((*value).clone()).unwrap();
            assert_eq!(decoded.code, info.code.name());
            assert_eq!(decoded.message, diagnostic.message.as_ref());
            assert_eq!(decoded.severity, info.severity.to_string());
            assert_eq!(decoded.page_index, Some(7));
            assert_eq!(decoded.location.as_ref().unwrap().object_number, 12);
            assert_eq!(decoded.location.as_ref().unwrap().generation_number, 3);
            assert_eq!(decoded.hint.as_deref(), Some(info.suggested_action));
        }
    }
}
