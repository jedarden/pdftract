use std::process::Command;
use super::super::{Check, CheckResult, CheckStatus, DoctorCtx};

/// Check: tesseract installation and version
///
/// OK: tesseract --version succeeds, major >= 5
/// WARN: major == 4
/// FAIL: binary missing or major <= 3
pub struct TesseractCheck;

impl Check for TesseractCheck {
    fn name(&self) -> &'static str {
        "tesseract install"
    }

    fn run(&self, _ctx: &DoctorCtx) -> CheckResult {
        let output = Command::new("tesseract")
            .arg("--version")
            .output();

        let (status, detail) = match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let version_output = format!("{}{}", stdout, stderr);

                // Parse version like "tesseract 5.3.0"
                let version_line = version_output
                    .lines()
                    .find(|line| line.to_lowercase().contains("tesseract"));

                if let Some(line) = version_line {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Some(version_str) = parts.get(1) {
                            if let Ok(version) = version_str.parse::<semver::Version>() {
                                let major = version.major;
                                return match major {
                                    m if m >= 5 => CheckResult {
                                        name: self.name(),
                                        status: CheckStatus::Ok,
                                        detail: format!("tesseract {} found (major >= 5)", version),
                                    },
                                    4 => CheckResult {
                                        name: self.name(),
                                        status: CheckStatus::Warn,
                                        detail: format!("tesseract {} found (major == 4: some glyphs may OCR incorrectly)", version),
                                    },
                                    _ => CheckResult {
                                        name: self.name(),
                                        status: CheckStatus::Fail,
                                        detail: format!("tesseract {} found (major <= 3: OCR results are unusable)", version),
                                    },
                                };
                            }
                        }
                    }
                }

                // Failed to parse version but binary exists
                CheckResult {
                    name: self.name(),
                    status: CheckStatus::Warn,
                    detail: format!("tesseract binary found but version could not be parsed: {}", version_output.trim()),
                }
            }
            Err(e) => {
                CheckResult {
                    name: self.name(),
                    status: CheckStatus::Fail,
                    detail: format!("tesseract not found: {}", e),
                }
            }
        };

        CheckResult { status, ..result }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tesseract_check_name() {
        assert_eq!(TesseractCheck.name(), "tesseract install");
    }

    // Note: Full integration tests require actual tesseract installation
    // These are covered by the CI test suite
}
