use pdftract_core::document::parse_pdf_file;
use pdftract_core::parser::xref::XrefResolver;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <pdf-path>", args[0]);
        std::process::exit(1);
    }

    let pdf_path = std::path::Path::new(&args[1]);

    println!("Parsing: {:?}", pdf_path);

    // Try parsing the PDF
    match parse_pdf_file(pdf_path) {
        Ok((fingerprint, catalog, pages, resolver)) => {
            println!("Success!");
            println!("  Fingerprint: {}", fingerprint);
            println!("  Pages ref: {:?}", catalog.pages_ref);
            println!("  Number of pages: {}", pages.len());
            println!("  Is tagged: {}", catalog.mark_info.map(|m| m.is_tagged).unwrap_or(false));
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}
