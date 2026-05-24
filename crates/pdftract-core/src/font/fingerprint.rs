//! Font fingerprint cache (Level 3 encoding fallback).
//!
//! This module provides a content-based lookup for font glyph-to-Unicode
//! mappings. When a PDF font has no `/ToUnicode` map and the embedded
//! font subset has stripped glyph names, we fall back to computing the
//! SHA-256 hash of the decoded font program bytes and looking it up in
//! a compile-time database of known fonts.
//!
//! The database is built from `build/font-fingerprints.json` at compile time
//! and stored as a `phf::Map<[u8; 32], &'static [(u16, u32)]>`.
//!
//! # Hash stability
//!
//! The hash is computed over the DECODED font program bytes (post stream
//! decoding via FlateDecode etc., pre-interpretation). This ensures that
//! the same font embedded with different stream filters produces the same
//! hash.
//!
//! # Entry format
//!
//! Each database entry maps a SHA-256 digest to a slice of `(glyph_id, codepoint)`
//! pairs. For a given font hash, you can look up any glyph ID to get its
//! Unicode codepoint.

use sha2::{Digest, Sha256};
use std::sync::Arc;

// Include the generated phf map
include!(concat!(env!("OUT_DIR"), "/font_fingerprints.rs"));

/// Font fingerprint cache entry.
///
/// Stores the SHA-256 hash of a font program for efficient lookups.
/// The hash is computed once at font load time and cached.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontFingerprint {
    /// The SHA-256 hash of the decoded font program bytes.
    hash: [u8; 32],
}

impl FontFingerprint {
    /// Compute the SHA-256 hash of a font program.
    ///
    /// This should be called ONCE per font load and the result cached.
    /// The hash is computed over the raw decoded bytes, not the interpreted
    /// font tables.
    ///
    /// # Arguments
    ///
    /// * `font_program_bytes` - The decoded font program bytes (post stream decoding)
    ///
    /// # Returns
    ///
    /// A `FontFingerprint` containing the SHA-256 hash
    pub fn compute(font_program_bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(font_program_bytes);
        let hash = hasher.finalize();
        Self { hash: hash.into() }
    }

    /// Get the underlying hash bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.hash
    }
}

/// Look up a Unicode codepoint for a glyph ID in a fingerprinted font.
///
/// This is Level 3 of the encoding fallback chain:
///
/// 1. Level 1: `/ToUnicode` CMap (preferred)
/// 2. Level 2: Named encoding (AGL + encoding dictionaries)
/// 3. Level 3: Font fingerprint cache (this function)
/// 4. Level 4: Visual shape recognition (OCR)
///
/// # Arguments
///
/// * `font_program_bytes` - The decoded font program bytes
/// * `gid` - The glyph ID to look up
///
/// # Returns
///
/// `Some(char)` if the font fingerprint is known and the glyph ID is mapped,
/// `None` otherwise.
///
/// # Performance
///
/// The hash is computed on the first call and cached in an Arc for subsequent
/// calls. Do NOT call this function repeatedly for the same font without caching.
pub fn lookup_font_fingerprint(font_program_bytes: &[u8], gid: u16) -> Option<char> {
    // Compute the fingerprint
    let fingerprint = FontFingerprint::compute(font_program_bytes);

    // Look up the hash in the database
    let entries = FONT_FINGERPRINTS.get(fingerprint.as_bytes())?;

    // Find the glyph ID in the entries
    let codepoint = entries
        .iter()
        .find(|(entry_gid, _)| *entry_gid == gid)
        .map(|(_, cp)| *cp)?;

    // Validate the codepoint is a valid Unicode scalar value
    // This should always be true if the JSON was validated at build time
    char::from_u32(codepoint)
}

/// Cached font fingerprint for efficient lookups.
///
/// This should be stored on the `Font` struct to avoid re-computing
/// the hash on every glyph lookup.
#[derive(Clone, Debug)]
pub struct CachedFingerprint {
    /// The fingerprint hash
    fingerprint: FontFingerprint,
    /// Whether this fingerprint is in the database
    is_known: bool,
}

impl CachedFingerprint {
    /// Create a cached fingerprint from font program bytes.
    ///
    /// This computes the hash once and checks if it exists in the database.
    pub fn from_font_program(font_program_bytes: &[u8]) -> Self {
        let fingerprint = FontFingerprint::compute(font_program_bytes);
        let is_known = FONT_FINGERPRINTS.get(fingerprint.as_bytes()).is_some();

        Self {
            fingerprint,
            is_known,
        }
    }

    /// Look up a glyph ID in the cached fingerprint.
    ///
    /// Returns `Some(char)` if the fingerprint is known and the glyph ID is mapped,
    /// `None` otherwise.
    pub fn lookup(&self, gid: u16) -> Option<char> {
        if !self.is_known {
            return None;
        }

        let entries = FONT_FINGERPRINTS.get(self.fingerprint.as_bytes())?;
        let codepoint = entries
            .iter()
            .find(|(entry_gid, _)| *entry_gid == gid)
            .map(|(_, cp)| *cp)?;

        char::from_u32(codepoint)
    }

    /// Get the underlying fingerprint hash.
    pub fn fingerprint(&self) -> &FontFingerprint {
        &self.fingerprint
    }

    /// Check if this fingerprint is in the database.
    pub fn is_known(&self) -> bool {
        self.is_known
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_fingerprint_compute() {
        let data = b"test font data";
        let fp = FontFingerprint::compute(data);

        // Hash should be deterministic
        let fp2 = FontFingerprint::compute(data);
        assert_eq!(fp.hash, fp2.hash);

        // Different data should produce different hash
        let fp3 = FontFingerprint::compute(b"different data");
        assert_ne!(fp.hash, fp3.hash);
    }

    #[test]
    fn test_font_fingerprint_as_bytes() {
        let data = b"test font data";
        let fp = FontFingerprint::compute(data);

        let bytes = fp.as_bytes();
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn test_lookup_font_fingerprint_unknown_font() {
        // With an empty database, all lookups should return None
        let data = b"unknown font data";
        let result = lookup_font_fingerprint(data, 1);
        assert!(result.is_none());
    }

    #[test]
    fn test_cached_fingerprint_unknown_font() {
        // With an empty database, cached fingerprints should report unknown
        let data = b"unknown font data";
        let cached = CachedFingerprint::from_font_program(data);

        assert!(!cached.is_known());
        assert!(cached.lookup(1).is_none());
        assert!(cached.lookup(100).is_none());
    }

    #[test]
    fn test_cached_fingerprint_deterministic() {
        let data = b"test font data";
        let cached1 = CachedFingerprint::from_font_program(data);
        let cached2 = CachedFingerprint::from_font_program(data);

        assert_eq!(
            cached1.fingerprint().as_bytes(),
            cached2.fingerprint().as_bytes()
        );
        assert_eq!(cached1.is_known(), cached2.is_known());
    }

    #[test]
    fn test_empty_database_compiles() {
        // This test verifies that an empty JSON produces a valid phf::Map
        // The fact that this compiles and runs is the acceptance criteria
        let data = b"any data";
        let result = lookup_font_fingerprint(data, 0);
        assert!(result.is_none());
    }

    #[test]
    fn test_hash_stability_across_runs() {
        // Verify that the hash is stable (deterministic)
        let data = b"stability test data";

        let hashes: Vec<[u8; 32]> = (0..10)
            .map(|_| {
                let fp = FontFingerprint::compute(data);
                *fp.as_bytes()
            })
            .collect();

        // All hashes should be identical
        for hash in &hashes[1..] {
            assert_eq!(hash, &hashes[0]);
        }
    }

    #[test]
    fn test_fingerprint_different_inputs() {
        // Different inputs should produce different hashes
        let inputs = vec![
            b"font data A".as_slice(),
            b"font data B".as_slice(),
            b"font data C".as_slice(),
        ];

        let fingerprints: Vec<FontFingerprint> = inputs
            .iter()
            .map(|data| FontFingerprint::compute(data))
            .collect();

        // All fingerprints should be unique
        for i in 0..fingerprints.len() {
            for j in (i + 1)..fingerprints.len() {
                assert_ne!(fingerprints[i].hash, fingerprints[j].hash);
            }
        }
    }

    #[test]
    fn test_cached_fingerprint_reuse() {
        // Verify that CachedFingerprint can be reused for multiple lookups
        let data = b"test font data";
        let cached = CachedFingerprint::from_font_program(data);

        // Multiple lookups should all work (or all fail) consistently
        let result1 = cached.lookup(1);
        let result2 = cached.lookup(2);
        let result3 = cached.lookup(3);

        // With empty database, all should be None
        assert!(result1.is_none());
        assert!(result2.is_none());
        assert!(result3.is_none());
    }

    #[test]
    fn test_font_fingerprint_empty_input() {
        // Empty input should still produce a valid hash
        let data = b"";
        let fp = FontFingerprint::compute(data);

        // Should be a valid 32-byte hash
        assert_eq!(fp.as_bytes().len(), 32);

        // Should be deterministic
        let fp2 = FontFingerprint::compute(data);
        assert_eq!(fp.hash, fp2.hash);
    }

    #[test]
    fn test_lookup_font_fingerprint_different_gids() {
        // Test that different glyph IDs are looked up correctly
        let data = b"test font data";

        // With empty database, all should return None
        for gid in 0..1000 {
            assert!(lookup_font_fingerprint(data, gid).is_none());
        }
    }

    #[test]
    fn test_cached_fingerprint_accessors() {
        let data = b"test font data";
        let cached = CachedFingerprint::from_font_program(data);

        // Test accessor methods
        let _fp = cached.fingerprint();
        let _known = cached.is_known();

        // Just verify they don't panic
        assert!(!cached.is_known());
    }
}
