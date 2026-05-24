use super::super::{Check, CheckResult, CheckStatus, DoctorCtx};

/// Check: ulimit -n (file descriptor limit)
///
/// OK: >= 1024
/// WARN: 512 <= n < 1024
/// FAIL: < 512
///
/// Platform: Linux and macOS only
pub struct UlimitCheck;

impl UlimitCheck {
    #[cfg(unix)]
    fn get_rlimit_nofile() -> Result<u64, String> {
        use libc::{getrlimit, rlimit, RLIMIT_NOFILE};

        unsafe {
            let mut limits = rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };

            if getrlimit(RLIMIT_NOFILE, &mut limits) == 0 {
                Ok(limits.rlim_cur as u64)
            } else {
                Err("getrlimit failed".to_string())
            }
        }
    }
}

impl Check for UlimitCheck {
    fn name(&self) -> &'static str {
        "ulimit -n"
    }

    fn run(&self, _ctx: &DoctorCtx) -> CheckResult {
        #[cfg(unix)]
        {
            match Self::get_rlimit_nofile() {
                Ok(limit) => {
                    if limit >= 1024 {
                        CheckResult {
                            name: self.name(),
                            status: CheckStatus::Ok,
                            detail: format!("File descriptor limit: {}", limit),
                        }
                    } else if limit >= 512 {
                        CheckResult {
                            name: self.name(),
                            status: CheckStatus::Warn,
                            detail: format!(
                                "File descriptor limit: {} (recommended: >= 1024)",
                                limit
                            ),
                        }
                    } else {
                        CheckResult {
                            name: self.name(),
                            status: CheckStatus::Fail,
                            detail: format!("File descriptor limit: {} (too low, may cause issues with many files)", limit),
                        }
                    }
                }
                Err(e) => CheckResult {
                    name: self.name(),
                    status: CheckStatus::Warn,
                    detail: format!("Could not read ulimit: {}", e),
                },
            }
        }

        #[cfg(not(unix))]
        {
            CheckResult {
                name: self.name(),
                status: CheckStatus::NotApplicable,
                detail: "ulimit not applicable on this platform".to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ulimit_check_name() {
        assert_eq!(UlimitCheck.name(), "ulimit -n");
    }

    #[cfg(unix)]
    #[test]
    fn test_get_rlimit_nofile() {
        let limit = UlimitCheck::get_rlimit_nofile();
        // Should return some value on a real Unix system
        // In tests, we just verify it doesn't panic
    }
}
