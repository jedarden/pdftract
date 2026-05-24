use super::super::{Check, CheckResult, CheckStatus, DoctorCtx};
use std::process::Command;

/// Check: libtiff installation (CCITT fax decoding)
///
/// OK: found via pkg-config
/// FAIL: not found
pub struct LibtiffCheck;

impl Check for LibtiffCheck {
    fn name(&self) -> &'static str {
        "libtiff"
    }

    fn run(&self, _ctx: &DoctorCtx) -> CheckResult {
        // First check if pkg-config exists
        let pkg_check = Command::new("pkg-config").arg("--version").output();

        let pkg_available = pkg_check.is_ok();

        if !pkg_available {
            // Fallback: try ldconfig -p | grep tiff
            let ldconfig = Command::new("ldconfig").arg("-p").output();

            if let Ok(output) = ldconfig {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("libtiff") || stdout.contains("tiff") {
                    return CheckResult {
                        name: self.name(),
                        status: CheckStatus::Ok,
                        detail: "libtiff found via ldconfig (pkg-config unavailable)".to_string(),
                    };
                }
            }

            return CheckResult {
                name: self.name(),
                status: CheckStatus::Fail,
                detail: "pkg-config not found and libtiff not detected via ldconfig".to_string(),
            };
        }

        // Use pkg-config --exists
        let output = Command::new("pkg-config")
            .args(["--exists", "libtiff-4"])
            .status();

        match output {
            Ok(status) if status.success() => {
                // Get version for detail
                let version = Command::new("pkg-config")
                    .args(["--modversion", "libtiff-4"])
                    .output();

                let detail = if let Ok(v_out) = version {
                    let v_str = String::from_utf8_lossy(&v_out.stdout).trim().to_string();
                    format!("libtiff {} found", v_str)
                } else {
                    "libtiff found".to_string()
                };

                CheckResult {
                    name: self.name(),
                    status: CheckStatus::Ok,
                    detail,
                }
            }
            Ok(_) => CheckResult {
                name: self.name(),
                status: CheckStatus::Fail,
                detail: "libtiff not found (pkg-config --exists libtiff-4 failed)".to_string(),
            },
            Err(e) => CheckResult {
                name: self.name(),
                status: CheckStatus::Fail,
                detail: format!("pkg-config check failed: {}", e),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_libtiff_check_name() {
        assert_eq!(LibtiffCheck.name(), "libtiff");
    }
}
