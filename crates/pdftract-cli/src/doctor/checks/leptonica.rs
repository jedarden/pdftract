use std::process::Command;
use super::super::{Check, CheckResult, CheckStatus, DoctorCtx};

/// Check: leptonica installation (transitive Tesseract dependency)
///
/// OK: pkg-config finds lept >= 1.79
/// WARN: older version found
/// FAIL: not found
pub struct LeptonicaCheck;

impl Check for LeptonicaCheck {
    fn name(&self) -> &'static str {
        "leptonica install"
    }

    fn run(&self, _ctx: &DoctorCtx) -> CheckResult {
        // First check if pkg-config exists
        let pkg_check = Command::new("pkg-config")
            .arg("--version")
            .output();

        let pkg_available = pkg_check.is_ok();

        if !pkg_available {
            // Fallback: try ldconfig -p | grep lept
            let ldconfig = Command::new("ldconfig")
                .arg("-p")
                .output();

            if let Ok(output) = ldconfig {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("lept") {
                    return CheckResult {
                        name: self.name(),
                        status: CheckStatus::Warn,
                        detail: "leptonica found via ldconfig but pkg-config unavailable (cannot check version)".to_string(),
                    };
                }
            }

            return CheckResult {
                name: self.name(),
                status: CheckStatus::Fail,
                detail: "pkg-config not found and leptonica not detected via ldconfig".to_string(),
            };
        }

        // Use pkg-config to check version
        let output = Command::new("pkg-config")
            .args(["--modversion", "lept"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();

                // Parse semver
                if let Ok(version) = semver::Version::parse(&version_str) {
                    let target = semver::Version::new(1, 79, 0);

                    if version >= target {
                        CheckResult {
                            name: self.name(),
                            status: CheckStatus::Ok,
                            detail: format!("leptonica {} found (>= 1.79)", version),
                        }
                    } else {
                        CheckResult {
                            name: self.name(),
                            status: CheckStatus::Warn,
                            detail: format!("leptonica {} found (< 1.79: may have compatibility issues)", version),
                        }
                    }
                } else {
                    CheckResult {
                        name: self.name(),
                        status: CheckStatus::Warn,
                        detail: format!("leptonica {} found but version could not be parsed", version_str),
                    }
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                CheckResult {
                    name: self.name(),
                    status: CheckStatus::Fail,
                    detail: format!("leptonica not found: {}", stderr.trim()),
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
    fn test_leptonica_check_name() {
        assert_eq!(LeptonicaCheck.name(), "leptonica install");
    }
}
