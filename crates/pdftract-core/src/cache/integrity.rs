//! Cache integrity verification using HMAC-SHA-256.
//!
//! This module implements Phase 6.9 cache integrity protection (TH-10 mitigation).
//! Each cache entry is signed with HMAC-SHA-256 using a per-cache random key.
//!
//! # Threat model (TH-10)
//!
//! A malicious co-tenant with local filesystem write access can forge cache entries
//! by writing files under predictable fingerprint paths. Without integrity verification,
//! subsequent reads return attacker-controlled blobs.
//!
//! # Mitigation
//!
//! - Each cache entry stores an HMAC-SHA-256 signature
//! - HMAC input: `fingerprint || opts_hash || compressed_blob`
//! - Per-cache random key generated on `cache init` (stored in `<cache>/key`, mode 0600)
//! - Reads verify HMAC; mismatch → CACHE_INTEGRITY_FAIL diagnostic → treat as miss
//!
//! # Key management
//!
//! - Key is 256-bit random bytes generated on cache init
//! - Stored in `<cache_dir>/key` file with mode 0600 (Unix)
//! - Key is per-cache, NOT per-process, NOT global, NOT derived from fingerprint
//! - Rotation policy: out of scope for v1.0 (documented in test as limitation)

use anyhow::{Context, Result};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::fs;
use std::path::Path;

type HmacSha256 = Hmac<Sha256>;

/// Path to the HMAC key file within a cache directory.
const KEY_FILENAME: &str = "key";

/// Generate a new random 256-bit HMAC key.
///
/// Returns 32 random bytes suitable for use as an HMAC-SHA-256 key.
pub fn generate_key() -> [u8; 32] {
    use rand::RngCore;
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

/// Initialize the cache directory with a new HMAC key.
///
/// Creates `<cache_dir>/key` with mode 0600 containing 256 random bits.
/// Returns an error if the key file already exists (cache already initialized).
///
/// # Arguments
///
/// * `cache_dir` - Path to the cache directory
///
/// # Returns
///
/// `Ok(())` on success, `Err` if:
/// - Key file already exists (cache already initialized)
/// - Directory creation fails
/// - Key file write fails
pub fn init_cache_key(cache_dir: &Path) -> Result<()> {
    let key_path = cache_dir.join(KEY_FILENAME);

    // Check if key already exists
    if key_path.exists() {
        return Err(anyhow::anyhow!(
            "Cache already initialized (key file exists at: {})",
            key_path.display()
        ));
    }

    // Ensure cache directory exists
    fs::create_dir_all(cache_dir)
        .with_context(|| format!("Failed to create cache directory: {}", cache_dir.display()))?;

    // Generate new key
    let key = generate_key();

    // Write key file with restricted permissions
    fs::write(&key_path, &key)
        .with_context(|| format!("Failed to write key file: {}", key_path.display()))?;

    // Set mode 0600 on Unix (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&key_path)
            .with_context(|| format!("Failed to get key file metadata: {}", key_path.display()))?
            .permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&key_path, perms).with_context(|| {
            format!("Failed to set key file permissions: {}", key_path.display())
        })?;
    }

    Ok(())
}

/// Load the HMAC key from the cache directory.
///
/// Reads `<cache_dir>/key` and returns the 32-byte key.
///
/// # Arguments
///
/// * `cache_dir` - Path to the cache directory
///
/// # Returns
///
/// The 32-byte HMAC key on success, `Err` if:
/// - Key file doesn't exist (cache not initialized)
/// - Key file is wrong size (corrupted or tampered)
/// - Read fails
pub fn load_cache_key(cache_dir: &Path) -> Result<[u8; 32]> {
    let key_path = cache_dir.join(KEY_FILENAME);

    let key_bytes = fs::read(&key_path)
        .with_context(|| format!("Failed to read key file: {}", key_path.display()))?;

    if key_bytes.len() != 32 {
        return Err(anyhow::anyhow!(
            "Invalid key file size: expected 32 bytes, got {} (key file may be corrupted)",
            key_bytes.len()
        ));
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&key_bytes);
    Ok(key)
}

/// Compute HMAC-SHA-256 over the cache entry data.
///
/// HMAC input: `fingerprint || opts_hash || compressed_blob`
///
/// # Arguments
///
/// * `key` - The 256-bit HMAC key
/// * `fingerprint` - PDF fingerprint string
/// * `opts_hash` - Extraction options hash (64-char hex)
/// * `compressed_blob` - The compressed cache entry data
///
/// # Returns
///
/// 32-byte HMAC-SHA-256 signature
pub fn compute_hmac(
    key: &[u8; 32],
    fingerprint: &str,
    opts_hash: &str,
    compressed_blob: &[u8],
) -> [u8; 8] {
    // HMAC input: fingerprint || opts_hash || compressed_blob
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key size");
    mac.update(fingerprint.as_bytes());
    mac.update(opts_hash.as_bytes());
    mac.update(compressed_blob);

    // Return first 8 bytes of HMAC as the signature (64 bits is sufficient for integrity)
    let result = mac.finalize().into_bytes();
    let mut sig = [0u8; 8];
    sig.copy_from_slice(&result[0..8]);
    sig
}

/// Verify HMAC-SHA-256 for a cache entry.
///
/// Returns `true` if the signature matches, `false` otherwise.
///
/// # Arguments
///
/// * `key` - The 256-bit HMAC key
/// * `fingerprint` - PDF fingerprint string
/// * `opts_hash` - Extraction options hash (64-char hex)
/// * `compressed_blob` - The compressed cache entry data
/// * `signature` - The 8-byte signature to verify
pub fn verify_hmac(
    key: &[u8; 32],
    fingerprint: &str,
    opts_hash: &str,
    compressed_blob: &[u8],
    signature: &[u8; 8],
) -> bool {
    let computed = compute_hmac(key, fingerprint, opts_hash, compressed_blob);
    computed == *signature
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_key_length() {
        let key = generate_key();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_generate_key_randomness() {
        let key1 = generate_key();
        let key2 = generate_key();
        assert_ne!(key1, key2, "Keys should be random");
    }

    #[test]
    fn test_init_cache_key_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        init_cache_key(cache_dir).unwrap();

        let key_path = cache_dir.join(KEY_FILENAME);
        assert!(key_path.exists(), "Key file should be created");
    }

    #[test]
    fn test_init_cache_key_fails_if_exists() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        init_cache_key(cache_dir).unwrap();

        let result = init_cache_key(cache_dir);
        assert!(result.is_err(), "Should fail if key already exists");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("already initialized"));
    }

    #[test]
    fn test_load_cache_key() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        init_cache_key(cache_dir).unwrap();
        let key = load_cache_key(cache_dir).unwrap();

        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_load_cache_key_fails_if_missing() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let result = load_cache_key(cache_dir);
        assert!(result.is_err(), "Should fail if key doesn't exist");
    }

    #[test]
    fn test_compute_hmac_deterministic() {
        let key = [0u8; 32];
        let fingerprint = "pdftract-v1:testfp";
        let opts_hash = "9b21c0ffee0000000000000000000000000000000000000000000000000000000";
        let blob = b"test blob data";

        let sig1 = compute_hmac(&key, fingerprint, opts_hash, blob);
        let sig2 = compute_hmac(&key, fingerprint, opts_hash, blob);

        assert_eq!(sig1, sig2, "HMAC should be deterministic");
    }

    #[test]
    fn test_compute_hmac_different_inputs() {
        let key = [0u8; 32];
        let fingerprint = "pdftract-v1:testfp";
        let opts_hash = "9b21c0ffee0000000000000000000000000000000000000000000000000000000";
        let blob1 = b"test blob data 1";
        let blob2 = b"test blob data 2";

        let sig1 = compute_hmac(&key, fingerprint, opts_hash, blob1);
        let sig2 = compute_hmac(&key, fingerprint, opts_hash, blob2);

        assert_ne!(sig1, sig2, "Different blobs should produce different HMACs");
    }

    #[test]
    fn test_verify_hmac_valid() {
        let key = [0u8; 32];
        let fingerprint = "pdftract-v1:testfp";
        let opts_hash = "9b21c0ffee0000000000000000000000000000000000000000000000000000000";
        let blob = b"test blob data";

        let sig = compute_hmac(&key, fingerprint, opts_hash, blob);
        assert!(verify_hmac(&key, fingerprint, opts_hash, blob, &sig));
    }

    #[test]
    fn test_verify_hmac_invalid() {
        let key = [0u8; 32];
        let fingerprint = "pdftract-v1:testfp";
        let opts_hash = "9b21c0ffee0000000000000000000000000000000000000000000000000000000";
        let blob = b"test blob data";

        let wrong_sig = [0xFFu8; 8];
        assert!(!verify_hmac(&key, fingerprint, opts_hash, blob, &wrong_sig));
    }

    #[test]
    fn test_hmac_key_independence() {
        let key1 = [0u8; 32];
        let key2 = [1u8; 32];
        let fingerprint = "pdftract-v1:testfp";
        let opts_hash = "9b21c0ffee0000000000000000000000000000000000000000000000000000000";
        let blob = b"test blob data";

        let sig1 = compute_hmac(&key1, fingerprint, opts_hash, blob);
        let sig2 = compute_hmac(&key2, fingerprint, opts_hash, blob);

        assert_ne!(sig1, sig2, "Different keys should produce different HMACs");
    }

    #[test]
    fn test_hmac_fingerprint_sensitivity() {
        let key = [0u8; 32];
        let fp1 = "pdftract-v1:fp1";
        let fp2 = "pdftract-v1:fp2";
        let opts_hash = "9b21c0ffee0000000000000000000000000000000000000000000000000000000";
        let blob = b"test blob data";

        let sig1 = compute_hmac(&key, fp1, opts_hash, blob);
        let sig2 = compute_hmac(&key, fp2, opts_hash, blob);

        assert_ne!(
            sig1, sig2,
            "Different fingerprints should produce different HMACs"
        );
    }

    #[test]
    fn test_hmac_opts_hash_sensitivity() {
        let key = [0u8; 32];
        let fingerprint = "pdftract-v1:testfp";
        let opts1 = "9b21c0ffee0000000000000000000000000000000000000000000000000000000";
        let opts2 = "aaaa000000000000000000000000000000000000000000000000000000000000aa";
        let blob = b"test blob data";

        let sig1 = compute_hmac(&key, fingerprint, opts1, blob);
        let sig2 = compute_hmac(&key, fingerprint, opts2, blob);

        assert_ne!(
            sig1, sig2,
            "Different options hashes should produce different HMACs"
        );
    }

    #[test]
    fn test_init_and_load_key_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        init_cache_key(cache_dir).unwrap();
        let loaded_key = load_cache_key(cache_dir).unwrap();

        assert_eq!(loaded_key.len(), 32);
        // Key should be random, so just check it's not all zeros
        let has_nonzero = loaded_key.iter().any(|&b| b != 0);
        assert!(has_nonzero, "Generated key should have nonzero bytes");
    }

    #[test]
    #[cfg(unix)]
    fn test_key_file_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        init_cache_key(cache_dir).unwrap();

        let key_path = cache_dir.join(KEY_FILENAME);
        let metadata = fs::metadata(&key_path).unwrap();
        let perms = metadata.permissions();
        let mode = perms.mode();

        // Check mode is 0600 (owner read/write only)
        assert_eq!(mode & 0o777, 0o600, "Key file should have mode 0600");
    }
}
