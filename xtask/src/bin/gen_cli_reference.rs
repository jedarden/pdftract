//! Generate CLI Reference documentation using clap-markdown.
//!
//! This binary generates the canonical CLI Reference documentation for pdftract,
//! which is checked into the repository at docs/user-docs/src/cli-reference.md.
//!
//! Usage: cargo run --manifest-path=xtask/Cargo.toml --bin gen_cli_reference

use std::fs;
use std::path::PathBuf;

const AUTOGEN_END_MARKER: &str = "<!-- AUTOGEN END -->";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments
    let args: Vec<String> = std::env::args().collect();
    let mut output_path: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--output" | "-o" => {
                if i + 1 < args.len() {
                    output_path = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    Err("--output requires a path argument")?;
                }
            }
            "--help" | "-h" => {
                println!("Usage: gen_cli_reference [OPTIONS]");
                println!();
                println!("Options:");
                println!("  -o, --output <PATH>  Output path for CLI reference (default: docs/user-docs/src/cli-reference.md)");
                println!("  -h, --help           Print this help");
                return Ok(());
            }
            _ => {
                Err(format!("Unknown argument: {}", args[i]))?;
            }
        }
    }

    // Find the workspace root
    let workspace_root = find_workspace_root();

    // Generate the CLI reference markdown using the actual CLI definition
    let generated_markdown = pdftract_cli::generate_cli_markdown();

    // Determine output path
    let cli_ref_path = if let Some(path) = output_path {
        // If path is relative, resolve it from workspace root
        if path.is_absolute() {
            path
        } else {
            workspace_root.join(&path)
        }
    } else {
        workspace_root.join("docs/user-docs/src/cli-reference.md")
    };

    // Create the directory if it doesn't exist
    if let Some(parent) = cli_ref_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Read existing file to preserve hand-curated content
    let hand_curated_content = if cli_ref_path.exists() {
        let existing = fs::read_to_string(&cli_ref_path)?;
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

    // Add autogen notice at the top
    final_output.push_str("> This page is auto-generated from the clap command tree.\n");
    final_output.push_str("> Run `cargo run --manifest-path=xtask/Cargo.toml --bin gen_cli_reference` to regenerate.\n\n");

    // Add the generated markdown
    final_output.push_str(&generated_markdown);
    final_output.push_str("\n\n");
    final_output.push_str(AUTOGEN_END_MARKER);
    final_output.push_str("\n\n");

    // Add hand-curated content if it exists
    if let Some(curated) = hand_curated_content {
        final_output.push_str(curated.trim_start());
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

    fs::write(&cli_ref_path, final_output)?;

    println!("Generated CLI reference at: {}", cli_ref_path.display());

    Ok(())
}

/// Find the workspace root by searching for Cargo.toml
fn find_workspace_root() -> PathBuf {
    let mut current = std::env::current_dir().unwrap();

    // If we're in the xtask directory, go to parent
    if current.ends_with("xtask") {
        current = current.parent().unwrap().to_path_buf();
    }

    // Search upward for Cargo.toml with workspace members
    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = fs::read_to_string(&cargo_toml).unwrap_or_default();
            if content.contains("[workspace]") {
                return current;
            }
        }

        // Move to parent directory
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => panic!("Could not find workspace root"),
        }
    }
}
