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
//!
//! # Document Type Profiles
//!
//! The [`types`] module defines the core types for document type classification
//! (Phase 5.6): [`ProfileType`], [`Profile`], and [`MatchPredicate`]. These
//! are the shared vocabulary between the rule engine, built-in profile definitions,
//! and user-authored YAML profiles.

mod loader;
mod types;

pub use loader::{check_forbidden_keys, ForbiddenKeyError, ProfileLoadError};
pub use types::{MatchPredicate, Profile, ProfileType};

use crate::diagnostics::DiagCode;

/// Diagnostic code for forbidden secret keys in profiles.
///
/// Emitted when a profile YAML contains keys that suggest credentials or secrets.
/// This is a security measure to prevent accidental publication of secrets in
/// profile files checked into source control.
pub const PROFILE_SECRETS_FORBIDDEN: DiagCode = DiagCode::ProfileSecretsForbidden;
