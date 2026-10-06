//! Public identity fields on the canonical core diagnostic.

use pdftract_core::diagnostics::{DiagCode, Diagnostic, DiagnosticContext, Severity};

#[test]
fn every_catalog_code_constructs_with_required_identity_fields() {
    for &code in DiagCode::ALL {
        let diagnostic = Diagnostic::from_context(code, DiagnosticContext::for_code(code));

        assert_eq!(diagnostic.code, code);
        assert_eq!(diagnostic.code.name(), code.name());
        assert_eq!(diagnostic.message, code.default_message());
        assert!(!diagnostic.message.trim().is_empty(), "{}", code.name());
        assert_eq!(diagnostic.severity, code.severity());
    }
}

#[test]
fn custom_message_keeps_typed_code_and_severity() {
    let diagnostic = Diagnostic::with_dynamic_no_offset(
        DiagCode::StreamBomb,
        "decompression limit exceeded".to_owned(),
    );

    assert_eq!(diagnostic.code, DiagCode::StreamBomb);
    assert_eq!(diagnostic.message, "decompression limit exceeded");
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.severity(), diagnostic.severity);
}

#[test]
#[should_panic(expected = "diagnostic context severity must come from its code policy")]
fn construction_rejects_severity_that_does_not_match_the_code() {
    let mut context = DiagnosticContext::for_code(DiagCode::XrefRepaired);
    context.severity = Severity::Error;
    let _ = Diagnostic::from_context(DiagCode::XrefRepaired, context);
}
