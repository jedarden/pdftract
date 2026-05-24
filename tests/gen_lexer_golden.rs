//! Generate golden token files for lexer fixtures.
//!
//! Run with: cargo run --bin gen_lexer_golden

use pdftract_core::parser::lexer::Lexer;
use std::fs;
use std::path::Path;

fn main() {
    let fixtures = [
        "tests/lexer/fixtures/empty.bin",
        "tests/lexer/fixtures/whitespace_only.bin",
        "tests/lexer/fixtures/every_token.pdf.in",
        "tests/lexer/fixtures/string_escapes.pdf.in",
        "tests/lexer/fixtures/name_edge_cases.pdf.in",
        "tests/lexer/fixtures/hex_string_edge_cases.pdf.in",
        "tests/lexer/fixtures/numeric_edge_cases.pdf.in",
        "tests/lexer/fixtures/bom_utf16_string.pdf.in",
    ];

    for fixture in fixtures {
        println!("Processing {}...", fixture);

        let input = fs::read(fixture)
            .unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", fixture, e));

        let mut lexer = Lexer::new(&input);
        let mut tokens = Vec::new();

        loop {
            match lexer.next_token() {
                Some(token) => {
                    tokens.push(token);
                }
                None => break,
            }
        }

        let formatted: Vec<String> = tokens.iter().map(|t| format!("{:?}", t)).collect();
        let golden_path = Path::new(fixture).with_extension("tokens.txt");

        fs::write(&golden_path, formatted.join("\n") + "\n")
            .unwrap_or_else(|e| panic!("Failed to write golden file {:?}: {}", golden_path, e));

        println!("  -> {}", golden_path.display());
    }
}
