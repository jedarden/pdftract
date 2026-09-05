//! Structure validation of Type 3 char_proc targets.
//!
//! Covers [`validate_char_proc_structure`] end to end through the public API:
//! stream/dict classification (bf-oufxf7), the required-key rules, and the
//! error contract. These live outside the crate's unit test target so they
//! run while that target is being repaired (bf-2o61br).

use pdftract_core::font::type3_rasterizer::{validate_char_proc_structure, Type3Error};
use pdftract_core::parser::object::types::{
    intern, ObjRef, PdfDict, PdfIndirect, PdfObject, PdfStream,
};

/// A stream dictionary carrying every key the validator requires.
fn valid_stream_dict() -> PdfDict {
    let mut dict = PdfDict::new();
    dict.insert(intern("/Type"), PdfObject::Name(intern("/XObject")));
    dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Form")));
    dict.insert(intern("/Width"), PdfObject::Integer(16));
    dict.insert(intern("/Height"), PdfObject::Integer(16));
    dict
}

fn stream(dict: PdfDict) -> PdfObject {
    PdfObject::Stream(Box::new(PdfStream::new(dict, 0, Some(24))))
}

fn indirect(obj: PdfObject) -> PdfObject {
    PdfObject::Indirect(Box::new(PdfIndirect {
        id: ObjRef::new(7, 0),
        obj,
    }))
}

#[test]
fn valid_stream_is_accepted() {
    assert_eq!(
        validate_char_proc_structure(&stream(valid_stream_dict())),
        Ok(())
    );
}

#[test]
fn filter_is_optional_on_stream_dicts() {
    let mut dict = valid_stream_dict();
    dict.insert(intern("/Filter"), PdfObject::Name(intern("/FlateDecode")));
    assert_eq!(validate_char_proc_structure(&stream(dict)), Ok(()));
}

#[test]
fn stream_missing_subtype_is_rejected() {
    let mut dict = valid_stream_dict();
    dict.remove("/Subtype");

    assert_eq!(
        validate_char_proc_structure(&stream(dict)),
        Err(Type3Error::MissingRequiredKey {
            key: "/Subtype".to_string(),
            object_type: "stream".to_string(),
        })
    );
}

#[test]
fn valid_dict_is_accepted() {
    let mut dict = PdfDict::new();
    dict.insert(intern("/Type"), PdfObject::Name(intern("/XObject")));
    dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Form")));

    assert_eq!(
        validate_char_proc_structure(&PdfObject::Dict(Box::new(dict))),
        Ok(())
    );
}

#[test]
fn dict_missing_type_is_rejected() {
    let mut dict = PdfDict::new();
    dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Form")));

    assert_eq!(
        validate_char_proc_structure(&PdfObject::Dict(Box::new(dict))),
        Err(Type3Error::MissingRequiredKey {
            key: "/Type".to_string(),
            object_type: "dictionary".to_string(),
        })
    );
}

#[test]
fn scalar_types_are_rejected_with_their_own_name() {
    for (object, type_name) in [
        (PdfObject::Integer(42), "integer"),
        (PdfObject::Real(1.5), "real"),
        (PdfObject::Null, "null"),
    ] {
        assert_eq!(
            validate_char_proc_structure(&object),
            Err(Type3Error::InvalidCharProcType {
                got: type_name.to_string(),
                expected: "stream or dictionary".to_string(),
            }),
            "unexpected rejection for {}",
            type_name
        );
    }
}

#[test]
fn unresolved_reference_is_reported_as_unknown() {
    // bf-5on6og: a reference this resolver-less check cannot follow is
    // unknown rather than a wrong type.
    assert_eq!(
        validate_char_proc_structure(&PdfObject::Ref(ObjRef::new(10, 0))),
        Err(Type3Error::InvalidCharProcType {
            got: "unknown".to_string(),
            expected: "stream or dictionary".to_string(),
        })
    );
}

#[test]
fn indirect_stream_is_accepted() {
    // bf-oufxf7: an indirect wrapper is not a type of its own, so a valid
    // stream behind one validates instead of being rejected as "indirect".
    assert_eq!(
        validate_char_proc_structure(&indirect(stream(valid_stream_dict()))),
        Ok(())
    );
}

#[test]
fn indirect_stream_still_checks_required_keys() {
    let mut dict = valid_stream_dict();
    dict.remove("/Height");

    assert_eq!(
        validate_char_proc_structure(&indirect(stream(dict))),
        Err(Type3Error::MissingRequiredKey {
            key: "/Height".to_string(),
            object_type: "stream".to_string(),
        })
    );
}

#[test]
fn indirect_scalar_reports_the_carried_type() {
    assert_eq!(
        validate_char_proc_structure(&indirect(PdfObject::Integer(42))),
        Err(Type3Error::InvalidCharProcType {
            got: "integer".to_string(),
            expected: "stream or dictionary".to_string(),
        })
    );
}
