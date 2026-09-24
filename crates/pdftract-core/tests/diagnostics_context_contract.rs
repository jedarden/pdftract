//! Focused contract tests for the typed diagnostic emission context.

use pdftract_core::diagnostics::{
    DiagCode, Diagnostic, DiagnosticContext, DiagnosticPolicy, ObjRef, Severity, DIAGNOSTIC_CATALOG,
};
use pdftract_core::schema::DiagnosticJson;

fn json_for(diagnostic: &Diagnostic) -> serde_json::Value {
    serde_json::to_value(DiagnosticJson::from(diagnostic)).expect("diagnostic JSON serializes")
}

#[test]
fn every_code_has_one_deterministic_policy() {
    for code in DiagCode::ALL {
        let first = code.policy();
        let second = code.policy();

        assert_eq!(first, second, "{} policy is not deterministic", code.name());
        assert_eq!(
            first.severity,
            code.severity(),
            "{} severity drift",
            code.name()
        );
        assert_eq!(
            first.message,
            code.default_message(),
            "{} message drift",
            code.name()
        );
        assert_eq!(
            first.hint,
            DIAGNOSTIC_CATALOG
                .iter()
                .find(|entry| entry.code == *code)
                .map(|entry| entry.suggested_action),
            "{} hint drift",
            code.name()
        );

        let context = DiagnosticContext::for_code(*code);
        assert_eq!(
            context.severity,
            first.severity,
            "{} context severity drift",
            code.name()
        );
        assert_eq!(context.message, first.message);
        assert_eq!(context.hint, first.hint);
    }
}

#[test]
fn complete_context_survives_emission_and_serialization() {
    let context = DiagnosticContext::new(DiagCode::StreamBomb)
        .with_message("decompression limit exceeded after 4096 bytes")
        .with_byte_offset(4096)
        .with_object_ref_parts(12, 3)
        .with_page_index(7);
    let diagnostic = Diagnostic::from_context(DiagCode::StreamBomb, context.clone());

    assert_eq!(diagnostic.context(), context);
    assert_eq!(diagnostic.severity(), Severity::Error);
    assert_eq!(diagnostic.hint(), context.hint);
    assert_eq!(diagnostic.byte_offset, Some(4096));
    assert_eq!(diagnostic.object_ref, Some(ObjRef::new(12, 3)));
    assert_eq!(diagnostic.page_index, Some(7));
    assert_eq!(
        diagnostic.message,
        "decompression limit exceeded after 4096 bytes"
    );

    let value = json_for(&diagnostic);
    assert_eq!(value["code"], "STREAM_BOMB");
    assert_eq!(value["severity"], "error");
    assert_eq!(
        value["message"],
        "decompression limit exceeded after 4096 bytes"
    );
    assert_eq!(value["page_index"], 7);
    assert_eq!(value["location"]["object_number"], 12);
    assert_eq!(value["location"]["generation_number"], 3);
    assert_eq!(value["hint"], context.hint.unwrap());
}

#[test]
fn unavailable_context_is_omitted_without_losing_policy_fields() {
    let diagnostic = Diagnostic::from_context(
        DiagCode::XrefRepaired,
        DiagnosticContext::for_code(DiagCode::XrefRepaired),
    );
    let value = json_for(&diagnostic);

    assert_eq!(diagnostic.byte_offset, None);
    assert_eq!(diagnostic.object_ref, None);
    assert_eq!(diagnostic.page_index, None);
    assert_eq!(diagnostic.message, DiagCode::XrefRepaired.default_message());
    assert_eq!(diagnostic.severity(), Severity::Info);
    assert!(diagnostic.hint().is_some());

    for key in ["page_index", "location"] {
        assert!(value.get(key).is_none(), "{key} must be omitted: {value}");
    }
    for key in ["code", "message", "severity", "hint"] {
        assert!(value.get(key).is_some(), "{key} must be present: {value}");
    }
    assert!(!value.to_string().contains(":null"));
}

#[test]
fn policy_type_is_explicitly_typed() {
    let policy: DiagnosticPolicy = DiagCode::EncryptionUnsupported.policy();
    assert_eq!(policy.severity, Severity::Fatal);
    assert!(!policy.message.is_empty());
    assert!(policy.hint.is_some());
}
