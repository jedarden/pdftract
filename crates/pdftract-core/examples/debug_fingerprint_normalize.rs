//! Debug test to trace fingerprint normalization for content_edit fixtures

use pdftract_core::fingerprint::canonicalize::normalize_content_stream;
use pdftract_core::parser::lexer::Lexer;

fn main() {
    let v1_stream = b"\n    BT\n    /F1 12 Tf\n    50 700 Td\n    (Hello World) Tj\n    ET\n    ";
    let v2_stream = b"\n    BT\n    /F1 12 Tf\n    50 700 Td\n    (Hello Worl) Tj\n    ET\n    ";

    println!("=== v1 stream (Hello World) ===");
    let v1_normalized = normalize_content_stream(v1_stream);
    println!("Normalized bytes: {:?}", v1_normalized);
    println!(
        "Normalized as text: {}",
        String::from_utf8_lossy(&v1_normalized)
    );

    println!("\n=== v2 stream (Hello Worl) ===");
    let v2_normalized = normalize_content_stream(v2_stream);
    println!("Normalized bytes: {:?}", v2_normalized);
    println!(
        "Normalized as text: {}",
        String::from_utf8_lossy(&v2_normalized)
    );

    println!("\n=== Are they equal? ===");
    println!("{}", v1_normalized == v2_normalized);

    println!("\n=== Hash comparison ===");
    use sha2::{Digest, Sha256};
    let v1_hash = Sha256::digest(&v1_normalized);
    let v2_hash = Sha256::digest(&v2_normalized);
    println!("v1 hash: {:x}", v1_hash);
    println!("v2 hash: {:x}", v2_hash);
    println!("Hashes equal: {}", v1_hash == v2_hash);

    println!("\n=== Lexer debug ===");
    println!("Tokenizing v1 stream:");
    let mut lexer = Lexer::new(v1_stream);
    while let Some(token) = lexer.next_token() {
        println!("  {:?}", token);
        if matches!(token, pdftract_core::parser::lexer::Token::Eof) {
            break;
        }
    }

    println!("\nTokenizing v2 stream:");
    let mut lexer = Lexer::new(v2_stream);
    while let Some(token) = lexer.next_token() {
        println!("  {:?}", token);
        if matches!(token, pdftract_core::parser::lexer::Token::Eof) {
            break;
        }
    }
}
