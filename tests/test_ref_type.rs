use pdftract_core::font::type3_rasterizer::{detect_char_proc_type, CharProcType};
use pdftract_core::parser::object::types::{ObjRef, PdfObject};

#[test]
fn test_ref_type() {
    let ref_obj = PdfObject::Ref(ObjRef::new(10, 0));
    let result = detect_char_proc_type(&ref_obj);
    assert_eq!(result, CharProcType::Other("unknown".to_string()));
}
