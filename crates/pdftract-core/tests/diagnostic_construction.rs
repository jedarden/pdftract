//! Focused construction tests for the canonical `Diagnostic` model
//! (`pdftract_core::diagnostics::Diagnostic`, bead pdftract-9d16d591).
//!
//! Contract under test — `docs/diagnostics-contract-canonical.md`:
//! required fields `code`, `message`, `severity`; optional fields
//! `page_index`, `location` (byte offset / indirect-object reference on the
//! typed model), and `hint`. Severity is derived from the code's policy
//! rather than chosen at each emission site, and absent optional fields are
//! omitted on the wire rather than serialized as `null`.
//!
//! These tests intentionally pin exact `None`/`Some` values so that any
//! change to field optionality or constructor semantics fails loudly here.

use std::borrow::Cow;

use pdftract_core::diagnostics::{
    suggested_action, DiagCode, Diagnostic, ObjRef, Severity, DIAGNOSTIC_CATALOG,
};
use pdftract_core::schema::{DiagnosticJson, ObjectLocationJson};

/// One representative code per severity tier of the four-value vocabulary.
const SEVERITY_REPRESENTATIVES: [(DiagCode, Severity, &str); 4] = [
    (DiagCode::XrefRepaired, Severity::Info, "info"),
    (DiagCode::StructInvalidName, Severity::Warning, "warning"),
    (DiagCode::StreamBomb, Severity::Error, "error"),
    (DiagCode::EncryptionUnsupported, Severity::Fatal, "fatal"),
];

#[test]
fn required_fields_are_present_for_every_constructor() {
    let code = DiagCode::StreamBomb;
    let expected_severity = code.policy().severity;

    let built: [(&str, Diagnostic, &str); 7] = [
        (
            "with_static",
            Diagnostic::with_static(code, 1024, "static with offset"),
            "static with offset",
        ),
        (
            "with_static_no_offset",
            Diagnostic::with_static_no_offset(code, "static bare"),
            "static bare",
        ),
        (
            "with_dynamic",
            Diagnostic::with_dynamic(code, 2048, "dynamic with offset".to_string()),
            "dynamic with offset",
        ),
        (
            "with_dynamic_no_offset",
            Diagnostic::with_dynamic_no_offset(code, "dynamic bare".to_string()),
            "dynamic bare",
        ),
        (
            "with_message(&'static str)",
            Diagnostic::with_message(code, "general static"),
            "general static",
        ),
        (
            "with_message(String)",
            Diagnostic::with_message(code, "general owned".to_string()),
            "general owned",
        ),
        (
            "with_context",
            Diagnostic::with_context(code, "context entry", None, None, None),
            "context entry",
        ),
    ];

    for (ctor, diag, expected_message) in built {
        assert_eq!(diag.code, code, "{ctor}: code is a required field");
        assert_eq!(
            diag.severity, expected_severity,
            "{ctor}: severity is a required field"
        );
        assert_eq!(
            diag.severity(),
            expected_severity,
            "{ctor}: severity getter must agree with the field"
        );
        assert_eq!(
            diag.message, expected_message,
            "{ctor}: message is a required field"
        );
    }
}

#[test]
fn static_messages_stay_borrowed_and_dynamic_messages_are_owned() {
    let code = DiagCode::StructInvalidHex;

    let static_diag = Diagnostic::with_static_no_offset(code, "borrowed message");
    assert!(
        matches!(static_diag.message, Cow::Borrowed("borrowed message")),
        "static construction must not allocate: {:?}",
        static_diag.message
    );

    let dynamic_diag = Diagnostic::with_dynamic_no_offset(code, "owned message".to_string());
    assert!(
        matches!(dynamic_diag.message, Cow::Owned(_)),
        "dynamic construction must take ownership: {:?}",
        dynamic_diag.message
    );
    assert_eq!(dynamic_diag.message, "owned message");
}

#[test]
fn optional_fields_default_to_absent() {
    let diag = Diagnostic::with_static_no_offset(DiagCode::StructInvalidName, "no context");

    assert_eq!(diag.byte_offset, None, "byte offset is optional");
    assert_eq!(diag.object_ref, None, "object reference is optional");
    assert_eq!(diag.page_index, None, "page index is optional");

    // The wire conversion mirrors the typed model: absent stays absent.
    let json = DiagnosticJson::from(&diag);
    assert_eq!(json.page_index, None);
    assert_eq!(json.location, None);
}

#[test]
fn every_documented_optional_field_combination_via_with_context() {
    let code = DiagCode::StreamDecodeError;

    for byte_offset in [None, Some(4096_u64)] {
        for object_ref in [None, Some(ObjRef::new(12, 1))] {
            for page_index in [None, Some(3_usize)] {
                let diag = Diagnostic::with_context(
                    code,
                    "combination probe",
                    byte_offset,
                    object_ref,
                    page_index,
                );

                assert_eq!(
                    diag.byte_offset, byte_offset,
                    "byte offset must round-trip {byte_offset:?}/{object_ref:?}/{page_index:?}"
                );
                assert_eq!(
                    diag.object_ref, object_ref,
                    "object ref must round-trip {byte_offset:?}/{object_ref:?}/{page_index:?}"
                );
                assert_eq!(
                    diag.page_index,
                    page_index.map(|page| page as u32),
                    "page index must round-trip {byte_offset:?}/{object_ref:?}/{page_index:?}"
                );
                // Required fields survive every combination unchanged.
                assert_eq!(diag.code, code);
                assert_eq!(diag.message, "combination probe");
                assert_eq!(diag.severity(), code.severity());
            }
        }
    }
}

#[test]
fn page_index_builders_set_present_and_absent_forms() {
    let code = DiagCode::PageOutOfRange;

    let base = || Diagnostic::with_static_no_offset(code, "page context");
    assert_eq!(base().page_index, None);
    assert_eq!(base().with_page_index(3).page_index, Some(3));
    assert_eq!(base().with_page_index_opt(None).page_index, None);
    assert_eq!(base().with_page_index_opt(Some(11)).page_index, Some(11));

    // The stored form is the zero-based u32 index the wire contract documents.
    let with_page = base().with_page_index(7);
    let json = DiagnosticJson::from(&with_page);
    assert_eq!(json.page_index, Some(7_usize));
}

#[test]
fn location_builders_set_present_and_absent_forms() {
    let code = DiagCode::StructMissingKey;
    let base = || Diagnostic::with_static_no_offset(code, "location context");

    // Byte-offset location.
    assert_eq!(base().byte_offset, None);
    assert_eq!(base().with_byte_offset(42).byte_offset, Some(42));

    // Indirect-object location, in every builder form.
    assert_eq!(base().object_ref, None);
    assert_eq!(
        base().with_object_ref(ObjRef::new(5, 1)).object_ref,
        Some(ObjRef::new(5, 1))
    );
    assert_eq!(base().with_object_ref_opt(None).object_ref, None);
    assert_eq!(
        base()
            .with_object_ref_opt(Some(ObjRef::new(6, 0)))
            .object_ref,
        Some(ObjRef::new(6, 0))
    );
    assert_eq!(
        base().with_object_ref_parts(12, 2).object_ref,
        Some(ObjRef::new(12, 2))
    );
    assert_eq!(base().with_object_ref_parts_opt(None).object_ref, None);
    assert_eq!(
        base().with_object_ref_parts_opt(Some((13, 4))).object_ref,
        Some(ObjRef::new(13, 4))
    );

    // Location is independent of page_index: a diagnostic may carry a page
    // without an object, or an object without a page (contract table).
    let page_only = base().with_page_index(2);
    assert_eq!(page_only.object_ref, None);
    assert_eq!(page_only.page_index, Some(2));

    let object_only = base().with_object_ref_parts(9, 0);
    assert_eq!(object_only.object_ref, Some(ObjRef::new(9, 0)));
    assert_eq!(object_only.page_index, None);

    // The wire `location` object carries both required parts when present.
    let located = base().with_object_ref_parts(12, 1);
    let json = DiagnosticJson::from(&located);
    let location = json
        .location
        .expect("object location must serialize when object_ref is set");
    assert_eq!(location.object_number, 12);
    assert_eq!(location.generation_number, 1);
    assert_eq!(json.page_index, None, "no page was attached");
}

#[test]
fn with_available_context_fills_only_absent_fields() {
    let code = DiagCode::StructMissingKey;

    let inner = Diagnostic::with_static(code, 100, "inner context").with_object_ref_parts(7, 0);
    let forwarded = inner
        .clone()
        .with_available_context(Some(ObjRef::new(99, 9)), Some(4));

    // The inner diagnostic's known location is never replaced by a
    // less-specific outer one; only the absent page index is filled.
    assert_eq!(forwarded.object_ref, Some(ObjRef::new(7, 0)));
    assert_eq!(forwarded.page_index, Some(4));
    assert_eq!(forwarded.byte_offset, Some(100));

    // `None` means "context unavailable": fields stay omitted.
    let unavailable =
        Diagnostic::with_static_no_offset(code, "bare").with_available_context(None, None);
    assert_eq!(unavailable.object_ref, None);
    assert_eq!(unavailable.page_index, None);
}

#[test]
fn hint_is_retained_from_the_code_policy() {
    for (code, _, _) in SEVERITY_REPRESENTATIVES {
        let diag = Diagnostic::with_static_no_offset(code, "hint context");

        let catalog_hint = DIAGNOSTIC_CATALOG
            .iter()
            .find(|info| info.code == code)
            .map(|info| info.suggested_action)
            .expect("every code must have a catalog entry");
        assert_eq!(
            diag.hint(),
            Some(catalog_hint),
            "hint must be the catalog's suggested action for {code:?}"
        );
        assert_eq!(diag.hint(), suggested_action(code));

        // The wire form carries the same hint as an owned string.
        assert_eq!(
            DiagnosticJson::from(&diag).hint,
            Some(catalog_hint.to_string())
        );
    }
}

#[test]
fn severity_values_flow_through_the_model() {
    for (code, expected_variant, wire_spelling) in SEVERITY_REPRESENTATIVES {
        let diag = Diagnostic::with_static_no_offset(code, "severity context");

        // Severity is required on the model and derived from the code
        // policy, never chosen at the emission site.
        assert_eq!(diag.severity, expected_variant, "severity for {code:?}");
        assert_eq!(diag.severity(), expected_variant);
        assert_eq!(code.policy().severity, expected_variant);

        // Each tier spells exactly as the wire contract documents.
        assert_eq!(diag.severity().to_string(), wire_spelling);
        assert_eq!(DiagnosticJson::from(&diag).severity, wire_spelling);
    }
}

#[test]
fn construction_matches_catalog_policy_for_every_code() {
    assert!(!DIAGNOSTIC_CATALOG.is_empty());
    for entry in DIAGNOSTIC_CATALOG {
        let diag = Diagnostic::with_static_no_offset(entry.code, "policy sweep");

        assert_eq!(diag.code, entry.code);
        assert_eq!(
            diag.severity(),
            entry.severity,
            "severity drift for {:?}",
            entry.code
        );
        assert_eq!(
            diag.hint(),
            Some(entry.suggested_action),
            "hint drift for {:?}",
            entry.code
        );
        assert_eq!(
            diag.is_recoverable(),
            entry.recoverable,
            "recoverability drift for {:?}",
            entry.code
        );

        let json = DiagnosticJson::from(&diag);
        assert_eq!(
            json.code,
            entry.code.name(),
            "wire code for {:?}",
            entry.code
        );
        assert_eq!(json.severity, entry.severity.to_string());
        assert_eq!(json.hint, Some(entry.suggested_action.to_string()));
        // The sweep constructs with no location context.
        assert_eq!(json.page_index, None);
        assert_eq!(json.location, None);
    }
}

#[test]
fn constructed_diagnostics_clone_and_compare_by_value() {
    let original = Diagnostic::with_context(
        DiagCode::StreamBomb,
        "comparison probe",
        Some(1),
        Some(ObjRef::new(2, 0)),
        Some(3),
    );

    assert_eq!(
        original.clone(),
        original,
        "clone must preserve every field"
    );

    let moved_page = original.clone().with_page_index(9);
    assert_ne!(original, moved_page, "page_index participates in equality");

    let moved_location = original.clone().with_byte_offset(77);
    assert_ne!(
        original, moved_location,
        "byte_offset participates in equality"
    );
}

#[cfg(feature = "serde")]
#[test]
fn wire_model_omits_absent_optional_fields_instead_of_null() {
    // The absent trio: no page, no location, no hint.
    let absent = DiagnosticJson {
        code: "STRUCT_INVALID_NAME".to_string(),
        message: "malformed name".to_string(),
        severity: "warning".to_string(),
        page_index: None,
        location: None,
        hint: None,
    };
    let text = serde_json::to_string(&absent).expect("serialize absent trio");
    assert!(
        !text.contains("page_index"),
        "absent page_index omitted: {text}"
    );
    assert!(
        !text.contains("location"),
        "absent location omitted: {text}"
    );
    assert!(!text.contains("hint"), "absent hint omitted: {text}");
    assert!(
        text.contains("\"code\""),
        "required code serialized: {text}"
    );
    assert!(
        text.contains("\"severity\""),
        "required severity serialized: {text}"
    );
    assert!(
        text.contains("\"message\""),
        "required message serialized: {text}"
    );

    // The present trio serializes every optional field.
    let present = DiagnosticJson {
        page_index: Some(3),
        location: Some(ObjectLocationJson {
            object_number: 12,
            generation_number: 0,
        }),
        hint: Some("inspect the source PDF".to_string()),
        ..absent.clone()
    };
    let parsed: DiagnosticJson =
        serde_json::from_str(&serde_json::to_string(&present).expect("serialize present trio"))
            .expect("round-trip present trio");
    assert_eq!(parsed.page_index, Some(3));
    assert_eq!(
        parsed.location,
        Some(ObjectLocationJson {
            object_number: 12,
            generation_number: 0
        })
    );
    assert_eq!(parsed.hint, Some("inspect the source PDF".to_string()));

    // Missing and explicit-null input both deserialize to None.
    let null_input = serde_json::json!({
        "code": "STREAM_BOMB",
        "message": "decompression bomb",
        "severity": "error",
        "page_index": null,
        "location": null,
        "hint": null,
    });
    let from_null: DiagnosticJson = serde_json::from_value(null_input).expect("null input parses");
    assert_eq!(from_null.page_index, None);
    assert_eq!(from_null.location, None);
    assert_eq!(from_null.hint, None);
}
