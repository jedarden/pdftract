//! Profile loader with secret-key detection.
//!
//! This module provides functionality to load and validate YAML profiles,
//! with special security checks to prevent accidental publication of
//! credentials in profile files.

use serde_yaml::Value;
use std::fmt;
use std::io;
use std::path::Path;

/// Error type for profile loading failures.
#[derive(Debug)]
pub enum ProfileLoadError {
    /// YAML parsing error
    YamlError(serde_yaml::Error),

    /// IO error reading file
    IoError(io::Error),

    /// Forbidden secret key found in profile
    ForbiddenKey {
        /// The forbidden key that was found
        key: String,
        /// Path to the key in the YAML structure (dot-separated)
        path: String,
        /// Line number where the key appears (0 if unknown)
        line: usize,
    },
}

impl fmt::Display for ProfileLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProfileLoadError::YamlError(e) => write!(f, "YAML parse error: {}", e),
            ProfileLoadError::IoError(e) => write!(f, "Failed to read file: {}", e),
            ProfileLoadError::ForbiddenKey { key, path, line } => {
                write!(f, "forbidden key '{}' at {} (line {})", key, path, line)
            }
        }
    }
}

impl std::error::Error for ProfileLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProfileLoadError::YamlError(e) => Some(e),
            ProfileLoadError::IoError(e) => Some(e),
            ProfileLoadError::ForbiddenKey { .. } => None,
        }
    }
}

impl From<serde_yaml::Error> for ProfileLoadError {
    fn from(e: serde_yaml::Error) -> Self {
        ProfileLoadError::YamlError(e)
    }
}

impl From<io::Error> for ProfileLoadError {
    fn from(e: io::Error) -> Self {
        ProfileLoadError::IoError(e)
    }
}

/// Error returned when forbidden keys are detected in a profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForbiddenKeyError {
    /// The forbidden key that was found
    pub key: String,
    /// Path to the key in the YAML structure (dot-separated)
    pub path: String,
    /// Line number where the key appears
    pub line: usize,
}

impl fmt::Display for ForbiddenKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "forbidden key '{}' at {} (line {})",
            self.key, self.path, self.line
        )
    }
}

impl std::error::Error for ForbiddenKeyError {}

/// Forbidden keys in profile YAML (case-insensitive).
///
/// This list includes common key names that suggest credentials or secrets.
/// The check is separator-tolerant: api_key, apiKey, api-key, and api.key
/// are all treated as the same forbidden key.
const FORBIDDEN_KEYS: &[&str] = &[
    "password",
    "passwd",
    "token",
    "secret",
    "api_key",
    "apikey",
    "api-key",
    "private_key",
    "privatekey",
    "private-key",
    "auth_token",
    "authtoken",
    "auth-token",
    "bearer",
    "credential",
    "credentials",
    "key",
];

/// Normalize a key name for forbidden-key comparison.
///
/// This removes common separators (underscore, hyphen, dot) and lowercases
/// the result, so that api_key, apiKey, api-key, and api.key all normalize
/// to the same canonical form "apikey".
fn normalize_key(key: &str) -> String {
    key.chars()
        .filter(|c| !matches!(c, '_' | '-' | '.'))
        .collect::<String>()
        .to_lowercase()
}

/// Check if a key is in the forbidden list.
///
/// This uses separator-tolerant matching, so variations like api_key, apiKey,
/// api-key, and api.key are all recognized as the forbidden key "api_key".
fn is_forbidden_key(key: &str) -> bool {
    let normalized = normalize_key(key);
    FORBIDDEN_KEYS
        .iter()
        .any(|forbidden| normalize_key(forbidden) == normalized)
}

/// Find the line number of a key in YAML content.
///
/// This is a best-effort search that looks for the key string in the content.
/// It may not be perfectly accurate but provides useful context for error messages.
fn find_line_number(content: &str, key: &str, path_prefix: &str) -> usize {
    // Count the depth of nesting by counting dots in the path prefix
    let depth = path_prefix.matches('.').count();

    // Split the content into lines
    let lines: Vec<&str> = content.lines().collect();

    // Search for the key in the content
    // We look for the key preceded by whitespace and a colon
    let search_pattern = format!("{}:", key);

    for (idx, line) in lines.iter().enumerate() {
        // Check if this line contains the key
        if line.contains(&search_pattern) {
            // Estimate indentation level to match nesting depth
            let indent = line.len() - line.trim_start().len();
            let expected_indent = depth * 2; // Assume 2 spaces per level

            // If indentation roughly matches or this is the first match, use it
            if indent >= expected_indent.saturating_sub(1) && indent <= expected_indent + 2 {
                return idx + 1; // Line numbers are 1-indexed
            }
        }
    }

    // If we couldn't find a good match, return the first occurrence
    for (idx, line) in lines.iter().enumerate() {
        if line.contains(&search_pattern) {
            return idx + 1;
        }
    }

    0 // Unknown line
}

/// Check a YAML value for forbidden secret keys.
///
/// This function recursively walks the YAML structure and checks all dictionary
/// keys against the forbidden list. If any forbidden key is found, it returns
/// an error with the key name, path, and line number.
///
/// # Arguments
///
/// * `value` - The YAML value to check
/// * `current_path` - Current path in the YAML structure (for error reporting)
/// * `content` - The original YAML content (for line number detection)
///
/// # Returns
///
/// * `Ok(())` if no forbidden keys are found
/// * `Err(ForbiddenKeyError)` if a forbidden key is found
pub fn check_forbidden_keys(
    value: &Value,
    current_path: &str,
    content: &str,
) -> Result<(), ForbiddenKeyError> {
    match value {
        Value::Mapping(map) => {
            for (key, value) in map {
                if let Some(key_str) = key.as_str() {
                    let key_lower = normalize_key(key_str);

                    if is_forbidden_key(key_str) {
                        // Try to get line number from the YAML content
                        let line = find_line_number(content, key_str, current_path);

                        let new_path = if current_path.is_empty() {
                            key_str.to_string()
                        } else {
                            format!("{}.{}", current_path, key_str)
                        };

                        return Err(ForbiddenKeyError {
                            key: key_str.to_string(),
                            path: new_path,
                            line,
                        });
                    }

                    // Recurse into nested values
                    let new_path = if current_path.is_empty() {
                        key_str.to_string()
                    } else {
                        format!("{}.{}", current_path, key_str)
                    };

                    if let Err(e) = check_forbidden_keys(value, &new_path, content) {
                        return Err(e);
                    }
                }
            }
            Ok(())
        }
        Value::Sequence(seq) => {
            for (idx, item) in seq.iter().enumerate() {
                let new_path = format!("{}[{}]", current_path, idx);
                if let Err(e) = check_forbidden_keys(item, &new_path, content) {
                    return Err(e);
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Load and validate a profile from a YAML string.
///
/// This function parses the YAML content and checks for forbidden keys.
/// If any forbidden key is found, it returns a ProfileLoadError::ForbiddenKey.
///
/// # Arguments
///
/// * `content` - The YAML content to parse
///
/// # Returns
///
/// * `Ok(Value)` - The parsed YAML value
/// * `Err(ProfileLoadError)` - If parsing fails or forbidden keys are found
pub fn load_profile_yaml(content: &str) -> Result<Value, ProfileLoadError> {
    let value: Value = serde_yaml::from_str(content)?;

    // Check for forbidden keys
    if let Err(e) = check_forbidden_keys(&value, "", content) {
        return Err(ProfileLoadError::ForbiddenKey {
            key: e.key,
            path: e.path,
            line: e.line,
        });
    }

    Ok(value)
}

/// Load and validate a profile from a file.
///
/// This function reads the file, parses the YAML content, and checks for
/// forbidden keys. If any forbidden key is found, it returns a
/// ProfileLoadError::ForbiddenKey with the file path included in the context.
///
/// # Arguments
///
/// * `path` - Path to the YAML file to load
///
/// # Returns
///
/// * `Ok(Value)` - The parsed YAML value
/// * `Err(ProfileLoadError)` - If reading, parsing, or validation fails
pub fn load_profile_file(path: &Path) -> Result<Value, ProfileLoadError> {
    let content = std::fs::read_to_string(path)?;
    load_profile_yaml(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_key() {
        assert_eq!(normalize_key("api_key"), "apikey");
        assert_eq!(normalize_key("apiKey"), "apikey");
        assert_eq!(normalize_key("api-key"), "apikey");
        assert_eq!(normalize_key("api.key"), "apikey");
        assert_eq!(normalize_key("API_KEY"), "apikey");
        assert_eq!(normalize_key("Api_Key"), "apikey");
    }

    #[test]
    fn test_is_forbidden_key() {
        // Exact matches
        assert!(is_forbidden_key("password"));
        assert!(is_forbidden_key("token"));
        assert!(is_forbidden_key("secret"));
        assert!(is_forbidden_key("api_key"));

        // Separator variants
        assert!(is_forbidden_key("api-key"));
        assert!(is_forbidden_key("apikey"));
        assert!(is_forbidden_key("apiKey"));
        assert!(is_forbidden_key("auth_token"));
        assert!(is_forbidden_key("auth-token"));
        assert!(is_forbidden_key("authtoken"));

        // Case insensitive
        assert!(is_forbidden_key("PASSWORD"));
        assert!(is_forbidden_key("Api_Key"));
        assert!(is_forbidden_key("Secret"));

        // Safe keys should not match
        assert!(!is_forbidden_key("name"));
        assert!(!is_forbidden_key("threshold"));
        assert!(!is_forbidden_key("vendor_api")); // substring "api" is ok
        assert!(!is_forbidden_key("documentation")); // substring "key" is not a match
    }

    #[test]
    fn test_check_forbidden_keys_detects_password() {
        let yaml = r#"
        password: "secret123"
        "#;

        let value: Value = serde_yaml::from_str(yaml).unwrap();
        let result = check_forbidden_keys(&value, "", yaml);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.key.contains("password"));
    }

    #[test]
    fn test_check_forbidden_keys_case_insensitive() {
        let yaml = r#"
        Password: "secret123"
        PASSWORD: "secret456"
        "#;

        let value: Value = serde_yaml::from_str(yaml).unwrap();
        let result = check_forbidden_keys(&value, "", yaml);

        assert!(result.is_err());
    }

    #[test]
    fn test_check_forbidden_keys_separator_variants() {
        let yaml = r#"
        api_key: "[REDACTED]"
        apiKey: "[REDACTED]"
        api-key: "sk-5555555555"
        "#;

        let value: Value = serde_yaml::from_str(yaml).unwrap();
        let result = check_forbidden_keys(&value, "", yaml);

        assert!(result.is_err());
    }

    #[test]
    fn test_check_forbidden_keys_nested() {
        let yaml = r#"
        name: "test"
        extraction:
          fields:
            api_key: "forbidden"
        "#;

        let value: Value = serde_yaml::from_str(yaml).unwrap();
        let result = check_forbidden_keys(&value, "", yaml);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.path, "extraction.fields.api_key");
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

        let value: Value = serde_yaml::from_str(yaml).unwrap();
        let result = check_forbidden_keys(&value, "", yaml);

        assert!(result.is_ok());
    }

    #[test]
    fn test_check_forbidden_keys_sequence() {
        let yaml = r#"
        rules:
          - name: "rule1"
          - name: "rule2"
            fields:
              api_key: "forbidden"
        "#;

        let value: Value = serde_yaml::from_str(yaml).unwrap();
        let result = check_forbidden_keys(&value, "", yaml);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.path.contains("rules"));
        assert!(err.path.contains("api_key"));
    }

    #[test]
    fn test_load_profile_yaml_valid() {
        let yaml = r#"
        name: "test_profile"
        threshold: 0.9
        "#;

        let result = load_profile_yaml(yaml);
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_profile_yaml_forbidden() {
        let yaml = r#"
        name: "test_profile"
        api_key: "[REDACTED]"
        "#;

        let result = load_profile_yaml(yaml);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProfileLoadError::ForbiddenKey { key, .. } => {
                assert_eq!(key, "api_key");
            }
            _ => panic!("Expected ForbiddenKey error"),
        }
    }

    #[test]
    fn test_load_profile_yaml_malformed() {
        let yaml = r#"
        name: "unclosed string
        "#;

        let result = load_profile_yaml(yaml);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ProfileLoadError::YamlError(_)
        ));
    }

    #[test]
    fn test_find_line_number() {
        let yaml = r#"
name: "test"
password: "secret"
api_key: "key"
"#;
        // First occurrence of password should be around line 3
        let line = find_line_number(yaml, "password", "");
        assert!(line >= 2 && line <= 4, "Expected line 2-4, got {}", line);
    }
}
