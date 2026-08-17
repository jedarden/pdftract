//! JSON validation subcommand.
//!
//! Implements the `pdftract validate` command that validates JSON files
//! against the pdftract schema. Useful for validating cached results,
//! MCP-tool responses captured to disk, and profile-extracted outputs.

use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::io::{self, Read};

/// The bundled JSON Schema for pdftract extraction output v1.0.
///
/// Loaded from the committed schema file at build time.
const BUNDLED_SCHEMA_JSON: &str = include_str!("../../../docs/schema/v1.0/pdftract.schema.json");

/// Arguments for the validate subcommand.
pub struct ValidateArgs {
    /// Path to the JSON file to validate, or "-" for stdin
    pub file: String,
    /// Optional path to a custom schema file
    pub schema_path: Option<String>,
    /// Quiet mode - suppress error output
    pub quiet: bool,
}

/// Load the schema from a path or use the bundled schema.
fn load_schema(schema_path: Option<&str>) -> Result<jsonschema::JSONSchema> {
    let schema_json = if let Some(path) = schema_path {
        // Load custom schema from file
        fs::read_to_string(path)
            .with_context(|| format!("Failed to read schema from '{}'", path))?
    } else {
        // Use bundled schema
        BUNDLED_SCHEMA_JSON.to_string()
    };

    let schema: Value = serde_json::from_str(&schema_json).context("Schema is not valid JSON")?;

    // Compile the schema - this takes ownership and returns a valid JSONSchema
    let compiled = jsonschema::JSONSchema::compile(&schema)
        .map_err(|e| anyhow::anyhow!("Schema is not valid JSON Schema Draft 2020-12: {}", e))?;

    Ok(compiled)
}

/// Read JSON from a file path or stdin.
fn read_json(file: &str) -> Result<Value> {
    let json_str = if file == "-" {
        // Read from stdin
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("Failed to read JSON from stdin")?;
        buffer
    } else {
        // Read from file
        fs::read_to_string(file).with_context(|| format!("Failed to read JSON from '{}'", file))?
    };

    serde_json::from_str(&json_str).with_context(|| format!("Failed to parse JSON from '{}'", file))
}

/// Format a JSON path to use '/' separators instead of JSON pointer notation.
///
/// The jsonschema crate returns paths like "/pages/0/spans/3/text" (JSON Pointer),
/// which is already human-readable. We just ensure it starts with a single slash.
fn format_path(instance_path: &str) -> String {
    if instance_path.is_empty() {
        "/".to_string()
    } else if instance_path.starts_with('/') {
        instance_path.to_string()
    } else {
        format!("/{}", instance_path)
    }
}

/// Run the validate subcommand.
///
/// Returns Ok(()) if validation passes, Err otherwise.
pub fn run_validate(args: ValidateArgs) -> Result<()> {
    let schema = load_schema(args.schema_path.as_deref())?;

    let json_value = read_json(&args.file)?;

    let result = schema.validate(&json_value);

    if let Err(errors) = result {
        // Collect all validation errors
        let error_details: Vec<String> = errors
            .map(|e| {
                let path = format_path(&e.instance_path.to_string());
                format!("{} {}", path, e)
            })
            .collect();

        if !args.quiet {
            for error in &error_details {
                println!("{}", error);
            }
        }

        // Return error to trigger exit code 1
        anyhow::bail!(
            "JSON validation failed with {} error(s)",
            error_details.len()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_path() {
        assert_eq!(format_path(""), "/");
        assert_eq!(
            format_path("/pages/0/spans/3/text"),
            "/pages/0/spans/3/text"
        );
        assert_eq!(format_path("pages/0/spans/3/text"), "/pages/0/spans/3/text");
    }

    #[test]
    fn test_bundled_schema_is_valid() {
        // Verify the bundled schema compiles successfully
        let _schema = load_schema(None).unwrap();
    }

    #[test]
    fn test_minimal_valid_json_passes() {
        let json_value = serde_json::json!({
            "schema_version": "1.0",
            "metadata": {
                "page_count": 1,
                "is_tagged": false,
                "is_encrypted": false,
                "contains_javascript": false,
                "contains_xfa": false,
                "ocg_present": false,
                "conformance": "none",
                "javascript_actions": []
            },
            "outline": [],
            "threads": [],
            "attachments": [],
            "signatures": [],
            "form_fields": [],
            "links": [],
            "pages": [{
                "page_index": 0,
                "page_number": 1,
                "width": 612.0,
                "height": 792.0,
                "rotation": 0,
                "type": "text",
                "spans": [],
                "blocks": [],
                "tables": [],
                "annotations": []
            }],
            "extraction_quality": {
                "overall_quality": "none"
            },
            "errors": []
        });

        let schema = load_schema(None).unwrap();
        let result = schema.validate(&json_value);
        assert!(result.is_ok(), "Minimal valid JSON should pass validation");
    }
}
