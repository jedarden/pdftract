//! Schema version migration library for pdftract JSON output.
//!
//! This module provides a public API for migrating pdftract JSON output
//! between minor versions of the schema. Following the plan's additive-evolution
//! rules, minor version changes are additive only (no field removal, no type changes).
//!
//! # Public API
//!
//! The main entry point is the [`migrate`] function:
//!
//! ```rust
//! use pdftract_schema_migrate::migrate;
//! use serde_json::json;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let input = json!({"schema_version": "1.0", "data": "test"});
//! let output = migrate("1.0", "1.0", input)?;
//! # Ok(())
//! # }
//! ```
//!
//! # Migration Registry
//!
//! Migrations are registered in a global registry mapping (from_version, to_version)
//! to migration functions. Each migration is a pure function that transforms a
//! [`serde_json::Value`] from one schema version to another.
//!
//! # Version Rules
//!
//! - Major version changes (v1 -> v2) are NOT allowed (breaking changes)
//! - Downgrades (v1.1 -> v1.0) are NOT allowed (data loss risk)
//! - Same version (v1.0 -> v1.0) is allowed (identity migration)
//! - Only v1.x migrations are currently supported
//!
//! # Adding New Migrations
//!
//! To add a new migration (e.g., v1.0 to v1.1):
//!
//! 1. Define the migration function with signature `fn(Value) -> Result<Value>`
//! 2. Register it in [`MigrationRegistry::new()`]
//! 3. Add tests for the migration

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::collections::HashMap;

/// Migrate JSON from one schema version to another.
///
/// This is the main public API entry point for schema migrations.
///
/// # Arguments
///
/// * `from_version` - Source schema version (e.g., "1.0", "1.1")
/// * `to_version` - Target schema version (e.g., "1.0", "1.1")
/// * `json` - Input JSON value to migrate
///
/// # Returns
///
/// Returns the migrated JSON value on success.
///
/// # Errors
///
/// Returns an error if:
/// - The version strings are invalid (not in "major.minor" format)
/// - Major version mismatch (v1.x to v2.y)
/// - Downgrade requested (v1.1 to v1.0)
/// - No migration is registered for the requested version pair
/// - The migration function itself fails
///
/// # Examples
///
/// ```rust
/// use pdftract_schema_migrate::migrate;
/// use serde_json::json;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Identity migration (1.0 -> 1.0)
/// let input = json!({"schema_version": "1.0", "data": "test"});
/// let output = migrate("1.0", "1.0", input.clone())?;
/// assert_eq!(input, output);
///
/// // Unsupported migration returns an error
/// let result = migrate("1.0", "1.1", json!({}));
/// assert!(result.is_err());
/// # Ok(())
/// # }
/// ```
pub fn migrate(from_version: &str, to_version: &str, json: Value) -> Result<Value> {
    // Validate that the migration direction is allowed
    validate_migration(from_version, to_version)?;

    // Create migration registry
    let registry = MigrationRegistry::new();

    // Check if the specific migration exists
    if !registry.has_migration(from_version, to_version) {
        // Give a helpful error message
        if from_version == to_version {
            // Same version should always be supported
            bail!(
                "Identity migration for v{} is missing from registry",
                from_version
            );
        } else {
            bail!(
                "No migration registered from v{} to v{}",
                from_version, to_version
            );
        }
    }

    // Perform migration
    let mut migrated_json = registry.migrate(from_version, to_version, json)?;

    // Update schema_version field if it exists and versions differ
    if from_version != to_version {
        if let Some(obj) = migrated_json.as_object_mut() {
            obj.insert("schema_version".to_string(), Value::String(to_version.to_string()));
        }
    }

    Ok(migrated_json)
}

/// Registry of available migrations.
///
/// Maps (from_version, to_version) to the migration function.
/// This is internal to the library - users should call the [`migrate()`] function instead.
pub struct MigrationRegistry {
    migrations: HashMap<(&'static str, &'static str), Box<dyn Fn(Value) -> Result<Value> + Send + Sync>>,
}

impl MigrationRegistry {
    /// Create a new registry with all known migrations registered.
    pub fn new() -> Self {
        let mut migrations: HashMap<(&'static str, &'static str), Box<dyn Fn(Value) -> Result<Value> + Send + Sync>> = HashMap::new();

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
                "No migration registered from version '{}' to '{}'",
                from, to
            ),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_migrate_identity() {
        let input = json!({
            "schema_version": "1.0",
            "test": "value"
        });

        let result = migrate("1.0", "1.0", input.clone()).unwrap();

        // Identity migration should return unchanged value
        assert_eq!(input, result);
    }

    #[test]
    fn test_migrate_unsupported() {
        let input = json!({"test": "value"});

        let result = migrate("1.0", "1.1", input);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No migration registered"));
    }

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
    }

    #[test]
    fn test_migration_registry_has_migration() {
        let registry = MigrationRegistry::new();

        assert!(registry.has_migration("1.0", "1.0"));
        assert!(!registry.has_migration("1.0", "1.1"));
        assert!(!registry.has_migration("2.0", "2.0"));
    }
}
