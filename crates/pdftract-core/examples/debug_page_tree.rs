//! Debug test for page tree resolution

use pdftract_core::document::parse_pdf_file;
use pdftract_core::parser::xref::XrefResolver;
use pdftract_core::parser::object::PdfObject;
use std::path::Path;

fn main() {
    let v1_path = Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");

    let (fp, cat, pages, resolver) = parse_pdf_file(v1_path).unwrap();

    println!("=== Debug Info ===");
    println!("Fingerprint: {}", fp);
    println!("Catalog pages_ref: {:?}", cat.pages_ref);
    println!("Number of pages: {}", pages.len());

    // Resolve the pages reference directly
    match resolver.resolve(cat.pages_ref) {
        Ok(pages_obj) => {
            println!("Resolved pages_obj: {:?}", pages_obj);
            if let Some(dict) = pages_obj.as_dict() {
                println!("Pages dict keys: {:?}", dict.keys().collect::<Vec<_>>());
                if let Some(count) = dict.get("Count") {
                    println!("Count: {:?}", count);
                }
                if let Some(kids) = dict.get("Kids") {
                    println!("Kids type: {:?}", std::mem::discriminant(kids));
                    if let Some(arr) = kids.as_array() {
                        println!("Kids array length: {}", arr.len());
                        for (i, kid) in arr.iter().enumerate() {
                            println!("  Kid {}: {:?}", i, kid);
                            if let PdfObject::Ref(ref_) = kid {
                                match resolver.resolve(*ref_) {
                                    Ok(kid_obj) => {
                                        println!("    Resolved to: {:?}", kid_obj);
                                        if let Some(kid_dict) = kid_obj.as_dict() {
                                            if let Some(type_name) = kid_dict.get("Type") {
                                                println!("      Type: {:?}", type_name);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        println!("    Failed to resolve: {:?}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("Failed to resolve pages_ref: {:?}", e);
        }
    }
}
