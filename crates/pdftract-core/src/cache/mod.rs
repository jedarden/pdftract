//! Content-addressed cache layer for extraction results.
//!
//! This module implements Phase 6.9 of the implementation plan: a filesystem-based
//! cache that stores extraction results keyed by PDF fingerprint and extraction options.
//! The cache uses a two-byte prefix scheme to keep directory fan-out balanced even
//! at millions of entries.
//!
//! # Layout
//!
//! ```text
//! <cache_dir>/
//!   index.json                              # cache version + metadata
//!   key                                     # HMAC-SHA-256 key (256-bit, mode 0600)
//!   sentinel.touched                        # O_APPEND sentinel for LRU tracking
//!   <fp[0:2]>/<fp[2:4]>/<full_fp>/         # fingerprint-based path
//!     <opts_hash>-<size>.json.zst           # cached extraction, zstd-compressed
//!                                          # Format: [8-byte HMAC][compressed JSON]
//! ```
//!
//! # Module Structure
//!
//! - [`layout`] — Path construction and directory creation
//! - [`key`] — Cache key construction from (fingerprint, options) pairs
//! - [`compression`] — Zstandard compression/decompression for cache entries
//! - [`integrity`] — HMAC-SHA-256 integrity verification (TH-10 mitigation)
//! - `metadata` — Cache index.json and metadata handling (TODO: 6.9.3)

pub mod compression;
pub mod integrity;
pub mod key;
pub mod layout;
pub mod lru;
pub mod multi_process;

pub use integrity::{compute_hmac, init_cache_key, load_cache_key, verify_hmac};
pub use key::CacheKey;
pub use layout::{
    entry_path, increment_hit_counter, increment_miss_counter, CacheIndex, CURRENT_SCHEMA_VERSION,
};
pub use lru::Lru;
pub use multi_process::{cleanup_stale_temp_files, Reader, Writer};

use crate::extract::ExtractionResult;
use crate::options::ExtractionOptions;
use anyhow::{Context, Result};
use serde_json;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Result of a cache lookup operation.
#[derive(Debug)]
pub enum CacheLookupResult {
    /// Cache hit: entry found and deserialized successfully
    Hit {
        /// The cached extraction result
        result: ExtractionResult,
        /// Age of the cache entry in seconds (time since creation)
        age_seconds: u64,
    },
    /// Cache miss: entry not found or corrupt (will be overwritten)
    Miss,
    /// Cache skipped: cache not configured or disabled
    Skipped,
}

/// Perform extraction with cache integration.
///
/// This function implements the full cache read/write flow:
/// 1. Compute cache key from fingerprint and options
/// 2. Try to read from cache
/// 3. On hit: increment hit counter, return cached result with age
/// 4. On miss: increment miss counter, run extraction, write to cache
/// 5. On skip: run extraction without cache operations
///
/// # Arguments
///
/// * `pdf_path` - Path to the PDF file
/// * `options` - Extraction options
/// * `cache_dir` - Optional cache directory path
/// * `cache_disabled` - Whether cache is globally disabled
/// * `cache_size_bytes` - Optional cache size limit in bytes (default: 1 GiB)
///
/// # Returns
///
/// A tuple of (ExtractionResult, cache_status, cache_age_seconds):
/// - cache_status: "hit", "miss", or "skipped"
/// - cache_age_seconds: Some(age) on hit, None otherwise
///
/// # Example
///
/// ```ignore
/// use pdftract_core::cache::extract_with_cache;
/// use pdftract_core::options::ExtractionOptions;
///
/// let options = ExtractionOptions::default();
/// let (result, status, age) = extract_with_cache(
///     Path::new("doc.pdf"),
///     &options,
///     Some(Path::new("/cache")),
///     false,
///     Some(1024 * 1024 * 1024) // 1 GiB
/// )?;
/// ```
pub fn extract_with_cache(
    pdf_path: &Path,
    options: &ExtractionOptions,
    cache_dir: Option<&Path>,
    cache_disabled: bool,
    cache_size_bytes: Option<u64>,
) -> Result<(ExtractionResult, String, Option<u64>)> {
    // Check if cache is disabled
    if cache_disabled || cache_dir.is_none() {
        let result = crate::extract::extract_pdf(pdf_path, options)?;
        return Ok((result, "skipped".to_string(), None));
    }

    let cache_dir = cache_dir.unwrap();

    // First, we need the fingerprint to compute the cache key
    // We can't get this without parsing the PDF, so we parse but don't extract
    let (fingerprint, _catalog, _pages, _resolver) = crate::document::parse_pdf_file(pdf_path)
        .context("Failed to parse PDF file for fingerprint")?;

    // Compute cache key
    let key = CacheKey::new(&fingerprint, options);

    // Try to read from cache
    let _reader = Reader::new(cache_dir);

    // We need to find the actual entry file size since we don't know it ahead of time
    // Walk the fingerprint directory to find the entry with matching opts_hash
    let cache_result = find_cached_entry(cache_dir, &fingerprint, &key.opts_hash);

    match cache_result {
        Ok(Some((compressed_data, age_seconds))) => {
            // Try to deserialize
            match serde_json::from_slice::<ExtractionResult>(&compressed_data) {
                Ok(result) => {
                    // Cache hit - increment counter and touch the entry
                    let _ = increment_hit_counter(cache_dir);
                    let lru = Lru::new(
                        cache_dir,
                        cache_size_bytes.unwrap_or(lru::DEFAULT_CACHE_SIZE_BYTES),
                    );
                    let _ = lru.touch(&fingerprint, &key.opts_hash);
                    return Ok((result, "hit".to_string(), Some(age_seconds)));
                }
                Err(_) => {
                    // Deserialization failed - treat as corrupt entry
                    // The caller will overwrite it
                }
            }
        }
        Ok(None) => {
            // Cache miss - continue to extraction
        }
        Err(_) => {
            // Error reading cache - continue to extraction
        }
    }

    // Cache miss - increment counter and run extraction
    let _ = increment_miss_counter(cache_dir);
    let result = crate::extract::extract_pdf(pdf_path, options)?;

    // Write to cache (serialize and compress)
    match serde_json::to_vec(&result) {
        Ok(json_data) => {
            match compression::encode(&json_data) {
                Ok(compressed) => {
                    let writer = Writer::new(cache_dir);
                    let _ =
                        writer.write(&fingerprint, &key.opts_hash, compressed.len(), &compressed);

                    // Update index entry count and total bytes
                    if let Ok(mut index) = layout::load_index(cache_dir) {
                        let index = index.get_or_insert_with(Default::default);
                        index.entry_count += 1;
                        index.total_bytes += compressed.len() as u64;
                        let _ = layout::save_index(cache_dir, index);
                    }

                    // Trigger LRU eviction if needed
                    let lru = Lru::new(
                        cache_dir,
                        cache_size_bytes.unwrap_or(lru::DEFAULT_CACHE_SIZE_BYTES),
                    );
                    let _ = lru.maybe_evict();
                }
                Err(_) => {
                    // Compression failed - continue without caching
                }
            }
        }
        Err(_) => {
            // Serialization failed - continue without caching
        }
    }

    Ok((result, "miss".to_string(), None))
}

/// Find a cached entry by fingerprint and opts_hash.
///
/// Returns Ok(Some((data, age_seconds))) on hit, Ok(None) on miss,
/// Err on I/O error.
fn find_cached_entry(
    cache_dir: &Path,
    fingerprint: &str,
    opts_hash: &str,
) -> Result<Option<(Vec<u8>, u64)>, std::io::Error> {
    let fp_dir = layout::fingerprint_dir(cache_dir, fingerprint);

    if !fp_dir.exists() {
        return Ok(None);
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Walk the fingerprint directory to find an entry with matching opts_hash
    for entry in fp_dir.read_dir()? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                // Check if filename starts with opts_hash and ends with .json.zst
                if filename.starts_with(opts_hash) && filename.ends_with(".json.zst") {
                    // Parse the size from the filename
                    if let Some(size) = layout::parse_size_from_filename(filename) {
                        let reader = Reader::new(cache_dir);
                        match reader.read(fingerprint, opts_hash, size) {
                            Ok(data) => {
                                // Get entry age from mtime
                                let age_seconds = if let Ok(metadata) = path.metadata() {
                                    if let Ok(modified) = metadata.modified() {
                                        if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                                            now.saturating_sub(duration.as_secs())
                                        } else {
                                            0
                                        }
                                    } else {
                                        0
                                    }
                                } else {
                                    0
                                };
                                return Ok(Some((data, age_seconds)));
                            }
                            Err(_) => {
                                // Entry corrupt - treat as miss
                                return Ok(None);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(None)
}
