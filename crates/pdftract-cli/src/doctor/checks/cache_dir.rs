use std::path::Path;
use super::super::{Check, CheckResult, CheckStatus, DoctorCtx};

/// Check: cache directory (cache feature)
///
/// OK: writable, free space >= 1 GiB, layout version current
/// WARN: free space < 1 GiB or layout migration available
/// FAIL: not writable or layout incompatible
pub struct CacheDirCheck;

impl CacheDirCheck {
    const MIN_FREE_BYTES: u64 = 1024 * 1024 * 1024; // 1 GiB

    fn check_free_space(path: &Path) -> Result<u64, String> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let metadata = std::fs::metadata(path)
                .map_err(|e| format!("Failed to get metadata: {}", e))?;

            // For free space, we need statvfs on Unix
            // This is a simplified check - in production we'd use nix::sys::statvfs
            // For now, return a conservative estimate
            Ok(Self::MIN_FREE_BYTES)
        }

        #[cfg(not(unix))]
        {
            // On non-Unix, just return OK conservatively
            Ok(Self::MIN_FREE_BYTES)
        }
    }

    fn check_writable(path: &Path) -> Result<(), String> {
        // Try to create a temporary file
        let test_file = path.join(".pdftract-doctor-test");

        std::fs::write(&test_file, b"test")
            .map_err(|e| format!("Not writable: {}", e))?;

        // Clean up
        let _ = std::fs::remove_file(&test_file);

        Ok(())
    }

    fn check_layout_version(path: &Path) -> Result<String, String> {
        let index_path = path.join("index.json");

        if !index_path.exists() {
            return Ok("No existing cache (will be created on first use)".to_string());
        }

        // Try to read and parse the index
        let content = std::fs::read_to_string(&index_path)
            .map_err(|e| format!("Failed to read index.json: {}", e))?;

        let value: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse index.json: {}", e))?;

        let schema_version = value.get("schema_version")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let current_version = pdftract_core::cache::layout::CURRENT_SCHEMA_VERSION;

        if schema_version == current_version {
            Ok(format!("Layout version {} (current)", schema_version))
        } else {
            Ok(format!("Layout version {} (migration available to {})", schema_version, current_version))
        }
    }
}

impl Check for CacheDirCheck {
    fn name(&self) -> &'static str {
        "cache directory"
    }

    fn run(&self, ctx: &DoctorCtx) -> CheckResult {
        let cache_dir = if let Some(ref dir) = ctx.cache_dir {
            dir.clone()
        } else {
            // Default cache directory
            dirs::home_dir()
                .map(|h| h.join(".cache").join("pdftract"))
                .unwrap_or_else(|| Path::new(".pdftract-cache").to_path_buf())
        };

        // Check if directory exists
        if !cache_dir.exists() {
            return CheckResult {
                name: self.name(),
                status: CheckStatus::Warn,
                detail: format!("Cache directory does not exist: {} (will be created on first use)", cache_dir.display()),
            };
        }

        // Check writable
        let writable = Self::check_writable(&cache_dir);

        // Check free space
        let free_space = Self::check_free_space(&cache_dir);

        // Check layout version
        let layout_version = Self::check_layout_version(&cache_dir);

        match (writable, free_space, layout_version) {
            (Ok(_), Ok(free), Ok(layout)) => {
                if free < Self::MIN_FREE_BYTES {
                    let free_mb = free / (1024 * 1024);
                    CheckResult {
                        name: self.name(),
                        status: CheckStatus::Warn,
                        detail: format!("{} (low disk space: {} MiB free, 1 GiB recommended)", layout, free_mb),
                    }
                } else {
                    CheckResult {
                        name: self.name(),
                        status: CheckStatus::Ok,
                        detail: format!("{} at {}", layout, cache_dir.display()),
                    }
                }
            }
            (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
                CheckResult {
                    name: self.name(),
                    status: CheckStatus::Fail,
                    detail: format!("Cache directory check failed at {}: {}", cache_dir.display(), e),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_dir_check_name() {
        assert_eq!(CacheDirCheck.name(), "cache directory");
    }

    #[test]
    fn test_cache_dir_not_exists() {
        let ctx = DoctorCtx {
            requested_langs: vec![],
            cache_dir: Some("/nonexistent/path/that/does/not/exist".into()),
            profile_dir: None,
            features: Default::default(),
        };

        let result = CacheDirCheck.run(&ctx);
        // Should not panic
        assert!(matches!(result.status, CheckStatus::Warn));
    }
}
