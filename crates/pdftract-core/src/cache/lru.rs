//! LRU (Least-Recently-Used) eviction policy for the cache directory.
//!
//! This module implements Phase 6.9.4: LRU eviction using an O_APPEND sentinel
//! file for touch-time tracking. Eviction is triggered on cache writes when
//! the total compressed size exceeds the configured limit (default 1 GiB).

use crate::cache::layout::{entry_path, parse_opts_hash_from_filename, parse_size_from_filename, sentinel_path};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Default cache size limit: 1 GiB.
pub const DEFAULT_CACHE_SIZE_BYTES: u64 = 1024 * 1024 * 1024;

/// Fingerprint version prefix that must be stripped for storage in sentinel.
const FINGERPRINT_PREFIX: &str = "pdftract-v1:";

/// Maximum size of the sentinel file before rotation (10 MB).
const SENTINEL_ROTATION_SIZE: u64 = 10 * 1024 * 1024;

/// Target percentage after eviction (evict to 80% to avoid churn).
const EVICTION_TARGET_PERCENT: u64 = 80;

/// Maximum entries to evict per sweep (to bound eviction time).
const MAX_EVICTIONS_PER_SWEEP: usize = 1000;

/// File extension for the rotated sentinel.
const SENTINEL_OLD_SUFFIX: &str = ".old";

/// LRU eviction manager for the cache directory.
///
/// Tracks least-recently-used entries via an append-only sentinel file
/// and evicts old entries when the cache exceeds its size limit.
///
/// # LRU mechanism
///
/// - **Touch**: On cache hit, append `<timestamp> <fingerprint>/<opts_hash>\n` to sentinel.touched
/// - **Eviction trigger**: On cache write, if total size > limit, run eviction sweep
/// - **Eviction sweep**: Enumerate entries, read sentinel backward to build LRU order, evict oldest
///
/// # Concurrency
///
/// - Touches are O_APPEND writes, atomic on POSIX (guaranteed for writes <= PIPE_BUF)
/// - Eviction is best-effort: concurrent sweeps may both evict; ENOENT from unlink is ignored
/// - No locks: multiple processes can share the same cache directory
///
/// # Example
///
/// ```ignore
/// use pdftract_core::cache::lru::Lru;
///
/// let lru = Lru::new(Path::new("/cache"), 1024 * 1024 * 1024); // 1 GiB
///
/// // On cache hit
/// lru.touch("pdftract-v1:e7a1f3...", "9b21c0ffee...");
///
/// // On cache write
/// lru.maybe_evict();
///
/// // Get current cache size
/// let size = lru.current_size_bytes();
/// ```
#[derive(Debug, Clone)]
pub struct Lru {
    /// Cache directory path
    cache_dir: PathBuf,
    /// Size limit in bytes (default: 1 GiB)
    limit_bytes: u64,
}

impl Lru {
    /// Create a new LRU manager for the given cache directory.
    ///
    /// # Arguments
    ///
    /// * `cache_dir` - Root cache directory
    /// * `limit_bytes` - Size limit in bytes (default: `DEFAULT_CACHE_SIZE_BYTES`)
    ///
    /// # Returns
    ///
    /// A new `Lru` instance.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use pdftract_core::cache::lru::{Lru, DEFAULT_CACHE_SIZE_BYTES};
    ///
    /// let lru = Lru::new(Path::new("/cache"), DEFAULT_CACHE_SIZE_BYTES);
    /// ```
    pub fn new(cache_dir: &Path, limit_bytes: u64) -> Self {
        Self {
            cache_dir: cache_dir.to_path_buf(),
            limit_bytes,
        }
    }

    /// Record a cache hit by appending a touch record to the sentinel file.
    ///
    /// This uses O_APPEND to ensure atomic writes without locks. Each record
    /// is a single line: `<timestamp> <fingerprint>/<opts_hash>\n`.
    ///
    /// # Arguments
    ///
    /// * `fingerprint` - PDF fingerprint (e.g., "pdftract-v1:e7a1f3...")
    /// * `opts_hash` - 64-char hex SHA-256 of extraction options
    ///
    /// # Errors
    ///
    /// Returns `Err` if the sentinel file cannot be opened or written to.
    /// Touch failures are non-fatal: the cache continues to operate, but
    /// LRU tracking may be degraded (entries fall back to mtime).
    ///
    /// # Performance
    ///
    /// - Completes in < 100 us on local SSD (single open + write + close)
    /// - Record size: ~80 bytes (timestamp + fingerprint + opts_hash + separators)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let lru = Lru::new(Path::new("/cache"), 1024 * 1024 * 1024);
    /// lru.touch("pdftract-v1:e7a1f3...", "9b21c0ffee...").unwrap();
    /// ```
    pub fn touch(&self, fingerprint: &str, opts_hash: &str) -> std::io::Result<()> {
        let sentinel_file = sentinel_path(&self.cache_dir);

        // Ensure parent directory exists
        if let Some(parent) = sentinel_file.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Get current timestamp as Unix seconds
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Strip the prefix to match filesystem layout
        let fp_normalized = fingerprint.strip_prefix(FINGERPRINT_PREFIX).unwrap_or(fingerprint);

        // Build the touch record: "<timestamp> <fingerprint>/<opts_hash>\n"
        let record = format!("{} {}/{}\n", timestamp, fp_normalized, opts_hash);

        // Check if we need to rotate the sentinel
        self.rotate_sentinel_if_needed(&sentinel_file)?;

        // Open with O_APPEND for atomic write
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&sentinel_file)?;

        // Write the record (atomic for writes <= PIPE_BUF on Linux, 4 KiB)
        file.write_all(record.as_bytes())?;

        Ok(())
    }

    /// Check if the cache exceeds its size limit and evict if necessary.
    ///
    /// This is called after cache writes. If the total compressed size
    /// exceeds the limit, an eviction sweep is triggered.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the cache directory cannot be read or entries
    /// cannot be deleted. Eviction failures are non-fatal: the cache
    /// continues to operate, but may grow beyond its limit.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let lru = Lru::new(Path::new("/cache"), 1024 * 1024 * 1024);
    /// lru.maybe_evict().unwrap(); // No-op if under limit
    /// ```
    pub fn maybe_evict(&self) -> std::io::Result<()> {
        let current_size = self.current_size_bytes()?;

        if current_size <= self.limit_bytes {
            // Under limit, no eviction needed
            return Ok(());
        }

        // Over limit, trigger eviction sweep
        self.evict_sweep()
    }

    /// Get the current total size of the cache in bytes.
    ///
    /// This enumerates all cache entries and sums their compressed sizes
    /// by parsing the filename's `-SIZE` suffix. No stat calls are made.
    ///
    /// # Returns
    ///
    /// Total compressed size in bytes.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the cache directory cannot be read.
    ///
    /// # Performance
    ///
    /// - 10,000 entries enumerated in < 1 s (no stat calls; parse filenames only)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let lru = Lru::new(Path::new("/cache"), 1024 * 1024 * 1024);
    /// let size = lru.current_size_bytes().unwrap();
    /// println!("Cache size: {} bytes", size);
    /// ```
    pub fn current_size_bytes(&self) -> std::io::Result<u64> {
        let mut total = 0u64;

        // Walk the two-byte prefix directories
        for prefix1_entry in std::fs::read_dir(&self.cache_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().is_dir()
                    && e.file_name().to_string_lossy().len() == 2
                    && e.file_name().to_string_lossy().chars().all(|c| c.is_ascii_hexdigit())
            })
        {
            let prefix1_dir = prefix1_entry.path();

            // Walk the second-level prefix directories
            for prefix2_entry in prefix1_dir.read_dir()?
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().is_dir()
                        && e.file_name().to_string_lossy().len() == 2
                        && e.file_name()
                            .to_string_lossy()
                            .chars()
                            .all(|c| c.is_ascii_hexdigit())
                })
            {
                let prefix2_dir = prefix2_entry.path();

                // Walk the fingerprint directories
                for fp_entry in prefix2_dir.read_dir()?.filter_map(|e| e.ok()).filter(|e| {
                    e.path().is_dir()
                }) {
                    let fp_dir = fp_entry.path();

                    // Walk the entry files
                    for entry in fp_dir.read_dir()?.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                                if let Some(size) = parse_size_from_filename(filename) {
                                    total += size as u64;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(total)
    }

    /// Rotate the sentinel file if it exceeds the rotation threshold.
    ///
    /// Moves `sentinel.touched` to `sentinel.touched.old` and creates
    /// a new empty sentinel file. The old file is still read during
    /// eviction (recent entries first).
    ///
    /// # Errors
    ///
    /// Returns `Err` if the rotation fails.
    fn rotate_sentinel_if_needed(&self, sentinel_file: &Path) -> std::io::Result<()> {
        // Check if sentinel exists and exceeds rotation threshold
        if let Ok(metadata) = sentinel_file.metadata() {
            if metadata.len() > SENTINEL_ROTATION_SIZE {
                let old_path = sentinel_file.with_extension(&format!(
                    "touched{}",
                    SENTINEL_OLD_SUFFIX
                ));

                // Move current to .old (replace existing .old)
                let _ = std::fs::remove_file(&old_path); // Ignore error if doesn't exist
                std::fs::rename(sentinel_file, &old_path)?;

                // Create a new empty sentinel (parent dir already exists)
                File::create(sentinel_file)?;
            }
        }

        Ok(())
    }

    /// Run an eviction sweep to remove least-recently-used entries.
    ///
    /// # Eviction algorithm
    ///
    /// 1. Enumerate all cache entries and sum their sizes
    /// 2. Read sentinel.touched backward to build LRU order
    /// 3. Evict oldest entries until under (limit * 0.8)
    /// 4. Truncate sentinel to recent entries
    ///
    /// # Errors
    ///
    /// Returns `Err` if enumeration or eviction fails.
    fn evict_sweep(&self) -> std::io::Result<()> {
        // Step 1: Enumerate all entries and build fingerprint -> (opts_hash, size) map
        let mut entries: Vec<(String, String, usize, PathBuf)> = Vec::new();
        let mut total_size = 0u64;

        for prefix1_entry in std::fs::read_dir(&self.cache_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                e.path().is_dir()
                    && name.len() == 2
                    && name.chars().all(|c| c.is_ascii_hexdigit())
            })
        {
            let prefix1_dir = prefix1_entry.path();

            for prefix2_entry in prefix1_dir.read_dir()?
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    e.path().is_dir()
                        && name.len() == 2
                        && name.chars().all(|c| c.is_ascii_hexdigit())
                })
            {
                let prefix2_dir = prefix2_entry.path();

                for fp_entry in prefix2_dir.read_dir()?.filter_map(|e| e.ok()).filter(|e| {
                    e.path().is_dir()
                }) {
                    let fp_dir = fp_entry.path();

                    // Extract fingerprint from path (last component)
                    let fingerprint = fp_dir
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();

                    for entry in fp_dir.read_dir()?.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path.is_file() {
                            let filename_opt = path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string());
                            if let Some(filename) = filename_opt {
                                if let (Some(opts_hash), Some(size)) = (
                                    parse_opts_hash_from_filename(&filename),
                                    parse_size_from_filename(&filename),
                                ) {
                                    // Skip zero-size or corrupt entries
                                    if size > 0 {
                                        total_size += size as u64;
                                        entries.push((
                                            fingerprint.clone(),
                                            opts_hash.to_string(),
                                            size,
                                            path,
                                        ));
                                    } else {
                                        // Delete zero-size entries immediately
                                        let _ = std::fs::remove_file(&path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check if we're still over limit
        let target_size = (self.limit_bytes * EVICTION_TARGET_PERCENT) / 100;
        if total_size <= target_size {
            return Ok(());
        }

        // Step 2: Read sentinel to build LRU order (returns indices in LRU order)
        let lru_order = self.build_lru_order(&entries)?;

        // Step 3: Evict oldest entries until under target
        let mut bytes_to_free = total_size - target_size;
        let mut evicted = std::collections::HashSet::new();

        for idx in lru_order.into_iter().take(MAX_EVICTIONS_PER_SWEEP) {
            if bytes_to_free == 0 {
                break;
            }

            if idx < entries.len() && !evicted.contains(&idx) {
                let (_fp, _opts, size, path) = &entries[idx];

                // Delete the entry (ignore ENOENT from concurrent eviction)
                let _ = std::fs::remove_file(&path);
                evicted.insert(idx);
                bytes_to_free = bytes_to_free.saturating_sub(*size as u64);
            }
        }

        // Clean up empty fingerprint directories
        self.cleanup_empty_dirs()?;

        // Step 4: Truncate sentinel to recent entries
        self.truncate_sentinel()?;

        Ok(())
    }

    /// Build LRU order from the sentinel file.
    ///
    /// Returns a Vec of entry indices in least-recently-used order (oldest first).
    /// Entries with no touch record use file mtime.
    fn build_lru_order(
        &self,
        entries: &[(String, String, usize, PathBuf)],
    ) -> std::io::Result<Vec<usize>> {
        let sentinel_file = sentinel_path(&self.cache_dir);
        let mut touch_times: HashMap<(String, String), u64> = HashMap::new();

        // Read the current sentinel file
        if let Ok(contents) = std::fs::read_to_string(&sentinel_file) {
            for line in contents.lines().rev() {
                // Parse "<timestamp> <fingerprint>/<opts_hash>"
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    if let Ok(timestamp) = parts[0].parse::<u64>() {
                        let key_parts: Vec<&str> = parts[1].splitn(2, '/').collect();
                        if key_parts.len() == 2 {
                            let key = (key_parts[0].to_string(), key_parts[1].to_string());
                            // Only record the most recent touch (we're reading backward)
                            touch_times.entry(key).or_insert(timestamp);
                        }
                    }
                }
            }
        }

        // Read the old sentinel file (.old) if it exists
        let old_sentinel = sentinel_file.with_extension(&format!(
            "touched{}",
            SENTINEL_OLD_SUFFIX
        ));
        if let Ok(contents) = std::fs::read_to_string(&old_sentinel) {
            for line in contents.lines().rev() {
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    if let Ok(timestamp) = parts[0].parse::<u64>() {
                        let key_parts: Vec<&str> = parts[1].splitn(2, '/').collect();
                        if key_parts.len() == 2 {
                            let key = (key_parts[0].to_string(), key_parts[1].to_string());
                            // Only record if not already in current sentinel
                            touch_times.entry(key).or_insert(timestamp);
                        }
                    }
                }
            }
        }

        // Build LRU order: sort entries by touch time (oldest first)
        let mut lru_entries: Vec<(usize, u64)> = entries
            .iter()
            .enumerate()
            .map(|(idx, (fp, opts, _size, path))| {
                let key = (fp.clone(), opts.clone());
                let touch_time = if let Some(&tt) = touch_times.get(&key) {
                    tt
                } else {
                    // Fall back to file mtime if no touch record
                    path.metadata()
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0)
                };
                (idx, touch_time)
            })
            .collect();

        // Sort by touch time ascending (oldest first)
        lru_entries.sort_by_key(|e| e.1);

        // Return just the indices in LRU order
        Ok(lru_entries.into_iter().map(|(idx, _tt)| idx).collect())
    }

    /// Clean up empty fingerprint directories.
    ///
    /// After eviction, some fingerprint directories may be empty.
    /// Remove them to keep the cache directory clean.
    fn cleanup_empty_dirs(&self) -> std::io::Result<()> {
        for prefix1_entry in std::fs::read_dir(&self.cache_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().is_dir()
                    && e.file_name().to_string_lossy().len() == 2
                    && e.file_name().to_string_lossy().chars().all(|c| c.is_ascii_hexdigit())
            })
        {
            let prefix1_dir = prefix1_entry.path();

            for prefix2_entry in prefix1_dir.read_dir()?
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().is_dir()
                        && e.file_name().to_string_lossy().len() == 2
                        && e.file_name()
                            .to_string_lossy()
                            .chars()
                            .all(|c| c.is_ascii_hexdigit())
                })
            {
                let prefix2_dir = prefix2_entry.path();

                for fp_entry in prefix2_dir.read_dir()?.filter_map(|e| e.ok()).filter(|e| {
                    e.path().is_dir()
                }) {
                    let fp_dir = fp_entry.path();

                    // Check if the fingerprint directory is empty
                    if let Ok(mut entries) = fp_dir.read_dir() {
                        if entries.next().is_none() {
                            // Empty directory, remove it
                            let _ = std::fs::remove_dir(&fp_dir);
                        }
                    }
                }

                // Check if the second-level prefix directory is empty
                if let Ok(mut entries) = prefix2_dir.read_dir() {
                    if entries.next().is_none() {
                        let _ = std::fs::remove_dir(&prefix2_dir);
                    }
                }
            }

            // Check if the first-level prefix directory is empty
            if let Ok(mut entries) = prefix1_dir.read_dir() {
                if entries.next().is_none() {
                    let _ = std::fs::remove_dir(&prefix1_dir);
                }
            }
        }

        Ok(())
    }

    /// Truncate the sentinel file to keep only recent entries.
    ///
    /// After eviction, we truncate the sentinel to remove records for
    /// evicted entries. This keeps the file size bounded.
    fn truncate_sentinel(&self) -> std::io::Result<()> {
        let sentinel_file = sentinel_path(&self.cache_dir);

        // Read the current sentinel (if it exists)
        let contents = match std::fs::read_to_string(&sentinel_file) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Sentinel doesn't exist yet (no entries touched), nothing to truncate
                return Ok(());
            },
            Err(e) => return Err(e),
        };
        let lines: Vec<&str> = contents.lines().collect();

        // Keep only the most recent 10,000 entries (append from the end)
        let start = if lines.len() > 10_000 {
            lines.len() - 10_000
        } else {
            0
        };

        let truncated: String = lines[start..].join("\n") + "\n";
        std::fs::write(&sentinel_file, truncated)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    const TEST_FINGERPRINT: &str = "pdftract-v1:e7a1f3deadbeef00000000000000000000000000000000000000000000000000";
    const TEST_FINGERPRINT_2: &str = "pdftract-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const TEST_OPTS_HASH: &str = "9b21c0ffee000000000000000000000000000000000000000000000000000000"; // 64 chars
    const TEST_OPTS_HASH_2: &str = "aaaaaaaa00000000000000000000000000000000000000000000000000000000"; // 64 chars

    /// Create a test cache entry file.
    fn create_test_entry(cache_dir: &Path, fp: &str, opts: &str, size: usize) -> PathBuf {
        let path = entry_path(cache_dir, fp, opts, size);
        let parent = path.parent().unwrap();
        fs::create_dir_all(parent).unwrap();
        fs::write(&path, b"x".repeat(size)).unwrap();
        path
    }

    #[test]
    fn test_lru_new() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let lru = Lru::new(cache_dir, DEFAULT_CACHE_SIZE_BYTES);

        assert_eq!(lru.cache_dir, cache_dir);
        assert_eq!(lru.limit_bytes, DEFAULT_CACHE_SIZE_BYTES);
    }

    #[test]
    fn test_touch_creates_sentinel() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let lru = Lru::new(cache_dir, DEFAULT_CACHE_SIZE_BYTES);
        lru.touch(TEST_FINGERPRINT, TEST_OPTS_HASH).unwrap();

        let sentinel_file = sentinel_path(cache_dir);
        assert!(sentinel_file.exists());

        let contents = fs::read_to_string(&sentinel_file).unwrap();
        // Sentinel stores fingerprint without prefix
        let fp_normalized = TEST_FINGERPRINT.strip_prefix(FINGERPRINT_PREFIX).unwrap_or(TEST_FINGERPRINT);
        assert!(contents.contains(&format!("{}/{}", fp_normalized, TEST_OPTS_HASH)));
    }

    #[test]
    fn test_touch_format() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let lru = Lru::new(cache_dir, DEFAULT_CACHE_SIZE_BYTES);
        lru.touch(TEST_FINGERPRINT, TEST_OPTS_HASH).unwrap();

        let sentinel_file = sentinel_path(cache_dir);
        let contents = fs::read_to_string(&sentinel_file).unwrap();

        // Format: "<timestamp> <fp>/<opts_hash>\n"
        let parts: Vec<&str> = contents.trim().splitn(2, ' ').collect();
        assert_eq!(parts.len(), 2);

        // First part should be a valid timestamp
        let timestamp: u64 = parts[0].parse().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // Timestamp should be recent (within last 10 seconds)
        assert!(now.saturating_sub(timestamp) < 10);

        // Second part should be "fp/opts_hash" (fp without prefix)
        let fp_normalized = TEST_FINGERPRINT.strip_prefix(FINGERPRINT_PREFIX).unwrap_or(TEST_FINGERPRINT);
        assert_eq!(parts[1], &format!("{}/{}", fp_normalized, TEST_OPTS_HASH));
    }

    #[test]
    fn test_current_size_empty() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let lru = Lru::new(cache_dir, DEFAULT_CACHE_SIZE_BYTES);
        let size = lru.current_size_bytes().unwrap();

        assert_eq!(size, 0);
    }

    #[test]
    fn test_current_size_with_entries() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create test entries: 1000 bytes + 2000 bytes
        create_test_entry(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, 1000);
        create_test_entry(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH_2, 2000);

        let lru = Lru::new(cache_dir, DEFAULT_CACHE_SIZE_BYTES);
        let size = lru.current_size_bytes().unwrap();

        assert_eq!(size, 3000);
    }

    #[test]
    fn test_maybe_evict_under_limit() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create a small entry (under limit)
        create_test_entry(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, 1000);

        let lru = Lru::new(cache_dir, 10_000); // 10 KB limit
        lru.maybe_evict().unwrap();

        // Entry should still exist
        let path = entry_path(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, 1000);
        assert!(path.exists());
    }

    #[test]
    fn test_maybe_evict_over_limit() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create entries totaling 6000 bytes (over 5000 byte limit)
        create_test_entry(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, 1000);
        std::thread::sleep(std::time::Duration::from_millis(10));
        create_test_entry(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH_2, 2000);
        std::thread::sleep(std::time::Duration::from_millis(10));
        create_test_entry(cache_dir, TEST_FINGERPRINT_2, TEST_OPTS_HASH, 3000);

        let lru = Lru::new(cache_dir, 5000); // 5 KB limit, evicts to 4 KB (80%)

        // Verify cache size before eviction
        let size_before = lru.current_size_bytes().unwrap();
        assert_eq!(size_before, 6000, "Initial cache size should be 6000");

        // Touch the first entry to make it recently used
        lru.touch(TEST_FINGERPRINT, TEST_OPTS_HASH).unwrap();

        // Verify touch was written
        let sentinel_file = sentinel_path(cache_dir);
        let sentinel_contents = fs::read_to_string(&sentinel_file).unwrap();
        assert!(sentinel_contents.contains(TEST_OPTS_HASH), "Sentinel should contain opts_hash");

        // Trigger eviction
        lru.maybe_evict().unwrap();

        // Recent entry should still exist
        let path1 = entry_path(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, 1000);
        assert!(path1.exists(), "Recent entry should still exist");

        // Cache size should be under target (4000 bytes)
        let size = lru.current_size_bytes().unwrap();
        assert!(size <= 4000, "Cache size {} exceeds target 4000", size);
    }

    #[test]
    fn test_zero_size_entry_deleted() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create a zero-size entry and a normal entry
        let zero_path = create_test_entry(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, 0);
        create_test_entry(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH_2, 1000);

        let lru = Lru::new(cache_dir, 100); // Very small limit to force eviction

        // Trigger eviction - this should delete zero-size entries during enumeration
        lru.maybe_evict().unwrap();

        // Zero-size entry should be deleted
        assert!(!zero_path.exists());
    }

    #[test]
    fn test_concurrent_touches() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let lru = Lru::new(cache_dir, DEFAULT_CACHE_SIZE_BYTES);

        // Simulate concurrent touches from multiple threads
        // Each thread touches a unique entry to avoid duplicates
        let handles: Vec<_> = (0..100)
            .map(|i| {
                let lru = lru.clone();
                std::thread::spawn(move || {
                    // Use unique fingerprints and opts per thread
                    let fp = format!("{}:fp{:04}", TEST_FINGERPRINT, i);
                    let opts = format!("{}:opts{:04}", TEST_OPTS_HASH, i);
                    lru.touch(&fp, &opts).unwrap()
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all lines are parseable and no records are garbled
        let sentinel_file = sentinel_path(cache_dir);
        let contents = fs::read_to_string(&sentinel_file).unwrap();

        // All lines should be parseable (no truncated/garbled records)
        let mut parseable_count = 0;
        for line in contents.lines() {
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() == 2 {
                if parts[0].parse::<u64>().is_ok() && parts[1].contains('/') {
                    parseable_count += 1;
                }
            }
        }

        // Should have at least 95 parseable records (allowing for some edge cases)
        assert!(parseable_count >= 95, "Expected at least 95 parseable records, got {}", parseable_count);
    }

    #[test]
    fn test_sentinel_rotation() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let lru = Lru::new(cache_dir, DEFAULT_CACHE_SIZE_BYTES);

        // Write a large sentinel (over 10 MB)
        let sentinel_file = sentinel_path(cache_dir);
        let large_data = "1234567890 ".repeat(200_000); // ~2 MB per write
        for _ in 0..6 {
            // 6 * 2 MB = 12 MB, over the 10 MB threshold
            lru.touch(TEST_FINGERPRINT, TEST_OPTS_HASH).unwrap();
        }

        // Manually pad to exceed rotation threshold
        {
            let mut file = OpenOptions::new()
                .append(true)
                .open(&sentinel_file)
                .unwrap();
            for _ in 0..5 {
                writeln!(file, "{} {}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(), large_data).unwrap();
            }
        }

        // Check metadata size
        let metadata = fs::metadata(&sentinel_file).unwrap();
        assert!(metadata.len() > SENTINEL_ROTATION_SIZE);

        // Next touch should trigger rotation
        lru.touch(TEST_FINGERPRINT_2, TEST_OPTS_HASH_2).unwrap();

        // Old sentinel should exist
        let old_sentinel = sentinel_file.with_extension(&format!(
            "touched{}",
            SENTINEL_OLD_SUFFIX
        ));
        assert!(old_sentinel.exists());

        // New sentinel should be smaller
        let new_metadata = fs::metadata(&sentinel_file).unwrap();
        assert!(new_metadata.len() < SENTINEL_ROTATION_SIZE);
    }

    #[test]
    fn test_cleanup_empty_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create entries
        let fp_dir = fingerprint_dir(cache_dir, TEST_FINGERPRINT);
        fs::create_dir_all(&fp_dir).unwrap();
        create_test_entry(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, 1000);

        let lru = Lru::new(cache_dir, DEFAULT_CACHE_SIZE_BYTES);

        // Delete the entry file
        let path = entry_path(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, 1000);
        fs::remove_file(&path).unwrap();

        // Cleanup should remove the empty fingerprint directory
        lru.cleanup_empty_dirs().unwrap();

        // Fingerprint directory should be gone
        assert!(!fp_dir.exists());
    }

    #[test]
    fn test_lru_order_with_touches() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create entries with delays for distinct mtimes
        create_test_entry(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, 1000);
        std::thread::sleep(std::time::Duration::from_millis(10));
        create_test_entry(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH_2, 2000);
        std::thread::sleep(std::time::Duration::from_millis(10));
        create_test_entry(cache_dir, TEST_FINGERPRINT_2, TEST_OPTS_HASH, 3000);

        let lru = Lru::new(cache_dir, DEFAULT_CACHE_SIZE_BYTES);

        // Touch entries in order (oldest to newest)
        lru.touch(TEST_FINGERPRINT, TEST_OPTS_HASH).unwrap(); // oldest
        std::thread::sleep(std::time::Duration::from_millis(10));
        lru.touch(TEST_FINGERPRINT, TEST_OPTS_HASH_2).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        lru.touch(TEST_FINGERPRINT_2, TEST_OPTS_HASH).unwrap(); // newest

        // Build LRU order (use fingerprints without prefix to match filesystem layout)
        let fp1 = TEST_FINGERPRINT.strip_prefix(FINGERPRINT_PREFIX).unwrap_or(TEST_FINGERPRINT);
        let fp2 = TEST_FINGERPRINT_2.strip_prefix(FINGERPRINT_PREFIX).unwrap_or(TEST_FINGERPRINT_2);
        let entries = vec![
            (fp1.to_string(), TEST_OPTS_HASH.to_string(), 1000,
             entry_path(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, 1000)),
            (fp1.to_string(), TEST_OPTS_HASH_2.to_string(), 2000,
             entry_path(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH_2, 2000)),
            (fp2.to_string(), TEST_OPTS_HASH.to_string(), 3000,
             entry_path(cache_dir, TEST_FINGERPRINT_2, TEST_OPTS_HASH, 3000)),
        ];

        let lru_order = lru.build_lru_order(&entries).unwrap();

        // Oldest should be first (index 0 - touched first)
        assert_eq!(lru_order[0], 0);
        let oldest_entry = &entries[lru_order[0]];
        assert_eq!(oldest_entry.0, fp1);
        assert_eq!(oldest_entry.1, TEST_OPTS_HASH);

        // Newest should be last (index 2 - touched last)
        assert_eq!(lru_order[2], 2);
        let newest_entry = &entries[lru_order[2]];
        assert_eq!(newest_entry.0, fp2);
        assert_eq!(newest_entry.1, TEST_OPTS_HASH);
    }

    #[test]
    fn test_evict_to_80_percent() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create entries totaling 11,000 bytes (exceeds 10 KB limit)
        create_test_entry(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, 2750);
        std::thread::sleep(std::time::Duration::from_millis(10));
        create_test_entry(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH_2, 2750);
        std::thread::sleep(std::time::Duration::from_millis(10));
        create_test_entry(cache_dir, TEST_FINGERPRINT_2, TEST_OPTS_HASH, 2750);
        std::thread::sleep(std::time::Duration::from_millis(10));
        create_test_entry(cache_dir, TEST_FINGERPRINT_2, TEST_OPTS_HASH_2, 2750);

        let lru = Lru::new(cache_dir, 10_000); // 10 KB limit, evicts to 8 KB (80%)

        // Touch only the first two entries to make them recent
        lru.touch(TEST_FINGERPRINT, TEST_OPTS_HASH).unwrap();
        lru.touch(TEST_FINGERPRINT, TEST_OPTS_HASH_2).unwrap();

        // Trigger eviction
        lru.maybe_evict().unwrap();

        // Cache size should be <= 80% of limit
        let size = lru.current_size_bytes().unwrap();
        assert!(
            size <= 8000,
            "Cache size {} exceeds 80% target (8000)",
            size
        );
    }

    #[test]
    fn test_touch_performance() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let lru = Lru::new(cache_dir, DEFAULT_CACHE_SIZE_BYTES);

        // Measure touch performance
        let start = std::time::Instant::now();
        for _ in 0..1000 {
            lru.touch(TEST_FINGERPRINT, TEST_OPTS_HASH).unwrap();
        }
        let duration = start.elapsed();

        // Should complete 1000 touches in < 100 ms on local SSD
        assert!(
            duration.as_millis() < 100,
            "1000 touches took {:?}, expected < 100 ms",
            duration
        );
    }

    #[test]
    fn test_current_size_performance() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create 1000 entries
        for i in 0..1000 {
            let fp = format!("{}:{:04x}", TEST_FINGERPRINT, i);
            let opts = format!("{}:{:04x}", TEST_OPTS_HASH, i);
            create_test_entry(cache_dir, &fp, &opts, 1000);
        }

        let lru = Lru::new(cache_dir, DEFAULT_CACHE_SIZE_BYTES);

        // Measure enumeration performance
        let start = std::time::Instant::now();
        let size = lru.current_size_bytes().unwrap();
        let duration = start.elapsed();

        assert_eq!(size, 1_000_000); // 1000 * 1000 bytes

        // Should enumerate 1000 entries in < 1 s on local SSD
        assert!(
            duration.as_secs_f64() < 1.0,
            "Enumerating 1000 entries took {:?}, expected < 1 s",
            duration
        );
    }

    #[test]
    fn test_eviction_sweep_performance() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Helper to generate valid 64-char hex opts hashes with a counter
        // Replace the last 4 chars of the base hash with hex counter
        let gen_opts = |i: u32| -> String {
            format!("{}{:04x}", &TEST_OPTS_HASH[..60], i)
        };

        // Helper to generate valid 64-char hex fingerprints with a counter
        // Replace the last 4 chars of the base fingerprint with hex counter
        let gen_fp = |i: u32| -> String {
            format!("{}{:04x}", &TEST_FINGERPRINT[FINGERPRINT_PREFIX.len()..60], i)
        };

        // Create 1000 entries totaling 100 MB (over limit)
        // Add small delays to ensure distinct mtimes for stable LRU ordering
        for i in 0..1000 {
            if i % 100 == 0 && i > 0 {
                // Small delay every 100 entries
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            let fp = format!("{}{}", FINGERPRINT_PREFIX, gen_fp(i));
            let opts = gen_opts(i);
            create_test_entry(cache_dir, &fp, &opts, 100_000); // 100 KB each
        }

        // Touch the most recent entries to keep them
        let lru = Lru::new(cache_dir, 50_000_000); // 50 MB limit
        for i in 800..1000 {
            let fp = format!("{}{}", FINGERPRINT_PREFIX, gen_fp(i));
            let opts = gen_opts(i);
            lru.touch(&fp, &opts).unwrap();
        }

        // Measure eviction performance
        let start = std::time::Instant::now();
        lru.maybe_evict().unwrap();
        let duration = start.elapsed();

        // Should evict in < 2 s on local SSD
        assert!(
            duration.as_secs_f64() < 2.0,
            "Eviction sweep took {:?}, expected < 2 s",
            duration
        );

        // Cache should be under 80% target
        let size = lru.current_size_bytes().unwrap();
        assert!(
            size <= 40_000_000,
            "Cache size {} after eviction exceeds 80% target (40 MB)",
            size
        );
    }

    #[test]
    fn test_best_effort_eviction() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create entries totaling 3000 bytes
        create_test_entry(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH, 1000);
        create_test_entry(cache_dir, TEST_FINGERPRINT, TEST_OPTS_HASH_2, 2000);

        let lru = Lru::new(cache_dir, 100); // Very small limit (100 bytes)

        // Trigger eviction - should evict as much as possible
        lru.maybe_evict().unwrap();

        // Even if another process evicted first, this should succeed
        lru.maybe_evict().unwrap();

        // Cache should be empty (all entries evicted since each is larger than 80% of limit)
        let size = lru.current_size_bytes().unwrap();
        assert_eq!(size, 0, "Cache should be empty after eviction");
    }

    // Helper function to get fingerprint dir (copied from layout module)
    fn fingerprint_dir(cache_dir: &Path, fingerprint: &str) -> PathBuf {
        const FINGERPRINT_PREFIX: &str = "pdftract-v1:";
        let fp = fingerprint.strip_prefix(FINGERPRINT_PREFIX).unwrap_or(fingerprint);
        let prefix1 = &fp[0..2.min(fp.len())];
        let prefix2 = &fp[2..4.min(fp.len())];
        cache_dir.join(prefix1).join(prefix2).join(fp)
    }
}
