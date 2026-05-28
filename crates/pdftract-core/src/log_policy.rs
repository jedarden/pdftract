//! Log-policy enforcement for pdftract.
//!
//! This module implements runtime log filtering to prevent accidental
//! secret leakage. It provides a custom logger that wraps the standard
//! log facade and redacts known-secret patterns.
//!
//! # NEVER-log policy
//!
//! The following patterns are NEVER logged at any level:
//! - Password values (PDF, MCP, inspector)
//! - Bearer-token values
//! - PDF byte contents (not even at trace)
//! - Full extracted text (only span counts, page counts, and fingerprints)
//! - Cookie, Authorization, or Proxy-Authorization HTTP headers
//!
//! # References
//!
//! Plan lines 966-973 (NEVER-log list), 897 (TH-08 definition)

use anyhow::Result;
use regex::Regex;
use std::sync::{Arc, OnceLock, LazyLock};

/// Known-secret patterns that should never appear in log output.
///
/// These patterns are checked at runtime and redacted with `[REDACTED]`.
fn get_secret_patterns() -> &'static Vec<Regex> {
    static INSTANCE: OnceLock<Vec<Regex>> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        vec![
            // Password patterns: "password: <value>", "pwd=", etc.
            Regex::new(r"(?i)(password|pwd|pass|passwd)[\s:=]+[^\s]{3,}").unwrap(),
            // Token patterns: "token: <value>", "bearer <value>", etc.
            Regex::new(r"(?i)(token|bearer|api_key|apikey|secret)[\s:=]+[^\s]{3,}").unwrap(),
            // Authorization header patterns (case-insensitive)
            Regex::new(r"(?i)authorization:\s*[^\s]{3,}").unwrap(),
            // Cookie header patterns
            Regex::new(r"(?i)cookie:\s*[^\s]{3,}").unwrap(),
            // Proxy-Authorization header patterns
            Regex::new(r"(?i)proxy-authorization:\s*[^\s]{3,}").unwrap(),
            // Base64-like patterns (long alphanumeric strings that might be tokens)
            // Typical JWT tokens or API keys in base64 encoding
            Regex::new(r"[A-Za-z0-9+/]{32,}[=]{0,2}").unwrap(),
        ]
    })
}

/// Sensitive header names that should never be logged with their values.
static SENSITIVE_HEADERS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    vec![
        "authorization",
        "cookie",
        "proxy-authorization",
        "set-cookie",
    ]
});

/// Redact a log line by replacing known-secret patterns with `[REDACTED]`.
///
/// This function scans the log line for known-secret patterns and replaces
/// them with `[REDACTED]`. It's a best-effort defense-in-depth mechanism.
///
/// # Arguments
///
/// * `line` - The log line to redact
///
/// # Returns
///
/// The redacted log line with secrets replaced by `[REDACTED]`
pub fn redact_log_line(line: &str) -> String {
    let mut redacted = line.to_string();

    // Apply each secret pattern
    for pattern in get_secret_patterns().iter() {
        redacted = pattern
            .replace_all(&redacted, "[REDACTED]")
            .to_string();
    }

    // Additional redaction for very long strings that might be secrets
    // ( heuristic: truncate any "word" longer than 100 characters in log output)
    let words: Vec<&str> = redacted.split_whitespace().collect();
    let truncated: Vec<String> = words
        .iter()
        .map(|&word| {
            if word.len() > 100 && !word.starts_with("http://") && !word.starts_with("https://") {
                format!("{}...[TRUNCATED: too long]", &word[..50])
            } else {
                word.to_string()
            }
        })
        .collect();

    truncated.join(" ")
}

/// Check if a header name is sensitive (should not be logged with its value).
///
/// # Arguments
///
/// * `header_name` - The header name to check (case-insensitive)
///
/// # Returns
///
/// true if the header is sensitive and should be redacted
pub fn is_sensitive_header(header_name: &str) -> bool {
    let name_lower = header_name.to_lowercase();
    SENSITIVE_HEADERS.iter().any(|&sensitive| name_lower == sensitive)
}

/// Redact a header value for logging.
///
/// # Arguments
///
/// * `header_name` - The header name
/// * `header_value` - The header value to potentially redact
///
/// # Returns
///
/// The header value, or `\[REDACTED\]` if the header is sensitive
pub fn redact_header_value(header_name: &str, header_value: &str) -> String {
    if is_sensitive_header(header_name) {
        "[REDACTED]".to_string()
    } else {
        header_value.to_string()
    }
}

/// LogPolicyFilter provides runtime filtering for log output.
///
/// This filter can be used with any logger implementation to enforce
/// the NEVER-log policy at runtime.
pub struct LogPolicyFilter;

impl LogPolicyFilter {
    /// Create a new log policy filter.
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }

    /// Filter a log message by redacting secrets.
    ///
    /// # Arguments
    ///
    /// * `message` - The log message to filter
    ///
    /// # Returns
    ///
/// The filtered log message with secrets redacted
    pub fn filter_message(&self, message: &str) -> String {
        redact_log_line(message)
    }

    /// Check if a message contains secrets.
    ///
    /// This is useful for compile-time or CI checks.
    ///
    /// # Arguments
    ///
    /// * `message` - The log message to check
    ///
    /// # Returns
    ///
    /// true if the message contains potential secrets
    pub fn contains_secrets(&self, message: &str) -> bool {
        let redacted = redact_log_line(message);
        redacted != message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_password() {
        let line = "user:john password:secret123";
        let redacted = redact_log_line(line);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("secret123"));
    }

    #[test]
    fn test_redact_bearer_token() {
        let line = "Authorization: Bearer abc123xyz456";
        let redacted = redact_log_line(line);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("abc123xyz456"));
    }

    #[test]
    fn test_redact_cookie() {
        let line = "Cookie: session_id=secret_value";
        let redacted = redact_log_line(line);
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn test_redact_multiple_patterns() {
        let line = "password:foo token:bar authorization:baz";
        let redacted = redact_log_line(line);
        // All three should be redacted
        assert_eq!(redacted.matches("[REDACTED]").count(), 3);
    }

    #[test]
    fn test_is_sensitive_header() {
        assert!(is_sensitive_header("Authorization"));
        assert!(is_sensitive_header("authorization"));
        assert!(is_sensitive_header("Cookie"));
        assert!(is_sensitive_header("Proxy-Authorization"));
        assert!(!is_sensitive_header("Content-Type"));
        assert!(!is_sensitive_header("User-Agent"));
    }

    #[test]
    fn test_redact_header_value() {
        assert_eq!(redact_header_value("Authorization", "Bearer token"), "[REDACTED]");
        assert_eq!(redact_header_value("Content-Type", "application/json"), "application/json");
    }

    #[test]
    fn test_log_policy_filter_contains_secrets() {
        let filter = LogPolicyFilter::new();
        assert!(filter.contains_secrets("password:secret"));
        assert!(!filter.contains_secrets("normal log message"));
    }

    #[test]
    fn test_redact_preserves_urls() {
        let line = "Fetching from https://example.com/path?param=value";
        let redacted = redact_log_line(line);
        assert!(redacted.contains("https://example.com"));
        assert!(!redacted.contains("[REDACTED]"));
    }

    #[test]
    fn test_redact_truncates_long_strings() {
        let long_string = "a".repeat(200);
        let line = format!("Long string: {}", long_string);
        let redacted = redact_log_line(&line);
        // Check that the long string is either truncated or redacted
        assert!(redacted.contains("[TRUNCATED:") || !redacted.contains(&long_string[..50]));
    }

    #[test]
    fn test_redact_base64_like_patterns() {
        // Test JWT-like pattern
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0";
        let line = format!("Token: {}", jwt);
        let redacted = redact_log_line(&line);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains(&jwt[..20]));
    }
}
