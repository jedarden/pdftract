// Debug test to see why fixtures return 0 pages
use pdftract_core::document::parse_pdf_file;
use pdftract_core::parser::stream::FileSource as ParserFileSource;
use pdftract_core::parser::xref::find_startxref;
use pdftract_core::parser::xref::load_xref_with_prev_chain;
use pdftract_core::parser::xref::XrefResolver;
use pdftract_core::parser::catalog::parse_catalog;
use pdftract_core::parser::pages::flatten_page_tree;

fn main() {
    let v1_path = "tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf";

    println!("Parsing: {}", v1_path);

    // Open and check startxref
    let source = ParserFileSource::open(std::path::Path::new(v1_path)).unwrap();
    let startxref = find_startxref(&source).unwrap();
    println!("startxref: {}", startxref);

    // Load xref
    let xref_section = load_xref_with_prev_chain(&source, startxref);
    println!("xref loaded, objects: {}", xref_section.objects.len());

    // Get root ref
    let root_ref = xref_section.trailer.as_ref()
        .and_then(|t| t.get("Root"))
        .and_then(|obj| obj.as_ref());
    println!("Root ref: {:?}", root_ref);

    // Create resolver
    let resolver = XrefResolver::from_section(xref_section.clone());

    // Parse catalog
    let catalog = parse_catalog(&resolver, root_ref.unwrap(), Some(&source as &dyn pdftract_core::parser::stream::PdfSource));
    println!("Catalog parsed: {:?}", catalog.is_ok());
    if let Ok(cat) = &catalog {
        println!("  pages_ref: {:?}", cat.pages_ref);
    }

    // Flatten page tree
    if let Ok(cat) = catalog {
        let pages = flatten_page_tree(&resolver, cat.pages_ref);
        println!("Page tree result: {:?}", pages);
        if let Ok(p) = &pages {
            println!("  Pages count: {}", p.len());
        }
    }
}
