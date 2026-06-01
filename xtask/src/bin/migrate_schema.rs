//! Schema version migration tool for pdftract JSON output.
//!
//! This binary implements migration between minor versions of the pdftract schema.
//! Following the plan's additive-evolution rules, minor version changes are additive only,
//! so migrations are primarily for field renames and default additions.
//!
//! # Usage
//!
//! ```bash
//! cargo run --bin migrate_schema -- --from 1.0 --to 1.0 input.json > output.json
//! ```
//!
//! # Exit Codes
//!
//! - 0: Migration successful
//! - 1: Migration failed (invalid JSON, unsupported version, or migration error)

use anyhow::{Context, Result};
use clap::Parser;
use serde_json::Value;
use std::io::{self, Read, Write};

// Import the migration library
use pdftract_schema_migrate::migrate;

/// Schema version migration tool for pdftract.
#[derive(Parser)]
#[command(name = "migrate_schema")]
#[command(about = "Migrate pdftract JSON output between schema versions", long_about = None)]
struct Args {
    /// Source schema version (e.g., "1.0", "1.1")
    #[arg(long)]
    from: String,

    /// Target schema version (e.g., "1.0", "1.1")
    #[arg(long)]
    to: String,

    /// Input JSON file (use "-" for stdin)
    #[arg(default_value = "-")]
    input: String,

    /// Output JSON file (use "-" for stdout)
    #[arg(short, long, default_value = "-")]
    output: String,

    /// Pretty-print output JSON
    #[arg(short, long)]
    pretty: bool,
}


/// Read JSON from a file path or stdin.
fn read_json(path: &str) -> Result<Value> {
    let json_str = if path == "-" {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)
            .context("Failed to read JSON from stdin")?;
        buffer
    } else {
        std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read JSON from '{}'", path))?
    };

    serde_json::from_str(&json_str)
        .with_context(|| format!("Failed to parse JSON from '{}'", path))
}

/// Write JSON to a file path or stdout.
fn write_json(path: &str, json: &Value, pretty: bool) -> Result<()> {
    let json_str = if pretty {
        serde_json::to_string_pretty(json)
    } else {
        serde_json::to_string(json)
    }
    .context("Failed to serialize output JSON")?;

    if path == "-" {
        io::stdout()
            .write_all(json_str.as_bytes())
            .context("Failed to write JSON to stdout")?;
    } else {
        std::fs::write(path, json_str)
            .with_context(|| format!("Failed to write JSON to '{}'", path))?;
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Read input JSON
    let json_value = read_json(&args.input)?;

    // Perform migration using the library
    let migrated_json = migrate(&args.from, &args.to, json_value)
        .with_context(|| format!("Migration from v{} to v{} failed", args.from, args.to))?;

    // Write output JSON
    write_json(&args.output, &migrated_json, args.pretty)?;

    Ok(())
}

