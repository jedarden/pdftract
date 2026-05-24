//! Profile loading and validation.
//!
//! This module provides functionality for loading and validating extraction
//! profiles from YAML files. Profiles define extraction options, field mappings,
//! and output formatting rules.
//!
//! # Security
//!
//! Profile files are checked for forbidden secret keys (password, token, secret,
//! api_key, etc.) to prevent accidental publication of credentials in profiles
//! that are checked into source control. See [`ProfileSecretsForbidden`] for details.

mod loader;

pub use loader::{check_forbidden_keys, ForbiddenKeyError, ProfileLoadError};

use crate::diagnostics::DiagCode;

/// Diagnostic code for forbidden secret keys in profiles.
///
/// Emitted when a profile YAML contains keys that suggest credentials or secrets.
/// This is a security measure to prevent accidental publication of secrets in
/// profile files checked into source control.
pub const PROFILE_SECRETS_FORBIDDEN: DiagCode = DiagCode::ProfileSecretsForbidden;
