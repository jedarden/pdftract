//! Generate JSON Schema from Rust output types.
//!
//! This binary generates the canonical JSON Schema for pdftract's
//! extraction output, which is checked into the repository at
//! docs/schema/v1.0/pdftract.schema.json.
//!
//! Usage: cargo run --bin gen_schema

use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Find the workspace root
    let workspace_root = find_workspace_root();

    // Generate the schema
    let schema_json = generate_schema();

    // Write to docs/schema/v1.0/pdftract.schema.json
    let schema_path = workspace_root.join("docs/schema/v1.0/pdftract.schema.json");

    // Create the directory if it doesn't exist
    if let Some(parent) = schema_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&schema_path, schema_json)?;

    println!("Generated schema at: {}", schema_path.display());

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

        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }

    // Fallback: use current directory if not found
    std::env::current_dir().unwrap()
}

/// Generate the JSON Schema for pdftract extraction output.
fn generate_schema() -> String {
    use pdftract_core::extract::ExtractionResult;
    use schemars::schema_for;

    let schema = schema_for!(ExtractionResult);

    // Convert to JSON string
    // The schema_for! macro already includes the $schema field
    serde_json::to_string_pretty(&schema)
        .expect("Failed to serialize schema")
}
