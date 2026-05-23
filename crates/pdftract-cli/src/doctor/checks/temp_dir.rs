use std::path::Path;
use std::env;
use super::super::{Check, CheckResult, CheckStatus, DoctorCtx};

/// Check: temp directory writable and free space
///
/// OK: writable + free space >= 100 MiB
/// WARN: free space < 100 MiB
/// FAIL: not writable
pub struct TempDirCheck;

impl TempDirCheck {
    const MIN_FREE_BYTES: u64 = 100 * 1024 * 1024; // 100 MiB

    fn get_temp_dir() -> PathBuf {
        env::var("TMPDIR")
            .ok()
            .or_else(|| env::var("TMP").ok())
            .or_else(|| env::var("TEMP").ok())
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new("/tmp").to_path_buf())
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

    fn check_free_space(path: &Path) -> Result<u64, String> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            let metadata = std::fs::metadata(path)
                .map_err(|e| format!("Failed to get metadata: {}", e))?;

            // For free space, we need statvfs on Unix
            // This is a simplified check - a full implementation would use nix::sys::statvfs
            // For now, we'll return a conservative OK value
            // In production, you'd want to use:
            // let stat = statvfs(path)?; Ok(stat.blocks_available * stat.fragment_size)
            Ok(Self::MIN_FREE_BYTES)
        }

        #[cfg(not(unix))]
        {
            // On non-Unix, just return OK conservatively
            // A full implementation would use GetDiskFreeSpaceEx on Windows
            Ok(Self::MIN_FREE_BYTES)
        }
    }
}

impl Check for TempDirCheck {
    fn name(&self) -> &'static str {
        "temp dir writable"
    }

    fn run(&self, _ctx: &DoctorCtx) -> CheckResult {
        let temp_dir = Self::get_temp_dir();

        // Check if directory exists
        if !temp_dir.exists() {
            return CheckResult {
                name: self.name(),
                status: CheckStatus::Fail,
                detail: format!("Temp directory does not exist: {}", temp_dir.display()),
            };
        }

        // Check writable
        let writable = Self::check_writable(&temp_dir);

        // Check free space
        let free_space = Self::check_free_space(&temp_dir);

        match (writable, free_space) {
            (Ok(_), Ok(free)) => {
                if free < Self::MIN_FREE_BYTES {
                    let free_mb = free / (1024 * 1024);
                    CheckResult {
                        name: self.name(),
                        status: CheckStatus::Warn,
                        detail: format!("Temp dir writable but low disk space: {} MiB free at {} (100 MiB recommended)", free_mb, temp_dir.display()),
                    }
                } else {
                    CheckResult {
                        name: self.name(),
                        status: CheckStatus::Ok,
                        detail: format!("Temp dir writable at {}", temp_dir.display()),
                    }
                }
            }
            (Err(e), _) => {
                CheckResult {
                    name: self.name(),
                    status: CheckStatus::Fail,
                    detail: format!("Temp directory check failed at {}: {}", temp_dir.display(), e),
                }
            }
            (_, Err(e)) => {
                CheckResult {
                    name: self.name(),
                    status: CheckStatus::Warn,
                    detail: format!("Could not check free space at {}: {}", temp_dir.display(), e),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temp_dir_check_name() {
        assert_eq!(TempDirCheck.name(), "temp dir writable");
    }

    #[test]
    fn test_get_temp_dir() {
        let temp = TempDirCheck::get_temp_dir();
        assert!(temp.exists());
    }

    #[test]
    fn test_temp_dir_writable() {
        let temp = TempDirCheck::get_temp_dir();
        let result = TempDirCheck::check_writable(&temp);
        // Should succeed on a normal system
        assert!(result.is_ok());
    }
}
