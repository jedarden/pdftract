use super::super::{Check, CheckResult, CheckStatus, DoctorCtx};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Check: profile search path (profiles feature)
///
/// OK: every YAML parses; no PROFILE_SECRETS_FORBIDDEN
/// WARN: dir empty
/// FAIL: parse errors or secret-keys present
pub struct ProfilePathCheck;

impl ProfilePathCheck {
    fn check_profile_file(path: &Path) -> Result<(), String> {
        let content = fs::read_to_string(path).map_err(|e| format!("Failed to read: {}", e))?;

        // Parse as YAML
        let value: serde_yaml::Value =
            serde_yaml::from_str(&content).map_err(|e| format!("YAML parse error: {}", e))?;

        // Check for forbidden keys using the enhanced detection
        #[cfg(feature = "profiles")]
        {
            if let Err(e) = pdftract_core::profiles::check_forbidden_keys(&value, "", &content) {
                return Err(format!(
                    "PROFILE_SECRETS_FORBIDDEN: {} at {} (line {})",
                    e.key, e.path, e.line
                ));
            }
        }

        // Fallback check for when profiles feature is disabled (legacy behavior)
        #[cfg(not(feature = "profiles"))]
        {
            if let Err(e) = Self::check_forbidden_keys_legacy(&value, path) {
                return Err(e);
            }
        }

        Ok(())
    }

    /// Legacy forbidden key check (used when profiles feature is disabled)
    ///
    /// This is the original implementation with a limited set of forbidden keys.
    fn check_forbidden_keys_legacy(value: &serde_yaml::Value, path: &Path) -> Result<(), String> {
        const FORBIDDEN_KEYS: &[&str] = &[
            "password",
            "token",
            "secret",
            "api_key",
            "apikey",
            "private_key",
            "privatekey",
        ];

        fn check(value: &serde_yaml::Value) -> Result<(), String> {
            match value {
                serde_yaml::Value::Mapping(map) => {
                    for (key, _value) in map {
                        if let Some(key_str) = key.as_str() {
                            let key_lower = key_str.to_lowercase();

                            if FORBIDDEN_KEYS.contains(&key_lower.as_str()) {
                                return Err(format!(
                                    "PROFILE_SECRETS_FORBIDDEN: found forbidden key '{}'",
                                    key_str
                                ));
                            }
                        }

                        // Recurse into nested values
                        check(_value)?;
                    }
                }
                serde_yaml::Value::Sequence(seq) => {
                    for item in seq {
                        check(item)?;
                    }
                }
                _ => {}
            }

            Ok(())
        }

        check(value)
    }
}

impl Check for ProfilePathCheck {
    fn name(&self) -> &'static str {
        "profile search path"
    }

    fn run(&self, ctx: &DoctorCtx) -> CheckResult {
        let profile_dir = if let Some(ref dir) = ctx.profile_dir {
            dir.clone()
        } else {
            // Default profile directory
            dirs::config_dir()
                .map(|c| c.join("pdftract").join("profiles"))
                .unwrap_or_else(|| Path::new("profiles").to_path_buf())
        };

        // Check if directory exists
        if !profile_dir.exists() {
            return CheckResult {
                name: self.name(),
                status: CheckStatus::Warn,
                detail: format!(
                    "Profile directory does not exist: {}",
                    profile_dir.display()
                ),
            };
        }

        // Check if directory is empty
        let entries: Vec<std::fs::DirEntry> = fs::read_dir(&profile_dir)
            .map(|it| it.filter_map(|e| e.ok()).collect())
            .unwrap_or_default();

        if entries.is_empty() {
            return CheckResult {
                name: self.name(),
                status: CheckStatus::Warn,
                detail: format!("Profile directory is empty: {}", profile_dir.display()),
            };
        }

        // Check each .yaml file
        let mut yaml_count = 0;
        let mut errors = vec![];

        for entry in &entries {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("yaml")
                || path.extension().and_then(|s| s.to_str()) == Some("yml")
            {
                yaml_count += 1;

                if let Err(e) = Self::check_profile_file(&path) {
                    errors.push(format!("{}: {}", path.display(), e));
                }
            }
        }

        if !errors.is_empty() {
            return CheckResult {
                name: self.name(),
                status: CheckStatus::Fail,
                detail: format!(
                    "Found {} profile(s) with errors:\n  {}",
                    errors.len(),
                    errors.join("\n  ")
                ),
            };
        }

        if yaml_count == 0 {
            CheckResult {
                name: self.name(),
                status: CheckStatus::Warn,
                detail: format!("No YAML profiles found in: {}", profile_dir.display()),
            }
        } else {
            CheckResult {
                name: self.name(),
                status: CheckStatus::Ok,
                detail: format!(
                    "All {} profile(s) valid at {}",
                    yaml_count,
                    profile_dir.display()
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_profile_check_name() {
        assert_eq!(ProfilePathCheck.name(), "profile search path");
    }

    #[test]
    fn test_check_forbidden_keys_detects_password() {
        let yaml = r#"
        password: "secret123"
        "#;

        let value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        let path = Path::new("test.yaml");

        #[cfg(feature = "profiles")]
        {
            let result = pdftract_core::profiles::check_forbidden_keys(&value, "", yaml);
            assert!(result.is_err());
            assert!(result.unwrap_err().key.contains("password"));
        }

        #[cfg(not(feature = "profiles"))]
        {
            let result = ProfilePathCheck::check_forbidden_keys_legacy(&value, path);
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("PROFILE_SECRETS_FORBIDDEN"));
        }
    }

    #[test]
    fn test_check_forbidden_keys_case_insensitive() {
        let yaml = r#"
        Password: "secret123"
        PASSWORD: "secret456"
        "#;

        let value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        let path = Path::new("test.yaml");

        #[cfg(feature = "profiles")]
        {
            let result = pdftract_core::profiles::check_forbidden_keys(&value, "", yaml);
            assert!(result.is_err());
        }

        #[cfg(not(feature = "profiles"))]
        {
            let result = ProfilePathCheck::check_forbidden_keys_legacy(&value, path);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_check_forbidden_keys_separator_variants() {
        let yaml = r#"
        api_key: "[REDACTED]"
        apiKey: "[REDACTED]"
        api-key: "sk-5555555555"
        "#;

        let value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();

        #[cfg(feature = "profiles")]
        {
            let result = pdftract_core::profiles::check_forbidden_keys(&value, "", yaml);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err.path.contains("api"));
        }

        #[cfg(not(feature = "profiles"))]
        {
            let path = Path::new("test.yaml");
            let result = ProfilePathCheck::check_forbidden_keys_legacy(&value, path);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_check_forbidden_keys_allows_safe_keys() {
        let yaml = r#"
        name: "test"
        threshold: 0.85
        rules:
          - name: "rule1"
        vendor_api: "https://api.example.com"
        "#;

        let value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        let path = Path::new("test.yaml");

        #[cfg(feature = "profiles")]
        {
            let result = pdftract_core::profiles::check_forbidden_keys(&value, "", yaml);
            assert!(result.is_ok());
        }

        #[cfg(not(feature = "profiles"))]
        {
            let result = ProfilePathCheck::check_forbidden_keys_legacy(&value, path);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_profile_check_valid_directory() {
        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("valid.yaml");

        fs::write(
            &profile_path,
            r#"
        name: "test_profile"
        threshold: 0.9
        "#,
        )
        .unwrap();

        let ctx = DoctorCtx {
            requested_langs: vec![],
            cache_dir: None,
            profile_dir: Some(temp_dir.path().to_path_buf()),
            features: Default::default(),
        };

        let result = ProfilePathCheck.run(&ctx);
        assert!(matches!(result.status, CheckStatus::Ok));
    }

    #[test]
    fn test_profile_check_detects_secrets() {
        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("invalid.yaml");

        fs::write(
            &profile_path,
            r#"
        name: "test_profile"
        api_key: "[REDACTED]"
        "#,
        )
        .unwrap();

        let ctx = DoctorCtx {
            requested_langs: vec![],
            cache_dir: None,
            profile_dir: Some(temp_dir.path().to_path_buf()),
            features: Default::default(),
        };

        let result = ProfilePathCheck.run(&ctx);
        assert!(matches!(result.status, CheckStatus::Fail));
        assert!(result.detail.contains("PROFILE_SECRETS_FORBIDDEN"));
    }

    #[test]
    fn test_profile_check_detects_auth_token() {
        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("invalid.yaml");

        fs::write(
            &profile_path,
            r#"
        name: "test_profile"
        auth_token: "Bearer xyz"
        "#,
        )
        .unwrap();

        let ctx = DoctorCtx {
            requested_langs: vec![],
            cache_dir: None,
            profile_dir: Some(temp_dir.path().to_path_buf()),
            features: Default::default(),
        };

        let result = ProfilePathCheck.run(&ctx);

        #[cfg(feature = "profiles")]
        assert!(matches!(result.status, CheckStatus::Fail));

        #[cfg(not(feature = "profiles"))]
        assert!(matches!(result.status, CheckStatus::Ok)); // Legacy check doesn't catch auth_token
    }

    #[test]
    fn test_profile_check_detects_nested_secrets() {
        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("invalid.yaml");

        fs::write(
            &profile_path,
            r#"
        name: "test_profile"
        extraction:
          fields:
            credentials: "user:pass"
        "#,
        )
        .unwrap();

        let ctx = DoctorCtx {
            requested_langs: vec![],
            cache_dir: None,
            profile_dir: Some(temp_dir.path().to_path_buf()),
            features: Default::default(),
        };

        let result = ProfilePathCheck.run(&ctx);

        #[cfg(feature = "profiles")]
        assert!(matches!(result.status, CheckStatus::Fail));

        #[cfg(not(feature = "profiles"))]
        assert!(matches!(result.status, CheckStatus::Ok)); // Legacy check doesn't catch credentials
    }
}
