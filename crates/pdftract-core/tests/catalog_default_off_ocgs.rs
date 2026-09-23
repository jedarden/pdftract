use std::collections::HashSet;

use pdftract_core::parser::catalog::{parse_catalog, Catalog};
use pdftract_core::parser::object::{intern, ObjRef, PdfDict, PdfObject};
use pdftract_core::parser::XrefResolver;

fn make_ocg(name: &str) -> PdfObject {
    let mut dict = PdfDict::new();
    dict.insert(intern("Type"), PdfObject::Name(intern("OCG")));
    dict.insert(
        intern("Name"),
        PdfObject::String(Box::new(name.as_bytes().to_vec())),
    );
    PdfObject::Dict(Box::new(dict))
}

#[test]
fn catalog_default_off_ocgs_returns_off_entries() {
    let resolver = XrefResolver::new();
    let root_ref = ObjRef::new(1, 0);
    let oc_properties_ref = ObjRef::new(5, 0);
    let on_ref = ObjRef::new(10, 0);
    let off_ref = ObjRef::new(11, 0);

    resolver.cache_object(on_ref, make_ocg("Visible layer"));
    resolver.cache_object(off_ref, make_ocg("Hidden layer"));

    let mut default_config = PdfDict::new();
    default_config.insert(intern("BaseState"), PdfObject::Name(intern("ON")));
    default_config.insert(
        intern("OFF"),
        PdfObject::Array(Box::new(vec![PdfObject::Ref(off_ref)])),
    );

    let mut oc_properties = PdfDict::new();
    oc_properties.insert(
        intern("OCGs"),
        PdfObject::Array(Box::new(vec![
            PdfObject::Ref(on_ref),
            PdfObject::Ref(off_ref),
        ])),
    );
    oc_properties.insert(intern("D"), PdfObject::Dict(Box::new(default_config)));
    resolver.cache_object(oc_properties_ref, PdfObject::Dict(Box::new(oc_properties)));

    let mut catalog_dict = PdfDict::new();
    catalog_dict.insert(intern("Pages"), PdfObject::Ref(ObjRef::new(2, 0)));
    catalog_dict.insert(intern("OCProperties"), PdfObject::Ref(oc_properties_ref));
    resolver.cache_object(root_ref, PdfObject::Dict(Box::new(catalog_dict)));

    let catalog = parse_catalog(&resolver, root_ref, None).expect("catalog should parse");

    assert_eq!(
        catalog.default_off_ocgs(),
        HashSet::from([off_ref]),
        "the catalog accessor should expose the parsed /D /OFF set"
    );
}

#[test]
fn catalog_default_off_ocgs_is_empty_without_configuration() {
    let constructed = Catalog::new(ObjRef::new(2, 0), PdfObject::Dict(Box::new(PdfDict::new())));
    assert!(constructed.default_off_ocgs().is_empty());

    let resolver = XrefResolver::new();
    let root_ref = ObjRef::new(1, 0);
    let mut catalog_dict = PdfDict::new();
    catalog_dict.insert(intern("Pages"), PdfObject::Ref(ObjRef::new(2, 0)));
    resolver.cache_object(root_ref, PdfObject::Dict(Box::new(catalog_dict)));

    let parsed = parse_catalog(&resolver, root_ref, None).expect("catalog should parse");
    assert!(
        parsed.default_off_ocgs().is_empty(),
        "a catalog without /OCProperties must not add visibility overrides"
    );
}
