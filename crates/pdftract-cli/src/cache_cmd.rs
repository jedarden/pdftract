//! Cache subcommand for managing the pdftract content-addressed cache.
//!
//! This module implements the `pdftract cache` subcommand with:
//! - `stats DIR` - show cache statistics
//! - `clear DIR` - delete all cache entries
//! - `purge DIR --older-than DURATION` - delete entries older than duration
//! - `purge DIR --version CONSTRAINT` - delete entries matching version constraint

use anyhow::{bail, Context, Result};
use pdftract_core::cache::layout::{self, CacheIndex};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Cache statistics for display.
#[derive(Debug)]
pub struct CacheStats {
    /// Number of cache entries
    pub entry_count: u64,
    /// Total compressed size in bytes
    pub total_compressed_bytes: u64,
    /// Total uncompressed size in bytes
    pub total_uncompressed_bytes: u64,
    /// Cache hits since last clear
    pub hits: u64,
    /// Total accesses since last clear
    pub total_accesses: u64,
    /// Oldest entry age in seconds
    pub oldest_entry_age_seconds: Option<u64>,
    /// Newest entry age in seconds
    pub newest_entry_age_seconds: Option<u64>,
    /// Age histogram buckets
    pub age_histogram: AgeHistogram,
}

/// Age histogram buckets.
#[derive(Debug, Default)]
pub struct AgeHistogram {
    pub less_than_1h: u64,
    pub less_than_1d: u64,
    pub less_than_7d: u64,
    pub less_than_30d: u64,
    pub greater_than_30d: u64,
}

impl AgeHistogram {
    /// Record an entry age in seconds.
    pub fn record(&mut self, age_seconds: u64) {
        if age_seconds < 3600 {
            self.less_than_1h += 1;
        } else if age_seconds < 86400 {
            self.less_than_1d += 1;
        } else if age_seconds < 604800 {
            self.less_than_7d += 1;
        } else if age_seconds < 2592000 {
            self.less_than_30d += 1;
        } else {
            self.greater_than_30d += 1;
        }
    }

    /// Total entries in histogram.
    pub fn total(&self) -> u64 {
        self.less_than_1h
            + self.less_than_1d
            + self.less_than_7d
            + self.less_than_30d
            + self.greater_than_30d
    }

    /// Get percentage for a bucket.
    pub fn percentage(&self, count: u64) -> f64 {
        let total = self.total();
        if total == 0 {
            0.0
        } else {
            (count as f64 / total as f64) * 100.0
        }
    }
}

/// Compute cache statistics for a given cache directory.
pub fn compute_stats(cache_dir: &Path) -> Result<CacheStats> {
    // If cache directory doesn't exist, return zero stats
    if !cache_dir.exists() {
        return Ok(CacheStats {
            entry_count: 0,
            total_compressed_bytes: 0,
            total_uncompressed_bytes: 0,
            hits: 0,
            total_accesses: 0,
            oldest_entry_age_seconds: None,
            newest_entry_age_seconds: None,
            age_histogram: AgeHistogram::default(),
        });
    }

    let index = layout::load_index(cache_dir)?.unwrap_or_default();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut stats = CacheStats {
        entry_count: 0,
        total_compressed_bytes: 0,
        total_uncompressed_bytes: 0,
        hits: 0,
        total_accesses: 0,
        oldest_entry_age_seconds: None,
        newest_entry_age_seconds: None,
        age_histogram: AgeHistogram::default(),
    };

    // Walk the cache directory to compute statistics
    let mut oldest_mtime = None;
    let mut newest_mtime = None;

    for prefix1_entry in fs::read_dir(cache_dir)?.filter_map(|e| e.ok()).filter(|e| {
        e.path().is_dir()
            && e.file_name().to_string_lossy().len() == 2
            && e.file_name()
                .to_string_lossy()
                .chars()
                .all(|c| c.is_ascii_hexdigit())
    }) {
        let prefix1_dir = prefix1_entry.path();

        for prefix2_entry in prefix1_dir.read_dir()?.filter_map(|e| e.ok()).filter(|e| {
            e.path().is_dir()
                && e.file_name().to_string_lossy().len() == 2
                && e.file_name()
                    .to_string_lossy()
                    .chars()
                    .all(|c| c.is_ascii_hexdigit())
        }) {
            let prefix2_dir = prefix2_entry.path();

            for fp_entry in prefix2_dir
                .read_dir()?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
            {
                let fp_dir = fp_entry.path();

                for entry in fp_dir.read_dir()?.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                            if let Some(size) = layout::parse_size_from_filename(filename) {
                                stats.entry_count += 1;
                                stats.total_compressed_bytes += size as u64;

                                // Get mtime for age tracking
                                if let Ok(metadata) = path.metadata() {
                                    if let Ok(modified) = metadata.modified() {
                                        if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                                            let mtime_secs = duration.as_secs();
                                            if oldest_mtime.is_none()
                                                || Some(mtime_secs) < oldest_mtime
                                            {
                                                oldest_mtime = Some(mtime_secs);
                                            }
                                            if newest_mtime.is_none()
                                                || Some(mtime_secs) > newest_mtime
                                            {
                                                newest_mtime = Some(mtime_secs);
                                            }

                                            // Record in histogram
                                            let age = now.saturating_sub(mtime_secs);
                                            stats.age_histogram.record(age);
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

    // Compute age stats
    if let Some(oldest) = oldest_mtime {
        stats.oldest_entry_age_seconds = Some(now.saturating_sub(oldest));
    }
    if let Some(newest) = newest_mtime {
        stats.newest_entry_age_seconds = Some(now.saturating_sub(newest));
    }

    // Estimate uncompressed size (assuming 8.5x compression ratio based on typical text)
    stats.total_uncompressed_bytes = stats.total_compressed_bytes * 85 / 10;

    // Hit ratio from index (if available)
    stats.hits = index.hits;
    stats.total_accesses = index.total_accesses;

    Ok(stats)
}

/// Display cache statistics in human-readable format.
pub fn display_stats(stats: &CacheStats) {
    let compressed_mb = stats.total_compressed_bytes as f64 / (1024.0 * 1024.0);
    let uncompressed_mb = stats.total_uncompressed_bytes as f64 / (1024.0 * 1024.0);
    let ratio = if stats.total_compressed_bytes > 0 {
        stats.total_uncompressed_bytes as f64 / stats.total_compressed_bytes as f64
    } else {
        0.0
    };

    let hit_ratio = if stats.total_accesses > 0 {
        (stats.hits as f64 / stats.total_accesses as f64) * 100.0
    } else {
        0.0
    };

    println!("Entries: {}", stats.entry_count);
    println!(
        "Total size: {:.1} MiB compressed / {:.1} GiB uncompressed ({:.1}x ratio)",
        compressed_mb,
        uncompressed_mb / 1024.0,
        ratio
    );
    println!(
        "Hit ratio (since last clear): {:.1}% ({} hits / {} total)",
        hit_ratio, stats.hits, stats.total_accesses
    );

    if let Some(oldest) = stats.oldest_entry_age_seconds {
        let days = oldest / 86400;
        let hours = (oldest % 86400) / 3600;
        println!("Oldest entry: {}d {}h ago", days, hours);
    } else {
        println!("Oldest entry: (none)");
    }

    if let Some(newest) = stats.newest_entry_age_seconds {
        if newest < 60 {
            println!("Newest entry: {}s ago", newest);
        } else if newest < 3600 {
            println!("Newest entry: {}m {}s ago", newest / 60, newest % 60);
        } else {
            let hours = newest / 3600;
            let minutes = (newest % 3600) / 60;
            println!("Newest entry: {}h {}m ago", hours, minutes);
        }
    } else {
        println!("Newest entry: (none)");
    }

    let h = &stats.age_histogram;
    println!(
        "Age histogram: <1h: {:.1}%, <1d: {:.1}%, <7d: {:.1}%, <30d: {:.1}%, >30d: {:.1}%",
        h.percentage(h.less_than_1h),
        h.percentage(h.less_than_1d),
        h.percentage(h.less_than_7d),
        h.percentage(h.less_than_30d),
        h.percentage(h.greater_than_30d)
    );
}

/// Display cache statistics in JSON format.
pub fn display_stats_json(stats: &CacheStats) -> Result<()> {
    let json = serde_json::json!({
        "entry_count": stats.entry_count,
        "total_compressed_bytes": stats.total_compressed_bytes,
        "total_uncompressed_bytes": stats.total_uncompressed_bytes,
        "compression_ratio": if stats.total_compressed_bytes > 0 {
            stats.total_uncompressed_bytes as f64 / stats.total_compressed_bytes as f64
        } else {
            0.0
        },
        "hits": stats.hits,
        "total_accesses": stats.total_accesses,
        "hit_ratio_percent": if stats.total_accesses > 0 {
            (stats.hits as f64 / stats.total_accesses as f64) * 100.0
        } else {
            0.0
        },
        "oldest_entry_age_seconds": stats.oldest_entry_age_seconds,
        "newest_entry_age_seconds": stats.newest_entry_age_seconds,
        "age_histogram": {
            "less_than_1h": stats.age_histogram.less_than_1h,
            "less_than_1d": stats.age_histogram.less_than_1d,
            "less_than_7d": stats.age_histogram.less_than_7d,
            "less_than_30d": stats.age_histogram.less_than_30d,
            "greater_than_30d": stats.age_histogram.greater_than_30d,
        }
    });
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

/// Clear all cache entries from the directory.
///
/// Prompts for confirmation unless -y is specified.
pub fn clear_cache(cache_dir: &Path, yes: bool) -> Result<()> {
    // Check if directory exists
    if !cache_dir.exists() {
        println!("Cache directory does not exist: {}", cache_dir.display());
        return Ok(());
    }

    // Count entries first
    let entry_count = count_entries(cache_dir)?;

    if entry_count == 0 {
        println!("Cache is empty (0 entries)");
        return Ok(());
    }

    // Confirm unless -y
    if !yes {
        if !prompt_confirmation(&format!("Delete all {} cache entries?", entry_count))? {
            println!("Cancelled");
            return Ok(());
        }
    }

    // Delete all entry files (preserve index.json and sentinel)
    let mut deleted = 0;
    for prefix1_entry in fs::read_dir(cache_dir)?.filter_map(|e| e.ok()).filter(|e| {
        e.path().is_dir()
            && e.file_name().to_string_lossy().len() == 2
            && e.file_name()
                .to_string_lossy()
                .chars()
                .all(|c| c.is_ascii_hexdigit())
    }) {
        let prefix1_dir = prefix1_entry.path();

        for prefix2_entry in prefix1_dir.read_dir()?.filter_map(|e| e.ok()).filter(|e| {
            e.path().is_dir()
                && e.file_name().to_string_lossy().len() == 2
                && e.file_name()
                    .to_string_lossy()
                    .chars()
                    .all(|c| c.is_ascii_hexdigit())
        }) {
            let prefix2_dir = prefix2_entry.path();

            for fp_entry in prefix2_dir
                .read_dir()?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
            {
                let fp_dir = fp_entry.path();

                // Delete all files in the fingerprint directory
                for entry in fp_dir.read_dir()?.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_file() {
                        let _ = fs::remove_file(&path);
                        deleted += 1;
                    }
                }

                // Remove the empty fingerprint directory
                let _ = fs::remove_dir(&fp_dir);
            }

            // Remove empty second-level prefix directory
            if prefix2_dir.read_dir()?.next().is_none() {
                let _ = fs::remove_dir(&prefix2_dir);
            }
        }

        // Remove empty first-level prefix directory
        if prefix1_dir.read_dir()?.next().is_none() {
            let _ = fs::remove_dir(&prefix1_dir);
        }
    }

    // Reset index.json entry count and hit statistics
    let mut index = layout::load_index(cache_dir)?.unwrap_or_default();
    index.entry_count = 0;
    index.total_bytes = 0;
    index.hits = 0;
    index.total_accesses = 0;
    layout::save_index(cache_dir, &index)?;

    println!("Deleted {} cache entries", deleted);
    Ok(())
}

/// Purge cache entries older than the specified duration.
pub fn purge_cache_older_than(cache_dir: &Path, duration_str: &str) -> Result<()> {
    use humantime::parse_duration;

    let duration = parse_duration(duration_str).context(format!(
        "Invalid duration '{}'. Use formats like '30d', '7d', '1h'",
        duration_str
    ))?;

    let cutoff_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .saturating_sub(duration.as_secs());

    let mut deleted = 0;

    for prefix1_entry in fs::read_dir(cache_dir)?.filter_map(|e| e.ok()).filter(|e| {
        e.path().is_dir()
            && e.file_name().to_string_lossy().len() == 2
            && e.file_name()
                .to_string_lossy()
                .chars()
                .all(|c| c.is_ascii_hexdigit())
    }) {
        let prefix1_dir = prefix1_entry.path();

        for prefix2_entry in prefix1_dir.read_dir()?.filter_map(|e| e.ok()).filter(|e| {
            e.path().is_dir()
                && e.file_name().to_string_lossy().len() == 2
                && e.file_name()
                    .to_string_lossy()
                    .chars()
                    .all(|c| c.is_ascii_hexdigit())
        }) {
            let prefix2_dir = prefix2_entry.path();

            for fp_entry in prefix2_dir
                .read_dir()?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
            {
                let fp_dir = fp_entry.path();

                for entry in fp_dir.read_dir()?.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_file() {
                        // Check mtime
                        if let Ok(metadata) = path.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                                    let mtime_secs = duration.as_secs();
                                    if mtime_secs < cutoff_secs {
                                        let _ = fs::remove_file(&path);
                                        deleted += 1;
                                    }
                                }
                            }
                        }
                    }
                }

                // Remove empty fingerprint directory
                if fp_dir.read_dir()?.next().is_none() {
                    let _ = fs::remove_dir(&fp_dir);
                }
            }

            // Remove empty second-level prefix directory
            if prefix2_dir.read_dir()?.next().is_none() {
                let _ = fs::remove_dir(&prefix2_dir);
            }
        }

        // Remove empty first-level prefix directory
        if prefix1_dir.read_dir()?.next().is_none() {
            let _ = fs::remove_dir(&prefix1_dir);
        }
    }

    // Update index (preserve hit stats, update entry count and bytes)
    let remaining = count_entries(cache_dir)?;
    let mut index = layout::load_index(cache_dir)?.unwrap_or_default();
    index.entry_count = remaining;
    index.total_bytes = compute_stats(cache_dir)?.total_compressed_bytes;
    // hits and total_accesses are preserved during purge
    layout::save_index(cache_dir, &index)?;

    println!("Deleted {} entries older than {}", deleted, duration_str);
    Ok(())
}

/// Purge cache entries matching a version constraint.
pub fn purge_cache_version(_cache_dir: &Path, version_constraint: &str) -> Result<()> {
    use semver::VersionReq;

    let _req = VersionReq::parse(version_constraint).context(format!(
        "Invalid version constraint '{}'",
        version_constraint
    ))?;

    // For now, this is a no-op since we don't track extraction versions per entry
    // This would require extending the cache entry metadata
    println!("Version-based purge not yet implemented");
    println!("Entries are tagged with extraction_version in the cache, but version constraint matching is not yet available");
    Ok(())
}

/// Count the total number of cache entries.
fn count_entries(cache_dir: &Path) -> Result<u64> {
    let mut count = 0;

    for prefix1_entry in fs::read_dir(cache_dir)?.filter_map(|e| e.ok()).filter(|e| {
        e.path().is_dir()
            && e.file_name().to_string_lossy().len() == 2
            && e.file_name()
                .to_string_lossy()
                .chars()
                .all(|c| c.is_ascii_hexdigit())
    }) {
        let prefix1_dir = prefix1_entry.path();

        for prefix2_entry in prefix1_dir.read_dir()?.filter_map(|e| e.ok()).filter(|e| {
            e.path().is_dir()
                && e.file_name().to_string_lossy().len() == 2
                && e.file_name()
                    .to_string_lossy()
                    .chars()
                    .all(|c| c.is_ascii_hexdigit())
        }) {
            let prefix2_dir = prefix2_entry.path();

            for fp_entry in prefix2_dir
                .read_dir()?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
            {
                let fp_dir = fp_entry.path();

                for entry in fp_dir.read_dir()?.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                            if layout::parse_size_from_filename(filename).is_some() {
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(count)
}

/// Prompt for confirmation on a TTY.
fn prompt_confirmation(prompt: &str) -> Result<bool> {
    // Check if we're on a TTY
    if !atty::is(atty::Stream::Stdin) {
        bail!("Cannot confirm without -y flag (not a TTY)");
    }

    print!("{} [y/N] ", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let response = input.trim().to_lowercase();
    Ok(response == "y" || response == "yes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_age_histogram() {
        let mut h = AgeHistogram::default();

        h.record(100); // < 1h
        h.record(3600 + 100); // < 1d
        h.record(86400 + 100); // < 7d
        h.record(604800 + 100); // < 30d
        h.record(2592000 + 100); // > 30d

        assert_eq!(h.less_than_1h, 1);
        assert_eq!(h.less_than_1d, 1);
        assert_eq!(h.less_than_7d, 1);
        assert_eq!(h.less_than_30d, 1);
        assert_eq!(h.greater_than_30d, 1);
        assert_eq!(h.total(), 5);

        // Each should be 20%
        assert!((h.percentage(1) - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_age_histogram_percentage() {
        let mut h = AgeHistogram::default();

        h.record(100);
        h.record(200);

        assert_eq!(h.total(), 2);
        assert!((h.percentage(h.less_than_1h) - 100.0).abs() < 0.01);
        assert_eq!(h.percentage(h.less_than_1d), 0.0);
    }

    #[test]
    fn test_compute_stats_empty() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create index.json
        let index = CacheIndex::default();
        layout::save_index(cache_dir, &index).unwrap();

        let stats = compute_stats(cache_dir).unwrap();

        assert_eq!(stats.entry_count, 0);
        assert_eq!(stats.total_compressed_bytes, 0);
        assert_eq!(stats.total_uncompressed_bytes, 0);
        assert!(stats.oldest_entry_age_seconds.is_none());
        assert!(stats.newest_entry_age_seconds.is_none());
    }

    #[test]
    fn test_compute_stats_with_entries() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create a test entry
        let fp = "e7a1f3deadbeef00000000000000000000000000000000000000000000000000";
        let opts = "9b21c0ffee000000000000000000000000000000000000000000000000000000";
        let fp_dir = cache_dir.join("e7").join("a1").join(fp);
        fs::create_dir_all(&fp_dir).unwrap();

        let entry_path = fp_dir.join(format!("{}-1000.json.zst", opts));
        fs::write(&entry_path, b"x".repeat(1000)).unwrap();

        let stats = compute_stats(cache_dir).unwrap();

        assert_eq!(stats.entry_count, 1);
        assert_eq!(stats.total_compressed_bytes, 1000);
        assert!(stats.oldest_entry_age_seconds.is_some());
        assert!(stats.newest_entry_age_seconds.is_some());
        assert_eq!(stats.age_histogram.total(), 1);
    }

    #[test]
    fn test_clear_cache_empty() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create index
        let index = CacheIndex::default();
        layout::save_index(cache_dir, &index).unwrap();

        clear_cache(cache_dir, true).unwrap();

        // Index should still exist but with 0 entries and reset hit stats
        let loaded = layout::load_index(cache_dir).unwrap().unwrap();
        assert_eq!(loaded.entry_count, 0);
        assert_eq!(loaded.hits, 0);
        assert_eq!(loaded.total_accesses, 0);
    }

    #[test]
    fn test_count_entries() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        // Create test entries
        let fp = "e7a1f3deadbeef00000000000000000000000000000000000000000000000000";
        let opts = "9b21c0ffee000000000000000000000000000000000000000000000000000000";
        let fp_dir = cache_dir.join("e7").join("a1").join(fp);
        fs::create_dir_all(&fp_dir).unwrap();

        fs::write(
            fp_dir.join(format!("{}-1000.json.zst", opts)),
            b"x".repeat(1000),
        )
        .unwrap();
        fs::write(
            fp_dir.join(format!("{}-2000.json.zst", opts)),
            b"x".repeat(2000),
        )
        .unwrap();

        let count = count_entries(cache_dir).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_count_entries_empty() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        let count = count_entries(cache_dir).unwrap();
        assert_eq!(count, 0);
    }
}
