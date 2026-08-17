//! CLI tool for migrating pdftract JSON output between schema versions.
//!
//! Usage:
//!   migrate-schema --from 1.0 --to 1.0 input.json > output.json
//!   cat input.json | migrate-schema --from 1.0 --to 1.0 > output.json
//!   migrate-schema --from 1.0 --to 1.0 input.json -o output.json

use anyhow::{Context, Result};
use pdftract_schema_migrate::run_migration;
use std::io::{self, IsTerminal};

fn main() -> Result<()> {
    let args = parse_args()?;

    // Validate migration direction first (fail fast)
    pdftract_schema_migrate::validate_migration(&args.from, &args.to)
        .context("Migration validation failed")?;

    // Run the migration
    run_migration(&args.from, &args.to, &args.input, &args.output, args.pretty)
        .context("Migration execution failed")?;

    Ok(())
}

/// CLI arguments
struct Args {
    from: String,
    to: String,
    input: String,
    output: String,
    pretty: bool,
}

/// Parse command-line arguments.
///
/// We use a simple parser to avoid additional dependencies for this small tool.
fn parse_args() -> Result<Args> {
    let mut args = std::env::args();
    let program_name = args.next().unwrap_or_else(|| "migrate-schema".to_string());

    let mut from = None;
    let mut to = None;
    let mut input = "-".to_string(); // Default to stdin
    let mut output = "-".to_string(); // Default to stdout
    let mut pretty = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--from" => {
                from = Some(args.next().context("--from requires a value")?);
            }
            "--to" => {
                to = Some(args.next().context("--to requires a value")?);
            }
            "-i" | "--input" => {
                input = args.next().context("--input requires a value")?;
            }
            "-o" | "--output" => {
                output = args.next().context("--output requires a value")?;
            }
            "-p" | "--pretty" => {
                pretty = true;
            }
            "-h" | "--help" => {
                print_usage(&program_name);
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("migrate-schema {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            arg if arg.starts_with('-') => {
                anyhow::bail!("Unknown option: {}", arg);
            }
            _ => {
                // Positional argument: input file
                if input == "-" {
                    input = arg;
                } else {
                    anyhow::bail!("Unexpected argument: {}", arg);
                }
            }
        }
    }

    let from = from.context("--from is required (use --help for usage)")?;
    let to = to.context("--to is required (use --help for usage)")?;

    // Auto-detect pretty-print: default to true when writing to terminal
    if !pretty && output == "-" {
        pretty = io::stdout().is_terminal();
    }

    Ok(Args {
        from,
        to,
        input,
        output,
        pretty,
    })
}

/// Print usage information.
fn print_usage(program_name: &str) {
    let program = program_name.rsplit('/').next().unwrap_or(program_name);
    println!(
        "Schema version migration tool for pdftract JSON output

Usage:
  {program} --from <version> --to <version> [options] [input]

Arguments:
  --from <version>     Source schema version (e.g., 1.0)
  --to <version>       Target schema version (e.g., 1.0, 1.1)
  [input]              Input JSON file (default: stdin)

Options:
  -o, --output <file>  Output JSON file (default: stdout)
  -p, --pretty         Pretty-print output JSON
  -h, --help           Show this help message
  -V, --version        Show version information

Examples:
  # Migrate with stdin/stdout
  cat input.json | {program} --from 1.0 --to 1.0

  # Migrate with file I/O
  {program} --from 1.0 --to 1.0 input.json -o output.json

  # Pretty-print output
  {program} --from 1.0 --to 1.0 input.json --pretty

Notes:
  - Only v1.x to v1.y migrations are supported (same major version)
  - Downgrades are not allowed (e.g., v1.1 to v1.0)
  - Use '-' for stdin/stdout (default for both input and output)

Available migrations:
  - v1.0 -> v1.0 (identity migration)"
    );
}
