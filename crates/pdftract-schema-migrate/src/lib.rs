//! Schema version migration for pdftract JSON output.
//!
//! This crate implements migration between minor versions of the pdftract schema.
//! Following the plan's additive-evolution rules, minor version changes are additive only,
//! so migrations are primarily for field renames and default additions.
//!
//! # Example
//!
//! ```rust
//! use pdftract_schema_migrate::{migrate, MigrationRegistry};
//! use serde_json::json;
//!
//! let registry = MigrationRegistry::new();
//! let input = json!({"schema_version": "1.0", "data": "test"});
//! let output = registry.migrate("1.0", "1.0", input).unwrap();
//! assert_eq!(input, output); // identity migration
//! ```

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{self, Read, Write};

/// Migration function type: transforms a JSON value from one schema version to another.
pub type MigrationFn = Box<dyn Fn(Value) -> Result<Value> + Send + Sync>;

/// Registry of available migrations.
///
/// Maps (from_version, to_version) to the migration function.
pub struct MigrationRegistry {
    migrations: HashMap<(&'static str, &'static str), MigrationFn>,
}

impl MigrationRegistry {
    /// Create a new registry with all known migrations registered.
    pub fn new() -> Self {
        let mut migrations: HashMap<(&'static str, &'static str), MigrationFn> = HashMap::new();

        // Register identity migration for v1.0 -> v1.0
        migrations.insert(("1.0", "1.0"), Box::new(|v| Ok(v)));

        // Future migrations would be registered here:
        // migrations.insert(("1.0", "1.1"), Box::new(migrate_1_0_to_1_1));

        Self { migrations }
    }

    /// Check if a migration is registered for the given version pair.
    pub fn has_migration(&self, from: &str, to: &str) -> bool {
        self.migrations.contains_key(&(from.as_ref(), to.as_ref()))
    }

    /// Execute the migration for the given version pair.
    pub fn migrate(&self, from: &str, to: &str, json: Value) -> Result<Value> {
        let key = (from.as_ref(), to.as_ref());

        match self.migrations.get(&key) {
            Some(migration_fn) => migration_fn(json),
            None => bail!(
                "No migration registered from version '{}' to '{}'. Available migrations: v1.0 -> v1.0 (identity)",
                from, to
            ),
        }
    }
}

impl Default for MigrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse and normalize a version string.
///
/// Ensures version strings follow the "major.minor" format.
/// For now, we only support major version 1 (v1.x series).
pub fn parse_version(version: &str) -> Result<(u32, u32)> {
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
pub fn validate_migration(from: &str, to: &str) -> Result<()> {
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

/// Convenience function for migrating a JSON value between schema versions.
///
/// This function creates a new registry and runs the migration.
/// For repeated migrations, creating a `MigrationRegistry` instance is more efficient.
///
/// # Arguments
///
/// * `from` - Source schema version (e.g., "1.0")
/// * `to` - Target schema version (e.g., "1.0", "1.1")
/// * `json` - JSON value to migrate
///
/// # Returns
///
/// Returns the migrated JSON value, or an error if the migration fails.
pub fn migrate(from: &str, to: &str, json: Value) -> Result<Value> {
    let registry = MigrationRegistry::new();
    registry.migrate(from, to, json)
}

/// Read JSON from a file path or stdin.
pub fn read_json(path: &str) -> Result<Value> {
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
pub fn write_json(path: &str, json: &Value, pretty: bool) -> Result<()> {
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

/// Run a schema migration.
///
/// # Arguments
///
/// * `from` - Source schema version (e.g., "1.0")
/// * `to` - Target schema version (e.g., "1.0", "1.1")
/// * `input` - Input JSON file path ("-" for stdin)
/// * `output` - Output JSON file path ("-" for stdout)
/// * `pretty` - Whether to pretty-print the output
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if the migration fails.
pub fn run_migration(from: &str, to: &str, input: &str, output: &str, pretty: bool) -> Result<()> {
    // Create migration registry
    let registry = MigrationRegistry::new();

    // Check if the specific migration exists
    if !registry.has_migration(from, to) {
        // Give a helpful error message
        if from == to {
            // Same version should always be supported
            bail!(
                "Identity migration for v{} is missing from registry - this is a bug",
                from
            );
        } else {
            bail!(
                "Migration from v{} to v{} is not yet implemented. Available migrations: v1.0 -> v1.0 (identity)",
                from, to
            );
        }
    }

    // Read input JSON
    let json_value = read_json(input)?;

    // Perform migration
    let mut migrated_json = registry
        .migrate(from, to, json_value)
        .with_context(|| format!("Migration from v{} to v{} failed", from, to))?;

    // Update schema_version field if it exists and versions differ
    if from != to {
        if let Some(obj) = migrated_json.as_object_mut() {
            // Update schema_version to the target version
            obj.insert("schema_version".to_string(), Value::String(to.to_string()));
        }
    }

    // Write output JSON
    write_json(output, &migrated_json, pretty)?;

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

    #[test]
    fn test_migrate_convenience_function() {
        let input = json!({"test": "value"});
        let result = migrate("1.0", "1.0", input.clone()).unwrap();
        assert_eq!(input, result);
    }

    #[test]
    fn test_migration_registry_default() {
        let registry = MigrationRegistry::default();
        assert!(registry.has_migration("1.0", "1.0"));
    }
}
