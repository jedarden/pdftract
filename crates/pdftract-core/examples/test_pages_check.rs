use pdftract_core::document::parse_pdf_file;

fn main() {
    let v1_path = "tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf";

    match parse_pdf_file(std::path::Path::new(v1_path)) {
        Ok((fp, cat, pages, resolver)) => {
            println!("Fingerprint: {}", fp);
            println!("Catalog pages_ref: {:?}", cat.pages_ref);
            println!("Pages count: {}", pages.len());
            if !pages.is_empty() {
                let page = &pages[0];
                println!("Page 0 contents: {:?}", page.contents);
                println!("Page 0 media_box: {:?}", page.media_box);
            }
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}
