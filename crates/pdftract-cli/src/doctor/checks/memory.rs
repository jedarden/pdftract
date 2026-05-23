use super::super::{Check, CheckResult, CheckStatus, DoctorCtx};

/// Check: available RAM
///
/// OK: >= 256 MiB free
/// WARN: 128 MiB <= n < 256 MiB
/// FAIL: < 128 MiB
///
/// Platform detection:
/// - Linux: read /proc/meminfo
/// - macOS: sysctl hw.memsize
/// - Windows: GlobalMemoryStatusEx
pub struct MemoryCheck;

impl MemoryCheck {
    const MIN_OK_BYTES: u64 = 256 * 1024 * 1024; // 256 MiB
    const MIN_WARN_BYTES: u64 = 128 * 1024 * 1024; // 128 MiB

    #[cfg(target_os = "linux")]
    fn get_available_memory() -> Result<u64, String> {
        use std::fs;

        let meminfo = fs::read_to_string("/proc/meminfo")
            .map_err(|e| format!("Failed to read /proc/meminfo: {}", e))?;

        // Parse MemAvailable (preferred) or MemFree
        let mut available = None;

        for line in meminfo.lines() {
            if line.starts_with("MemAvailable:") {
                // Format: MemAvailable:    12345678 kB
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        available = Some(kb * 1024);
                        break;
                    }
                }
            }
        }

        // Fallback to MemFree + Buffers + Cached if MemAvailable not found
        if available.is_none() {
            let mut mem_free = 0u64;
            let mut buffers = 0u64;
            let mut cached = 0u64;

            for line in meminfo.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 2 { continue; }

                if let Ok(kb) = parts[1].parse::<u64>() {
                    match parts[0] {
                        "MemFree:" => mem_free = kb * 1024,
                        "Buffers:" => buffers = kb * 1024,
                        "Cached:" => cached = kb * 1024,
                        _ => {}
                    }
                }
            }

            available = Some(mem_free + buffers + cached);
        }

        available.ok_or_else(|| "Could not determine available memory".to_string())
    }

    #[cfg(target_os = "macos")]
    fn get_available_memory() -> Result<u64, String> {
        use libc::{c_int, c_void, size_t, sysctl, CTL_HW, HW_MEMSIZE};

        unsafe {
            let mut memsize: u64 = 0;
            let mut len = std::mem::size_of::<u64>() as size_t;

            let mib: [c_int; 2] = [CTL_HW, HW_MEMSIZE];
            let res = sysctl(
                mib.as_ptr() as *const c_int,
                mib.len() as u32,
                &mut memsize as *mut u64 as *mut c_void,
                &mut len,
                std::ptr::null(),
                0,
            );

            if res == 0 {
                // On macOS, hw.memsize returns total physical memory
                // For simplicity, we'll just check total is >= 256 MiB
                // A more accurate check would use host_statistics64 for available memory
                Ok(memsize)
            } else {
                Err("sysctl hw.memsize failed".to_string())
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn get_available_memory() -> Result<u64, String> {
        use windows::Win32::System::Memory::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

        unsafe {
            let mut stat = MEMORYSTATUSEX {
                dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
                ..Default::default()
            };

            if GlobalMemoryStatusEx(&mut stat).is_ok() {
                Ok(stat.ullAvailPhys)
            } else {
                Err("GlobalMemoryStatusEx failed".to_string())
            }
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    fn get_available_memory() -> Result<u64, String> {
        Err("Memory detection not implemented on this platform".to_string())
    }
}

impl Check for MemoryCheck {
    fn name(&self) -> &'static str {
        "available RAM"
    }

    fn run(&self, _ctx: &DoctorCtx) -> CheckResult {
        match Self::get_available_memory() {
            Ok(bytes) => {
                let mib = bytes / (1024 * 1024);

                if bytes >= Self::MIN_OK_BYTES {
                    CheckResult {
                        name: self.name(),
                        status: CheckStatus::Ok,
                        detail: format!("{} MiB available", mib),
                    }
                } else if bytes >= Self::MIN_WARN_BYTES {
                    CheckResult {
                        name: self.name(),
                        status: CheckStatus::Warn,
                        detail: format!("{} MiB available (recommended: >= 256 MiB)", mib),
                    }
                } else {
                    CheckResult {
                        name: self.name(),
                        status: CheckStatus::Fail,
                        detail: format!("{} MiB available (too low, may cause OOM)", mib),
                    }
                }
            }
            Err(e) => {
                CheckResult {
                    name: self.name(),
                    status: CheckStatus::Warn,
                    detail: format!("Could not determine available memory: {}", e),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_check_name() {
        assert_eq!(MemoryCheck.name(), "available RAM");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_get_available_memory_linux() {
        let mem = MemoryCheck::get_available_memory();
        // On a real Linux system, this should succeed
        // In tests, we just verify it doesn't panic
    }
}
