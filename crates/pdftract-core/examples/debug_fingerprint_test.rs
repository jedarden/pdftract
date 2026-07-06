use pdftract_core::document::parse_pdf_file;
use pdftract_core::parser::object::PdfObject;
use pdftract_core::parser::stream::decode_stream;
use pdftract_core::parser::stream::ExtractionOptions;
use pdftract_core::parser::stream::FileSource as ParserFileSource;

fn main() {
    let v1_path = "../../../tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf";
    let v2_path = "../../../tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf";

    // Check v1
    let (_fp1, _cat1, pages1, resolver1) = parse_pdf_file(std::path::Path::new(v1_path)).unwrap();
    println!("v1 pages: {}", pages1.len());
    if !pages1.is_empty() {
        let page = &pages1[0];
        println!("v1 contents refs: {:?}", page.contents);

        if !page.contents.is_empty() {
            let obj_ref = page.contents[0];
            if let Ok(PdfObject::Stream(stream)) = resolver1.resolve(obj_ref) {
                println!("v1 stream offset: {:?}", stream.offset);
                println!("v1 stream length: {:?}", stream.length());
                println!("v1 stream dict: {:?}", stream.dict);

                let source = ParserFileSource::open(std::path::Path::new(v1_path)).unwrap();
                let opts = ExtractionOptions::default();
                let mut counter = 0u64;
                let decoded = decode_stream(&*stream, &source, &opts, &mut counter);
                println!(
                    "v1 decoded bytes ({}): {:?}",
                    String::from_utf8_lossy(&decoded),
                    decoded
                );
            }
        }
    }

    // Check v2
    let (_fp2, _cat2, pages2, resolver2) = parse_pdf_file(std::path::Path::new(v2_path)).unwrap();
    println!("\nv2 pages: {}", pages2.len());
    if !pages2.is_empty() {
        let page = &pages2[0];
        println!("v2 contents refs: {:?}", page.contents);

        if !page.contents.is_empty() {
            let obj_ref = page.contents[0];
            if let Ok(PdfObject::Stream(stream)) = resolver2.resolve(obj_ref) {
                println!("v2 stream offset: {:?}", stream.offset);
                println!("v2 stream length: {:?}", stream.length());
                println!("v2 stream dict: {:?}", stream.dict);

                let source = ParserFileSource::open(std::path::Path::new(v2_path)).unwrap();
                let opts = ExtractionOptions::default();
                let mut counter = 0u64;
                let decoded = decode_stream(&*stream, &source, &opts, &mut counter);
                println!(
                    "v2 decoded bytes ({}): {:?}",
                    String::from_utf8_lossy(&decoded),
                    decoded
                );
            }
        }
    }
}
