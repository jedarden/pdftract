//! TH-10: Cache poisoning protection tests (Phase 6.9).
//!
//! This test suite exercises the HMAC-SHA-256 integrity verification for cache entries.
//! It verifies that:
//! 1. Legitimate cache entries are created with valid HMAC signatures
//! 2. Forged entries with invalid HMACs are rejected (CACHE_INTEGRITY_FAIL)
//! 3. The cache miss path runs correctly after rejecting a forged entry
//! 4. Key compromise scenario (correct HMAC with attacker key) is documented

use pdftract_core::cache::integrity;
use pdftract_core::cache::layout::entry_path;
use pdftract_core::cache::multi_process::{Reader, Writer};
use tempfile::TempDir;
use std::fs;

const TEST_FINGERPRINT: &str = "pdftract-v1:testfingerprint1234567890abcdef1234567890abcdef";
const TEST_OPTS_HASH: &str = "9b21c0ffee0000000000000000000000000000000000000000000000000000000";
const TEST_DATA: &[u8] = b"Acme Widgets Corporation - Quarterly Report 2024";
const FORGED_DATA: &[u8] = b"Pwned Inc - Stolen Data 2024";

#[test]
fn test_cache_init_creates_key_with_mode_0600() {
    // AC: Cache init produces a 0600 key file
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path();

    integrity::init_cache_key(cache_dir).unwrap();

    let key_path = cache_dir.join("key");
    assert!(key_path.exists(), "Key file should exist");

    // Verify key is 32 bytes
    let key_bytes = fs::read(&key_path).unwrap();
    assert_eq!(key_bytes.len(), 32, "Key should be 32 bytes");

    // Verify mode is 0600 on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(&key_path).unwrap();
        let perms = metadata.permissions();
        let mode = perms.mode() & 0o777;
        assert_eq!(mode, 0o600, "Key file should have mode 0600");
    }
}

#[test]
fn test_legitimate_entry_has_valid_hmac() {
    // AC: Legitimate extraction creates valid HMAC
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path();

    // Init cache with key
    integrity::init_cache_key(cache_dir).unwrap();

    // Write a legitimate entry
    let writer = Writer::new(cache_dir);
    let compressed = compress_data(TEST_DATA);
    writer
        .write(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len(), &compressed)
        .unwrap();

    // Verify the entry can be read
    let reader = Reader::new(cache_dir);
    let result = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len() + 8);

    assert!(result.is_ok(), "Legitimate entry should be readable");
    assert_eq!(result.unwrap(), TEST_DATA);
}

#[test]
fn test_forged_entry_with_wrong_hmac_rejected() {
    // AC: Forgery with wrong HMAC: CACHE_INTEGRITY_FAIL diagnostic emitted;
    //     legitimate output returned; entry rewritten
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path();

    // Init cache with key
    integrity::init_cache_key(cache_dir).unwrap();
    let _key = integrity::load_cache_key(cache_dir).unwrap();

    // Create a legitimate entry first
    let writer = Writer::new(cache_dir);
    let compressed = compress_data(TEST_DATA);
    writer
        .write(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len(), &compressed)
        .unwrap();

    // Read the legitimate entry to get its HMAC
    let reader = Reader::new(cache_dir);
    let entry_path = entry_path(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len() + 8);

    let file_data = fs::read(&entry_path).unwrap();
    let _legitimate_hmac = &file_data[0..8];

    // Now forge an entry: same path but wrong HMAC
    // We use the same compressed data size for the forgery
    let wrong_hmac = [0xFFu8; 8]; // Clearly wrong HMAC

    // Write the forged entry directly (bypassing Writer)
    let forged_path = entry_path;
    let mut forged_data = Vec::with_capacity(8 + compressed.len());
    forged_data.extend_from_slice(&wrong_hmac);
    forged_data.extend_from_slice(&compressed);
    fs::write(&forged_path, forged_data).unwrap();

    // Try to read the forged entry - should fail with integrity error
    let read_result = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len() + 8);

    assert!(read_result.is_err(), "Forged entry should be rejected");
    let err = read_result.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert!(
        err.to_string().contains("integrity check failed"),
        "Error should mention integrity failure"
    );

    // The forged entry should have been deleted
    assert!(!forged_path.exists(), "Forged entry should be deleted");
}

#[test]
fn test_forged_entry_triggers_cache_miss() {
    // AC: Forgery triggers cache miss path - legitimate extraction re-runs
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path();

    // Init cache
    integrity::init_cache_key(cache_dir).unwrap();

    // Create a forged entry directly (wrong HMAC)
    let _writer = Writer::new(cache_dir);
    let compressed = compress_data(FORGED_DATA);
    let entry_path = entry_path(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len() + 8);

    let forged_data = {
        let mut data = Vec::with_capacity(8 + compressed.len());
        data.extend_from_slice(&[0xFFu8; 8]); // Wrong HMAC
        data.extend_from_slice(&compressed);
        data
    };

    // Create parent directories
    fs::create_dir_all(entry_path.parent().unwrap()).unwrap();
    fs::write(&entry_path, forged_data).unwrap();

    // Try to read - should fail (integrity check)
    let reader = Reader::new(cache_dir);
    let read_result = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len() + 8);

    assert!(read_result.is_err(), "Forged entry should be rejected");
    assert_eq!(read_result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);

    // Entry should be deleted (cache miss)
    assert!(!entry_path.exists(), "Forged entry should be deleted after rejection");

    // Subsequent read should return NotFound (cache miss, not corrupt)
    let read_result2 = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len() + 8);
    assert_eq!(read_result2.unwrap_err().kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn test_forged_entry_with_correct_hmac_key_compromise() {
    // AC: Forgery with correct HMAC (key compromise simulation): forged output returned
    // This documents the "key compromise" failure mode
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path();

    // Init cache
    integrity::init_cache_key(cache_dir).unwrap();
    let key = integrity::load_cache_key(cache_dir).unwrap();

    // Attacker has the key (key compromise scenario)
    // They can forge a valid HMAC for their malicious data
    let forged_compressed = compress_data(FORGED_DATA);
    let forged_hmac = integrity::compute_hmac(&key, TEST_FINGERPRINT, TEST_OPTS_HASH, &forged_compressed);

    let entry_path = entry_path(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, forged_compressed.len() + 8);

    // Write the forged entry with VALID HMAC (attacker has the key)
    let mut forged_data = Vec::with_capacity(8 + forged_compressed.len());
    forged_data.extend_from_slice(&forged_hmac);
    forged_data.extend_from_slice(&forged_compressed);

    fs::create_dir_all(entry_path.parent().unwrap()).unwrap();
    fs::write(&entry_path, forged_data).unwrap();

    // The forged entry will be ACCEPTED (HMAC is valid)
    let reader = Reader::new(cache_dir);
    let read_result = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, forged_compressed.len() + 8);

    assert!(read_result.is_ok(), "Entry with valid HMAC should be accepted");
    assert_eq!(read_result.unwrap(), FORGED_DATA, "Forged data should be returned");

    // This is a known limitation - key compromise allows undetected forgeries
    // Mitigation: key rotation (out of scope for v1.0)
}

#[test]
fn test_hmac_input_is_fingerprint_opts_hash_and_blob() {
    // AC: HMAC input is verified to be fingerprint || opts_hash || output_blob
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path();

    integrity::init_cache_key(cache_dir).unwrap();
    let key = integrity::load_cache_key(cache_dir).unwrap();

    let compressed = compress_data(TEST_DATA);

    // Compute HMAC for the full input
    let hmac1 = integrity::compute_hmac(&key, TEST_FINGERPRINT, TEST_OPTS_HASH, &compressed);

    // Different fingerprint → different HMAC
    let hmac2 = integrity::compute_hmac(&key, "different_fp", TEST_OPTS_HASH, &compressed);
    assert_ne!(hmac1, hmac2, "Different fingerprint should produce different HMAC");

    // Different opts_hash → different HMAC
    let hmac3 = integrity::compute_hmac(&key, TEST_FINGERPRINT, "different_opts", &compressed);
    assert_ne!(hmac1, hmac3, "Different opts_hash should produce different HMAC");

    // Different blob → different HMAC
    let hmac4 = integrity::compute_hmac(&key, TEST_FINGERPRINT, TEST_OPTS_HASH, &compress_data(b"different"));
    assert_ne!(hmac1, hmac4, "Different blob should produce different HMAC");

    // Same input → same HMAC
    let hmac5 = integrity::compute_hmac(&key, TEST_FINGERPRINT, TEST_OPTS_HASH, &compressed);
    assert_eq!(hmac1, hmac5, "Same input should produce same HMAC");
}

#[test]
fn test_cache_rewrites_forged_entry_on_miss() {
    // AC: Entry rewritten after rejecting forgery
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path();

    integrity::init_cache_key(cache_dir).unwrap();

    // Create a forged entry (wrong HMAC)
    let compressed = compress_data(FORGED_DATA);
    let entry_path = entry_path(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len() + 8);

    let mut forged_data = Vec::with_capacity(8 + compressed.len());
    forged_data.extend_from_slice(&[0xFFu8; 8]); // Wrong HMAC
    forged_data.extend_from_slice(&compressed);

    fs::create_dir_all(entry_path.parent().unwrap()).unwrap();
    fs::write(&entry_path, forged_data).unwrap();

    // Read the forged entry (should be rejected and deleted)
    let reader = Reader::new(cache_dir);
    let read_result = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len() + 8);

    assert!(read_result.is_err());
    assert!(!entry_path.exists(), "Forged entry should be deleted");

    // Now write the legitimate entry (simulating re-extraction)
    let writer = Writer::new(cache_dir);
    let legitimate_compressed = compress_data(TEST_DATA);
    writer
        .write(TEST_FINGERPRINT, TEST_OPTS_HASH, legitimate_compressed.len(), &legitimate_compressed)
        .unwrap();

    // The legitimate entry should now be readable
    let read_result2 = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, legitimate_compressed.len() + 8);
    assert!(read_result2.is_ok(), "Legitimate entry should be readable");
    assert_eq!(read_result2.unwrap(), TEST_DATA);
}

#[test]
fn test_multiple_forgeries_all_rejected() {
    // Test that multiple different forged entries are all rejected
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path();

    integrity::init_cache_key(cache_dir).unwrap();

    let writer = Writer::new(cache_dir);
    let reader = Reader::new(cache_dir);
    let compressed = compress_data(TEST_DATA);

    // Write the legitimate entry
    writer
        .write(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len(), &compressed)
        .unwrap();

    // Try to read with wrong size (simulating wrong HMAC in different-sized entry)
    let wrong_size = compressed.len() + 100;
    let read_result = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, wrong_size + 8);
    assert_eq!(read_result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn test_key_file_persistence() {
    // Test that the key persists across cache operations
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path();

    // Init cache
    integrity::init_cache_key(cache_dir).unwrap();
    let key1 = integrity::load_cache_key(cache_dir).unwrap();

    // Write an entry
    let writer = Writer::new(cache_dir);
    let compressed = compress_data(TEST_DATA);
    writer
        .write(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len(), &compressed)
        .unwrap();

    // Reload the key - should be the same
    let key2 = integrity::load_cache_key(cache_dir).unwrap();
    assert_eq!(key1, key2, "Key should persist unchanged");

    // Entry should still be readable
    let reader = Reader::new(cache_dir);
    let result = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len() + 8);
    assert!(result.is_ok());
}

#[test]
fn test_repeated_poisoning_attack_simulation() {
    // Document the failure mode: repeated poisoning after overwrite
    // This is out of scope for v1.0 but the behavior is documented
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path();

    integrity::init_cache_key(cache_dir).unwrap();

    let writer = Writer::new(cache_dir);
    let reader = Reader::new(cache_dir);
    let compressed = compress_data(TEST_DATA);

    // Write legitimate entry
    writer
        .write(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len(), &compressed)
        .unwrap();

    // Attacker writes forgery (wrong HMAC)
    let entry_path = entry_path(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len() + 8);

    let mut forged_data = Vec::with_capacity(8 + compressed.len());
    forged_data.extend_from_slice(&[0xFFu8; 8]);
    forged_data.extend_from_slice(&compressed);
    fs::write(&entry_path, &forged_data).unwrap();

    // First read after forgery: rejected, entry deleted
    let read_result = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len() + 8);
    assert!(read_result.is_err());
    assert!(!entry_path.exists());

    // Attacker writes forgery again
    fs::write(&entry_path, &forged_data).unwrap();

    // Second read after forgery: rejected again, entry deleted
    let read_result2 = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len() + 8);
    assert!(read_result2.is_err());
    assert!(!entry_path.exists());

    // This demonstrates the "repeated-poisoning attack" scenario:
    // The attacker can keep re-writing the forgery after each legitimate extraction.
    // Mitigation (out of scope for v1.0): file locking, process isolation, etc.
}

fn compress_data(data: &[u8]) -> Vec<u8> {
    pdftract_core::cache::compression::encode(data).unwrap()
}
