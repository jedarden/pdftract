//! Generate JSON Schema from Rust output types.
//!
//! This binary generates the canonical JSON Schema for pdftract's
//! extraction output, which is checked into the repository at
//! docs/schema/v1.0/pdftract.schema.json.
//!
//! Usage: cargo run --manifest-path=xtask/Cargo.toml --bin gen_schema

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use serde_json::Value;

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

/// Add explicit enum constraints to schema fields.
///
/// This function post-processes the generated JSON schema to add explicit
/// enum constraints to fields that should have restricted value sets.
fn add_enum_constraints(value: &mut Value) {
    if let Some(obj) = value.as_object_mut() {
        // Add enum constraints to $defs for specific fields
        if let Some(defs) = obj.get_mut("$defs").and_then(|v| v.as_object_mut()) {
            // Add enum to DiagnosticJson.severity
            if let Some(diag) = defs.get_mut("DiagnosticJson").and_then(|v| v.as_object_mut()) {
                if let Some(props) = diag.get_mut("properties").and_then(|v| v.as_object_mut()) {
                    if let Some(severity) = props.get_mut("severity").and_then(|v| v.as_object_mut()) {
                        severity.insert("enum".to_string(), Value::Array(vec![
                            Value::String("info".to_string()),
                            Value::String("warning".to_string()),
                            Value::String("error".to_string()),
                            Value::String("fatal".to_string()),
                        ]));
                    }
                }
            }

            // Add enum to PageJson.page_type (type field)
            if let Some(page) = defs.get_mut("PageJson").and_then(|v| v.as_object_mut()) {
                if let Some(props) = page.get_mut("properties").and_then(|v| v.as_object_mut()) {
                    if let Some(page_type) = props.get_mut("type").and_then(|v| v.as_object_mut()) {
                        page_type.insert("enum".to_string(), Value::Array(vec![
                            Value::String("text".to_string()),
                            Value::String("scanned".to_string()),
                            Value::String("mixed".to_string()),
                            Value::String("broken_vector".to_string()),
                            Value::String("blank".to_string()),
                            Value::String("figure_only".to_string()),
                        ]));
                    }
                }
            }

            // Add enum to SpanJson.confidence_source
            if let Some(span) = defs.get_mut("SpanJson").and_then(|v| v.as_object_mut()) {
                if let Some(props) = span.get_mut("properties").and_then(|v| v.as_object_mut()) {
                    if let Some(conf_src) = props.get_mut("confidence_source").and_then(|v| v.as_object_mut()) {
                        conf_src.insert("enum".to_string(), Value::Array(vec![
                            Value::String("native".to_string()),
                            Value::String("heuristic".to_string()),
                            Value::String("ocr".to_string()),
                        ]));
                    }
                }
            }

            // Add contentEncoding: base64 to AttachmentJson.data field
            if let Some(attachment) = defs.get_mut("AttachmentJson").and_then(|v| v.as_object_mut()) {
                if let Some(props) = attachment.get_mut("properties").and_then(|v| v.as_object_mut()) {
                    if let Some(data) = props.get_mut("data").and_then(|v| v.as_object_mut()) {
                        data.insert("contentEncoding".to_string(), Value::String("base64".to_string()));
                    }
                }
            }
        }
    }
}

/// Generate the JSON Schema for pdftract extraction output.
fn generate_schema() -> String {
    use pdftract_core::schema::Output;
    use schemars::schema_for;

    let schema = schema_for!(Output);

    // Convert to JSON value
    let mut value = serde_json::to_value(&schema).expect("Failed to serialize schema");

    // Set $id, title, and description on the root schema object
    if let Some(obj) = value.as_object_mut() {
        // Set $id to stable URL
        obj.insert("$id".to_string(), Value::String(
            "https://pdftract.com/schema/v1.0/pdftract.schema.json".to_string()
        ));

        // Update title
        obj.insert("title".to_string(), Value::String(
            "pdftract Output v1.0".to_string()
        ));

        // Update description
        obj.insert("description".to_string(), Value::String(
            "JSON Schema for pdftract PDF extraction output v1.0. \
            This schema defines the structure of extraction results including pages, \
            spans, blocks, tables, form fields, signatures, and metadata."
            .to_string()
        ));
    }

    // Add explicit enum constraints
    add_enum_constraints(&mut value);

    // Sort keys recursively for stable ordering
    let sorted = sort_keys_recursive(value);

    // Serialize with pretty printing
    serde_json::to_string_pretty(&sorted).expect("Failed to serialize sorted schema")
}

/// Recursively sort all object keys alphabetically for stable diff output.
///
/// This function walks the entire JSON value tree and sorts all object keys
/// in BTreeMap order, ensuring that regenerating the schema produces
/// byte-identical output.
fn sort_keys_recursive(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted = BTreeMap::new();
            for (k, v) in map {
                sorted.insert(k, sort_keys_recursive(v));
            }
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(arr) => {
            let sorted: Vec<Value> = arr.into_iter().map(sort_keys_recursive).collect();
            Value::Array(sorted)
        }
        _ => value,
    }
}
