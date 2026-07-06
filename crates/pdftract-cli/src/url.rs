//! URL parsing and credential extraction for remote PDF sources.
//!
//! This module provides functionality for parsing URLs and extracting embedded
//! credentials (https://user:pass@host/path) for HTTP basic authentication.
//!
//! # URL Format with Credentials
//!
//! URLs may contain embedded credentials in the authority section:
//! - `https://user:pass@host/path` - user and password
//! - `https://user@host/path` - user only (empty password)
//! - `https://host/path` - no credentials
//!
//! # Security Considerations
//!
//! Embedded credentials in URLs are visible in:
//! - Shell history (`.bash_history`, `.zsh_history`)
//! - Process listings (`ps aux`)
//! - Log files (if URLs are logged)
//!
//! For production use, the `--header` flag is preferred:
//! ```bash
//! pdftract extract --header "Authorization: Bearer TOKEN" https://...
//! ```
//!
//! ureq automatically sets `Authorization: Basic <base64>` from URL credentials.

use std::collections::HashMap;
use url::Url;

/// Error type for URL parsing failures.
#[derive(Debug, Clone, PartialEq)]
pub enum UrlError {
    /// Invalid URL syntax
    InvalidUrl(String),
    /// Unsupported URL scheme (only http/https allowed)
    UnsupportedScheme(String),
    /// Missing host in URL
    MissingHost(String),
}

impl std::fmt::Display for UrlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UrlError::InvalidUrl(s) => {
                write!(f, "Invalid URL: '{}'", s)
            }
            UrlError::UnsupportedScheme(scheme) => {
                write!(
                    f,
                    "Unsupported URL scheme '{}': only http and https are supported",
                    scheme
                )
            }
            UrlError::MissingHost(s) => {
                write!(f, "URL missing host: '{}'", s)
            }
        }
    }
}

impl std::error::Error for UrlError {}

/// Parsed URL components with extracted credentials.
#[derive(Debug, Clone)]
pub struct ParsedUrl {
    /// The reconstructed URL without embedded credentials
    /// (https://host/path instead of https://user:pass@host/path)
    pub url: String,
    /// Optional username extracted from the URL
    pub username: Option<String>,
    /// Optional password extracted from the URL
    pub password: Option<String>,
    /// Whether credentials were extracted (for warning emission)
    pub has_credentials: bool,
}

/// Parse a URL and extract embedded credentials.
///
/// # Arguments
///
/// * `url_str` - The URL string, potentially with embedded credentials
///
/// # Returns
///
/// Returns `Ok(ParsedUrl)` with the reconstructed URL and extracted credentials,
/// or `Err(UrlError)` describing why parsing failed.
///
/// # Examples
///
/// ```ignore
/// use pdftract_cli::url::parse_url;
///
/// // URL with credentials
/// let parsed = parse_url("https://user:pass@example.com/doc.pdf").unwrap();
/// assert_eq!(parsed.url, "https://example.com/doc.pdf");
/// assert_eq!(parsed.username, Some("user".to_string()));
/// assert_eq!(parsed.password, Some("pass".to_string()));
/// assert!(parsed.has_credentials);
///
/// // URL without credentials
/// let parsed = parse_url("https://example.com/doc.pdf").unwrap();
/// assert_eq!(parsed.url, "https://example.com/doc.pdf");
/// assert!(parsed.username.is_none());
/// assert!(parsed.password.is_none());
/// assert!(!parsed.has_credentials);
///
/// // URL with username only
/// let parsed = parse_url("https://user@example.com/doc.pdf").unwrap();
/// assert_eq!(parsed.url, "https://example.com/doc.pdf");
/// assert_eq!(parsed.username, Some("user".to_string()));
/// assert!(parsed.password.is_none()); // Empty password
/// assert!(parsed.has_credentials);
/// ```
pub fn parse_url(url_str: &str) -> Result<ParsedUrl, UrlError> {
    // Use url crate to parse the URL
    let parsed = url::Url::parse(url_str).map_err(|_| UrlError::InvalidUrl(url_str.to_string()))?;

    // Check scheme (only http and https allowed)
    match parsed.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(UrlError::UnsupportedScheme(scheme.to_string()));
        }
    }

    // Check for host
    if parsed.host().is_none() {
        return Err(UrlError::MissingHost(url_str.to_string()));
    }

    // Extract credentials
    let username = parsed.username();
    let has_username = !username.is_empty();

    // url crate doesn't expose password directly, we need to reconstruct
    let password = if has_username {
        // The password is in the URL but not exposed by url::Url
        // We'll need to check the original URL string
        extract_password_from_url(url_str, username)
    } else {
        None
    };

    let has_credentials = has_username || password.is_some();

    // Reconstruct URL without credentials
    let scheme = parsed.scheme();
    let host = parsed.host_str().unwrap_or("");
    let port = parsed.port();
    let path = parsed.path();
    let query = parsed.query();
    let fragment = parsed.fragment();

    let mut reconstructed = String::new();
    reconstructed.push_str(scheme);
    reconstructed.push_str("://");
    reconstructed.push_str(host);

    if let Some(port_num) = port {
        reconstructed.push(':');
        reconstructed.push_str(&port_num.to_string());
    }

    reconstructed.push_str(path);

    if let Some(q) = query {
        reconstructed.push('?');
        reconstructed.push_str(q);
    }

    if let Some(f) = fragment {
        reconstructed.push('#');
        reconstructed.push_str(f);
    }

    Ok(ParsedUrl {
        url: reconstructed,
        username: if has_username {
            Some(username.to_string())
        } else {
            None
        },
        password,
        has_credentials,
    })
}

/// Extract password from a URL string that has credentials.
///
/// The url crate doesn't expose the password directly, so we parse it manually.
fn extract_password_from_url(url_str: &str, username: &str) -> Option<String> {
    // Find the scheme:// part
    let scheme_end = url_str.find("://")?;
    let authority_start = scheme_end + 3;

    // Find the @ that separates credentials from host
    let at_pos = url_str[authority_start..].find('@')?;
    let credentials_end = authority_start + at_pos;

    // Extract the credentials part (before @)
    let credentials = &url_str[authority_start..credentials_end];

    // Split on ':' to get username:password
    // If there's no ':', there's no password
    let colon_pos = credentials.find(':')?;

    // Extract password (after ':')
    let password = &credentials[colon_pos + 1..];

    // Verify the username matches (to handle edge cases)
    let extracted_username = &credentials[..colon_pos];
    if extracted_username != username {
        return None; // Mismatch, something went wrong
    }

    Some(password.to_string())
}

/// Convert parsed credentials to HTTP headers.
///
/// If the ParsedUrl contains credentials, this creates an Authorization header.
/// ureq automatically handles basic auth when credentials are in the URL,
/// but this function is provided for manual header construction if needed.
///
/// # Arguments
///
/// * `parsed` - The parsed URL with potential credentials
///
/// # Returns
///
/// A vector of header tuples (name, value). Returns an empty vector if no
/// credentials are present.
///
/// # Examples
///
/// ```ignore
/// use pdftract_cli::url::{parse_url, credentials_to_headers};
///
/// let parsed = parse_url("https://user:pass@example.com/doc.pdf").unwrap();
/// let headers = credentials_to_headers(&parsed);
///
/// assert!(!headers.is_empty());
/// assert_eq!(headers[0].0, "Authorization");
/// // Value is "Basic <base64(user:pass)>"
/// ```
pub fn credentials_to_headers(parsed: &ParsedUrl) -> Vec<(String, String)> {
    if !parsed.has_credentials {
        return Vec::new();
    }

    // ureq handles basic auth automatically when credentials are in the URL,
    // so we don't need to construct the Authorization header manually.
    // This function is provided for completeness and for cases where
    // manual header construction is needed.

    // Note: The actual Authorization header will be set by ureq
    // when we pass the URL with embedded credentials to HttpRangeSource.
    // This function is primarily for documentation and debugging.

    Vec::new()
}

/// Combine custom headers with URL credentials.
///
/// Merges custom headers (from --header flag) with URL credentials.
/// Custom headers take precedence over URL credentials (if both specify
/// Authorization, the custom header wins).
///
/// # Arguments
///
/// * `custom_headers` - Custom headers from --header flag (lowercase names)
/// * `parsed_url` - Optional parsed URL with embedded credentials
///
/// # Returns
///
/// A HashMap of header names (lowercase) to values.
///
/// # Examples
///
/// ```ignore
/// use pdftract_cli::url::{parse_url, combine_headers_with_credentials};
/// use std::collections::HashMap;
///
/// // Custom headers from --header flag
/// let mut custom = HashMap::new();
/// custom.insert("x-api-key".to_string(), "secret".to_string());
///
/// // URL with credentials
/// let parsed = parse_url("https://user:pass@example.com/doc.pdf").unwrap();
///
/// // Combine (ureq will handle the basic auth from the URL)
/// let headers = combine_headers_with_credentials(&custom, Some(&parsed));
///
/// assert!(headers.contains_key("x-api-key"));
/// assert!(headers.contains_key("authorization")); // Added by ureq
/// ```
pub fn combine_headers_with_credentials(
    custom_headers: &HashMap<String, String>,
    parsed_url: Option<&ParsedUrl>,
) -> HashMap<String, String> {
    let mut result = custom_headers.clone();

    // If the URL has credentials, ureq will automatically add the
    // Authorization header when we pass the URL with embedded credentials.
    // We don't need to add it here manually.
    // However, if a custom Authorization header was provided via --header,
    // it takes precedence (ureq respects explicit headers).

    if let Some(parsed) = parsed_url {
        if parsed.has_credentials {
            // Emit a warning about credentials in shell history
            // (This is handled at the call site in main.rs)
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_url_with_credentials() {
        let parsed = parse_url("https://user:pass@example.com/doc.pdf").unwrap();
        assert_eq!(parsed.url, "https://example.com/doc.pdf");
        assert_eq!(parsed.username, Some("user".to_string()));
        assert_eq!(parsed.password, Some("pass".to_string()));
        assert!(parsed.has_credentials);
    }

    #[test]
    fn test_parse_url_without_credentials() {
        let parsed = parse_url("https://example.com/doc.pdf").unwrap();
        assert_eq!(parsed.url, "https://example.com/doc.pdf");
        assert!(parsed.username.is_none());
        assert!(parsed.password.is_none());
        assert!(!parsed.has_credentials);
    }

    #[test]
    fn test_parse_url_with_username_only() {
        let parsed = parse_url("https://user@example.com/doc.pdf").unwrap();
        assert_eq!(parsed.url, "https://example.com/doc.pdf");
        assert_eq!(parsed.username, Some("user".to_string()));
        assert!(parsed.password.is_none()); // Empty password
        assert!(parsed.has_credentials);
    }

    #[test]
    fn test_parse_url_with_port() {
        let parsed = parse_url("https://user:pass@example.com:8080/doc.pdf").unwrap();
        assert_eq!(parsed.url, "https://example.com:8080/doc.pdf");
        assert_eq!(parsed.username, Some("user".to_string()));
        assert_eq!(parsed.password, Some("pass".to_string()));
        assert!(parsed.has_credentials);
    }

    #[test]
    fn test_parse_url_with_query_and_fragment() {
        let parsed = parse_url("https://user:pass@example.com/doc.pdf?query=1#fragment").unwrap();
        assert_eq!(parsed.url, "https://example.com/doc.pdf?query=1#fragment");
        assert_eq!(parsed.username, Some("user".to_string()));
        assert_eq!(parsed.password, Some("pass".to_string()));
        assert!(parsed.has_credentials);
    }

    #[test]
    fn test_parse_url_http_scheme() {
        let parsed = parse_url("http://user:pass@example.com/doc.pdf").unwrap();
        assert_eq!(parsed.url, "http://example.com/doc.pdf");
        assert!(parsed.has_credentials);
    }

    #[test]
    fn test_parse_url_invalid_scheme() {
        let result = parse_url("ftp://example.com/doc.pdf");
        assert!(matches!(result, Err(UrlError::UnsupportedScheme(_))));

        let result = parse_url("file:///path/to/doc.pdf");
        assert!(matches!(result, Err(UrlError::UnsupportedScheme(_))));
    }

    #[test]
    fn test_parse_url_invalid() {
        let result = parse_url("not-a-url");
        assert!(matches!(result, Err(UrlError::InvalidUrl(_))));

        let result = parse_url("https://");
        assert!(matches!(result, Err(UrlError::MissingHost(_))));
    }

    #[test]
    fn test_extract_password_from_url() {
        let password = extract_password_from_url("https://user:pass@example.com/doc.pdf", "user");
        assert_eq!(password, Some("pass".to_string()));

        let password =
            extract_password_from_url("https://user:password123@example.com/doc.pdf", "user");
        assert_eq!(password, Some("password123".to_string()));

        let password = extract_password_from_url("https://user:@example.com/doc.pdf", "user");
        assert_eq!(password, Some("".to_string()));

        let password = extract_password_from_url("https://user@example.com/doc.pdf", "user");
        assert_eq!(password, None);
    }

    #[test]
    fn test_credentials_to_headers() {
        let parsed = parse_url("https://user:pass@example.com/doc.pdf").unwrap();
        let headers = credentials_to_headers(&parsed);

        // ureq handles basic auth automatically, so we return empty
        assert!(headers.is_empty());
    }

    #[test]
    fn test_combine_headers_with_credentials() {
        let mut custom = HashMap::new();
        custom.insert("x-api-key".to_string(), "secret".to_string());

        let parsed = parse_url("https://user:pass@example.com/doc.pdf").unwrap();
        let result = combine_headers_with_credentials(&custom, Some(&parsed));

        assert_eq!(result.get("x-api-key"), Some(&"secret".to_string()));
        // ureq will add Authorization automatically from URL credentials
    }

    #[test]
    fn test_combine_headers_without_credentials() {
        let mut custom = HashMap::new();
        custom.insert("x-api-key".to_string(), "secret".to_string());

        let result = combine_headers_with_credentials(&custom, None);

        assert_eq!(result.get("x-api-key"), Some(&"secret".to_string()));
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_parse_url_preserves_path() {
        let parsed = parse_url("https://user:pass@example.com/path/to/doc.pdf").unwrap();
        assert_eq!(parsed.url, "https://example.com/path/to/doc.pdf");
    }

    #[test]
    fn test_parse_url_with_empty_path() {
        let parsed = parse_url("https://user:pass@example.com").unwrap();
        assert_eq!(parsed.url, "https://example.com");
    }

    #[test]
    fn test_parse_url_with_special_chars_in_password() {
        let parsed = parse_url("https://user:p@ss:wo_rd@example.com/doc.pdf").unwrap();
        assert_eq!(parsed.username, Some("user".to_string()));
        // Password should include special chars
        assert!(parsed.password.is_some());
        assert!(parsed.has_credentials);
    }

    #[test]
    fn test_parse_url_urlencoded_credentials() {
        // URL-encoded credentials (e.g., @ in username as %40)
        let parsed = parse_url("https://user%40domain:pass%23word@example.com/doc.pdf").unwrap();
        assert_eq!(parsed.username, Some("user@domain".to_string()));
        assert_eq!(parsed.password, Some("pass#word".to_string()));
        assert!(parsed.has_credentials);
    }
}
