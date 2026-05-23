use std::process::Command;
use super::super::{Check, CheckResult, CheckStatus, DoctorCtx};

/// Check: tesseract language availability
///
/// OK: all required languages (eng + any --lang) present
/// WARN: optional languages missing
/// FAIL: eng missing
pub struct TesseractLangsCheck;

impl Check for TesseractLangsCheck {
    fn name(&self) -> &'static str {
        "tesseract languages"
    }

    fn run(&self, ctx: &DoctorCtx) -> CheckResult {
        let output = Command::new("tesseract")
            .arg("--list-langs")
            .output();

        match output {
            Ok(output) => {
                if !output.status.success() {
                    return CheckResult {
                        name: self.name(),
                        status: CheckStatus::Fail,
                        detail: format!("tesseract --list-langs failed: {}", String::from_utf8_lossy(&output.stderr)),
                    };
                }

                let stdout = String::from_utf8_lossy(&output.stdout);
                let installed_langs: Vec<&str> = stdout
                    .lines()
                    .skip(1) // Skip header line
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty())
                    .collect();

                // eng is always required
                let required_langs: Vec<&str> = vec!["eng"]
                    .into_iter()
                    .chain(ctx.requested_langs.iter().map(|s| s.as_str()))
                    .collect();

                let missing_required: Vec<&str> = required_langs
                    .iter()
                    .filter(|lang| !installed_langs.contains(lang))
                    .copied()
                    .collect();

                if missing_required.contains(&"eng") {
                    return CheckResult {
                        name: self.name(),
                        status: CheckStatus::Fail,
                        detail: format!("Required language 'eng' not found. Installed: {:?}", installed_langs),
                    };
                }

                if !missing_required.is_empty() {
                    return CheckResult {
                        name: self.name(),
                        status: CheckStatus::Warn,
                        detail: format!("Requested languages not found: {:?}. Installed: {:?}", missing_required, installed_langs),
                    };
                }

                CheckResult {
                    name: self.name(),
                    status: CheckStatus::Ok,
                    detail: format!("All required languages present: {:?}", installed_langs),
                }
            }
            Err(e) => {
                CheckResult {
                    name: self.name(),
                    status: CheckStatus::Fail,
                    detail: format!("tesseract --list-langs failed: {}", e),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tesseract_langs_check_name() {
        assert_eq!(TesseractLangsCheck.name(), "tesseract languages");
    }
}
