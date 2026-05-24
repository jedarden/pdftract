use super::super::{Check, CheckResult, CheckStatus, DoctorCtx};
use std::env;
use std::path::{Path, PathBuf};

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

        std::fs::write(&test_file, b"test").map_err(|e| format!("Not writable: {}", e))?;

        // Clean up
        let _ = std::fs::remove_file(&test_file);

        Ok(())
    }

    #[cfg(unix)]
    fn check_free_space(path: &Path) -> Result<u64, String> {
        use libc::{c_char, statvfs};
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let path_cstr = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| "Failed to convert path to CString".to_string())?;

        unsafe {
            let mut stat: libc::statvfs = std::mem::zeroed();
            if statvfs(path_cstr.as_ptr() as *const c_char, &mut stat) != 0 {
                return Err("Failed to stat filesystem".to_string());
            }

            // f_frsize is the fundamental file system block size
            // f_bavail is the number of free blocks available to a non-privileged process
            let block_size = stat.f_frsize as u64;
            let available_blocks = stat.f_bavail as u64;
            Ok(block_size * available_blocks)
        }
    }

    #[cfg(windows)]
    fn check_free_space(path: &Path) -> Result<u64, String> {
        use std::os::windows::fs::GetDiskFreeSpaceEx;

        let available = GetDiskFreeSpaceEx::new(path)
            .map_err(|e| format!("Failed to get disk free space: {}", e))?
            .available_bytes();
        Ok(available)
    }

    #[cfg(not(any(unix, windows)))]
    fn check_free_space(_path: &Path) -> Result<u64, String> {
        // On other platforms, conservatively return OK
        Ok(Self::MIN_FREE_BYTES)
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
            (Err(e), _) => CheckResult {
                name: self.name(),
                status: CheckStatus::Fail,
                detail: format!(
                    "Temp directory check failed at {}: {}",
                    temp_dir.display(),
                    e
                ),
            },
            (_, Err(e)) => CheckResult {
                name: self.name(),
                status: CheckStatus::Warn,
                detail: format!(
                    "Could not check free space at {}: {}",
                    temp_dir.display(),
                    e
                ),
            },
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
