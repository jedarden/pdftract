//! Exhaustive acceptance tests for the structured diagnostics contract.
//!
//! The catalog and the published diagnostics guide are the test matrix: every
//! documented code gets a context-rich JSON round trip, a context-free policy
//! fallback check, and a legacy/display compatibility check.  Keeping the
//! matrix data-driven makes adding a code fail loudly until its documentation,
//! catalog policy, and wire behavior are all updated together.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use pdftract_core::diagnostics::{
    DiagCode, Diagnostic, DiagnosticContext, DiagnosticsCollector, ObjRef, DIAGNOSTIC_CATALOG,
};
use pdftract_core::diagnostics_compat::to_legacy_strings;
use pdftract_core::extract::{result_to_json, ExtractionResult};
use pdftract_core::output::json::result_to_output;
use pdftract_core::output::ndjson::footer_errors;
use pdftract_core::output::ndjson::frames::{write_frame, FooterFrame, NdjsonFrame, PageFrame};
use pdftract_core::schema::{DiagnosticJson, ExtractionQuality, ObjectLocationJson};

const DIAGNOSTICS_DOC: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/integrations/diagnostics-codes.md"
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContextProfile {
    DocumentPdf,
    DocumentOperation,
    Page,
    Contextual,
}

impl ContextProfile {
    fn from_document(value: &str) -> Self {
        match value {
            "D-PDF" => Self::DocumentPdf,
            "D-OP" => Self::DocumentOperation,
            "P" => Self::Page,
            "C" => Self::Contextual,
            other => panic!("unknown diagnostic context profile {other:?}"),
        }
    }

    fn known_context(self) -> (Option<usize>, Option<ObjRef>) {
        match self {
            // The fixture represents a known originating object, but not a
            // page, for document-level events.
            Self::DocumentPdf => (None, Some(ObjRef::new(12, 3))),
            Self::DocumentOperation => (None, None),
            Self::Page | Self::Contextual => (Some(7), Some(ObjRef::new(12, 3))),
        }
    }
}

#[derive(Debug, Clone)]
struct DocumentedCode {
    name: String,
    severity: String,
    hint: String,
    profile: ContextProfile,
    required_feature: Option<String>,
}

fn documented_codes() -> BTreeMap<String, DocumentedCode> {
    let document = fs::read_to_string(DIAGNOSTICS_DOC)
        .unwrap_or_else(|error| panic!("cannot read {DIAGNOSTICS_DOC}: {error}"));
    let mut rows = BTreeMap::new();
    let mut hints = BTreeMap::new();
    let mut profiles = BTreeMap::new();
    let mut in_code_catalog = false;
    let mut in_catalog_hints = false;
    let mut in_inventory = false;

    for (line_number, line) in document.lines().enumerate() {
        if line.trim() == "## Code Categories" {
            in_code_catalog = true;
            in_catalog_hints = false;
            continue;
        }
        if line.trim() == "## Catalog Hints" {
            in_code_catalog = false;
            in_catalog_hints = true;
            continue;
        }
        if line.contains("The complete inventory below assigns every code") {
            in_inventory = true;
            continue;
        }
        if line.starts_with("Thus a child implementation") {
            in_inventory = false;
        }
        if line.starts_with("## ") && line.trim() != "## Code Categories" {
            in_code_catalog = false;
            in_catalog_hints = false;
        }
        if in_inventory && line.starts_with("| `") {
            let names: Vec<&str> = line
                .split('`')
                .enumerate()
                .filter_map(|(index, value)| (index % 2 == 1).then_some(value))
                .collect();
            let profile = names
                .first()
                .map(|name| ContextProfile::from_document(name))
                .unwrap_or_else(|| panic!("diagnostic inventory row has no profile: {line}"));
            for name in names.into_iter().skip(1) {
                assert!(
                    name.chars().all(|character| {
                        character.is_ascii_uppercase()
                            || character.is_ascii_digit()
                            || character == '_'
                    }),
                    "diagnostic inventory has invalid code {name:?}"
                );
                assert!(
                    profiles.insert(name.to_owned(), profile).is_none(),
                    "diagnostic inventory assigns {name} more than one profile"
                );
            }
            continue;
        }
        if in_code_catalog && line.starts_with("## ") {
            break;
        }

        if in_code_catalog || in_catalog_hints {
            let Some(rest) = line.strip_prefix("| `") else {
                continue;
            };
            let Some((name, after_name)) = rest.split_once('`') else {
                continue;
            };
            if name.is_empty()
                || !name.chars().all(|character| {
                    character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
                })
            {
                continue;
            }

            let cells: Vec<&str> = after_name.split('|').collect();
            if in_catalog_hints {
                assert_eq!(
                    cells.len(),
                    3,
                    "diagnostics-codes.md hint row {} for {name} must have two cells",
                    line_number + 1
                );
                let hint = cells[1].trim();
                assert!(
                    !hint.is_empty(),
                    "diagnostics-codes.md hint row {} for {name} is empty",
                    line_number + 1
                );
                assert!(
                    hints.insert(name.to_owned(), hint.to_owned()).is_none(),
                    "diagnostics-codes.md documents hint {name} more than once"
                );
                continue;
            }

            // The code catalog has four columns. Other tables, such as the
            // two-column Catalog Hints table, are outside this section and
            // must never be mistaken for severity rows.
            assert!(
                cells.len() >= 4,
                "diagnostics-codes.md row {} for {name} has too few catalog cells",
                line_number + 1
            );
            let severity = match cells[1].trim().to_ascii_lowercase().as_str() {
                "info" => "info",
                "warning" | "warn" => "warning",
                "error" => "error",
                "fatal" => "fatal",
                value => panic!(
                    "diagnostics-codes.md row {} for {name} has unknown severity {value:?}",
                    line_number + 1
                ),
            };
            let description = cells[2].trim();
            let required_feature = description
                .split_once("requires `")
                .and_then(|(_, rest)| rest.split('`').next())
                .map(str::to_owned);

            let row = DocumentedCode {
                name: name.to_owned(),
                severity: severity.to_owned(),
                hint: String::new(),
                profile: ContextProfile::Contextual,
                required_feature,
            };
            assert!(
                rows.insert(name.to_owned(), row).is_none(),
                "diagnostics-codes.md documents {name} more than once"
            );
        }
    }

    assert_eq!(
        rows.len(),
        113,
        "the documented diagnostics matrix changed; update this acceptance test deliberately"
    );

    for (name, row) in &mut rows {
        row.hint = hints.remove(name).unwrap_or_else(|| {
            panic!("diagnostics-codes.md is missing a Catalog Hints row for {name}")
        });
        row.profile = profiles.remove(name).unwrap_or_else(|| {
            panic!("diagnostics-codes.md is missing an emission profile for {name}")
        });
    }
    assert!(
        hints.is_empty(),
        "diagnostics-codes.md has hints for undocumented codes: {hints:?}"
    );
    assert!(
        profiles.is_empty(),
        "diagnostics-codes.md has profiles for undocumented codes: {profiles:?}"
    );
    rows
}

fn active_documented_codes() -> Vec<(DiagCode, DocumentedCode)> {
    let documented = documented_codes();
    let mut active = Vec::new();

    for row in documented.values() {
        match DiagCode::from_name(&row.name) {
            Some(code) => {
                assert!(
                    DiagCode::ALL.contains(&code),
                    "{} resolves by name but is missing from DiagCode::ALL",
                    row.name
                );
                assert_eq!(
                    code.name(),
                    row.name,
                    "DiagCode::from_name must round-trip the documented wire name"
                );
                active.push((code, row.clone()));
            }
            None => assert_eq!(
                row.required_feature.as_deref(),
                Some("cjk"),
                "undocumented feature gating for {}",
                row.name
            ),
        }
    }

    if cfg!(feature = "cjk") {
        assert!(
            documented
                .values()
                .all(|row| row.required_feature.as_deref() != Some("cjk")
                    || DiagCode::from_name(&row.name).is_some()),
            "all cjk diagnostics must be available in an all-features run"
        );
    }

    let active_names: BTreeSet<&str> = active.iter().map(|(code, _)| code.name()).collect();
    let enum_names: BTreeSet<&str> = DiagCode::ALL.iter().map(|code| code.name()).collect();
    assert_eq!(
        active_names, enum_names,
        "the documented active code set and DiagCode::ALL must agree"
    );
    assert_eq!(
        DIAGNOSTIC_CATALOG.len(),
        DiagCode::ALL.len(),
        "every active enum code needs exactly one catalog row"
    );

    active.sort_by_key(|(code, _)| code.name());
    active
}

fn catalog_entry(code: DiagCode) -> &'static pdftract_core::diagnostics::DiagInfo {
    DIAGNOSTIC_CATALOG
        .iter()
        .find(|info| info.code == code)
        .unwrap_or_else(|| panic!("{} is missing from DIAGNOSTIC_CATALOG", code.name()))
}

fn emitted_diagnostic(code: DiagCode, profile: ContextProfile) -> Diagnostic {
    let (page_index, object_ref) = profile.known_context();
    let context = DiagnosticContext::new(code)
        .with_message("contract message")
        .with_byte_offset(4096)
        .with_object_ref_opt(object_ref)
        .with_page_index_opt(page_index);
    let collector = DiagnosticsCollector::new();
    collector.emit_diagnostic(Diagnostic::from_context(code, context));
    collector
        .into_vec()
        .pop()
        .expect("the collector emitted one diagnostic")
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

fn empty_result() -> ExtractionResult {
    serde_json::from_value(serde_json::json!({
        "fingerprint": "diagnostics-roundtrip-contract",
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
    .expect("the deterministic extraction fixture must deserialize")
}

#[test]
fn every_documented_code_round_trips_all_structured_fields() {
    for (code, documented) in active_documented_codes() {
        let catalog = catalog_entry(code);
        assert_eq!(
            catalog.severity.to_string(),
            documented.severity,
            "{}: severity differs from documentation",
            code.name()
        );
        assert_eq!(catalog.severity, code.severity(), "{}", code.name());
        assert!(
            !catalog.suggested_action.trim().is_empty(),
            "{} must have a non-empty catalog hint",
            code.name()
        );
        assert_eq!(
            catalog.suggested_action,
            documented.hint,
            "{}: hint differs from documentation",
            code.name()
        );

        let typed = emitted_diagnostic(code, documented.profile);
        let encoded = DiagnosticJson::from(&typed);
        assert_eq!(encoded.code, code.name(), "{}: code changed", code.name());
        assert_eq!(
            encoded.message,
            "contract message",
            "{}: message changed",
            code.name()
        );
        assert_eq!(
            encoded.severity,
            documented.severity,
            "{}: severity changed",
            code.name()
        );
        let (expected_page_index, expected_object_ref) = documented.profile.known_context();
        assert_eq!(
            encoded.page_index,
            expected_page_index,
            "{}: page_index was not preserved",
            code.name()
        );
        assert_eq!(
            encoded.location,
            expected_object_ref.map(|object_ref| ObjectLocationJson {
                object_number: object_ref.object,
                generation_number: object_ref.generation,
            }),
            "{}: location was not preserved",
            code.name()
        );
        assert_eq!(
            encoded.hint.as_deref(),
            Some(documented.hint.as_str()),
            "{}: hint was not preserved",
            code.name()
        );

        let wire = serde_json::to_string(&encoded).expect("DiagnosticJson must serialize");
        assert!(
            !wire.contains("byte_offset"),
            "{}: byte offsets are in-process only: {wire}",
            code.name()
        );
        let value: serde_json::Value = serde_json::from_str(&wire)
            .unwrap_or_else(|error| panic!("{}: wire JSON must parse: {error}", code.name()));
        assert_no_nulls(&value, code.name());
        for field in [
            "code",
            "message",
            "severity",
            "page_index",
            "location",
            "hint",
        ] {
            let expected_present = match field {
                "page_index" => expected_page_index.is_some(),
                "location" => expected_object_ref.is_some(),
                "hint" => true,
                _ => true,
            };
            assert!(
                value.get(field).is_some() == expected_present,
                "{}: {field} presence was {}, expected {expected_present}: {value}",
                code.name(),
                value.get(field).is_some()
            );
        }
        let decoded: DiagnosticJson = serde_json::from_value(value)
            .unwrap_or_else(|error| panic!("{}: wire JSON must decode: {error}", code.name()));
        assert_eq!(
            decoded,
            encoded,
            "{} lost a field during round trip",
            code.name()
        );
    }
}

#[test]
fn every_documented_code_uses_policy_values_without_context() {
    for (code, documented) in active_documented_codes() {
        let typed = Diagnostic::from_context(code, DiagnosticContext::for_code(code));
        let policy = code.policy();
        let catalog = catalog_entry(code);
        assert_eq!(typed.message.as_ref(), policy.message, "{}", code.name());
        assert_eq!(
            typed.severity().to_string(),
            documented.severity,
            "{}",
            code.name()
        );
        assert_eq!(typed.severity(), policy.severity, "{}", code.name());
        assert_eq!(typed.hint(), policy.hint, "{}", code.name());
        assert_eq!(
            typed.hint(),
            Some(documented.hint.as_str()),
            "{}",
            code.name()
        );
        assert_eq!(catalog.suggested_action, documented.hint, "{}", code.name());
        assert_eq!(
            typed.byte_offset,
            None,
            "{} has unexpected offset",
            code.name()
        );
        assert_eq!(
            typed.object_ref,
            None,
            "{} has unexpected object location",
            code.name()
        );
        assert_eq!(
            typed.page_index,
            None,
            "{} has unexpected page context",
            code.name()
        );

        let encoded = DiagnosticJson::from(&typed);
        assert_eq!(encoded.code, code.name());
        assert_eq!(encoded.message, policy.message);
        assert_eq!(encoded.severity, documented.severity);
        assert_eq!(encoded.page_index, None);
        assert_eq!(encoded.location, None);
        assert_eq!(encoded.hint.as_deref(), policy.hint);

        let value = serde_json::to_value(&encoded).expect("fallback diagnostic must serialize");
        assert_no_nulls(&value, code.name());
        for field in ["page_index", "location"] {
            assert!(
                value.get(field).is_none(),
                "{field} must be omitted for {}: {value}",
                code.name()
            );
        }
        for field in ["code", "message", "severity", "hint"] {
            assert!(
                value.get(field).is_some(),
                "{field} must be present for {}: {value}",
                code.name()
            );
        }
    }
}

#[test]
fn legacy_projection_and_display_keep_byte_compatible_context_rules() {
    let documented = active_documented_codes();
    let diagnostics: Vec<Diagnostic> = documented
        .iter()
        .map(|(code, documented)| emitted_diagnostic(*code, documented.profile))
        .collect();
    let legacy = to_legacy_strings(&diagnostics);

    assert_eq!(legacy.len(), diagnostics.len());
    for ((diagnostic, legacy_message), (_, documented)) in
        diagnostics.iter().zip(&legacy).zip(&documented)
    {
        let message = "contract message";
        assert_eq!(
            legacy_message,
            &message,
            "legacy message changed for {}",
            diagnostic.code.name()
        );
        assert!(!legacy_message.contains(diagnostic.code.name()));
        assert!(!legacy_message.contains("byte offset"));
        assert!(!legacy_message.contains("12 3 R"));
        let object_suffix = documented
            .profile
            .known_context()
            .1
            .map(|object_ref| format!(" [{object_ref}]"))
            .unwrap_or_default();
        assert_eq!(
            diagnostic.to_string(),
            format!(
                "{}: {} (byte offset 4096){}",
                diagnostic.code.name(),
                message,
                object_suffix
            ),
            "Display formatting changed for {}",
            diagnostic.code.name()
        );
    }

    let duplicate = Diagnostic::with_static_no_offset(DiagCode::XrefRepaired, "same bytes");
    assert_eq!(
        to_legacy_strings(&[duplicate.clone(), duplicate]),
        vec!["same bytes", "same bytes"],
        "legacy projection must preserve order, length, and duplicates"
    );
}

#[test]
fn malformed_json_uses_optional_null_fallback_and_rejects_required_breakage() {
    let complete = serde_json::json!({
        "code": "PAGE_OUT_OF_RANGE",
        "message": "page 12 exceeds document page count (10)",
        "severity": "error",
        "page_index": 11,
        "location": {"object_number": 12, "generation_number": 3},
        "hint": "adjust the page selection"
    });
    let decoded: DiagnosticJson =
        serde_json::from_value(complete).expect("complete input must decode");
    assert_eq!(decoded.page_index, Some(11));
    assert_eq!(
        decoded.location,
        Some(ObjectLocationJson {
            object_number: 12,
            generation_number: 3,
        })
    );
    assert_eq!(decoded.hint.as_deref(), Some("adjust the page selection"));

    let mut null_optional = serde_json::json!({
        "code": "XREF_REPAIRED",
        "message": "Xref was reconstructed via forward scan",
        "severity": "info",
        "page_index": null,
        "location": null,
        "hint": null
    });
    let decoded_nulls: DiagnosticJson =
        serde_json::from_value(null_optional.clone()).expect("optional nulls must be tolerated");
    assert_eq!(decoded_nulls.page_index, None);
    assert_eq!(decoded_nulls.location, None);
    assert_eq!(decoded_nulls.hint, None);
    let reencoded = serde_json::to_value(&decoded_nulls).expect("decoded nulls must reserialize");
    assert_no_nulls(&reencoded, "null optional fallback");
    for field in ["page_index", "location", "hint"] {
        assert!(
            reencoded.get(field).is_none(),
            "{field} must be omitted: {reencoded}"
        );
    }

    for field in ["code", "message", "severity"] {
        null_optional[field] = serde_json::Value::Null;
        assert!(
            serde_json::from_value::<DiagnosticJson>(null_optional.clone()).is_err(),
            "malformed null required field {field} must be rejected"
        );
        null_optional[field] = match field {
            "code" => serde_json::json!("XREF_REPAIRED"),
            "message" => serde_json::json!("Xref was reconstructed via forward scan"),
            "severity" => serde_json::json!("info"),
            _ => unreachable!(),
        };
    }

    let mut missing = serde_json::json!({
        "code": "XREF_REPAIRED",
        "message": "Xref was reconstructed via forward scan",
        "severity": "info"
    });
    for field in ["code", "message", "severity"] {
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<DiagnosticJson>(missing.clone()).is_err(),
            "malformed missing required field {field} must be rejected"
        );
        missing[field] = match field {
            "code" => serde_json::json!("XREF_REPAIRED"),
            "message" => serde_json::json!("Xref was reconstructed via forward scan"),
            "severity" => serde_json::json!("info"),
            _ => unreachable!(),
        };
    }
}

#[test]
fn json_and_ndjson_surfaces_preserve_the_same_diagnostic_objects() {
    let diagnostics: Vec<DiagnosticJson> = active_documented_codes()
        .into_iter()
        .map(|(code, documented)| {
            DiagnosticJson::from(&emitted_diagnostic(code, documented.profile))
        })
        .collect();
    let legacy = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    let mut result = empty_result();
    result.metadata.diagnostics = legacy;
    result.metadata.diagnostics_detailed = diagnostics.clone();

    let compact = result_to_json(&result);
    let compact_detailed = compact["metadata"]["diagnostics_detailed"]
        .as_array()
        .expect("compact structured diagnostics array");
    let compact_legacy = compact["metadata"]["diagnostics"]
        .as_array()
        .expect("compact legacy diagnostics array");
    assert_eq!(compact_detailed.len(), diagnostics.len());
    assert_eq!(compact_legacy.len(), diagnostics.len());
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        assert_eq!(
            &compact_detailed[index],
            &serde_json::to_value(diagnostic).unwrap()
        );
        assert_eq!(compact_legacy[index], diagnostic.message);
    }

    let full = serde_json::to_value(result_to_output(&result)).expect("full JSON must serialize");
    let full_errors = full["errors"].as_array().expect("full errors array");
    assert_eq!(full_errors.len(), diagnostics.len());
    assert_eq!(full_errors, compact_detailed);

    let footer_values = footer_errors(&result).expect("footer diagnostics must serialize");
    assert_eq!(footer_values, full_errors.clone());
    let footer = FooterFrame::new(
        ExtractionQuality::new()
            .with_quality("high")
            .with_ocr_fraction(0.0),
        footer_values,
    );
    let mut footer_bytes = Vec::new();
    write_frame(&mut footer_bytes, &NdjsonFrame::Footer(footer)).expect("footer must serialize");
    assert!(footer_bytes.ends_with(b"\n"));
    let footer_frame: NdjsonFrame = serde_json::from_slice(
        footer_bytes
            .strip_suffix(b"\n")
            .expect("NDJSON footer must have one trailing newline"),
    )
    .expect("footer line must deserialize as an NDJSON frame");
    let NdjsonFrame::Footer(decoded_footer) = footer_frame else {
        panic!("diagnostic stream must end in a footer frame");
    };
    assert_eq!(decoded_footer.errors, full_errors.clone());

    let page_error = serde_json::json!({
        "code": "page_extraction_error",
        "message": "page failed deterministically",
        "severity": "error"
    });
    let page = PageFrame::new(7, "blank".to_owned(), vec![], vec![], vec![])
        .with_errors(vec![page_error.clone()]);
    let mut page_bytes = Vec::new();
    write_frame(&mut page_bytes, &NdjsonFrame::Page(page)).expect("page must serialize");
    assert!(page_bytes.ends_with(b"\n"));
    let page_frame: NdjsonFrame = serde_json::from_slice(
        page_bytes
            .strip_suffix(b"\n")
            .expect("NDJSON page must have one trailing newline"),
    )
    .expect("page line must deserialize as an NDJSON frame");
    let NdjsonFrame::Page(decoded_page) = page_frame else {
        panic!("diagnostic stream page fixture must remain a page frame");
    };
    assert_eq!(decoded_page.page_index, 7);
    assert_eq!(decoded_page.errors, Some(vec![page_error]));

    let clean_page = PageFrame::new(8, "blank".to_owned(), vec![], vec![], vec![]);
    let mut clean_page_bytes = Vec::new();
    write_frame(&mut clean_page_bytes, &NdjsonFrame::Page(clean_page)).unwrap();
    let clean_page_value: serde_json::Value = serde_json::from_slice(
        clean_page_bytes
            .strip_suffix(b"\n")
            .expect("clean NDJSON page must have one trailing newline"),
    )
    .unwrap();
    assert!(
        clean_page_value.get("errors").is_none(),
        "a successful NDJSON page must omit errors, not emit an empty field"
    );
}
