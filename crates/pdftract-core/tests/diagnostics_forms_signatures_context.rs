//! Structured diagnostic coverage for the forms and signature boundaries.
//!
//! These paths historically accumulated diagnostics locally and then returned
//! only their successful model, so the public diagnostic surface never saw
//! malformed AcroForm data. Keep the tests at the boundary where object
//! context is attached and serialized.

use indexmap::IndexMap;
use pdftract_core::diagnostics::{DiagCode, ObjRef as DiagnosticObjRef, Severity};
use pdftract_core::forms::walk_acroform_fields_with_diagnostics;
use pdftract_core::parser::catalog::{parse_catalog, Catalog};
use pdftract_core::parser::object::{intern, ObjRef, PdfObject};
use pdftract_core::parser::xref::XrefResolver;
use pdftract_core::schema::DiagnosticJson;
use pdftract_core::signature::{discover_with_diagnostics, extract_signatures_with_diagnostics};
use std::sync::Arc;

fn dict(entries: impl IntoIterator<Item = (&'static str, PdfObject)>) -> PdfObject {
    let mut value = IndexMap::new();
    for (key, entry) in entries {
        value.insert(intern(key), entry);
    }
    PdfObject::Dict(Box::new(value))
}

fn catalog_with_acroform(acroform_ref: ObjRef) -> Catalog {
    let mut catalog = Catalog::new(
        ObjRef::new(1, 0),
        dict([("Pages", PdfObject::Ref(ObjRef::new(2, 0)))]),
    );
    catalog.acroform_ref = Some(acroform_ref);
    catalog
}

#[test]
fn malformed_form_field_reaches_serialized_diagnostics_with_location() {
    let resolver = XrefResolver::new();
    let acroform_ref = ObjRef::new(10, 2);
    let field_ref = ObjRef::new(20, 7);
    resolver.cache_object(
        acroform_ref,
        dict([(
            "Fields",
            PdfObject::Array(Box::new(vec![PdfObject::Ref(field_ref)])),
        )]),
    );

    let (_fields, diagnostics) = walk_acroform_fields_with_diagnostics(
        &resolver,
        &catalog_with_acroform(acroform_ref),
        None,
    );
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::StructUnexpectedEof)
        .expect("missing field must emit a structured diagnostic");

    assert_eq!(diagnostic.severity(), Severity::Warning);
    assert_eq!(diagnostic.object_ref, Some(DiagnosticObjRef::new(20, 7)));
    assert_eq!(diagnostic.page_index, None);

    let json = serde_json::to_value(DiagnosticJson::from(diagnostic)).unwrap();
    assert_eq!(json["code"], "STRUCT_UNEXPECTED_EOF");
    assert_eq!(json["severity"], "warning");
    assert_eq!(json["location"]["object_number"], 20);
    assert_eq!(json["location"]["generation_number"], 7);
    assert!(json.get("page_index").is_none());
}

#[test]
fn catalog_diagnostic_retains_root_context_through_serialization() {
    let resolver = XrefResolver::new();
    let root_ref = ObjRef::new(41, 6);
    resolver.cache_object(
        root_ref,
        dict([("Type", PdfObject::Name(intern("Catalog")))]),
    );

    let catalog = parse_catalog(&resolver, root_ref, None).expect("missing /Pages is recoverable");
    let diagnostic = catalog
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::StructMissingKey)
        .expect("missing /Pages must emit a structured diagnostic");

    assert_eq!(diagnostic.object_ref, Some(DiagnosticObjRef::new(41, 6)));
    assert_eq!(diagnostic.page_index, None);
    let json = serde_json::to_value(DiagnosticJson::from(diagnostic)).unwrap();
    assert_eq!(json["code"], "STRUCT_MISSING_KEY");
    assert_eq!(json["severity"], "warning");
    assert_eq!(json["location"]["object_number"], 41);
    assert_eq!(json["location"]["generation_number"], 6);
    assert!(json.get("page_index").is_none());
}

#[test]
fn malformed_signature_text_preserves_field_context_through_serialization() {
    let resolver = XrefResolver::new();
    let acroform_ref = ObjRef::new(10, 0);
    let field_ref = ObjRef::new(30, 4);
    let signature_ref = ObjRef::new(31, 1);

    resolver.cache_object(
        acroform_ref,
        dict([(
            "Fields",
            PdfObject::Array(Box::new(vec![PdfObject::Ref(field_ref)])),
        )]),
    );
    resolver.cache_object(
        field_ref,
        dict([
            ("FT", PdfObject::Name(intern("Sig"))),
            ("T", PdfObject::String(Box::new(b"signature".to_vec()))),
            ("V", PdfObject::Ref(signature_ref)),
        ]),
    );
    // UTF-16BE after the BOM has an odd number of bytes, which exercises the
    // signature decoder's structured error path.
    resolver.cache_object(
        signature_ref,
        dict([("Name", PdfObject::String(Box::new(vec![0xfe, 0xff, 0x00])))]),
    );

    let (fields, discovery_diagnostics) =
        discover_with_diagnostics(&resolver, &catalog_with_acroform(acroform_ref));
    assert!(discovery_diagnostics.is_empty());
    assert_eq!(fields.len(), 1);

    let (_signatures, diagnostics) = extract_signatures_with_diagnostics(&fields, &resolver, None);
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::StructInvalidUtf16)
        .expect("invalid signature text must emit a structured diagnostic");
    assert_eq!(diagnostic.object_ref, Some(DiagnosticObjRef::new(30, 4)));
    assert_eq!(diagnostic.page_index, None);

    let json = serde_json::to_value(DiagnosticJson::from(diagnostic)).unwrap();
    assert_eq!(json["code"], "STRUCT_INVALID_UTF16");
    assert_eq!(json["severity"], "warning");
    assert_eq!(json["location"]["object_number"], 30);
    assert_eq!(json["location"]["generation_number"], 4);
    assert_eq!(json["hint"], diagnostic.hint().unwrap());
}

#[test]
fn form_and_signature_compatibility_wrappers_remain_fields_only() {
    // Keep the public fields-only entry points usable while extraction opts
    // into the tuple-returning diagnostic-aware variants.
    let resolver = Arc::new(XrefResolver::new());
    let catalog = catalog_with_acroform(ObjRef::new(10, 0));
    assert!(pdftract_core::forms::walk_acroform_fields(&resolver, &catalog, None).is_empty());
    assert!(pdftract_core::signature::discover(&resolver, &catalog).is_empty());
}
