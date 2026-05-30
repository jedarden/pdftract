use pdftract_core::parser::lexer::Lexer;
use pdftract_core::parser::inline_image::parse_inline_image_header;
use pdftract_core::parser::lexer::Token;

fn main() {
    // Test 1: /W 10 /H /BPC 8 ID
    println!("=== Test 1: Missing value after /H ===");
    let input = b"/W 10 /H /BPC 8 ID";
    let mut lexer = Lexer::new(input);

    println!("Tokens:");
    let mut lex = Lexer::new(input);
    loop {
        let tok = lex.next_token();
        println!("  {:?}", tok);
        if matches!(tok, None | Some(Token::Eof)) {
            break;
        }
    }

    let mut lexer2 = Lexer::new(input);
    let result = parse_inline_image_header(&mut lexer2);
    println!("Result: {:?}", result);

    let diags = lexer2.take_diagnostics();
    println!("Diagnostics:");
    for d in &diags {
        println!("  {:?}: {}", d.code, d.message);
    }

    // Test 2: /W 10 IDEI
    println!("\n=== Test 2: ID without whitespace ===");
    let input2 = b"/W 10 IDEI";
    let mut lexer3 = Lexer::new(input2);

    println!("Tokens:");
    let mut lex2 = Lexer::new(input2);
    loop {
        let tok = lex2.next_token();
        println!("  {:?}", tok);
        if matches!(tok, None | Some(Token::Eof)) {
            break;
        }
    }

    let mut lexer4 = Lexer::new(input2);
    let result2 = parse_inline_image_header(&mut lexer4);
    println!("Result: {:?}", result2);

    let diags2 = lexer4.take_diagnostics();
    println!("Diagnostics:");
    for d in &diags2 {
        println!("  {:?}: {}", d.code, d.message);
    }
}
