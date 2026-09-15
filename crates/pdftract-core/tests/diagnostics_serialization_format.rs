//! Pin the serialized shape of structured diagnostics against the two spec
//! documents: `docs/integrations/diagnostics-codes.md` (field rule and
//! severity enum) and `docs/errors-array-format.md` (surfaces and their
//! empty-array behavior).
//!
//! The canonical rules under test:
//!
//! 1. `code`, `message`, and `severity` are always present on a serialized
//!    [`DiagnosticJson`]; `page_index`, `location`, and `hint` are
//!    **omitted, never `null`**, when they do not apply.
//! 2. Severity values are exactly the documented enum
//!    `info | warning | error | fatal`, and every catalog entry's severity
//!    and suggested action can populate the envelope without leaving that
//!    enum or producing an empty hint source.
//! 3. Empty-array behavior differs by surface: the compact extraction JSON
//!    omits `metadata.diagnostics` entirely when empty, while the full
//!    output's top-level `errors` array and the NDJSON footer's `errors`
//!    array are stable schema fields — always present, `[]` when clean. An
//!    NDJSON page frame carries `errors` only when that page failed.
//!
//! These tests pin the serialization contract itself (`DiagnosticJson`'s
//! serde attributes and the output frames), not any particular conversion
//! into it. The string↔structured mirror invariant across output surfaces
//! is covered by its own test work; the extraction pipeline that feeds the
//! full output's `errors` array is exercised by the ndjson pipeline tests.

use std::collections::BTreeSet;
use std::fs;

use pdftract_core::diagnostics::{DiagCode, Diagnostic, DIAGNOSTIC_CATALOG};
use pdftract_core::extract::{result_to_json, ExtractionResult};
use pdftract_core::output::json::result_to_output;
use pdftract_core::output::ndjson::frames::{FooterFrame, PageFrame};
use pdftract_core::schema::{DiagnosticJson, ObjectLocationJson};

const DIAGNOSTICS_DOC: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/integrations/diagnostics-codes.md"
);

/// A minimal result with no diagnostics and no content, for the empty and
/// populated serialization cases below.
///
/// Built through `Deserialize` with the full metadata field set (including
/// `diagnostics_detailed`, which the metadata carries alongside
/// `diagnostics` per `docs/errors-array-format.md`) so this file keeps
/// compiling — and keeps pinning the same contract — as the metadata struct
/// gains the structured array.
fn empty_result() -> ExtractionResult {
    serde_json::from_value(serde_json::json!({
        "fingerprint": "diagnostics-serialization-format",
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

/// Build the envelope for a catalog entry exactly the way the documented
/// conversion does: severity from the typed code, hint from the catalog
/// entry's suggested action.
fn envelope_for(info: &pdftract_core::diagnostics::DiagInfo) -> DiagnosticJson {
    DiagnosticJson {
        code: info.code.name().to_string(),
        message: "test diagnostic".to_string(),
        severity: info.code.severity().to_string(),
        page_index: None,
        location: None,
        hint: Some(info.suggested_action.to_string()),
    }
}

/// Fail if any `null` appears anywhere in a serialized diagnostic — absent
/// optional fields must be omitted, not `null` (see
/// `docs/integrations/diagnostics-codes.md`, "Diagnostic Format").
fn assert_no_nulls(value: &serde_json::Value) {
    match value {
        serde_json::Value::Null => panic!("serialized diagnostic contains null: {value}"),
        serde_json::Value::Array(items) => items.iter().for_each(assert_no_nulls),
        serde_json::Value::Object(map) => map.values().for_each(assert_no_nulls),
        _ => {}
    }
}

#[test]
fn optional_fields_are_omitted_not_null() {
    // Every optional field populated: all six keys serialize, none as null.
    let full = DiagnosticJson {
        code: "STREAM_DECODE_ERROR".to_string(),
        message: "zlib stream truncated mid-inflation".to_string(),
        severity: "warning".to_string(),
        page_index: Some(3),
        location: Some(ObjectLocationJson {
            object_number: 12,
            generation_number: 0,
        }),
        hint: Some("Inspect the source PDF for corrupt stream data".to_string()),
    };
    let value = serde_json::to_value(&full).unwrap();
    assert_no_nulls(&value);
    assert_eq!(value["code"], "STREAM_DECODE_ERROR");
    assert_eq!(value["page_index"], 3, "page_index serializes as a number");
    assert_eq!(
        value["location"]["object_number"], 12,
        "location serializes as an object when known"
    );
    assert_eq!(value["location"]["generation_number"], 0);
    assert_eq!(value["hint"], "Inspect the source PDF for corrupt stream data");

    // A document-level diagnostic with no location and no hint: the optional
    // keys must be absent entirely, never `"key": null`.
    let bare = DiagnosticJson {
        code: "XREF_REPAIRED".to_string(),
        message: "Xref was reconstructed via forward scan".to_string(),
        severity: "info".to_string(),
        page_index: None,
        location: None,
        hint: None,
    };
    let value = serde_json::to_value(&bare).unwrap();
    assert_no_nulls(&value);
    for optional in ["page_index", "location", "hint"] {
        assert!(
            value.get(optional).is_none(),
            "`{optional}` must be omitted (not null) when absent, got: {value}"
        );
    }
    for always_present in ["code", "message", "severity"] {
        assert!(
            value.get(always_present).is_some(),
            "`{always_present}` must always be present, got: {value}"
        );
    }
}

#[test]
fn catalog_entries_populate_the_documented_envelope() {
    // `hint` is omitted when the catalog entry carries no suggested action;
    // every entry in DIAGNOSTIC_CATALOG currently carries one, so each code
    // must yield a populated hint — and no entry may carry an empty action
    // that would serialize as an empty string.
    for info in DIAGNOSTIC_CATALOG {
        assert!(
            !info.suggested_action.trim().is_empty(),
            "{}: catalog action must be non-empty (omit the hint instead of \
             serializing an empty string)",
            info.code.name()
        );
        let value = serde_json::to_value(&envelope_for(info)).unwrap();
        assert_no_nulls(&value);
        assert_eq!(
            value["hint"], info.suggested_action,
            "{}: hint must come from the catalog entry",
            info.code.name()
        );
    }
}

#[test]
fn emitted_severities_match_documented_enum() {
    let doc = fs::read_to_string(DIAGNOSTICS_DOC).unwrap();
    // The format block declares the enum on a single line:
    //   "severity": "info|warning|error|fatal",
    let line = doc
        .lines()
        .find(|l| l.trim().starts_with("\"severity\":"))
        .expect("diagnostics-codes.md format block must declare the severity enum");
    let documented: BTreeSet<String> = line
        .rsplit('"')
        .nth(1)
        .expect("severity enum line must quote the enum values")
        .split('|')
        .map(str::to_string)
        .collect();
    assert!(
        documented.contains("fatal"),
        "the documented enum must include every severity level, got: {documented:?}"
    );

    let mut emitted = BTreeSet::new();
    for info in DIAGNOSTIC_CATALOG {
        // The typed classification is the sole severity source.
        assert_eq!(
            info.code.severity().to_string(),
            info.severity.to_string(),
            "{}: catalog severity disagrees with DiagCode::severity()",
            info.code.name()
        );
        let value = serde_json::to_value(&envelope_for(info)).unwrap();
        let severity = value["severity"]
            .as_str()
            .unwrap_or_else(|| panic!("{}: severity must serialize as a string, got: {value}", info.code.name()));
        assert!(
            documented.contains(severity),
            "{}: emitted severity {severity:?} is outside the documented enum {documented:?}",
            info.code.name()
        );
        emitted.insert(severity.to_string());
    }

    assert_eq!(
        emitted, documented,
        "the emitted severity set must equal the documented enum — update the \
         format block in diagnostics-codes.md when a severity level is added \
         or retired"
    );
}

#[test]
fn compact_json_omits_metadata_diagnostics_when_empty() {
    let metadata = result_to_json(&empty_result())["metadata"].clone();
    assert!(
        metadata.get("diagnostics").is_none(),
        "`metadata.diagnostics` must be omitted entirely from the compact \
         extraction JSON when empty (docs/errors-array-format.md), got: {metadata}"
    );

    // Populating the array republishes it.
    let mut result = empty_result();
    let diag = Diagnostic::with_static(
        DiagCode::StreamDecodeError,
        42,
        "zlib stream truncated mid-inflation",
    );
    result.metadata.diagnostics.push(diag.to_string());

    let metadata = result_to_json(&result)["metadata"].clone();
    let strings = metadata["diagnostics"].as_array().unwrap();
    assert_eq!(strings.len(), 1);
    assert!(
        strings[0].as_str().unwrap().starts_with("STREAM_DECODE_ERROR: "),
        "the string form must carry the documented CODE: prefix, got: {}",
        strings[0]
    );
}

#[test]
fn full_output_errors_array_is_always_present() {
    // The top-level `errors` array is a stable schema field of the v1.0
    // output: present even for a clean extraction, `[]` when empty — never
    // omitted. Which metadata array feeds it (the legacy string array or the
    // structured one) is the extraction pipeline's concern and is covered by
    // the output-flow tests; the contract pinned here is the documented
    // empty-array behavior of the serialized field.
    let clean = result_to_output(&empty_result());
    let value = serde_json::to_value(&clean).unwrap();
    let errors = value
        .get("errors")
        .expect("top-level `errors` must be present in the full JSON output even when clean");
    assert!(
        errors.as_array().expect("`errors` must be an array").is_empty(),
        "a clean extraction must serialize an empty `errors` array, got: {errors}"
    );
}

#[test]
fn footer_frame_errors_are_always_present() {
    let footer = FooterFrame::new(
        pdftract_core::schema::ExtractionQuality::new(),
        vec![],
    );
    let value = serde_json::to_value(&footer).unwrap();
    let errors = value
        .get("errors")
        .expect("the NDJSON footer must carry an `errors` array even when clean");
    assert!(
        errors.as_array().expect("footer `errors` must be an array").is_empty(),
        "a clean footer must serialize an empty `errors` array, got: {errors}"
    );
}

#[test]
fn page_frame_omits_errors_when_page_succeeded() {
    let frame = PageFrame::new(0, "blank".to_string(), vec![], vec![], vec![]);
    let value = serde_json::to_value(&frame).unwrap();
    assert!(
        value.get("errors").is_none(),
        "a page frame must omit `errors` entirely when the page did not fail, got: {value}"
    );
}
