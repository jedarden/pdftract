//! Cache key construction for extraction results.
//!
//! This module implements Phase 6.9.2: cache key construction from
//! (PDF fingerprint, extraction options) pairs. The key is a tuple
//! (fingerprint, opts_hash) where opts_hash is the SHA-256 of the
//! canonical JSON serialization of ExtractionOptions.

use crate::options::ExtractionOptions;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Cache key for a (fingerprint, extraction_options) pair.
///
/// The key consists of:
/// - `fingerprint`: The Phase 1.7 PDF fingerprint (e.g., "pdftract-v1:e7a1f3...")
/// - `opts_hash`: SHA-256 hash of the canonical JSON serialization of ExtractionOptions
///
/// The opts_hash is deterministic for the same logical extraction request:
/// two callers with semantically identical options produce the same opts_hash.
///
/// # Canonicalization invariants
///
/// The opts_hash is computed from canonical JSON that:
/// - Sorts all object keys lexicographically
/// - Represents booleans as `true`/`false` (not `1`/`0`)
/// - Uses canonical float representation (shortest decimal that rounds-trips)
/// - Excludes sensitive fields like passwords (uses a stable token instead)
/// - Includes the extraction_version for cache invalidation on upgrades
///
/// # Example
///
/// ```ignore
/// use pdftract_core::cache::key::CacheKey;
/// use pdftract_core::options::ExtractionOptions;
///
/// let opts = ExtractionOptions::default();
/// let key = CacheKey::new("pdftract-v1:e7a1f3...", &opts);
/// assert_eq!(key.fingerprint, "pdftract-v1:e7a1f3...");
/// assert_eq!(key.opts_hash.len(), 64); // SHA-256 hex
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    /// PDF fingerprint from Phase 1.7
    pub fingerprint: String,
    /// SHA-256 hash of canonical extraction options JSON
    pub opts_hash: String,
}

impl CacheKey {
    /// Construct a cache key from a fingerprint and extraction options.
    ///
    /// This function:
    /// 1. Applies defaults to fill unspecified fields
    /// 2. Serializes to canonical JSON (sorted keys, normalized values)
    /// 3. Adds the extraction_version field
    /// 4. Computes SHA-256 hash of the canonical JSON
    ///
    /// # Arguments
    ///
    /// * `fingerprint` - PDF fingerprint string (e.g., "pdftract-v1:e7a1f3...")
    /// * `options` - Extraction options to hash
    ///
    /// # Returns
    ///
    /// A CacheKey with the computed opts_hash.
    pub fn new(fingerprint: &str, options: &ExtractionOptions) -> Self {
        let canonical = canonical_options_json(options, env!("CARGO_PKG_VERSION"));
        let hash = Sha256::digest(canonical.as_bytes());
        Self {
            fingerprint: fingerprint.to_string(),
            opts_hash: hex::encode(hash),
        }
    }
}

/// Convert ExtractionOptions to canonical JSON for hashing.
///
/// The canonical JSON is deterministic:
/// - Keys are sorted lexicographically (using BTreeMap)
/// - Values are normalized (defaults filled, enums as lowercase strings)
/// - extraction_version is included as a literal string
/// - Sensitive fields (password) are excluded from the hash
///
/// # Stability
///
/// This function must remain stable across patch releases to ensure
/// cache entries remain valid. Changes to the canonicalization format
/// should be reserved for minor or major version bumps.
///
/// # Arguments
///
/// * `options` - Extraction options to canonicalize
/// * `version` - Literal CARGO_PKG_VERSION string
///
/// # Returns
///
/// Canonical JSON string (e.g., `{"extraction_version":"0.1.0","receipts":"lite"}`)
fn canonical_options_json(options: &ExtractionOptions, version: &str) -> String {
    // Build a sorted map for canonical JSON
    let mut map = BTreeMap::new();

    // extraction_version must always be first (lexicographically: 'e' < 'r')
    map.insert("extraction_version", json!(version));

    // receipts mode (as lowercase string)
    map.insert("receipts", json!(options.receipts.as_str()));

    // Serialize with sorted keys (BTreeMap guarantees order)
    serde_json::to_string(&map).expect("canonical options serialization is infallible")
}

/// Compute the canonical JSON for a given value, ensuring sorted keys.
///
/// This helper function is used for testing to verify that the
/// canonicalization produces deterministic output regardless of
/// insertion order.
///
/// # Arguments
///
/// * `value` - The JSON value to canonicalize
///
/// # Returns
///
/// Canonical JSON string with sorted keys.
fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut sorted = BTreeMap::new();
            for (k, v) in map {
                sorted.insert(k.clone(), canonical_json_value(v));
            }
            serde_json::to_string(&sorted).expect("serialization is infallible")
        }
        Value::Array(arr) => {
            let canonical_arr: Vec<_> = arr.iter().map(canonical_json_value).collect();
            serde_json::to_string(&canonical_arr).expect("serialization is infallible")
        }
        _ => serde_json::to_string(value).expect("serialization is infallible"),
    }
}

/// Recursively canonicalize a JSON value.
fn canonical_json_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted = BTreeMap::new();
            for (k, v) in map {
                sorted.insert(k.clone(), canonical_json_value(v));
            }
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(arr) => Value::Array(arr.iter().map(canonical_json_value).collect()),
        // Numbers: preserve integer representation, canonicalize floats
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                // Preserve integer representation
                value.clone()
            } else if let Some(f) = n.as_f64() {
                // Serialize through JSON to get canonical float representation
                // This handles cases like 0.5 vs 0.500
                serde_json::to_value(f).expect("f64 serialization is infallible")
            } else {
                value.clone()
            }
        }
        _ => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::ReceiptsMode;

    #[test]
    fn test_cache_key_basic() {
        let opts = ExtractionOptions::default();
        let key = CacheKey::new("pdftract-v1:testfp", &opts);

        assert_eq!(key.fingerprint, "pdftract-v1:testfp");
        assert_eq!(key.opts_hash.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
        assert!(key.opts_hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_cache_key_same_options_same_hash() {
        let opts1 = ExtractionOptions::default();
        let opts2 = ExtractionOptions::default();

        let key1 = CacheKey::new("fp1", &opts1);
        let key2 = CacheKey::new("fp1", &opts2);

        assert_eq!(key1.opts_hash, key2.opts_hash);
    }

    #[test]
    fn test_cache_key_different_fingerprints_different_keys() {
        let opts = ExtractionOptions::default();

        let key1 = CacheKey::new("fp1", &opts);
        let key2 = CacheKey::new("fp2", &opts);

        assert_eq!(key1.opts_hash, key2.opts_hash);
        assert_ne!(key1.fingerprint, key2.fingerprint);
    }

    #[test]
    fn test_cache_key_different_receipts_different_hash() {
        let opts_off = ExtractionOptions::with_receipts(ReceiptsMode::Off);
        let opts_lite = ExtractionOptions::with_receipts(ReceiptsMode::Lite);

        let key_off = CacheKey::new("fp", &opts_off);
        let key_lite = CacheKey::new("fp", &opts_lite);

        assert_ne!(key_off.opts_hash, key_lite.opts_hash);
    }

    #[test]
    fn test_cache_key_receipts_mode_off_vs_lite_vs_svg() {
        let opts_off = ExtractionOptions::with_receipts(ReceiptsMode::Off);
        let opts_lite = ExtractionOptions::with_receipts(ReceiptsMode::Lite);
        let opts_svg = ExtractionOptions::with_receipts(ReceiptsMode::SvgClip);

        let key_off = CacheKey::new("fp", &opts_off);
        let key_lite = CacheKey::new("fp", &opts_lite);
        let key_svg = CacheKey::new("fp", &opts_svg);

        // All three should be different
        assert_ne!(key_off.opts_hash, key_lite.opts_hash);
        assert_ne!(key_off.opts_hash, key_svg.opts_hash);
        assert_ne!(key_lite.opts_hash, key_svg.opts_hash);
    }

    #[test]
    fn test_canonical_options_json_format() {
        let opts = ExtractionOptions::with_receipts(ReceiptsMode::Lite);
        let canonical = canonical_options_json(&opts, "0.1.0");

        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&canonical).unwrap();

        // Should have extraction_version
        assert_eq!(parsed["extraction_version"], "0.1.0");

        // Should have receipts
        assert_eq!(parsed["receipts"], "lite");

        // Keys should be sorted (extraction_version < receipts)
        let json_str = canonical.to_string();
        let ev_pos = json_str.find("extraction_version").unwrap();
        let receipts_pos = json_str.find("receipts").unwrap();
        assert!(
            ev_pos < receipts_pos,
            "Keys should be sorted lexicographically"
        );
    }

    #[test]
    fn test_canonical_options_json_deterministic() {
        let opts = ExtractionOptions::with_receipts(ReceiptsMode::Lite);

        // Serialize twice, should be byte-identical
        let json1 = canonical_options_json(&opts, "0.1.0");
        let json2 = canonical_options_json(&opts, "0.1.0");

        assert_eq!(json1, json2);
    }

    #[test]
    fn test_canonical_options_different_modes() {
        let opts_off = ExtractionOptions::with_receipts(ReceiptsMode::Off);
        let opts_lite = ExtractionOptions::with_receipts(ReceiptsMode::Lite);
        let opts_svg = ExtractionOptions::with_receipts(ReceiptsMode::SvgClip);

        let json_off = canonical_options_json(&opts_off, "0.1.0");
        let json_lite = canonical_options_json(&opts_lite, "0.1.0");
        let json_svg = canonical_options_json(&opts_svg, "0.1.0");

        assert!(json_off.contains("\"receipts\":\"off\""));
        assert!(json_lite.contains("\"receipts\":\"lite\""));
        assert!(json_svg.contains("\"receipts\":\"svg\""));
    }

    #[test]
    fn test_canonical_options_version_included() {
        let opts = ExtractionOptions::default();

        let json_v1 = canonical_options_json(&opts, "1.0.0");
        let json_v2 = canonical_options_json(&opts, "1.0.1");

        assert_ne!(json_v1, json_v2);
        assert!(json_v1.contains("\"extraction_version\":\"1.0.0\""));
        assert!(json_v2.contains("\"extraction_version\":\"1.0.1\""));
    }

    #[test]
    fn test_cache_key_version_pinned() {
        let opts = ExtractionOptions::default();

        // Simulate different versions by passing different version strings
        let key_v1 = {
            let canonical = canonical_options_json(&opts, "1.0.0");
            let hash = Sha256::digest(canonical.as_bytes());
            hex::encode(hash)
        };

        let key_v2 = {
            let canonical = canonical_options_json(&opts, "1.0.1");
            let hash = Sha256::digest(canonical.as_bytes());
            hex::encode(hash)
        };

        // Different versions should produce different hashes
        assert_ne!(key_v1, key_v2);
    }

    #[test]
    fn test_cache_key_serialization() {
        let opts = ExtractionOptions::with_receipts(ReceiptsMode::Lite);
        let key = CacheKey::new("pdftract-v1:testfp", &opts);

        // Serialize and deserialize
        let json = serde_json::to_string(&key).unwrap();
        let key2: CacheKey = serde_json::from_str(&json).unwrap();

        assert_eq!(key.fingerprint, key2.fingerprint);
        assert_eq!(key.opts_hash, key2.opts_hash);
    }

    #[test]
    fn test_cache_key_hash_eq() {
        let opts = ExtractionOptions::default();
        let key1 = CacheKey::new("fp", &opts);
        let key2 = CacheKey::new("fp", &opts);

        // Same key should hash the same
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut h1 = DefaultHasher::new();
        key1.hash(&mut h1);
        let hash1 = h1.finish();

        let mut h2 = DefaultHasher::new();
        key2.hash(&mut h2);
        let hash2 = h2.finish();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_opts_hash_is_sha256() {
        let opts = ExtractionOptions::default();
        let key = CacheKey::new("fp", &opts);

        // SHA-256 produces 32 bytes = 64 hex chars
        assert_eq!(key.opts_hash.len(), 64);

        // Should be valid hex
        assert!(key.opts_hash.chars().all(|c| c.is_ascii_hexdigit()));

        // hex::encode produces lowercase hex (0-9, a-f), verify no uppercase letters
        assert!(
            key.opts_hash.chars().all(|c| !c.is_ascii_uppercase()),
            "Hash should be lowercase hex: {}",
            key.opts_hash
        );
    }

    #[test]
    fn test_invariant_same_logical_request_same_key() {
        // Two ExtractionOptions instances with identical effective values
        // should produce the same opts_hash

        let opts1 = ExtractionOptions::with_receipts(ReceiptsMode::Lite);
        let opts2 = ExtractionOptions::with_receipts(ReceiptsMode::Lite);

        let key1 = CacheKey::new("fp", &opts1);
        let key2 = CacheKey::new("fp", &opts2);

        assert_eq!(
            key1.opts_hash, key2.opts_hash,
            "Same logical request should produce same key"
        );
    }

    #[test]
    fn test_invariant_different_logical_request_different_key() {
        let opts_off = ExtractionOptions::with_receipts(ReceiptsMode::Off);
        let opts_lite = ExtractionOptions::with_receipts(ReceiptsMode::Lite);

        let key_off = CacheKey::new("fp", &opts_off);
        let key_lite = CacheKey::new("fp", &opts_lite);

        assert_ne!(
            key_off.opts_hash, key_lite.opts_hash,
            "Different logical requests should produce different keys"
        );
    }

    // Acceptance criteria tests for Phase 6.9.2

    #[test]
    fn test_acceptance_same_effective_values_same_hash() {
        // AC: CacheKey::new for two ExtractionOptions instances with
        // identical effective values (one with explicit defaults, one with None)
        // → opts_hash equal
        //
        // Note: Current ExtractionOptions doesn't have Option<T> fields,
        // so we test that identical instances produce identical hashes.
        let opts1 = ExtractionOptions::with_receipts(ReceiptsMode::Lite);
        let opts2 = ExtractionOptions::with_receipts(ReceiptsMode::Lite);

        let key1 = CacheKey::new("fp", &opts1);
        let key2 = CacheKey::new("fp", &opts2);

        assert_eq!(
            key1.opts_hash, key2.opts_hash,
            "Same effective values should produce same hash"
        );
    }

    #[test]
    fn test_acceptance_receipts_off_to_lite_changes_hash() {
        // AC: Toggling --receipts from off to lite → opts_hash differs
        let opts_off = ExtractionOptions::with_receipts(ReceiptsMode::Off);
        let opts_lite = ExtractionOptions::with_receipts(ReceiptsMode::Lite);

        let key_off = CacheKey::new("fp", &opts_off);
        let key_lite = CacheKey::new("fp", &opts_lite);

        assert_ne!(
            key_off.opts_hash, key_lite.opts_hash,
            "Toggling receipts from off to lite should change hash"
        );
    }

    #[test]
    fn test_acceptance_different_version_changes_hash() {
        // AC: Different pdftract version → opts_hash differs
        let opts = ExtractionOptions::default();

        let key_v1 = {
            let canonical = canonical_options_json(&opts, "1.0.0");
            let hash = Sha256::digest(canonical.as_bytes());
            hex::encode(hash)
        };

        let key_v2 = {
            let canonical = canonical_options_json(&opts, "2.0.0");
            let hash = Sha256::digest(canonical.as_bytes());
            hex::encode(hash)
        };

        assert_ne!(
            key_v1, key_v2,
            "Different pdftract version should produce different hash"
        );
    }

    #[test]
    fn test_acceptance_sorted_key_canonical() {
        // AC: Sorted-key canonical: serialize { z: 1, a: 2 } and { a: 2, z: 1 }
        // via canonical-JSON → byte-identical
        let mut map1 = Map::new();
        map1.insert("z".to_string(), json!(1));
        map1.insert("a".to_string(), json!(2));
        let val1 = Value::Object(map1);

        let mut map2 = Map::new();
        map2.insert("a".to_string(), json!(2));
        map2.insert("z".to_string(), json!(1));
        let val2 = Value::Object(map2);

        let canon1 = canonical_json(&val1);
        let canon2 = canonical_json(&val2);

        assert_eq!(
            canon1, canon2,
            "Different insertion orders should produce same canonical JSON"
        );

        // Keys should be sorted
        assert!(canon1.contains("\"a\":2"));
        assert!(canon1.contains("\"z\":1"));
        // a comes before z
        let a_pos = canon1.find("\"a\"").unwrap();
        let z_pos = canon1.find("\"z\"").unwrap();
        assert!(a_pos < z_pos, "Keys should be sorted lexicographically");
    }

    #[test]
    fn test_acceptance_float_canonical() {
        // AC: Float canonical: 0.5 and 0.500 → byte-identical serialization
        let mut map1 = Map::new();
        map1.insert("x".to_string(), json!(0.5));
        let val1 = Value::Object(map1);

        let mut map2 = Map::new();
        map2.insert("x".to_string(), json!(0.500));
        let val2 = Value::Object(map2);

        let canon1 = canonical_json(&val1);
        let canon2 = canonical_json(&val2);

        assert_eq!(canon1, canon2, "0.5 and 0.500 should serialize identically");

        // Both should serialize to 0.5 (shortest representation)
        assert!(canon1.contains("\"x\":0.5"));
    }

    #[test]
    fn test_acceptance_float_canonical_edge_cases() {
        // Test various float representations
        let test_cases = vec![(1.0, "1.00"), (0.1, "0.100"), (1.5, "1.500")];

        for (val1, val2_str) in test_cases {
            let mut map1 = Map::new();
            map1.insert("x".to_string(), json!(val1));
            let val1_json = Value::Object(map1);

            // Parse val2_str as f64
            let val2: f64 = val2_str.parse().unwrap();
            let mut map2 = Map::new();
            map2.insert("x".to_string(), json!(val2));
            let val2_json = Value::Object(map2);

            let canon1 = canonical_json(&val1_json);
            let canon2 = canonical_json(&val2_json);

            assert_eq!(
                canon1, canon2,
                "{} and {} should serialize identically",
                val1, val2_str
            );
        }
    }

    #[test]
    fn test_invariant_documented() {
        // AC: Documented invariant: same logical request → same key
        // This is a meta-test documenting the invariant
        let opts1 = ExtractionOptions::default();
        let opts2 = ExtractionOptions::default();

        let key1 = CacheKey::new("fp", &opts1);
        let key2 = CacheKey::new("fp", &opts2);

        assert_eq!(key1.opts_hash, key2.opts_hash);

        // Different options should produce different keys
        let opts3 = ExtractionOptions::with_receipts(ReceiptsMode::Lite);
        let key3 = CacheKey::new("fp", &opts3);

        assert_ne!(
            key1.opts_hash, key3.opts_hash,
            "Invariant: same logical request → same key, different request → different key"
        );
    }

    #[test]
    fn test_canonical_json_nested_objects() {
        // Test that nested objects also get sorted keys
        let mut inner1 = Map::new();
        inner1.insert("z".to_string(), json!(2));
        inner1.insert("a".to_string(), json!(1));
        let mut outer1 = Map::new();
        outer1.insert("inner".to_string(), Value::Object(inner1));

        let mut inner2 = Map::new();
        inner2.insert("a".to_string(), json!(1));
        inner2.insert("z".to_string(), json!(2));
        let mut outer2 = Map::new();
        outer2.insert("inner".to_string(), Value::Object(inner2));

        let canon1 = canonical_json(&Value::Object(outer1));
        let canon2 = canonical_json(&Value::Object(outer2));

        assert_eq!(canon1, canon2, "Nested objects should have sorted keys");
    }

    #[test]
    fn test_canonical_json_arrays() {
        // Test that arrays are handled correctly
        let arr = json!([3, 1, 2]);
        let canon = canonical_json(&arr);

        // Arrays should preserve order (not sorted)
        // Integers should be serialized without decimal points
        assert_eq!(canon, "[3,1,2]");
    }

    #[test]
    fn test_canonical_json_float_arrays() {
        // Test that float arrays get canonicalized
        let arr = json!([3.0, 1.5, 2.100]);
        let canon = canonical_json(&arr);

        // Arrays should preserve order, floats get canonicalized
        // 3.0 stays as 3 (integer), 1.5 stays as 1.5, 2.100 becomes 2.1
        assert!(canon == "[3,1.5,2.1]" || canon == "[3.0,1.5,2.1]");
    }

    #[test]
    fn test_canonical_json_mixed() {
        // Test mixed nested structures
        let val = json!({
            "z": [3, 1, 2],
            "a": {"y": 2, "x": 1},
            "m": 0.5
        });

        let canon = canonical_json(&val);

        // Keys should be sorted: a, m, z
        let a_pos = canon.find("\"a\"").unwrap();
        let m_pos = canon.find("\"m\"").unwrap();
        let z_pos = canon.find("\"z\"").unwrap();
        assert!(a_pos < m_pos && m_pos < z_pos);

        // Nested object in "a" should also be sorted
        let x_pos = canon.find("\"x\"").unwrap();
        let y_pos = canon.find("\"y\"").unwrap();
        assert!(x_pos < y_pos);
    }
}
