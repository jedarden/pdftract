use std::process::Command;
use super::super::{Check, CheckResult, CheckStatus, DoctorCtx};

/// Check: libopenjp2 installation (JPEG2000 decoding)
///
/// OK: found via pkg-config
/// FAIL: not found
pub struct Libopenjp2Check;

impl Check for Libopenjp2Check {
    fn name(&self) -> &'static str {
        "libopenjp2"
    }

    fn run(&self, _ctx: &DoctorCtx) -> CheckResult {
        // First check if pkg-config exists
        let pkg_check = Command::new("pkg-config")
            .arg("--version")
            .output();

        let pkg_available = pkg_check.is_ok();

        if !pkg_available {
            // Fallback: try ldconfig -p | grep openjp2
            let ldconfig = Command::new("ldconfig")
                .arg("-p")
                .output();

            if let Ok(output) = ldconfig {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("openjp2") {
                    return CheckResult {
                        name: self.name(),
                        status: CheckStatus::Ok,
                        detail: "libopenjp2 found via ldconfig (pkg-config unavailable)".to_string(),
                    };
                }
            }

            return CheckResult {
                name: self.name(),
                status: CheckStatus::Fail,
                detail: "pkg-config not found and libopenjp2 not detected via ldconfig".to_string(),
            };
        }

        // Use pkg-config --exists
        let output = Command::new("pkg-config")
            .args(["--exists", "libopenjp2"])
            .status();

        match output {
            Ok(status) if status.success() => {
                // Get version for detail
                let version = Command::new("pkg-config")
                    .args(["--modversion", "libopenjp2"])
                    .output();

                let detail = if let Ok(v_out) = version {
                    let v_str = String::from_utf8_lossy(&v_out.stdout).trim().to_string();
                    format!("libopenjp2 {} found", v_str)
                } else {
                    "libopenjp2 found".to_string()
                };

                CheckResult {
                    name: self.name(),
                    status: CheckStatus::Ok,
                    detail,
                }
            }
            Ok(_) => {
                CheckResult {
                    name: self.name(),
                    status: CheckStatus::Fail,
                    detail: "libopenjp2 not found (pkg-config --exists libopenjp2 failed)".to_string(),
                }
            }
            Err(e) => {
                CheckResult {
                    name: self.name(),
                    status: CheckStatus::Fail,
                    detail: format!("pkg-config check failed: {}", e),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_libopenjp2_check_name() {
        assert_eq!(Libopenjp2Check.name(), "libopenjp2");
    }
}
