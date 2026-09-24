//! Focused parser, recovery, object-resolution, and decoder context tests.

use indexmap::IndexMap;
use pdftract_core::diagnostics::{DiagCode, Diagnostic, ObjRef, Severity};
use pdftract_core::parser::object::cache::ObjectCache;
use pdftract_core::parser::object::{intern, ObjectParser, PdfObject, PdfStream};
use pdftract_core::parser::objstm::ObjectStmParser;
use pdftract_core::parser::stream::{decode_stream_with_context, ExtractionOptions, MemorySource};
use pdftract_core::schema::DiagnosticJson;

fn json_for(diagnostic: &Diagnostic) -> serde_json::Value {
    serde_json::to_value(DiagnosticJson::from(diagnostic)).expect("diagnostic JSON serializes")
}

#[test]
fn indirect_object_recovery_retains_object_policy_and_serializes_context() {
    let mut parser = ObjectParser::new(b"7 2 obj << /Key >> endobj");
    let indirect = parser
        .parse_indirect_object()
        .expect("malformed object should recover into an indirect object");
    assert_eq!(
        indirect.id,
        pdftract_core::parser::object::ObjRef::new(7, 2)
    );

    let diagnostic = parser
        .take_diagnostics()
        .into_iter()
        .find(|diagnostic| diagnostic.code == DiagCode::StructInvalidDictValue)
        .expect("missing dictionary value should be diagnosed");

    assert_eq!(diagnostic.object_ref, Some(ObjRef::new(7, 2)));
    assert_eq!(diagnostic.page_index, None);
    assert_eq!(diagnostic.severity(), Severity::Warning);
    assert_eq!(
        diagnostic.hint(),
        DiagCode::StructInvalidDictValue.policy().hint
    );
    assert!(!diagnostic.message.contains("7 2 R"));

    let value = json_for(&diagnostic);
    assert_eq!(value["code"], "STRUCT_INVALID_DICT_VALUE");
    assert_eq!(value["severity"], "warning");
    assert_eq!(value["location"]["object_number"], 7);
    assert_eq!(value["location"]["generation_number"], 2);
    assert!(value.get("page_index").is_none());
    assert_eq!(value["hint"], diagnostic.hint().unwrap());
}

#[test]
fn object_resolution_recovery_retains_the_failing_object_reference() {
    let cache = ObjectCache::new();
    let object_ref = pdftract_core::parser::object::ObjRef::new(23, 4);
    let guard = cache
        .begin_resolution(object_ref)
        .expect("first resolution should be allowed");

    let diagnostic = cache
        .begin_resolution(object_ref)
        .expect_err("re-entering an object should emit a cycle diagnostic");
    drop(guard);

    assert_eq!(diagnostic.code, DiagCode::StructCircularRef);
    assert_eq!(diagnostic.object_ref, Some(ObjRef::new(23, 4)));
    assert_eq!(diagnostic.severity(), Severity::Warning);
    let value = json_for(&diagnostic);
    assert_eq!(value["location"]["object_number"], 23);
    assert_eq!(value["location"]["generation_number"], 4);
    assert!(value.get("page_index").is_none());
}

#[test]
fn stream_decoder_retains_object_page_policy_and_omits_unavailable_context() {
    let source = MemorySource::from_slice(b"raw data");
    let mut dict = IndexMap::new();
    dict.insert(intern("Filter"), PdfObject::Name(intern("CustomDecode")));
    dict.insert(intern("Length"), PdfObject::Integer(8));
    let stream = PdfStream::new(dict, 0, Some(8));

    let mut counter = 0;
    let result = decode_stream_with_context(
        &stream,
        &source,
        &ExtractionOptions::default(),
        &mut counter,
        Some(pdftract_core::parser::object::ObjRef::new(19, 2)),
        Some(4),
    );
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::StreamUnknownFilter)
        .expect("unknown filter should be diagnosed");

    assert_eq!(diagnostic.object_ref, Some(ObjRef::new(19, 2)));
    assert_eq!(diagnostic.page_index, Some(4));
    assert_eq!(diagnostic.severity(), Severity::Warning);
    let value = json_for(diagnostic);
    assert_eq!(value["location"]["object_number"], 19);
    assert_eq!(value["page_index"], 4);
    assert_eq!(value["hint"], diagnostic.hint().unwrap());

    let mut counter = 0;
    let result = decode_stream_with_context(
        &stream,
        &source,
        &ExtractionOptions::default(),
        &mut counter,
        None,
        None,
    );
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::StreamUnknownFilter)
        .expect("unknown filter should still be diagnosed without context");
    let value = json_for(diagnostic);
    assert!(diagnostic.object_ref.is_none());
    assert!(diagnostic.page_index.is_none());
    assert!(value.get("location").is_none());
    assert!(value.get("page_index").is_none());
}

#[test]
fn object_stream_forwards_decoder_diagnostics_with_host_context() {
    let host_ref = pdftract_core::parser::object::ObjRef::new(31, 1);
    let mut dict = IndexMap::new();
    dict.insert(intern("Filter"), PdfObject::Name(intern("CustomDecode")));
    dict.insert(intern("/N"), PdfObject::Integer(1));
    dict.insert(intern("/First"), PdfObject::Integer(4));
    dict.insert(intern("/Length"), PdfObject::Integer(7));
    let stream = PdfStream::new(dict, 0, Some(7));
    let source = MemorySource::from_slice(b"1 0 123");
    let parser = ObjectStmParser::new(1024);

    let object = parser.get_object(host_ref, 0, &source, |reference| {
        (reference == host_ref).then(|| stream.clone())
    });
    assert_eq!(object, PdfObject::Integer(123));

    let diagnostic = parser
        .take_diagnostics()
        .into_iter()
        .find(|diagnostic| diagnostic.code == DiagCode::StreamUnknownFilter)
        .expect("object-stream decoder diagnostic should be retained");
    assert_eq!(diagnostic.object_ref, Some(ObjRef::new(31, 1)));
    assert_eq!(diagnostic.page_index, None);
    assert_eq!(diagnostic.severity(), Severity::Warning);
    let value = json_for(&diagnostic);
    assert_eq!(value["location"]["object_number"], 31);
    assert_eq!(value["location"]["generation_number"], 1);
    assert!(value.get("page_index").is_none());
}
