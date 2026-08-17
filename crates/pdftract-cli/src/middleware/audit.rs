//! Audit logging middleware for axum.
//!
//! Provides a tower middleware that logs per-request audit records.
//! Extracts client IP from the immediate peer address (not headers by default).
//!
//! # Client IP Detection
//!
//! By default, the middleware uses the immediate peer address from the HTTP
//! connection (the TCP socket's peer address). This prevents IP spoofing via
//! X-Forwarded-For headers.
//!
//! When --trust-forwarded-for is set, the middleware uses the leftmost address
//! from the X-Forwarded-For header. This should only be enabled when behind
//! a trusted reverse proxy that sets this header correctly.

use axum::{
    extract::{ConnectInfo, Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use pdftract_core::audit::AuditLogWriter;
use std::sync::Arc;
use std::time::Instant;

/// Request metadata for audit logging.
///
/// This is stored in the request's state/extensions and used by handlers
/// to write audit records after extraction completes.
#[derive(Clone, Debug)]
pub struct RequestMetadata {
    /// Request start time (for duration calculation)
    pub start_time: Instant,
    /// Client IP address (if available)
    pub client_ip: Option<String>,
    /// Tool name (extracted from path)
    pub tool: String,
}

/// Audit log state.
///
/// Holds the optional audit log writer wrapped in an Arc for shared access.
#[derive(Clone)]
pub struct AuditState {
    pub writer: Option<Arc<AuditLogWriter>>,
    /// Whether to trust X-Forwarded-For header for client IP detection.
    /// When false (default), uses the immediate peer address.
    pub trust_forwarded_for: bool,
}

impl AuditState {
    /// Create a new audit state.
    pub fn new(writer: Option<AuditLogWriter>) -> Self {
        Self {
            writer: writer.map(Arc::new),
            trust_forwarded_for: false,
        }
    }

    /// Create a new audit state with X-Forwarded-For trust enabled.
    pub fn with_trusted_forwarded_for(writer: Option<AuditLogWriter>) -> Self {
        Self {
            writer: writer.map(Arc::new),
            trust_forwarded_for: true,
        }
    }
}

/// Extract client IP from headers (only when --trust-forwarded-for is enabled).
///
/// When enabled, uses the leftmost address from X-Forwarded-For.
/// The X-Real-IP header is NOT used (deprecated in favor of X-Forwarded-For).
///
/// # Security
///
/// X-Forwarded-For is easily spoofed by clients. Only use this when behind
/// a trusted reverse proxy that correctly sets this header.
fn extract_client_ip_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            // X-Forwarded-For format: "client, proxy1, proxy2"
            // The leftmost address is the original client
            s.split(',').next().map(|addr| addr.trim().to_string())
        })
}

/// Audit logging middleware.
///
/// Stores request metadata for later audit logging by handlers.
/// The actual audit record is written after extraction completes,
/// when the fingerprint and diagnostics are available.
///
/// # Client IP Detection
///
/// - Default: Uses the immediate peer address from the TCP connection.
///   This prevents IP spoofing.
/// - With --trust-forwarded-for: Uses the leftmost address from X-Forwarded-For.
///   Only enable this behind a trusted reverse proxy.
pub async fn audit_middleware(
    State(state): State<AuditState>,
    ConnectInfo(peer_addr): ConnectInfo<std::net::SocketAddr>,
    mut req: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let path = req.uri().path().to_string();

    // Extract client IP based on trust_forwarded_for setting
    let client_ip = if state.trust_forwarded_for {
        // Use X-Forwarded-For header (leftmost address)
        extract_client_ip_from_headers(req.headers())
    } else {
        // Use immediate peer address (IP only, no port)
        Some(peer_addr.ip().to_string())
    };

    // Extract tool name from path (e.g., "/extract" -> "extract", "/sse" -> "mcp")
    let tool = path
        .strip_prefix('/')
        .unwrap_or(&path)
        .split('/')
        .next()
        .unwrap_or("unknown");

    // Store request metadata for later use by handlers
    let metadata = RequestMetadata {
        start_time: start,
        client_ip,
        tool: tool.to_string(),
    };
    req.extensions_mut().insert(metadata);

    // Run the handler (which will write the audit record)
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_client_ip_from_headers_single() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "10.0.0.1".parse().unwrap());
        let ip = extract_client_ip_from_headers(&headers);
        assert_eq!(ip, Some("10.0.0.1".to_string()));
    }

    #[test]
    fn test_extract_client_ip_from_headers_multiple() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "10.0.0.1, 10.0.0.2, 10.0.0.3".parse().unwrap(),
        );
        let ip = extract_client_ip_from_headers(&headers);
        // Leftmost address should be used
        assert_eq!(ip, Some("10.0.0.1".to_string()));
    }

    #[test]
    fn test_extract_client_ip_from_headers_whitespace() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "  10.0.0.1  , 10.0.0.2".parse().unwrap());
        let ip = extract_client_ip_from_headers(&headers);
        assert_eq!(ip, Some("10.0.0.1".to_string()));
    }

    #[test]
    fn test_extract_client_ip_from_headers_none() {
        let headers = HeaderMap::new();
        let ip = extract_client_ip_from_headers(&headers);
        assert!(ip.is_none());
    }

    #[test]
    fn test_audit_state_defaults() {
        let state = AuditState::new(None);
        assert!(state.writer.is_none());
        assert!(!state.trust_forwarded_for);
    }

    #[test]
    fn test_audit_state_with_writer() {
        // This test just verifies the constructor works
        // Actual file I/O is tested in pdftract-core
        let _state = AuditState::new(Some(
            AuditLogWriter::open(Path::new("/dev/stdout")).unwrap(),
        ));
    }

    #[test]
    fn test_audit_state_with_trusted_forwarded_for() {
        let state = AuditState::with_trusted_forwarded_for(None);
        assert!(state.writer.is_none());
        assert!(state.trust_forwarded_for);
    }
}
