use pdftract_core::parser::lexer::Lexer;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;

fn decode_flate(data: &[u8]) -> Result<Vec<u8>, String> {
    use flate2::read::DeflateDecoder;
    use std::io::Read;

    let mut decoder = DeflateDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).map_err(|e| format!("Decompression failed: {}", e))?;
    Ok(decompressed)
}

fn find_and_decode_stream(pdf_data: &[u8]) -> Option<Vec<u8>> {
    let stream_start = pdf_data.windows(7).position(|w| w == b"stream\n")?;
    let start = stream_start + 7;
    let end = pdf_data[start..].windows(9).position(|w| w == b"endstream")? + start;

    let compressed = &pdf_data[start..end];

    // Try deflate decompression
    match decode_flate(compressed) {
        Ok(decompressed) => Some(decompressed),
        Err(e) => {
            eprintln!("Decompression error: {}", e);
            None
        }
    }
}

fn normalize_content(bytes: &[u8]) -> Vec<u8> {
    if bytes.is_empty() {
        return Vec::new();
    }

    let mut lexer = Lexer::new(bytes);
    let mut result = Vec::new();
    let mut first_token = true;

    while let Some(token) = lexer.next_token() {
        match token {
            pdftract_core::parser::lexer::Token::Eof => break,
            _ => {
                if !first_token {
                    result.push(b' ');
                }
                first_token = false;
                serialize_token(&mut result, &token);
            }
        }
    }

    result
}

fn serialize_token(output: &mut Vec<u8>, token: &pdftract_core::parser::lexer::Token) {
    use pdftract_core::parser::lexer::Token;
    match token {
        Token::Bool(true) => output.extend_from_slice(b"true"),
        Token::Bool(false) => output.extend_from_slice(b"false"),
        Token::Integer(i) => {
            let s = i.to_string();
            output.extend_from_slice(s.as_bytes());
        }
        Token::Real(r) => {
            let s = format!("{:.6}", r);
            output.extend_from_slice(s.as_bytes());
        }
        Token::String(bytes) => {
            output.push(b'(');
            for &byte in bytes.as_ref() {
                match byte {
                    b'(' | b')' | b'\\' => {
                        output.push(b'\\');
                        output.push(byte);
                    }
                    _ => output.push(byte),
                }
            }
            output.push(b')');
        }
        Token::Name(bytes) => {
            output.push(b'/');
            output.extend_from_slice(bytes);
        }
        Token::ArrayStart => output.push(b'['),
        Token::ArrayEnd => output.push(b']'),
        Token::DictStart => output.extend_from_slice(b"<<"),
        Token::DictEnd => output.extend_from_slice(b">>"),
        Token::Stream => output.extend_from_slice(b"stream"),
        Token::EndStream => output.extend_from_slice(b"endstream"),
        Token::Obj => output.extend_from_slice(b"obj"),
        Token::EndObj => output.extend_from_slice(b"endobj"),
        Token::IndirectRef => output.push(b'R'),
        Token::Null => output.extend_from_slice(b"null"),
        Token::Keyword(bytes) => output.extend_from_slice(bytes),
        Token::Eof => {}
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <pdf-path>", args[0]);
        return;
    }

    let pdf_path = Path::new(&args[1]);
    let mut pdf_data = Vec::new();

    if let Err(e) = File::open(pdf_path).and_then(|mut f| f.read_to_end(&mut pdf_data)) {
        eprintln!("Failed to read PDF: {}", e);
        return;
    }

    if let Some(decoded) = find_and_decode_stream(&pdf_data) {
        println!("Decoded stream bytes:");
        println!("{:?}", decoded);
        println!();

        let normalized = normalize_content(&decoded);
        println!("Normalized content:");
        println!("{}", String::from_utf8_lossy(&normalized));
        println!("Normalized bytes:");
        println!("{:?}", normalized);
    } else {
        eprintln!("Failed to find/decode stream");
    }
}
