//! Generate CLI reference markdown documentation.
//!
//! This binary generates CLI reference documentation from the clap command tree
//! and writes it to the specified output file. Hand-curated content after the
//! <!-- AUTOGEN END --> marker is preserved across regenerations.
//!
//! Usage:
//!     cargo run --bin gen-cli-reference -- <output-file>
//!     cargo run --bin gen-cli-reference -- --output docs/user-docs/src/cli-reference.md

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

const AUTOGEN_END_MARKER: &str = "<!-- AUTOGEN END -->";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let mut output_path = PathBuf::from("docs/user-docs/src/cli-reference.md");

    // Parse arguments
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--output" | "-o" => {
                if i + 1 < args.len() {
                    output_path = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --output requires a path argument");
                    std::process::exit(1);
                }
            }
            arg if arg.starts_with("--") => {
                eprintln!("Error: Unknown argument {}", arg);
                std::process::exit(1);
            }
            _ => {
                // Positional argument: output file
                output_path = PathBuf::from(&args[i]);
                i += 1;
            }
        }
    }

    println!("Generating CLI reference to: {}", output_path.display());

    // Generate the markdown from clap
    let generated_markdown = pdftract_cli::generate_cli_markdown();

    // Read existing file to preserve hand-curated content
    let hand_curated_content = if output_path.exists() {
        let existing = fs::read_to_string(&output_path)?;
        if let Some(idx) = existing.find(AUTOGEN_END_MARKER) {
            Some(existing[idx + AUTOGEN_END_MARKER.len()..].to_string())
        } else {
            None
        }
    } else {
        None
    };

    // Build the final output
    let mut final_output = String::new();

    // Add header
    final_output.push_str("# CLI Reference\n\n");
    final_output.push_str("> This page is auto-generated from the clap command tree.\n");
    final_output.push_str("> Run `cargo run --bin gen-cli-reference` to regenerate.\n\n");
    final_output.push_str(&generated_markdown);
    final_output.push_str("\n\n");
    final_output.push_str(AUTOGEN_END_MARKER);
    final_output.push_str("\n\n");

    // Add hand-curated content if it exists
    if let Some(curated) = hand_curated_content {
        final_output.push_str(&curated);
        println!("Preserved hand-curated content after AUTOGEN END marker.");
    } else {
        // Add a default hand-curated section header
        final_output.push_str("## Hand-Curated Content\n\n");
        final_output.push_str("> **Note:** Any content added after this marker will be preserved\n");
        final_output.push_str("> when the CLI reference is regenerated. This section is for\n");
        final_output.push_str("> additional context that doesn't fit in the auto-generated sections.\n\n");
        final_output.push_str("### Common Patterns\n\n");
        final_output.push_str("#### Basic Extraction\n\n");
        final_output.push_str("```bash\npdftract extract document.pdf\n```\n\n");
        final_output.push_str("#### JSON Output\n\n");
        final_output.push_str("```bash\npdftract extract --json output.json document.pdf\n```\n\n");
        final_output.push_str("#### Markdown with Anchors\n\n");
        final_output.push_str("```bash\npdftract extract --md-anchors --md output.md document.pdf\n```\n\n");
        final_output.push_str("### Exit Codes\n\n");
        final_output.push_str("- `0`: Success\n");
        final_output.push_str("- `1`: General error (extraction failed, file not found, etc.)\n");
        final_output.push_str("- `2`: Usage error (invalid arguments, conflicting flags)\n");
        final_output.push_str("- `3`: Decryption error (wrong or missing password)\n");
    }

    // Write to file
    let mut file = fs::File::create(&output_path)?;
    file.write_all(final_output.as_bytes())?;

    println!("CLI reference generated successfully!");

    Ok(())
}
