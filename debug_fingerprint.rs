// Debug script to check content stream bytes
use std::path::Path;

fn main() {
    let v1_path = Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");
    let v2_path = Path::new("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf");

    // Simple: just read the files and print the raw bytes around content streams
    let v1_bytes = std::fs::read(v1_path).expect("Failed to read v1");
    let v2_bytes = std::fs::read(v2_path).expect("Failed to read v2");

    // Find content stream markers
    println!("=== v1.pdf content stream ===");
    if let Some(pos) = v1_bytes.windows(3).position(|w| w == b"end") {
        // Look for the stream content
        let stream_start = v1_bytes.windows(6).position(|w| w == b"stream").unwrap_or(0);
        if stream_start > 0 {
            let after_newline = stream_start + 6;
            while after_newline < v1_bytes.len() && v1_bytes[after_newline] == b'\r' || v1_bytes[after_newline] == b'\n' {
                // skip whitespace
            }
            let endstream = v1_bytes.windows(9).position(|w| w == b"endstream").unwrap_or(v1_bytes.len());
            println!("Stream bytes: {:?}", &v1_bytes[stream_start+6..stream_start+200]);
        }
    }

    // Just search for the string literal "(Hello" in both files
    println!("\n=== Searching for '(Hello' ===");
    let v1_hello = v1_bytes.windows(6).position(|w| w == b"(Hello").unwrap_or(usize::MAX);
    let v2_hello = v2_bytes.windows(6).position(|w| w == b"(Hello").unwrap_or(usize::MAX);

    if v1_hello < v1_bytes.len() {
        println!("v1 found at {}: {:?}", v1_hello, &v1_bytes[v1_hello..v1_hello+20]));
    }
    if v2_hello < v2_bytes.len() {
        println!("v2 found at {}: {:?}", v2_hello, &v2_bytes[v2_hello..v2_hello+20]));
    }
}
