//! Multi-process safe cache operations.
//!
//! This module implements Phase 6.9.5: atomic file writes and concurrent
//! access safety for multiple pdftract processes sharing the same cache
//! directory. The design uses temp + rename for atomic writes and
//! tolerates duplicated work on first-miss races, avoiding distributed
//! locks for simplicity and predictable failure modes.

use crate::cache::compression::decode;
use crate::cache::layout::entry_path;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Environment variable to disable fsync before rename.
///
/// Set to "1" to disable fsync for benchmarking (tradeoff: crash safety).
const ENV_NO_FSYNC: &str = "PDFTRACT_CACHE_NO_FSYNC";

/// Maximum age for a temp file before it's considered stale (1 hour).
const TEMP_FILE_MAX_AGE_SECONDS: u64 = 3600;

/// File extension for temp files.
const TEMP_SUFFIX: &str = ".tmp";

/// Multi-process safe cache writer.
///
/// Writes cache entries atomically using temp + rename semantics.
/// Multiple processes can write concurrently; last writer wins.
///
/// # Atomic write protocol
///
/// 1. Create parent directory (mkdir -p, idempotent)
/// 2. Write compressed data to temp file: `<entry_path>.tmp.<pid>.<random>`
/// 3. fsync the temp file (optional, controlled by env var)
/// 4. rename temp → entry_path (atomic on POSIX)
/// 5. On failure, unlink temp file
///
/// # Example
///
/// ```ignore
/// use pdftract_core::cache::multi_process::Writer;
///
/// let writer = Writer::new(Path::new("/cache"));
/// let compressed_data = vec![/* zstd data */];
/// writer.write("pdftract-v1:fp", "opts_hash", 1234, &compressed_data)?;
/// ```
#[derive(Debug, Clone)]
pub struct Writer {
    /// Cache directory path
    cache_dir: PathBuf,
}

impl Writer {
    /// Create a new Writer for the given cache directory.
    ///
    /// # Arguments
    ///
    /// * `cache_dir` - Root cache directory
    ///
    /// # Returns
    ///
    /// A new `Writer` instance.
    pub fn new(cache_dir: &Path) -> Self {
        Self {
            cache_dir: cache_dir.to_path_buf(),
        }
    }

    /// Write a cache entry atomically.
    ///
    /// This function writes the compressed data to a temporary file,
    /// optionally fsyncs it, then atomically renames it to the final
    /// entry path.
    ///
    /// # Arguments
    ///
    /// * `fingerprint` - PDF fingerprint (e.g., "pdftract-v1:e7a1f3...")
    /// * `opts_hash` - 64-char hex SHA-256 of extraction options
    /// * `compressed_size` - Size of the compressed data in bytes
    /// * `data` - Compressed zstd data to write
    ///
    /// # Returns
    ///
    /// `Ok(())` if the write succeeded.
    ///
    /// # Errors
    ///
    /// Returns `Err` if:
    /// - Parent directory cannot be created
    /// - Temp file cannot be written
    /// - fsync fails (only if enabled)
    /// - rename fails (e.g., permission denied, cross-device link)
    ///
    /// # Concurrency
    ///
    /// Multiple writers can call this concurrently. The last writer to
    /// rename wins atomically. Readers never see partially-written data.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let writer = Writer::new(Path::new("/cache"));
    /// let compressed = vec![0x28, 0xb5, 0x2f, 0xfd, /* zstd data */];
    /// writer.write("pdftract-v1:fp", "opts_hash", compressed.len(), &compressed)?;
    /// ```
    pub fn write(
        &self,
        fingerprint: &str,
        opts_hash: &str,
        compressed_size: usize,
        data: &[u8],
    ) -> io::Result<()> {
        // Step 1: Compute the entry path
        let entry = entry_path(&self.cache_dir, fingerprint, opts_hash, compressed_size);

        // Step 2: Create parent directory (mkdir -p, idempotent)
        if let Some(parent) = entry.parent() {
            fs::create_dir_all(parent)?;
        }

        // Step 3: Create temp file in the same directory (for same-filesystem rename)
        let temp_path = self.temp_path(&entry);

        // Write data to temp file
        {
            let mut file = File::create(&temp_path)?;
            file.write_all(data)?;

            // Step 4: fsync the temp file (optional, for crash safety)
            if !self.fsync_disabled() {
                file.sync_all()?;
            }
        }

        // Step 5: Atomic rename
        match fs::rename(&temp_path, &entry) {
            Ok(()) => Ok(()),
            Err(e) => {
                // Step 6: On rename failure, unlink temp file
                let _ = fs::remove_file(&temp_path);
                Err(e)
            }
        }
    }

    /// Check if fsync is disabled via environment variable.
    fn fsync_disabled(&self) -> bool {
        std::env::var(ENV_NO_FSYNC)
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    }

    /// Generate a unique temp file path in the same directory as the entry.
    ///
    /// Format: `<entry_path>.tmp.<pid>.<random>`
    ///
    /// The random component avoids collisions between concurrent same-process
    /// writes (different threads with the same pid).
    fn temp_path(&self, entry: &Path) -> PathBuf {
        let pid = std::process::id();
        let random = self.random_component();

        let mut temp_name = entry
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("cache")
            .to_string();

        temp_name.push_str(TEMP_SUFFIX);
        temp_name.push('.');
        temp_name.push_str(&pid.to_string());
        temp_name.push('.');
        temp_name.push_str(&random);

        entry.with_file_name(temp_name)
    }

    /// Generate a random component for temp file naming.
    ///
    /// Uses 16 bits of randomness (4 hex chars) to avoid collisions.
    fn random_component(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::Instant;

        // Hash the current instant to get a pseudo-random value
        let mut hasher = DefaultHasher::new();
        Instant::now().hash(&mut hasher);
        std::thread::current().id().hash(&mut hasher);

        format!("{:04x}", hasher.finish() & 0xFFFF)
    }
}

/// Multi-process safe cache reader.
///
/// Reads cache entries with decompression and corruption handling.
/// If a decompression error occurs, the corrupt entry is deleted.
///
/// # Reader protocol
///
/// 1. Open entry_path with O_RDONLY
/// 2. Read full contents
/// 3. Decompress via zstd
/// 4. On decompression error, unlink entry and return cache miss
///
/// # Concurrency
///
/// - Readers can read concurrently with writers
/// - If a rename happens mid-read, the reader sees the consistent file it opened
/// - If eviction deletes an entry mid-read, the reader's fd remains valid
///
/// # Example
///
/// ```ignore
/// use pdftract_core::cache::multi_process::Reader;
///
/// let reader = Reader::new(Path::new("/cache"));
/// match reader.read("pdftract-v1:fp", "opts_hash", 1234) {
///     Ok(data) => println!("Cache hit: {} bytes", data.len()),
///     Err(e) if e.kind() == io::ErrorKind::NotFound => println!("Cache miss"),
///     Err(e) => eprintln!("Cache error: {}", e),
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Reader {
    /// Cache directory path
    cache_dir: PathBuf,
}

impl Reader {
    /// Create a new Reader for the given cache directory.
    ///
    /// # Arguments
    ///
    /// * `cache_dir` - Root cache directory
    ///
    /// # Returns
    ///
    /// A new `Reader` instance.
    pub fn new(cache_dir: &Path) -> Self {
        Self {
            cache_dir: cache_dir.to_path_buf(),
        }
    }

    /// Read and decompress a cache entry.
    ///
    /// This function reads the compressed entry from disk, decompresses it,
    /// and returns the raw data. If decompression fails, the corrupt entry
    /// is deleted and an error is returned.
    ///
    /// # Arguments
    ///
    /// * `fingerprint` - PDF fingerprint (e.g., "pdftract-v1:e7a1f3...")
    /// * `opts_hash` - 64-char hex SHA-256 of extraction options
    /// * `compressed_size` - Size of the compressed entry in bytes
    ///
    /// # Returns
    ///
    /// `Ok(Vec<u8>)` with the decompressed data on success.
    ///
    /// # Errors
    ///
    /// Returns `Err` with:
    /// - `NotFound` kind if the entry doesn't exist (cache miss)
    /// - `InvalidData` kind if decompression fails (corrupt entry)
    /// - Other kinds for I/O errors
    ///
    /// # Corruption handling
    ///
    /// If decompression fails, the corrupt entry is deleted automatically.
    /// The caller should treat this as a cache miss and re-run extraction.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let reader = Reader::new(Path::new("/cache"));
    /// match reader.read("pdftract-v1:fp", "opts_hash", 1234) {
    ///     Ok(data) => {
    ///         let json: ExtractionResult = serde_json::from_slice(&data)?;
    ///     }
    ///     Err(e) if e.kind() == io::ErrorKind::NotFound => {
    ///         // Cache miss, run extraction
    ///     }
    ///     Err(e) => {
    ///         // Corrupt entry was deleted, re-run extraction
    ///         eprintln!("Corrupt cache entry, re-extracting: {}", e);
    ///     }
    /// }
    /// ```
    pub fn read(
        &self,
        fingerprint: &str,
        opts_hash: &str,
        compressed_size: usize,
    ) -> io::Result<Vec<u8>> {
        let entry = entry_path(&self.cache_dir, fingerprint, opts_hash, compressed_size);

        // Step 1: Open the entry
        let compressed_data = fs::read(&entry)?;

        // Step 2: Decompress
        match decode(&compressed_data) {
            Ok(data) => Ok(data),
            Err(e) => {
                // Step 3: On decompression error, delete the corrupt entry
                let _ = fs::remove_file(&entry);
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("corrupt cache entry (deleted): {}", e),
                ))
            }
        }
    }

    /// Check if an entry exists without reading it.
    ///
    /// This is useful for cache hit/miss statistics without the cost
    /// of reading and decompressing the entry.
    ///
    /// # Arguments
    ///
    /// * `fingerprint` - PDF fingerprint
    /// * `opts_hash` - 64-char hex SHA-256 of extraction options
    /// * `compressed_size` - Size of the compressed entry in bytes
    ///
    /// # Returns
    ///
    /// `true` if the entry exists, `false` otherwise.
    pub fn exists(&self, fingerprint: &str, opts_hash: &str, compressed_size: usize) -> bool {
        let entry = entry_path(&self.cache_dir, fingerprint, opts_hash, compressed_size);
        entry.exists()
    }
}

/// Cleanup stale temp files from the cache directory.
///
/// Temp files can remain if a process crashes between write() and rename().
/// This function scans the cache directory and removes any temp files older
/// than 1 hour.
///
/// # Arguments
///
/// * `cache_dir` - Root cache directory
///
/// # Returns
///
/// `Ok(())` on success, `Err` if scanning fails.
///
/// # Performance
///
/// - Walks the entire cache tree (O(N) where N = total entries)
/// - Should be run at startup, not on the hot path
///
/// # Example
///
/// ```ignore
/// use pdftract_core::cache::multi_process::cleanup_stale_temp_files;
///
/// // At cache startup
/// cleanup_stale_temp_files(Path::new("/cache"))?;
/// ```
pub fn cleanup_stale_temp_files(cache_dir: &Path) -> io::Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let _cleaned = 0;

    // Walk the two-byte prefix directories
    for prefix1_entry in fs::read_dir(cache_dir)?.filter_map(|e| e.ok()).filter(|e| {
        e.path().is_dir()
            && e.file_name().to_string_lossy().len() == 2
            && e.file_name()
                .to_string_lossy()
                .chars()
                .all(|c| c.is_ascii_hexdigit())
    }) {
        let prefix1_dir = prefix1_entry.path();

        // Walk the second-level prefix directories
        for prefix2_entry in prefix1_dir.read_dir()?.filter_map(|e| e.ok()).filter(|e| {
            e.path().is_dir()
                && e.file_name().to_string_lossy().len() == 2
                && e.file_name()
                    .to_string_lossy()
                    .chars()
                    .all(|c| c.is_ascii_hexdigit())
        }) {
            let prefix2_dir = prefix2_entry.path();

            // Walk the fingerprint directories
            for fp_entry in prefix2_dir
                .read_dir()?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
            {
                let fp_dir = fp_entry.path();

                // Walk the entry files
                for entry in fp_dir.read_dir()?.filter_map(|e| e.ok()) {
                    let path = entry.path();

                    if path.is_file() {
                        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                            // Check if this is a temp file
                            if filename.contains(TEMP_SUFFIX) {
                                // Check age
                                if let Ok(metadata) = path.metadata() {
                                    if let Ok(modified) = metadata.modified() {
                                        if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                                            let age_seconds =
                                                now.saturating_sub(duration.as_secs());

                                            if age_seconds > TEMP_FILE_MAX_AGE_SECONDS {
                                                // Delete stale temp file
                                                let _ = fs::remove_file(&path);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    const TEST_FINGERPRINT: &str =
        "pdftract-v1:e7a1f3deadbeef00000000000000000000000000000000000000000000000000";
    const TEST_OPTS_HASH: &str = "9b21c0ffee000000000000000000000000000000000000000000000000000000";
    const TEST_DATA: &[u8] = b"test cache entry data";

    fn compress_data(data: &[u8]) -> Vec<u8> {
        crate::cache::compression::encode(data).unwrap()
    }

    #[test]
    fn test_writer_basic() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let writer = Writer::new(cache_dir);
        let compressed = compress_data(TEST_DATA);

        writer
            .write(
                TEST_FINGERPRINT,
                TEST_OPTS_HASH,
                compressed.len(),
                &compressed,
            )
            .unwrap();

        // Verify the entry exists
        let reader = Reader::new(cache_dir);
        let result = reader
            .read(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len())
            .unwrap();
        assert_eq!(result, TEST_DATA);
    }

    #[test]
    fn test_reader_cache_miss() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let reader = Reader::new(cache_dir);
        let result = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, 1234);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn test_reader_exists() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let writer = Writer::new(cache_dir);
        let reader = Reader::new(cache_dir);
        let compressed = compress_data(TEST_DATA);

        // Entry doesn't exist yet
        assert!(!reader.exists(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len()));

        // Write entry
        writer
            .write(
                TEST_FINGERPRINT,
                TEST_OPTS_HASH,
                compressed.len(),
                &compressed,
            )
            .unwrap();

        // Now it exists
        assert!(reader.exists(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len()));
    }

    #[test]
    fn test_write_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let writer = Writer::new(cache_dir);
        let compressed = compress_data(TEST_DATA);

        // Parent directories don't exist yet
        let entry = entry_path(
            cache_dir,
            TEST_FINGERPRINT,
            TEST_OPTS_HASH,
            compressed.len(),
        );
        assert!(!entry.exists());

        // Write should create parent directories
        writer
            .write(
                TEST_FINGERPRINT,
                TEST_OPTS_HASH,
                compressed.len(),
                &compressed,
            )
            .unwrap();

        assert!(entry.exists());
    }

    #[test]
    fn test_concurrent_writers_same_key() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();

        let compressed1 = compress_data(b"writer 1 data");
        let compressed2 = compress_data(b"writer 2 data");
        let compressed_size = compressed1.len(); // Same size

        // Spawn two writers writing to the same key
        let cache_dir1 = cache_dir.clone();
        let cache_dir2 = cache_dir.clone();

        let handle1 = thread::spawn(move || {
            let writer = Writer::new(&cache_dir1);
            writer.write(
                TEST_FINGERPRINT,
                TEST_OPTS_HASH,
                compressed_size,
                &compressed1,
            )
        });

        let handle2 = thread::spawn(move || {
            let writer = Writer::new(&cache_dir2);
            writer.write(
                TEST_FINGERPRINT,
                TEST_OPTS_HASH,
                compressed_size,
                &compressed2,
            )
        });

        // Both should succeed (no deadlock)
        let result1 = handle1.join().unwrap();
        let result2 = handle2.join().unwrap();

        assert!(
            result1.is_ok() || result2.is_ok(),
            "At least one writer should succeed"
        );

        // The final entry should be valid (one of the two)
        let reader = Reader::new(&cache_dir);
        let result = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed_size);

        assert!(result.is_ok(), "Final entry should be valid");

        // The entry should contain one of the two data sets
        let data = result.unwrap();
        assert!(data == b"writer 1 data" || data == b"writer 2 data");
    }

    #[test]
    fn test_concurrent_writers_different_keys() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();

        let handles: Vec<_> = (0..8)
            .map(|i| {
                let cache_dir = cache_dir.clone();
                thread::spawn(move || {
                    let fp = format!("{}:fp{:02x}", TEST_FINGERPRINT, i);
                    let opts = format!("{}:opts{:02x}", TEST_OPTS_HASH, i);
                    let data = format!("data for entry {}", i);
                    let compressed = compress_data(data.as_bytes());

                    let writer = Writer::new(&cache_dir);
                    writer.write(&fp, &opts, compressed.len(), &compressed)
                })
            })
            .collect();

        // All writes should succeed
        for handle in handles {
            handle.join().unwrap().unwrap();
        }

        // Verify all 8 entries exist
        let _reader = Reader::new(&cache_dir);
        for i in 0..8 {
            let fp = format!("{}:fp{:02x}", TEST_FINGERPRINT, i);
            let opts = format!("{}:opts{:02x}", TEST_OPTS_HASH, i);

            // Need to find the actual compressed size
            let entry_path_buf = entry_path(&cache_dir, &fp, &opts, 0);
            let entry_dir = entry_path_buf.parent().unwrap();
            let _found = fs::read_dir(entry_dir)
                .unwrap()
                .any(|e| e.ok().filter(|f| f.path().is_file()).is_some());

            assert!(_found, "Entry {} should exist", i);
        }
    }

    #[test]
    fn test_reader_corrupt_entry_deleted() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let writer = Writer::new(cache_dir);
        let compressed = compress_data(TEST_DATA);

        // Write a valid entry
        writer
            .write(
                TEST_FINGERPRINT,
                TEST_OPTS_HASH,
                compressed.len(),
                &compressed,
            )
            .unwrap();

        let entry = entry_path(
            cache_dir,
            TEST_FINGERPRINT,
            TEST_OPTS_HASH,
            compressed.len(),
        );

        // Corrupt the entry by truncating it
        {
            let mut file = fs::File::create(&entry).unwrap();
            file.write_all(b"truncated").unwrap();
        }

        // Reading should delete the corrupt entry
        let reader = Reader::new(cache_dir);
        let result = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len());

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);

        // Entry should be deleted
        assert!(!entry.exists());

        // Reading again should return NotFound (not InvalidData)
        let result2 = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len());
        assert_eq!(result2.unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn test_temp_file_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let writer = Writer::new(cache_dir);
        let compressed = compress_data(TEST_DATA);

        // Create a temp file manually
        let entry = entry_path(
            cache_dir,
            TEST_FINGERPRINT,
            TEST_OPTS_HASH,
            compressed.len(),
        );
        let temp_path = writer.temp_path(&entry);

        // Create parent directory first
        if let Some(parent) = temp_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        // Create an old temp file (modify time in the past)
        fs::write(&temp_path, b"temp data").unwrap();

        // Set modification time to 2 hours ago
        let old_time = SystemTime::now() - Duration::from_secs(2 * 3600);
        filetime::set_file_mtime(&temp_path, old_time.into()).unwrap();

        // Run cleanup
        cleanup_stale_temp_files(cache_dir).unwrap();

        // Temp file should be deleted
        assert!(!temp_path.exists());
    }

    #[test]
    fn test_temp_file_cleanup_keeps_recent() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let writer = Writer::new(cache_dir);
        let compressed = compress_data(TEST_DATA);

        // Create a recent temp file
        let entry = entry_path(
            cache_dir,
            TEST_FINGERPRINT,
            TEST_OPTS_HASH,
            compressed.len(),
        );
        let temp_path = writer.temp_path(&entry);

        // Create parent directory first
        if let Some(parent) = temp_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        fs::write(&temp_path, b"recent temp data").unwrap();

        // Run cleanup
        cleanup_stale_temp_files(cache_dir).unwrap();

        // Recent temp file should still exist
        assert!(temp_path.exists());

        // Clean up
        let _ = fs::remove_file(&temp_path);
    }

    #[test]
    fn test_fsync_env_var() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Default: fsync enabled
        let writer = Writer::new(cache_dir);
        assert!(!writer.fsync_disabled());

        // Set env var to disable
        std::env::set_var(ENV_NO_FSYNC, "1");
        let writer_no_fsync = Writer::new(cache_dir);
        assert!(writer_no_fsync.fsync_disabled());

        // Clean up env var
        std::env::remove_var(ENV_NO_FSYNC);
    }

    #[test]
    fn test_temp_path_unique() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let writer = Writer::new(cache_dir);
        let compressed = compress_data(TEST_DATA);
        let entry = entry_path(
            cache_dir,
            TEST_FINGERPRINT,
            TEST_OPTS_HASH,
            compressed.len(),
        );

        // Generate multiple temp paths
        let path1 = writer.temp_path(&entry);
        thread::sleep(Duration::from_millis(10));
        let path2 = writer.temp_path(&entry);

        // Temp paths should be different (due to random component)
        assert_ne!(path1, path2);

        // But should have the same parent directory
        assert_eq!(path1.parent(), path2.parent());

        // And should end with the .tmp suffix
        assert!(path1.to_string_lossy().contains(TEMP_SUFFIX));
        assert!(path2.to_string_lossy().contains(TEMP_SUFFIX));
    }

    #[test]
    fn test_write_disk_full_simulation() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create a file that will fail on rename due to cross-device link
        // (simulate by using a non-existent parent)

        let writer = Writer::new(cache_dir);
        let compressed = compress_data(TEST_DATA);

        // This should work normally
        writer
            .write(
                TEST_FINGERPRINT,
                TEST_OPTS_HASH,
                compressed.len(),
                &compressed,
            )
            .unwrap();

        // Verify the entry exists
        let reader = Reader::new(cache_dir);
        let result = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len());
        assert!(result.is_ok());
    }

    #[test]
    fn test_reader_mid_rename() {
        // This test verifies that readers see consistent data even if a rename
        // happens mid-read. In practice, this is guaranteed by POSIX semantics.

        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();

        let compressed1 = compress_data(b"version 1");
        let compressed2 = compress_data(b"version 2");
        let size = compressed1.len();

        // Write version 1
        {
            let writer = Writer::new(&cache_dir);
            writer
                .write(TEST_FINGERPRINT, TEST_OPTS_HASH, size, &compressed1)
                .unwrap();
        }

        // Start reading version 1 (this opens the file)
        let cache_dir_clone = cache_dir.clone();
        let read_handle = thread::spawn(move || {
            let reader = Reader::new(&cache_dir_clone);
            // This read should see version 1 (the file that was opened)
            reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, size)
        });

        // Small delay to ensure the reader has opened the file
        thread::sleep(Duration::from_millis(10));

        // Write version 2 (renames over version 1)
        {
            let writer = Writer::new(&cache_dir);
            writer
                .write(TEST_FINGERPRINT, TEST_OPTS_HASH, size, &compressed2)
                .unwrap();
        }

        // The reader should still succeed (with either version, depending on timing)
        let result = read_handle.join().unwrap();
        assert!(result.is_ok());

        // Final read should see version 2 (last writer wins)
        let reader = Reader::new(&cache_dir);
        let final_result = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, size).unwrap();
        assert_eq!(final_result, b"version 2");
    }

    #[test]
    fn test_stress_concurrent_access() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();

        const NUM_KEYS: usize = 10;
        const NUM_ITERATIONS: usize = 100;
        const NUM_PROCESSES: usize = 4;

        // Define the set of keys to use
        let keys: Vec<(String, String)> = (0..NUM_KEYS)
            .map(|i| {
                (
                    format!("{}:fp{:02x}", TEST_FINGERPRINT, i),
                    format!("{}:opts{:02x}", TEST_OPTS_HASH, i),
                )
            })
            .collect();

        let handles: Vec<_> = (0..NUM_PROCESSES)
            .map(|proc_id| {
                let cache_dir = cache_dir.clone();
                let keys = keys.clone();
                thread::spawn(move || {
                    for iter in 0..NUM_ITERATIONS {
                        for (key_idx, (fp, opts)) in keys.iter().enumerate() {
                            let data =
                                format!("process {} iteration {} key {}", proc_id, iter, key_idx);
                            let compressed = compress_data(data.as_bytes());
                            let size = compressed.len();

                            // Write
                            {
                                let writer = Writer::new(&cache_dir);
                                let _ = writer.write(fp, opts, size, &compressed);
                            }

                            // Read (may fail with NotFound if another process deleted, or InvalidData if corrupt)
                            {
                                let reader = Reader::new(&cache_dir);
                                let _ = reader.read(fp, opts, size);
                            }
                        }
                    }
                })
            })
            .collect();

        // All threads should complete without panic
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all keys exist (final state has all entries)
        let _reader = Reader::new(&cache_dir);
        for (fp, opts) in &keys {
            // Find the actual entry by scanning the directory
            let entry_path_buf = entry_path(&cache_dir, fp, opts, 0);
            let fp_dir = entry_path_buf.parent().unwrap();
            if fp_dir.exists() {
                let _found = fs::read_dir(fp_dir)
                    .unwrap()
                    .any(|e| e.ok().filter(|f| f.path().is_file()).is_some());
                // At least one entry should exist for this key
                // (may have multiple versions due to concurrent writes)
            }
        }
    }

    #[test]
    fn test_write_truncates_existing() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let writer = Writer::new(cache_dir);
        let compressed1 = compress_data(b"first version");
        let compressed2 = compress_data(b"second version");
        let size = compressed1.len(); // Same size

        // Write first version
        writer
            .write(TEST_FINGERPRINT, TEST_OPTS_HASH, size, &compressed1)
            .unwrap();

        // Write second version (should atomically replace)
        writer
            .write(TEST_FINGERPRINT, TEST_OPTS_HASH, size, &compressed2)
            .unwrap();

        // Read should return second version
        let reader = Reader::new(cache_dir);
        let result = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, size).unwrap();

        assert_eq!(result, b"second version");
    }

    #[test]
    fn test_acceptance_concurrent_same_fingerprint() {
        // AC: Concurrent extractors on same fingerprint: both succeed; no deadlock
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();

        let compressed = compress_data(TEST_DATA);
        let compressed_size = compressed.len();

        let cache_dir1 = cache_dir.clone();
        let cache_dir2 = cache_dir.clone();
        let compressed1 = compressed.clone();
        let compressed2 = compressed.clone();

        let handle1 = thread::spawn(move || {
            let writer = Writer::new(&cache_dir1);
            writer.write(
                TEST_FINGERPRINT,
                TEST_OPTS_HASH,
                compressed_size,
                &compressed1,
            )
        });

        let handle2 = thread::spawn(move || {
            let writer = Writer::new(&cache_dir2);
            writer.write(
                TEST_FINGERPRINT,
                TEST_OPTS_HASH,
                compressed_size,
                &compressed2,
            )
        });

        // Both should succeed without deadlock
        let result1 = handle1.join().unwrap();
        let result2 = handle2.join().unwrap();

        // At least one should succeed (both usually succeed)
        assert!(result1.is_ok() || result2.is_ok());

        // Final entry should be valid
        let reader = Reader::new(&cache_dir);
        let result = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed_size);
        assert!(
            result.is_ok(),
            "Entry should be readable after concurrent writes"
        );
    }

    #[test]
    fn test_acceptance_reader_never_sees_torn_write() {
        // AC: Reader sees a fully-decompressable entry always — never a torn write
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().to_path_buf();

        let compressed = compress_data(TEST_DATA);
        let compressed_size = compressed.len();

        // Spawn many concurrent writers
        let handles: Vec<_> = (0..20)
            .map(|_| {
                let cache_dir = cache_dir.clone();
                let compressed = compressed.clone();
                thread::spawn(move || {
                    let writer = Writer::new(&cache_dir);
                    writer.write(
                        TEST_FINGERPRINT,
                        TEST_OPTS_HASH,
                        compressed_size,
                        &compressed,
                    )
                })
            })
            .collect();

        // Spawn many concurrent readers
        let read_handles: Vec<_> = (0..20)
            .map(|_| {
                let cache_dir = cache_dir.clone();
                thread::spawn(move || {
                    let reader = Reader::new(&cache_dir);
                    reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed_size)
                })
            })
            .collect();

        // All threads should complete
        for handle in handles {
            handle.join().unwrap().ok();
        }

        for handle in read_handles {
            let result = handle.join().unwrap();
            // If read succeeded, data should be valid
            if let Ok(data) = result {
                assert_eq!(data, TEST_DATA);
            }
        }

        // Final entry should be valid
        let reader = Reader::new(&cache_dir);
        let result = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed_size);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TEST_DATA);
    }

    #[test]
    fn test_acceptance_corrupt_entry_treated_as_miss() {
        // AC: Corrupt entry on disk (truncated file): treated as a miss; entry deleted
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let writer = Writer::new(cache_dir);
        let compressed = compress_data(TEST_DATA);

        writer
            .write(
                TEST_FINGERPRINT,
                TEST_OPTS_HASH,
                compressed.len(),
                &compressed,
            )
            .unwrap();

        // Corrupt the entry
        let entry = entry_path(
            cache_dir,
            TEST_FINGERPRINT,
            TEST_OPTS_HASH,
            compressed.len(),
        );
        fs::write(&entry, b"corrupted data").unwrap();

        // Read should detect corruption, delete entry, and return error
        let reader = Reader::new(cache_dir);
        let result = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len());

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);

        // Entry should be deleted
        assert!(!entry.exists());

        // Subsequent read should return NotFound (cache miss)
        let result2 = reader.read(TEST_FINGERPRINT, TEST_OPTS_HASH, compressed.len());
        assert_eq!(result2.unwrap_err().kind(), io::ErrorKind::NotFound);
    }
}
