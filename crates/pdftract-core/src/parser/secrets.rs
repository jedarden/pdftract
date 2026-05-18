//! Secret handling utilities for pdftract.
//!
//! This module provides types and helpers for managing sensitive values
//! (passwords, tokens, etc.) that must never be logged or debug-printed.
//!
//! # CI Check Requirement
//!
//! Per pdftract-5l9m, CI MUST include a check that rejects unauthorized
//! `expose_secret()` call sites. The only legitimate uses of `expose_secret()`
//! are:
//! - PDF decryptor (when PDF decryption is implemented)
//! - Auth header constructor (for MCP bearer tokens)
//! - Basic-auth header builder (for HTTP basic-auth passwords)
//! - `SecretFingerprint::from_secret()` (for audit logging - this module)
//!
//! CI should run: `rg "expose_secret\(\)" crates/ --type rust` and fail the
//! build if any matches are found outside of these approved locations.

use secrecy::{SecretString, ExposeSecret};
use sha2::{Digest, Sha256};

/// A fingerprint of a secret value for use in audit logs.
///
/// This type wraps a SHA-256 hash of a secret, allowing audit logs to
/// correlate secret usage without exposing the actual value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretFingerprint(String);

impl SecretFingerprint {
    /// Create a fingerprint from a secret string.
    ///
    /// The fingerprint is a hex-encoded SHA-256 hash of the secret value.
    /// This allows audit logs to verify that the same secret was used
    /// across multiple operations without ever logging the secret itself.
    pub fn from_secret(secret: &SecretString) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(secret.expose_secret().as_bytes());
        let result = hasher.finalize();
        Self(hex::encode(result))
    }

    /// Create a fingerprint from a string slice.
    pub fn from_str(s: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(s.as_bytes());
        let result = hasher.finalize();
        Self(hex::encode(result))
    }

    /// Get the hex-encoded fingerprint value.
    pub fn as_hex(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SecretFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_consistency() {
        let secret1 = SecretString::new("password123".to_string().into());
        let secret2 = SecretString::new("password123".to_string().into());
        let secret3 = SecretString::new("different".to_string().into());

        let fp1 = SecretFingerprint::from_secret(&secret1);
        let fp2 = SecretFingerprint::from_secret(&secret2);
        let fp3 = SecretFingerprint::from_secret(&secret3);

        assert_eq!(fp1, fp2, "same secret produces same fingerprint");
        assert_ne!(fp1, fp3, "different secrets produce different fingerprints");
    }

    #[test]
    fn test_fingerprint_from_str() {
        let fp1 = SecretFingerprint::from_str("test");
        let fp2 = SecretFingerprint::from_str("test");
        let fp3 = SecretFingerprint::from_str("other");

        assert_eq!(fp1, fp2);
        assert_ne!(fp1, fp3);
    }

    #[test]
    fn test_fingerprint_display() {
        let fp = SecretFingerprint::from_str("test");
        let display = format!("{}", fp);
        assert!(!display.contains("test"), "fingerprint doesn't contain secret");
        assert_eq!(display.len(), 64, "SHA-256 produces 64 hex chars");
    }
}
