//! HTTP header parsing and validation for the --header CLI flag.
//!
//! This module provides functionality for parsing and validating custom HTTP headers
//! passed via the --header flag. Headers are used when fetching remote PDFs via
//! HttpRangeSource (Phase 1.8).
//!
//! # Header Format
//!
//! Headers are specified as `HEADER:VALUE` where:
//! - `HEADER` is the header name (case-insensitive per HTTP spec)
//! - `VALUE` is the header value
//! - The colon is the delimiter between name and value
//! - Whitespace around the colon is trimmed
//!
//! # Validation Rules
//!
//! 1. Header name must match `[A-Za-z0-9_-]+` (HTTP token format)
//! 2. Header value must not contain CRLF sequences (HTTP injection protection)
//! 3. Managed headers (Host, Content-Length, etc.) are rejected
//! 4. Empty header names or values are rejected
//!
//! # Examples
//!
//! ```ignore
//! use pdftract_cli::header::parse_header;
//!
//! // Valid header
//! let (name, value) = parse_header("X-API-Key:abc123").unwrap();
//! assert_eq!(name, "X-API-Key");
//! assert_eq!(value, "abc123");
//!
//! // Header with spaces around colon (trimmed)
//! let (name, value) = parse_header("Authorization : Bearer token").unwrap();
//! assert_eq!(name, "Authorization");
//! assert_eq!(value, "Bearer token");
//!
//! // Invalid: no colon
//! assert!(parse_header("NoColon").is_err());
//!
//! // Invalid: CRLF in value
//! assert!(parse_header("X-Bad:\r\nInjected").is_err());
//!
//! // Invalid: managed header
//! assert!(parse_header("Host:example.com").is_err());
//! ```

use std::collections::HashMap;

/// Error type for header parsing failures.
#[derive(Debug, Clone, PartialEq)]
pub enum HeaderError {
    /// No colon found in header string
    MissingColon(String),
    /// Empty header name
    EmptyName(String),
    /// Empty header value
    EmptyValue(String),
    /// Invalid header name (must be [A-Za-z0-9_-]+)
    InvalidName(String),
    /// CRLF injection attempt in name or value
    CrlfInjection(String),
    /// Managed header cannot be set via --header
    ManagedHeader(String),
}

impl std::fmt::Display for HeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeaderError::MissingColon(s) => {
                write!(
                    f,
                    "Header '{}' must contain a ':' delimiter (format: HEADER:VALUE)",
                    s
                )
            }
            HeaderError::EmptyName(s) => {
                write!(f, "Header '{}' has an empty name", s)
            }
            HeaderError::EmptyValue(s) => {
                write!(f, "Header '{}' has an empty value", s)
            }
            HeaderError::InvalidName(name) => {
                write!(
                    f,
                    "Header name '{}' is invalid (must contain only letters, digits, hyphens, and underscores)",
                    name
                )
            }
            HeaderError::CrlfInjection(s) => {
                write!(
                    f,
                    "Header '{}' contains CRLF characters (HTTP header injection protection)",
                    s
                )
            }
            HeaderError::ManagedHeader(name) => {
                write!(
                    f,
                    "Header '{}' is managed automatically by pdftract and cannot be set via --header",
                    name
                )
            }
        }
    }
}

impl std::error::Error for HeaderError {}

/// Headers that are managed by the HTTP client and cannot be set via --header.
///
/// These headers are either:
/// 1. Computed automatically by the HTTP client (Host, Content-Length)
/// 2. Security-critical and must be set via other mechanisms (Authorization via URL credentials)
/// 3. Would break HTTP semantics if user-set (Connection, Transfer-Encoding)
const MANAGED_HEADERS: &[&str] = &[
    "Host",
    "Content-Length",
    "Content-Encoding",
    "Transfer-Encoding",
    "Connection",
    "Upgrade",
    "Proxy-Connection",
    "Keep-Alive",
    "TE",
    "Trailer",
    "Expect",
    "Cookie",
    "Set-Cookie",
    // Note: Authorization is NOT in this list - it's allowed via --header for API keys
];

/// Check if a header name is managed (i.e., cannot be set via --header).
fn is_managed_header(name: &str) -> bool {
    // Case-insensitive comparison per HTTP spec
    let name_lower = name.to_lowercase();
    MANAGED_HEADERS
        .iter()
        .any(|&managed| managed.to_lowercase() == name_lower)
}

/// Validate that a header name matches the HTTP token format.
///
/// HTTP header names must be tokens per RFC 7230 Section 3.2:
/// token = 1*tchar
/// tchar = "!" / "#" / "$" / "%" / "&" / "'" / "*" / "+" / "-" / "." /
///         "^" / "_" / "`" / "|" / "~" / DIGIT / ALPHA
///
/// We use a stricter subset for compatibility: [A-Za-z0-9_-]
/// This excludes special characters that might cause issues.
fn is_valid_header_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name.chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Check for CRLF injection in a string.
///
/// Returns true if the string contains \r or \n characters.
fn contains_crlf(s: &str) -> bool {
    s.contains('\r') || s.contains('\n')
}

/// Parse a single header string into (name, value) tuple.
///
/// # Arguments
///
/// * `header_str` - The header string in format "HEADER:VALUE"
///
/// # Returns
///
/// Returns `Ok((name, value))` where both strings are trimmed, or `Err(HeaderError)`
/// describing why parsing failed.
///
/// # Examples
///
/// ```ignore
/// use pdftract_cli::header::parse_header;
///
/// let (name, value) = parse_header("X-API-Key:abc123").unwrap();
/// assert_eq!(name, "X-API-Key");
/// assert_eq!(value, "abc123");
///
/// // Spaces around colon are trimmed
/// let (name, value) = parse_header("Authorization : Bearer token").unwrap();
/// assert_eq!(name, "Authorization");
/// assert_eq!(value, "Bearer token");
/// ```
pub fn parse_header(header_str: &str) -> Result<(String, String), HeaderError> {
    // Check for CRLF injection FIRST (before trimming, so injection attempts are caught)
    if contains_crlf(header_str) {
        return Err(HeaderError::CrlfInjection(header_str.to_string()));
    }

    // Split on the FIRST colon only (values may contain colons, e.g., URLs)
    let colon_pos = header_str.find(':').ok_or_else(|| {
        HeaderError::MissingColon(header_str.to_string())
    })?;

    let name = header_str[..colon_pos].trim();
    let value = header_str[colon_pos + 1..].trim();

    // Validate name is not empty
    if name.is_empty() {
        return Err(HeaderError::EmptyName(header_str.to_string()));
    }

    // Validate value is not empty
    if value.is_empty() {
        return Err(HeaderError::EmptyValue(header_str.to_string()));
    }

    // Validate header name format
    if !is_valid_header_name(name) {
        return Err(HeaderError::InvalidName(name.to_string()));
    }

    // Check for managed headers
    if is_managed_header(name) {
        return Err(HeaderError::ManagedHeader(name.to_string()));
    }

    Ok((name.to_string(), value.to_string()))
}

/// Parse multiple header strings into a HashMap.
///
/// # Arguments
///
/// * `header_strings` - Iterator of header strings in format "HEADER:VALUE"
///
/// # Returns
///
/// Returns `Ok(HashMap)` mapping header names to values, or `Err(HeaderError)`
/// describing why parsing failed. Headers are case-insensitive per HTTP spec,
/// so later headers with the same name override earlier ones (with a warning).
///
/// # Examples
///
/// ```ignore
/// use pdftract_cli::header::parse_headers;
///
/// let headers = parse_headers(&[
///     "X-API-Key:abc123",
///     "Authorization:Bearer token",
/// ]).unwrap();
/// assert_eq!(headers.get("x-api-key"), Some(&"abc123".to_string()));
/// assert_eq!(headers.get("authorization"), Some(&"Bearer token".to_string()));
/// ```
pub fn parse_headers<'a, I>(header_strings: I) -> Result<HashMap<String, String>, HeaderError>
where
    I: IntoIterator<Item = &'a String>,
{
    let mut headers = HashMap::new();

    for header_str in header_strings {
        let (name, value) = parse_header(header_str)?;
        // HTTP headers are case-insensitive; normalize to lowercase for lookup
        let name_lower = name.to_lowercase();
        if let Some(existing) = headers.get(&name_lower) {
            eprintln!(
                "Warning: Header '{}' was already set to '{}'; overriding with '{}'",
                name, existing, value
            );
        }
        headers.insert(name_lower, value);
    }

    Ok(headers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_header_valid() {
        let (name, value) = parse_header("X-API-Key:abc123").unwrap();
        assert_eq!(name, "X-API-Key");
        assert_eq!(value, "abc123");
    }

    #[test]
    fn test_parse_header_with_spaces() {
        let (name, value) = parse_header("Authorization : Bearer token").unwrap();
        assert_eq!(name, "Authorization");
        assert_eq!(value, "Bearer token");
    }

    #[test]
    fn test_parse_header_value_with_colon() {
        // URLs in values may contain colons
        let (name, value) = parse_header("X-Url:https://example.com:8080/path").unwrap();
        assert_eq!(name, "X-Url");
        assert_eq!(value, "https://example.com:8080/path");
    }

    #[test]
    fn test_parse_header_no_colon() {
        let result = parse_header("NoColon");
        assert!(matches!(result, Err(HeaderError::MissingColon(_))));
    }

    #[test]
    fn test_parse_header_empty_name() {
        let result = parse_header(":value");
        assert!(matches!(result, Err(HeaderError::EmptyName(_))));
    }

    #[test]
    fn test_parse_header_empty_value() {
        let result = parse_header("Name:");
        assert!(matches!(result, Err(HeaderError::EmptyValue(_))));
    }

    #[test]
    fn test_parse_header_crlf_in_name() {
        let result = parse_header("X-Bad\rInjected:value");
        assert!(matches!(result, Err(HeaderError::CrlfInjection(_))));
    }

    #[test]
    fn test_parse_header_crlf_in_value() {
        let result = parse_header("X-Bad:\r\nInjected");
        assert!(matches!(result, Err(HeaderError::CrlfInjection(_))));
    }

    #[test]
    fn test_parse_header_invalid_name_chars() {
        let result = parse_header("X Bad:value");
        assert!(matches!(result, Err(HeaderError::InvalidName(_))));
    }

    #[test]
    fn test_parse_header_host_rejected() {
        let result = parse_header("Host:example.com");
        assert!(matches!(result, Err(HeaderError::ManagedHeader(_))));
    }

    #[test]
    fn test_parse_header_content_length_rejected() {
        let result = parse_header("Content-Length:1234");
        assert!(matches!(result, Err(HeaderError::ManagedHeader(_))));
    }

    #[test]
    fn test_parse_header_authorization_allowed() {
        // Authorization is explicitly allowed (common use case for API keys)
        let (name, value) = parse_header("Authorization:Bearer token").unwrap();
        assert_eq!(name, "Authorization");
        assert_eq!(value, "Bearer token");
    }

    #[test]
    fn test_parse_header_with_quotes() {
        let (name, value) = parse_header("X-Custom:\"quoted value\"").unwrap();
        assert_eq!(name, "X-Custom");
        assert_eq!(value, "\"quoted value\"");
    }

    #[test]
    fn test_is_managed_header() {
        assert!(is_managed_header("Host"));
        assert!(is_managed_header("host")); // Case-insensitive
        assert!(is_managed_header("HOST"));
        assert!(is_managed_header("Content-Length"));
        assert!(!is_managed_header("X-API-Key"));
        assert!(!is_managed_header("Authorization")); // Not managed
    }

    #[test]
    fn test_is_valid_header_name() {
        assert!(is_valid_header_name("X-API-Key"));
        assert!(is_valid_header_name("Content-Type"));
        assert!(is_valid_header_name("X_Custom"));
        assert!(!is_valid_header_name("X Bad"));
        assert!(!is_valid_header_name("X@Bad"));
        assert!(!is_valid_header_name(""));
    }

    #[test]
    fn test_contains_crlf() {
        assert!(contains_crlf("value\r\ninjected"));
        assert!(contains_crlf("value\rinjected"));
        assert!(contains_crlf("value\ninjected"));
        assert!(!contains_crlf("normal value"));
    }

    #[test]
    fn test_parse_headers_multiple() {
        let headers = parse_headers(&[
            "X-API-Key:abc123".to_string(),
            "Authorization:Bearer token".to_string(),
        ])
        .unwrap();

        assert_eq!(headers.get("x-api-key"), Some(&"abc123".to_string()));
        assert_eq!(
            headers.get("authorization"),
            Some(&"Bearer token".to_string())
        );
    }

    #[test]
    fn test_parse_headers_duplicate() {
        let headers = parse_headers(&[
            "X-API-Key:abc123".to_string(),
            "X-API-Key:def456".to_string(),
        ])
        .unwrap();

        // Later header overrides earlier one
        assert_eq!(headers.get("x-api-key"), Some(&"def456".to_string()));
    }

    #[test]
    fn test_parse_headers_empty() {
        let headers = parse_headers(&[]).unwrap();
        assert!(headers.is_empty());
    }

    #[test]
    fn test_parse_headers_invalid_fails() {
        let result = parse_headers(&["NoColon".to_string()]);
        assert!(matches!(result, Err(HeaderError::MissingColon(_))));
    }
}
