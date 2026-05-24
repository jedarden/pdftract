//! URL validation for SSRF protection (Phase 1.8, TH-05).
//!
//! This module provides URL validation logic to prevent Server-Side Request Forgery
//! attacks. It validates URLs against a set of dangerous address ranges including:
//! - RFC 1918 private IPv4 ranges (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16)
//! - IPv6 Unique Local Addresses (ULA) (fc00::/7, fd00::/8)
//! - Loopback addresses (127.0.0.0/8, ::1)
//! - Link-local addresses (169.254.0.0/16, fe80::/10)
//! - Cloud metadata endpoints (169.254.169.254, 100.100.100.200, etc.)
//!
//! URLs targeting these addresses are rejected unless the `--allow-private-networks`
//! flag is set.

use crate::diagnostics::{Diagnostic, DiagCode};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Error type for URL validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlValidationError {
    /// URL scheme is not https://
    InvalidScheme(String),
    /// URL targets a private network address (SSRF protection)
    PrivateNetwork(String),
    /// DNS resolution failed
    DnsFailed(String),
    /// Invalid URL format
    InvalidUrl(String),
}

impl std::fmt::Display for UrlValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UrlValidationError::InvalidScheme(scheme) => {
                write!(f, "Invalid URL scheme: '{}'. Only 'https://' is allowed.", scheme)
            }
            UrlValidationError::PrivateNetwork(addr) => {
                write!(f, "URL targets private network address: {}. Use --allow-private-networks to enable (WARNING: security risk).", addr)
            }
            UrlValidationError::DnsFailed(host) => {
                write!(f, "DNS resolution failed for host: {}", host)
            }
            UrlValidationError::InvalidUrl(url) => {
                write!(f, "Invalid URL format: {}", url)
            }
        }
    }
}

impl std::error::Error for UrlValidationError {}

/// Result type for URL validation.
pub type Result<T> = std::result::Result<T, UrlValidationError>;

/// Check if an IPv4 address is in a private network range.
///
/// This checks RFC 1918 private addresses:
/// - 10.0.0.0/8 (10.0.0.0 – 10.255.255.255)
/// - 172.16.0.0/12 (172.16.0.0 – 172.31.255.255)
/// - 192.168.0.0/16 (192.168.0.0 – 192.168.255.255)
///
/// Plus other reserved ranges:
/// - 127.0.0.0/8 (loopback)
/// - 169.254.0.0/16 (link-local)
/// - 0.0.0.0/8 (current network)
fn is_private_ipv4(addr: Ipv4Addr) -> bool {
    let octets = addr.octets();

    match octets {
        // 10.0.0.0/8
        [10, _, _, _] => true,
        // 172.16.0.0/12
        [172, 16..=31, _, _] => true,
        // 192.168.0.0/16
        [192, 168, _, _] => true,
        // 127.0.0.0/8 (loopback)
        [127, _, _, _] => true,
        // 169.254.0.0/16 (link-local)
        [169, 254, _, _] => true,
        // 0.0.0.0/8 (current network)
        [0, _, _, _] => true,
        _ => false,
    }
}

/// Check if an IPv6 address is in a private network range.
///
/// This checks:
/// - fc00::/7 (Unique Local Addresses - ULA)
/// - ::1 (loopback)
/// - fe80::/10 (link-local)
/// - ff00::/8 (multicast)
fn is_private_ipv6(addr: &Ipv6Addr) -> bool {
    let segments = addr.segments();

    // fc00::/7 (ULA) - fc00::/7 and fd00::/8
    if (segments[0] & 0xfe00) == 0xfc00 {
        return true;
    }

    // ::1 (loopback)
    if addr.is_loopback() {
        return true;
    }

    // fe80::/10 (link-local)
    if (segments[0] & 0xffc0) == 0xfe80 {
        return true;
    }

    // ff00::/8 (multicast)
    if (segments[0] & 0xff00) == 0xff00 {
        return true;
    }

    false
}

/// Known cloud metadata endpoint addresses.
///
/// These are well-known endpoints that return cloud instance credentials:
/// - AWS: 169.254.169.254
/// - GCP: metadata.google.internal (resolves to various internal IPs)
/// - Azure: 168.63.129.16
/// - Alibaba: 100.100.100.200
fn is_metadata_endpoint(addr: &IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => {
            // AWS metadata endpoint
            if v4 == &Ipv4Addr::new(169, 254, 169, 254) {
                return true;
            }
            // Azure metadata endpoint
            if v4 == &Ipv4Addr::new(168, 63, 129, 16) {
                return true;
            }
            // Alibaba metadata endpoint
            if v4 == &Ipv4Addr::new(100, 100, 100, 200) {
                return true;
            }
            false
        }
        IpAddr::V6(_v6) => {
            // IPv6 metadata endpoints would go here
            // (e.g., fd00:ec2::254 for some AWS regions)
            false
        }
    }
}

/// Known metadata endpoint hostnames.
///
/// These hostnames are checked before DNS resolution to prevent
/// DNS rebinding attacks.
const METADATA_HOSTNAMES: &[&str] = &[
    "metadata.google.internal",
    "instance-data.google.internal",
];

/// Check if a hostname is a known metadata endpoint.
fn is_metadata_hostname(hostname: &str) -> bool {
    let hostname_lower = hostname.to_lowercase();
    METADATA_HOSTNAMES
        .iter()
        .any(|&h| hostname_lower == h || hostname_lower.ends_with(&format!(".{}", h)))
}

/// Validate a URL for SSRF protection.
///
/// This function performs the following checks:
/// 1. URL scheme must be `https://`
/// 2. Hostname is not a known metadata endpoint
/// 3. Resolved IP address is not in a private network range
///
/// DNS resolution happens once and the resolved address is checked.
/// This prevents DNS rebinding attacks.
///
/// # Arguments
///
/// * `url_str` - The URL string to validate
/// * `allow_private_networks` - If true, private network addresses are allowed
///
/// # Returns
///
/// Returns `Ok(())` if the URL is valid, or an error describing the validation failure.
pub fn validate_url(url_str: &str, allow_private_networks: bool) -> Result<()> {
    // Check for IPv6 zone IDs in the raw URL (before parsing)
    // The url crate strips zone IDs, so we need to check the raw string
    if url_str.contains('%') {
        return Err(UrlValidationError::PrivateNetwork(
            "IPv6 link-local address (zone ID)".to_string()
        ));
    }

    // Parse the URL
    let url = url::Url::parse(url_str)
        .map_err(|_| UrlValidationError::InvalidUrl(url_str.to_string()))?;

    // Check scheme: only https:// is allowed
    match url.scheme() {
        "https" => {},
        scheme => {
            return Err(UrlValidationError::InvalidScheme(scheme.to_string()));
        }
    }

    // Extract hostname
    let hostname = url.host_str()
        .ok_or_else(|| UrlValidationError::InvalidUrl(url_str.to_string()))?;

    // Check for metadata hostnames (before DNS resolution)
    if is_metadata_hostname(hostname) {
        return Err(UrlValidationError::PrivateNetwork(
            format!("metadata endpoint: {}", hostname)
        ));
    }

    // Resolve the hostname to an IP address
    // Note: We use std::net::ToSocketAddrs which performs DNS resolution
    use std::net::ToSocketAddrs;
    let addrs: std::vec::Vec<std::net::SocketAddr> = format!("{}:443", hostname)
        .to_socket_addrs()
        .map_err(|_| UrlValidationError::DnsFailed(hostname.to_string()))?
        .collect();

    if addrs.is_empty() {
        return Err(UrlValidationError::DnsFailed(hostname.to_string()));
    }

    // Check all resolved addresses
    for addr in addrs {
        let ip_addr = addr.ip();

        // Check for metadata endpoints
        if is_metadata_endpoint(&ip_addr) {
            return Err(UrlValidationError::PrivateNetwork(
                format!("cloud metadata endpoint: {}", ip_addr)
            ));
        }

        // If private networks are not allowed, check the IP ranges
        if !allow_private_networks {
            match ip_addr {
                IpAddr::V4(v4) => {
                    if is_private_ipv4(v4) {
                        return Err(UrlValidationError::PrivateNetwork(
                            format!("private IPv4: {}", v4)
                        ));
                    }
                }
                IpAddr::V6(v6) => {
                    if is_private_ipv6(&v6) {
                        return Err(UrlValidationError::PrivateNetwork(
                            format!("private IPv6: {}", v6)
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}

/// Validate a URL and return a diagnostic if validation fails.
///
/// This is a convenience function for use in the extraction pipeline.
/// It returns `Ok(())` if the URL is valid, or `Err(diagnostic)` if it fails.
///
/// # Arguments
///
/// * `url_str` - The URL string to validate
/// * `allow_private_networks` - If true, private network addresses are allowed
///
/// # Returns
///
/// Returns `Ok(())` if the URL is valid, or `Err(Diagnostic)` if validation fails.
pub fn validate_url_with_diagnostic(
    url_str: &str,
    allow_private_networks: bool,
) -> std::result::Result<(), Diagnostic> {
    validate_url(url_str, allow_private_networks)
        .map_err(|err| {
            let message = err.to_string();
            Diagnostic::with_dynamic_no_offset(DiagCode::RemoteUrlPrivateNetwork, message)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_private_ipv4() {
        // RFC 1918 private addresses
        assert!(is_private_ipv4(Ipv4Addr::new(10, 0, 0, 1)));
        assert!(is_private_ipv4(Ipv4Addr::new(10, 255, 255, 254)));
        assert!(is_private_ipv4(Ipv4Addr::new(172, 16, 0, 1)));
        assert!(is_private_ipv4(Ipv4Addr::new(172, 31, 255, 254)));
        assert!(is_private_ipv4(Ipv4Addr::new(192, 168, 0, 1)));
        assert!(is_private_ipv4(Ipv4Addr::new(192, 168, 255, 254)));

        // Loopback
        assert!(is_private_ipv4(Ipv4Addr::new(127, 0, 0, 1)));
        assert!(is_private_ipv4(Ipv4Addr::new(127, 255, 255, 255)));

        // Link-local
        assert!(is_private_ipv4(Ipv4Addr::new(169, 254, 0, 1)));

        // Public addresses
        assert!(!is_private_ipv4(Ipv4Addr::new(8, 8, 8, 8)));
        assert!(!is_private_ipv4(Ipv4Addr::new(1, 1, 1, 1)));
        assert!(!is_private_ipv4(Ipv4Addr::new(172, 15, 255, 255))); // Just outside 172.16.0.0/12
        assert!(!is_private_ipv4(Ipv4Addr::new(172, 32, 0, 1))); // Just outside 172.16.0.0/12
    }

    #[test]
    fn test_is_private_ipv6() {
        // ULA
        assert!(is_private_ipv6(&"fc00::1".parse().unwrap()));
        assert!(is_private_ipv6(&"fd00::1".parse().unwrap()));

        // Loopback
        assert!(is_private_ipv6(&"::1".parse().unwrap()));

        // Link-local
        assert!(is_private_ipv6(&"fe80::1".parse().unwrap()));

        // Multicast
        assert!(is_private_ipv6(&"ff00::1".parse().unwrap()));

        // Public addresses
        assert!(!is_private_ipv6(&"2001:4860:4860::8888".parse().unwrap()));
        assert!(!is_private_ipv6(&"2606:2800:220:1:248:1893:25c8:1946".parse().unwrap()));
    }

    #[test]
    fn test_is_metadata_endpoint() {
        // AWS
        assert!(is_metadata_endpoint(&IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))));

        // Azure
        assert!(is_metadata_endpoint(&IpAddr::V4(Ipv4Addr::new(168, 63, 129, 16))));

        // Alibaba
        assert!(is_metadata_endpoint(&IpAddr::V4(Ipv4Addr::new(100, 100, 100, 200))));

        // Non-metadata
        assert!(!is_metadata_endpoint(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
    }

    #[test]
    fn test_is_metadata_hostname() {
        assert!(is_metadata_hostname("metadata.google.internal"));
        assert!(is_metadata_hostname("instance-data.google.internal"));
        assert!(is_metadata_hostname("foo.metadata.google.internal"));
        assert!(!is_metadata_hostname("example.com"));
        assert!(!is_metadata_hostname("google.com"));
    }

    #[test]
    fn test_validate_url_rejects_http() {
        let result = validate_url("http://example.com/", false);
        assert!(matches!(result, Err(UrlValidationError::InvalidScheme(_))));
    }

    #[test]
    fn test_validate_url_rejects_ftp() {
        let result = validate_url("ftp://example.com/", false);
        assert!(matches!(result, Err(UrlValidationError::InvalidScheme(_))));
    }

    #[test]
    fn test_validate_url_rejects_file() {
        let result = validate_url("file:///etc/passwd", false);
        assert!(matches!(result, Err(UrlValidationError::InvalidScheme(_))));
    }

    #[test]
    fn test_validate_url_rejects_metadata_hostname() {
        let result = validate_url("https://metadata.google.internal/", false);
        assert!(matches!(result, Err(UrlValidationError::PrivateNetwork(_))));
    }
}
