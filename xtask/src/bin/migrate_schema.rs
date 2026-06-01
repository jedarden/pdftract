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

use anyhow::{bail, Context, Result};
use clap::Parser;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{self, Read, Write};

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

/// Registry of available migrations.
///
/// Maps (from_version, to_version) to the migration function.
struct MigrationRegistry {
    migrations: HashMap<(&'static str, &'static str), Box<dyn Fn(Value) -> Result<Value>>>,
}

impl MigrationRegistry {
    /// Create a new registry with all known migrations registered.
    fn new() -> Self {
        let mut migrations: HashMap<(&'static str, &'static str), Box<dyn Fn(Value) -> Result<Value>>> = HashMap::new();

        // Register identity migration for v1.0 -> v1.0
        migrations.insert(("1.0", "1.0"), Box::new(|v| Ok(v)));

        // Future migrations would be registered here:
        // migrations.insert(("1.0", "1.1"), Box::new(migrate_1_0_to_1_1));

        Self { migrations }
    }

    /// Check if a migration is registered for the given version pair.
    fn has_migration(&self, from: &str, to: &str) -> bool {
        self.migrations.contains_key(&(from.as_ref(), to.as_ref()))
    }

    /// Execute the migration for the given version pair.
    fn migrate(&self, from: &str, to: &str, json: Value) -> Result<Value> {
        let key = (from.as_ref(), to.as_ref());

        match self.migrations.get(&key) {
            Some(migration_fn) => migration_fn(json),
            None => bail!(
                "No migration registered from version '{}' to '{}'",
                from, to
            ),
        }
    }
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

/// Parse and normalize a version string.
///
/// Ensures version strings follow the "major.minor" format.
/// For now, we only support major version 1 (v1.x series).
fn parse_version(version: &str) -> Result<(u32, u32)> {
    let parts: Vec<&str> = version.split('.').collect();

    if parts.len() != 2 {
        bail!(
            "Invalid version format '{}': expected 'major.minor' (e.g., '1.0')",
            version
        );
    }

    let major: u32 = parts[0]
        .parse()
        .context("Major version must be a number")?;
    let minor: u32 = parts[1]
        .parse()
        .context("Minor version must be a number")?;

    // Only support v1.x for now
    if major != 1 {
        bail!("Major version {} is not supported (only v1.x migrations are implemented)", major);
    }

    Ok((major, minor))
}

/// Validate that migration is allowed between versions.
///
/// Rules:
/// - Major version changes (v1 -> v2) are NOT allowed (breaking changes)
/// - Downgrades (v1.1 -> v1.0) are NOT allowed (data loss risk)
/// - Same version (v1.0 -> v1.0) is allowed (identity migration)
fn validate_migration(from: &str, to: &str) -> Result<()> {
    let (from_major, from_minor) = parse_version(from)?;
    let (to_major, to_minor) = parse_version(to)?;

    // Reject major version changes
    if from_major != to_major {
        bail!(
            "Cannot migrate from v{}.{} to v{}.{}: major version changes are breaking changes and require a full data migration plan",
            from_major, from_minor, to_major, to_minor
        );
    }

    // Reject downgrades
    if to_minor < from_minor {
        bail!(
            "Cannot downgrade from v{}.{} to v{}.{}: downgrades may lose data and are not supported",
            from_major, from_minor, to_major, to_minor
        );
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Validate that the migration direction is allowed
    validate_migration(&args.from, &args.to)?;

    // Create migration registry
    let registry = MigrationRegistry::new();

    // Check if the specific migration exists
    if !registry.has_migration(&args.from, &args.to) {
        // Give a helpful error message
        if args.from == args.to {
            // Same version should always be supported
            bail!(
                "Identity migration for v{} is missing from registry",
                args.from
            );
        } else {
            bail!(
                "Migration from v{} to v{} is not yet implemented. Available migrations: v1.0 -> v1.0 (identity)",
                args.from, args.to
            );
        }
    }

    // Read input JSON
    let json_value = read_json(&args.input)?;

    // Perform migration
    let mut migrated_json = registry
        .migrate(&args.from, &args.to, json_value)
        .with_context(|| {
            format!(
                "Migration from v{} to v{} failed",
                args.from, args.to
            )
        })?;

    // Update schema_version field if it exists and versions differ
    if args.from != args.to {
        if let Some(obj) = migrated_json.as_object_mut() {
            // Update schema_version to the target version
            obj.insert("schema_version".to_string(), Value::String(args.to.clone()));
        }
    }

    // Write output JSON
    write_json(&args.output, &migrated_json, args.pretty)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_version_valid() {
        assert_eq!(parse_version("1.0").unwrap(), (1, 0));
        assert_eq!(parse_version("1.1").unwrap(), (1, 1));
        assert_eq!(parse_version("1.10").unwrap(), (1, 10));
    }

    #[test]
    fn test_parse_version_invalid() {
        assert!(parse_version("1").is_err());
        assert!(parse_version("1.0.0").is_err());
        assert!(parse_version("v1.0").is_err());
        assert!(parse_version("2.0").is_err()); // Only v1.x supported
    }

    #[test]
    fn test_validate_migration_same_version() {
        assert!(validate_migration("1.0", "1.0").is_ok());
        assert!(validate_migration("1.1", "1.1").is_ok());
    }

    #[test]
    fn test_validate_migration_upgrade_allowed() {
        assert!(validate_migration("1.0", "1.1").is_ok());
        assert!(validate_migration("1.0", "1.10").is_ok());
    }

    #[test]
    fn test_validate_migration_downgrade_rejected() {
        assert!(validate_migration("1.1", "1.0").is_err());
        assert!(validate_migration("1.10", "1.0").is_err());
    }

    #[test]
    fn test_validate_migration_major_version_change_rejected() {
        assert!(validate_migration("1.0", "2.0").is_err());
        // This test will fail once we actually support v2, but that's intentional
    }

    #[test]
    fn test_migration_registry_identity() {
        let registry = MigrationRegistry::new();

        let input = json!({
            "schema_version": "1.0",
            "test": "value"
        });

        let result = registry.migrate("1.0", "1.0", input.clone()).unwrap();

        // Identity migration should return unchanged value
        assert_eq!(input, result);
    }

    #[test]
    fn test_migration_registry_unsupported() {
        let registry = MigrationRegistry::new();

        let input = json!({"test": "value"});

        let result = registry.migrate("1.0", "1.1", input);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No migration registered"));
    }

    #[test]
    fn test_migration_registry_has_migration() {
        let registry = MigrationRegistry::new();

        assert!(registry.has_migration("1.0", "1.0"));
        assert!(!registry.has_migration("1.0", "1.1"));
        assert!(!registry.has_migration("2.0", "2.0"));
    }
}
