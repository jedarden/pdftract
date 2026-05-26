use pdftract_core::extract::extract_pdf;
use pdftract_core::options::{ExtractionOptions, ReceiptsMode};

fn main() {
    let pdf_path = std::path::Path::new("tests/fixtures/tagged-suspects-false.pdf");

    let options = ExtractionOptions::with_receipts(ReceiptsMode::Lite);
    match extract_pdf(pdf_path, &options) {
        Ok(result) => {
            println!("Pages: {}", result.pages.len());
            println!("Fingerprint: {}", result.fingerprint);
            println!("Receipts mode: {:?}", result.metadata.receipts_mode);

            if !result.pages.is_empty() {
                let page = &result.pages[0];
                println!("Page 0 spans: {}", page.spans.len());
                println!("Page 0 blocks: {}", page.blocks.len());
            }
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}
