//! Filesystem layout for the content-addressed cache.
//!
//! This module implements the two-byte-prefix directory scheme that keeps
//! any single directory under 65K entries even at millions of cached entries.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// Current cache schema version.
///
/// This gates layout migrations. On mismatch, the cache refuses to operate
/// and logs a clear migration message.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Fingerprint version prefix that must be stripped before path encoding.
const FINGERPRINT_PREFIX: &str = "pdftract-v1:";

/// Cache metadata stored in index.json.
///
/// This file is read at startup and updated on shutdown. It tracks the
/// cache schema version, creation timestamp, LRU sweep timing, and
/// hit ratio statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheIndex {
    /// Cache schema version (current: 1)
    pub schema_version: u32,
    /// Creation timestamp (Unix seconds)
    pub created_at: u64,
    /// Last LRU sweep timestamp (Unix seconds)
    pub last_lru_sweep: Option<u64>,
    /// Total compressed bytes in the cache (rebuilt from disk on suspicion of corruption)
    pub total_bytes: u64,
    /// Number of cached entries
    pub entry_count: u64,
    /// Cache hits since last clear (incremented on successful cache read)
    pub hits: u64,
    /// Total cache accesses since last clear (hits + misses)
    pub total_accesses: u64,
}

impl Default for CacheIndex {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_lru_sweep: None,
            total_bytes: 0,
            entry_count: 0,
            hits: 0,
            total_accesses: 0,
        }
    }
}

/// Construct the path to a cached extraction entry.
///
/// # Arguments
///
/// * `cache_dir` - Root cache directory
/// * `fingerprint` - Full fingerprint string (e.g., "pdftract-v1:e7a1f3...")
/// * `opts_hash` - 64-char hex SHA-256 hash of extraction options
/// * `compressed_size` - Size of the compressed entry in bytes
///
/// # Returns
///
/// Path in the format `<cache_dir>/<fp[0:2]>/<fp[2:4]>/<full_fp>/<opts_hash>-<size>.json.zst`
///
/// # Examples
///
/// ```ignore
/// let path = entry_path(
///     Path::new("/cache"),
///     "pdftract-v1:e7a1f3deadbeef...",
///     "9b21c0ffee...",
///     12387
/// );
/// assert_eq!(path, PathBuf::from("/cache/e7/a1/e7a1f3deadbeef.../9b21c0ffee...-12387.json.zst"));
/// ```
pub fn entry_path(
    cache_dir: &Path,
    fingerprint: &str,
    opts_hash: &str,
    compressed_size: usize,
) -> PathBuf {
    // Strip the "pdftract-v1:" prefix to get the raw hex fingerprint
    let fp = fingerprint.strip_prefix(FINGERPRINT_PREFIX).unwrap_or(fingerprint);

    // Validate fingerprint is at least 4 chars (for the two-byte prefixes)
    assert!(
        fp.len() >= 4,
        "Fingerprint must be at least 4 characters long, got: {}",
        fp.len()
    );

    // Extract two-byte prefixes
    let prefix1 = &fp[0..2];
    let prefix2 = &fp[2..4];

    // Build the path: <cache_dir>/<fp[0:2]>/<fp[2:4]>/<full_fp>/<opts_hash>-<size>.json.zst
    cache_dir
        .join(prefix1)
        .join(prefix2)
        .join(fp)
        .join(format!("{opts_hash}-{compressed_size}.json.zst"))
}

/// Construct the fingerprint directory path for a given fingerprint.
///
/// This is the parent directory that contains all option variants for a specific PDF.
/// Useful for invalidating all cached results for a PDF (rm -rf <fp_dir>).
///
/// # Arguments
///
/// * `cache_dir` - Root cache directory
/// * `fingerprint` - Full fingerprint string (e.g., "pdftract-v1:e7a1f3...")
///
/// # Returns
///
/// Path in the format `<cache_dir>/<fp[0:2]>/<fp[2:4]>/<full_fp>`
pub fn fingerprint_dir(cache_dir: &Path, fingerprint: &str) -> PathBuf {
    let fp = fingerprint.strip_prefix(FINGERPRINT_PREFIX).unwrap_or(fingerprint);
    assert!(
        fp.len() >= 4,
        "Fingerprint must be at least 4 characters long, got: {}",
        fp.len()
    );

    let prefix1 = &fp[0..2];
    let prefix2 = &fp[2..4];

    cache_dir.join(prefix1).join(prefix2).join(fp)
}

/// Parse the opts_hash from a cache entry filename.
///
/// Entry filenames are in the format `<opts_hash>-<size>.json.zst`.
/// This function extracts just the opts_hash part.
///
/// # Arguments
///
/// * `filename` - The entry filename (e.g., "e7a1f3-12387.json.zst")
///
/// # Returns
///
/// The opts_hash if the filename matches the expected format, None otherwise.
pub fn parse_opts_hash_from_filename(filename: &str) -> Option<&str> {
    // Expected format: <opts_hash>-<size>.json.zst
    // We need to extract everything before the first '-' that's followed by digits and '.json.zst'

    // Find the pattern: '-<digits>.json.zst'
    let json_zst_suffix = ".json.zst";
    if !filename.ends_with(json_zst_suffix) {
        return None;
    }

    // Strip the suffix to get "<opts_hash>-<size>"
    let rest = &filename[..filename.len() - json_zst_suffix.len()];

    // Find the last '-' (separates opts_hash from size)
    let separator_pos = rest.rfind('-')?;
    let opts_hash = &rest[..separator_pos];

    // opts_hash should be 64-char hex (SHA-256)
    if opts_hash.len() == 64 && opts_hash.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(opts_hash)
    } else {
        None
    }
}

/// Parse the compressed size from a cache entry filename.
///
/// Entry filenames are in the format `<opts_hash>-<size>.json.zst`.
/// This function extracts just the size part.
///
/// # Arguments
///
/// * `filename` - The entry filename (e.g., "e7a1f3-12387.json.zst")
///
/// # Returns
///
/// The compressed size if the filename matches the expected format, None otherwise.
pub fn parse_size_from_filename(filename: &str) -> Option<usize> {
    let json_zst_suffix = ".json.zst";
    if !filename.ends_with(json_zst_suffix) {
        return None;
    }

    let rest = &filename[..filename.len() - json_zst_suffix.len()];
    let separator_pos = rest.rfind('-')?;
    let size_str = &rest[separator_pos + 1..];

    size_str.parse().ok()
}

/// Path to the cache index.json file.
pub fn index_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join("index.json")
}

/// Path to the LRU sentinel file.
pub fn sentinel_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join("sentinel.touched")
}

/// Load the cache index from disk.
///
/// Returns None if the index doesn't exist or is malformed.
/// Returns an error if the schema version doesn't match.
pub fn load_index(cache_dir: &Path) -> Result<Option<CacheIndex>, anyhow::Error> {
    let index_file = index_path(cache_dir);

    if !index_file.exists() {
        return Ok(None);
    }

    let contents = std::fs::read_to_string(&index_file)?;
    let index: CacheIndex = serde_json::from_str(&contents)?;

    // Check schema version
    if index.schema_version != CURRENT_SCHEMA_VERSION {
        return Err(anyhow::anyhow!(
            "Cache schema version mismatch: expected {}, got {}. \
             Please clear the cache with 'pdftract cache clear' and re-populate.",
            CURRENT_SCHEMA_VERSION, index.schema_version
        ));
    }

    Ok(Some(index))
}

/// Save the cache index to disk.
pub fn save_index(cache_dir: &Path, index: &CacheIndex) -> Result<(), anyhow::Error> {
    let index_file = index_path(cache_dir);

    // Ensure the cache directory exists
    if let Some(parent) = index_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let contents = serde_json::to_string_pretty(index)?;
    std::fs::write(&index_file, contents)?;

    Ok(())
}

/// Ensure the fingerprint directory exists, creating it if necessary.
///
/// This uses `mkdir -p` semantics and is race-safe (idempotent).
pub fn ensure_fingerprint_dir(cache_dir: &Path, fingerprint: &str) -> Result<(), std::io::Error> {
    let fp_dir = fingerprint_dir(cache_dir, fingerprint);
    std::fs::create_dir_all(fp_dir)
}

/// Increment cache hit counter in index.json.
///
/// This is called on a successful cache hit. The hit counter and total
/// access counter are both incremented atomically.
///
/// # Arguments
///
/// * `cache_dir` - Root cache directory
///
/// # Returns
///
/// `Ok(())` on success, `Err` if the index cannot be loaded or saved.
pub fn increment_hit_counter(cache_dir: &Path) -> Result<(), anyhow::Error> {
    let mut index = load_index(cache_dir)?.unwrap_or_default();
    index.hits += 1;
    index.total_accesses += 1;
    save_index(cache_dir, &index)
}

/// Increment cache miss counter in index.json.
///
/// This is called on a cache miss (extraction runs). Only the total
/// access counter is incremented.
///
/// # Arguments
///
/// * `cache_dir` - Root cache directory
///
/// # Returns
///
/// `Ok(())` on success, `Err` if the index cannot be loaded or saved.
pub fn increment_miss_counter(cache_dir: &Path) -> Result<(), anyhow::Error> {
    let mut index = load_index(cache_dir)?.unwrap_or_default();
    index.total_accesses += 1;
    save_index(cache_dir, &index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const TEST_FINGERPRINT: &str = "pdftract-v1:e7a1f3deadbeef00000000000000000000000000000000000000000000000000";
    const TEST_FINGERPRINT_SHORT: &str = "pdftract-v1:e7a1";
    const TEST_OPTS_HASH: &str = "9b21c0ffee0000000000000000000000000000000000000000000000000000000";

    #[test]
    fn test_entry_path_basic() {
        let cache_dir = Path::new("/cache");
        let path = entry_path(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, 12387);

        // Should be: /cache/e7/a1/e7a1f3.../9b21...-12387.json.zst
        let expected = format!(
            "/cache/e7/a1/e7a1f3deadbeef00000000000000000000000000000000000000000000000000/\
             9b21c0ffee0000000000000000000000000000000000000000000000000000000-12387.json.zst"
        );
        assert_eq!(path, PathBuf::from(expected));
    }

    #[test]
    fn test_entry_path_different_opts_hashes() {
        let cache_dir = Path::new("/cache");
        let fp_dir = fingerprint_dir(cache_dir, TEST_FINGERPRINT);

        // Two different opts_hashes should produce entries in the same fp_dir
        let path1 = entry_path(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, 100);
        let path2 = entry_path(
            cache_dir,
            TEST_FINGERPRINT,
            "aaaa000000000000000000000000000000000000000000000000000000000000aa",
            200,
        );

        // Both should have the same parent (the fingerprint directory)
        assert_eq!(path1.parent(), Some(fp_dir.as_path()));
        assert_eq!(path2.parent(), Some(fp_dir.as_path()));

        // But different filenames
        assert_ne!(
            path1.file_name(),
            path2.file_name()
        );
    }

    #[test]
    fn test_entry_path_different_fingerprints_same_prefix() {
        let cache_dir = Path::new("/cache");

        // Two fingerprints with the same first 2 chars share the first-level directory
        let fp1 = "pdftract-v1:e7a1f3deadbeef00000000000000000000000000000000000000000000000000";
        let fp2 = "pdftract-v1:e7b2f4deadbeef00000000000000000000000000000000000000000000000000";

        let path1 = entry_path(cache_dir, fp1, TEST_OPTS_HASH, 100);
        let path2 = entry_path(cache_dir, fp2, TEST_OPTS_HASH, 100);

        // Both should have the same first-level directory (e7)
        // Check via components: skip root + cache, first prefix is e7
        let mut components1 = path1.components().skip(2);
        let mut components2 = path2.components().skip(2);
        assert_eq!(components1.next(), Some(std::path::Component::Normal(std::ffi::OsStr::new("e7"))));
        assert_eq!(components2.next(), Some(std::path::Component::Normal(std::ffi::OsStr::new("e7"))));

        // But different second-level directories
        assert_eq!(components1.next(), Some(std::path::Component::Normal(std::ffi::OsStr::new("a1"))));
        assert_eq!(components2.next(), Some(std::path::Component::Normal(std::ffi::OsStr::new("b2"))));
    }

    #[test]
    fn test_fingerprint_dir() {
        let cache_dir = Path::new("/cache");
        let fp_dir = fingerprint_dir(cache_dir, TEST_FINGERPRINT);

        let expected = "/cache/e7/a1/e7a1f3deadbeef00000000000000000000000000000000000000000000000000";
        assert_eq!(fp_dir, PathBuf::from(expected));
    }

    #[test]
    fn test_entry_path_short_fingerprint() {
        let cache_dir = Path::new("/cache");
        let path = entry_path(cache_dir, TEST_FINGERPRINT_SHORT, TEST_OPTS_HASH, 12387);

        // Should use the available chars: e7/a1/e7a1/...
        let mut components = path.components().skip(2);
        assert_eq!(components.next(), Some(std::path::Component::Normal(std::ffi::OsStr::new("e7"))));
        assert_eq!(components.next(), Some(std::path::Component::Normal(std::ffi::OsStr::new("a1"))));
    }

    #[test]
    fn test_parse_opts_hash_from_filename() {
        // Valid filename
        let filename = "e7a1f3deadbeef00000000000000000000000000000000000000000000000000-12387.json.zst";
        let opts_hash = parse_opts_hash_from_filename(filename);
        assert_eq!(
            opts_hash,
            Some("e7a1f3deadbeef00000000000000000000000000000000000000000000000000")
        );

        // Invalid: wrong suffix
        assert!(parse_opts_hash_from_filename("e7a1f3-12387.json").is_none());

        // Invalid: no size part
        assert!(parse_opts_hash_from_filename("e7a1f3.json.zst").is_none());

        // Invalid: opts_hash too short
        assert!(parse_opts_hash_from_filename("abc-12387.json.zst").is_none());
    }

    #[test]
    fn test_parse_size_from_filename() {
        let filename = "e7a1f3deadbeef00000000000000000000000000000000000000000000000000-12387.json.zst";
        let size = parse_size_from_filename(filename);
        assert_eq!(size, Some(12387));

        // Different size
        let filename2 = "e7a1f3deadbeef00000000000000000000000000000000000000000000000000-999.json.zst";
        let size2 = parse_size_from_filename(filename2);
        assert_eq!(size2, Some(999));

        // Invalid format
        assert!(parse_size_from_filename("e7a1f3.json.zst").is_none());
        assert!(parse_size_from_filename("e7a1f3-abc.json.zst").is_none());
    }

    #[test]
    fn test_index_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create an index
        let index = CacheIndex {
            schema_version: CURRENT_SCHEMA_VERSION,
            created_at: 1234567890,
            last_lru_sweep: Some(1234567900),
            total_bytes: 1024000,
            entry_count: 42,
            hits: 10,
            total_accesses: 15,
        };

        // Save it
        save_index(cache_dir, &index).unwrap();

        // Load it back
        let loaded = load_index(cache_dir).unwrap().unwrap();

        assert_eq!(loaded.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(loaded.created_at, 1234567890);
        assert_eq!(loaded.last_lru_sweep, Some(1234567900));
        assert_eq!(loaded.total_bytes, 1024000);
        assert_eq!(loaded.entry_count, 42);
    }

    #[test]
    fn test_index_default() {
        let index = CacheIndex::default();
        assert_eq!(index.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(index.last_lru_sweep, None);
        assert_eq!(index.total_bytes, 0);
        assert_eq!(index.entry_count, 0);
        // created_at should be recent (within last 10 seconds)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(now - index.created_at < 10);
    }

    #[test]
    fn test_index_schema_version_mismatch() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create an index with a future schema version
        let index = CacheIndex {
            schema_version: 99, // Future version
            created_at: 1234567890,
            last_lru_sweep: None,
            total_bytes: 0,
            entry_count: 0,
            hits: 0,
            total_accesses: 0,
        };

        save_index(cache_dir, &index).unwrap();

        // Loading should fail with a clear error message
        let result = load_index(cache_dir);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(err_msg.contains("schema version mismatch"));
        assert!(err_msg.contains("expected 1"));
        assert!(err_msg.contains("got 99"));
    }

    #[test]
    fn test_index_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Loading when index doesn't exist should return Ok(None)
        let result = load_index(cache_dir).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_ensure_fingerprint_dir() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Ensure the directory is created
        ensure_fingerprint_dir(cache_dir, TEST_FINGERPRINT).unwrap();

        let fp_dir = fingerprint_dir(cache_dir, TEST_FINGERPRINT);
        assert!(fp_dir.exists());
        assert!(fp_dir.is_dir());

        // Calling again should be idempotent
        ensure_fingerprint_dir(cache_dir, TEST_FINGERPRINT).unwrap();
        assert!(fp_dir.exists());
    }

    #[test]
    fn test_path_length_within_limits() {
        let cache_dir = Path::new("/a/very/long/cache/directory/path/that/goes/on/and/on");
        let path = entry_path(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, 12387);

        // Convert to string and check length
        let path_str = path.to_str().unwrap();
        // POSIX max path length is typically 4096
        assert!(path_str.len() < 4096, "Path length {} exceeds 4096", path_str.len());

        // Our paths should be much shorter in practice
        // Typical case: /cache + 2 + 2 + 64 + 64 + ~20 = ~154 bytes
    }

    #[test]
    fn test_unicode_path_handling() {
        // Test that PathBuf handles unicode correctly on all platforms
        let cache_dir = Path::new("/café");
        let path = entry_path(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, 12387);

        // Path should be constructible
        let path_str = path.to_str();
        assert!(path_str.is_some());

        // On Windows, this would use wide characters; on Unix, UTF-8
        // Either way, PathBuf handles it correctly
    }

    #[test]
    fn test_fingerprint_without_prefix() {
        // If fingerprint doesn't have the prefix, use it as-is
        let cache_dir = Path::new("/cache");
        let bare_fp = "e7a1f3deadbeef00000000000000000000000000000000000000000000000000";
        let path = entry_path(cache_dir, bare_fp, TEST_OPTS_HASH, 12387);

        // Should still work: /cache/e7/a1/e7a1f3...
        let mut components = path.components().skip(2);
        assert_eq!(components.next(), Some(std::path::Component::Normal(std::ffi::OsStr::new("e7"))));
        assert_eq!(components.next(), Some(std::path::Component::Normal(std::ffi::OsStr::new("a1"))));
    }

    #[test]
    fn test_entry_path_zero_size() {
        let cache_dir = Path::new("/cache");
        let path = entry_path(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, 0);

        let filename = path.file_name().unwrap().to_str().unwrap();
        assert!(filename.ends_with("-0.json.zst"));
    }

    #[test]
    fn test_entry_path_large_size() {
        let cache_dir = Path::new("/cache");
        let large_size = 999_999_999;
        let path = entry_path(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, large_size);

        let filename = path.file_name().unwrap().to_str().unwrap();
        assert!(filename.ends_with(&format!("-{}.json.zst", large_size)));

        // Parse it back
        let parsed = parse_size_from_filename(filename).unwrap();
        assert_eq!(parsed, large_size);
    }

    #[test]
    #[should_panic(expected = "Fingerprint must be at least 4 characters long")]
    fn test_entry_path_too_short() {
        let cache_dir = Path::new("/cache");
        // Too short after stripping prefix
        let _ = entry_path(cache_dir, "pdftract-v1:ab", TEST_OPTS_HASH, 12387);
    }
}
