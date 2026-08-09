//! Debug test to examine normalized content streams for fingerprinting.

use pdftract_core::document::parse_pdf_file;
use pdftract_core::parser::lexer::Lexer;
use pdftract_core::fingerprint::serialize_token;
use std::path::PathBuf;

#[test]
fn test_debug_content_streams() {
    let v1_path = PathBuf::from("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");
    let v2_path = PathBuf::from("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf");

    let (_fp1, _catalog1, pages1, _resolver1) = parse_pdf_file(&v1_path).unwrap();
    let (_fp2, _catalog2, pages2, _resolver2) = parse_pdf_file(&v2_path).unwrap();

    // Get content stream references for page 0
    let page1 = &pages1[0];
    let page2 = &pages2[0];

    println!("=== v1.pdf ===");
    println!("Page 0 contents: {:?}", page1.contents);
    println!("MediaBox: {:?}", page1.media_box);

    println!("\n=== v2.pdf ===");
    println!("Page 0 contents: {:?}", page2.contents);
    println!("MediaBox: {:?}", page2.media_box);

    // Now manually read and normalize the content streams
    use pdftract_core::parser::stream::FileSource as ParserFileSource;
    use pdftract_core::parser::PdfSource as ParserPdfSource;
    use pdftract_core::parser::xref::XrefResolver;
    use pdftract_core::parser::stream::{ExtractionOptions, decode_stream};
    use pdftract_core::fingerprint::normalize_content_bytes;

    let source1 = ParserFileSource::open(&v1_path).unwrap();
    let source2 = ParserFileSource::open(&v2_path).unwrap();

    // Read v1 content stream
    let content_ref1 = page1.contents[0];
    let (_fp1, _catalog1, pages1, resolver1) = parse_pdf_file(&v1_path).unwrap();
    let page1 = &pages1[0];
    let obj1 = resolver1.resolve(page1.contents[0]).unwrap();
    if let pdftract_core::parser::object::PdfObject::Stream(stream1) = obj1 {
        let mut decompress_counter1 = 0u64;
        let decoded1 = decode_stream(&*stream1, &source1 as &dyn ParserPdfSource, &ExtractionOptions::default(), &mut decompress_counter1);
        let normalized1 = normalize_content_bytes(&decoded1);
        println!("\n=== v1 normalized content: ===");
        println!("{}", String::from_utf8_lossy(&normalized1));

        // Tokenize manually
        let mut lexer = Lexer::new(&decoded1);
        println!("\n=== v1 tokens: ===");
        let mut token_count = 0;
        while let Some(token) = lexer.next_token() {
            match token {
                pdftract_core::parser::lexer::Token::Eof => break,
                _ => {
                    let mut token_bytes = vec![];
                    serialize_token(&mut token_bytes, &token);
                    println!("Token {}: {:?}", token_count, String::from_utf8_lossy(&token_bytes));
                    token_count += 1;
                }
            }
        }
    }

    // Read v2 content stream
    let (_fp2, _catalog2, pages2, resolver2) = parse_pdf_file(&v2_path).unwrap();
    let page2 = &pages2[0];
    let obj2 = resolver2.resolve(page2.contents[0]).unwrap();
    if let pdftract_core::parser::object::PdfObject::Stream(stream2) = obj2 {
        let mut decompress_counter2 = 0u64;
        let decoded2 = decode_stream(&*stream2, &source2 as &dyn ParserPdfSource, &ExtractionOptions::default(), &mut decompress_counter2);
        let normalized2 = normalize_content_bytes(&decoded2);
        println!("\n=== v2 normalized content: ===");
        println!("{}", String::from_utf8_lossy(&normalized2));

        // Tokenize manually
        let mut lexer = Lexer::new(&decoded2);
        println!("\n=== v2 tokens: ===");
        let mut token_count = 0;
        while let Some(token) = lexer.next_token() {
            match token {
                pdftract_core::parser::Token::Eof => break,
                _ => {
                    let mut token_bytes = vec![];
                    serialize_token(&mut token_bytes, &token);
                    println!("Token {}: {:?}", token_count, String::from_utf8_lossy(&token_bytes));
                    token_count += 1;
                }
            }
        }
    }
}
