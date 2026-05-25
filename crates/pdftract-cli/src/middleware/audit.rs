//! Audit logging middleware for axum.
//!
//! Provides a tower middleware that logs per-request audit records.
//! Extracts client IP from headers and records request duration.

use anyhow::Result;
use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use pdftract_core::audit::AuditLogWriter;
use std::sync::Arc;
use std::time::Instant;

/// Audit log state.
///
/// Holds the optional audit log writer wrapped in an Arc for shared access.
#[derive(Clone)]
pub struct AuditState {
    pub writer: Option<Arc<AuditLogWriter>>,
}

impl AuditState {
    /// Create a new audit state.
    pub fn new(writer: Option<AuditLogWriter>) -> Self {
        Self {
            writer: writer.map(Arc::new),
        }
    }
}

/// Extract client IP from headers.
///
/// Checks X-Real-IP and X-Forwarded-For headers (set by reverse proxies).
/// Returns None if no headers are present.
fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-real-ip")
        .or_else(|| headers.get("x-forwarded-for"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Audit logging middleware.
///
/// Records per-request audit logs including:
/// - Timestamp
/// - Client IP (from X-Real-IP or X-Forwarded-For)
/// - Tool name (extracted from URI path)
/// - Request duration
/// - Status code
pub async fn audit_middleware(
    State(state): State<AuditState>,
    req: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let path = req.uri().path().to_string();
    let client_ip = extract_client_ip(req.headers());

    // Extract tool name from path (e.g., "/extract" -> "extract")
    let tool = path
        .strip_prefix('/')
        .unwrap_or(&path)
        .split('/')
        .next()
        .unwrap_or("unknown");

    let response = next.run(req).await;
    let duration_ms = start.elapsed().as_millis() as u64;
    let status = response.status().as_u16();

    // Write audit record if audit log is enabled
    if let Some(ref writer) = state.writer {
        let status_str = if status < 400 { "ok" } else { "error" };
        if let Err(e) = writer.log(
            tool,
            client_ip.as_deref(),
            None, // fingerprint not available at middleware level
            duration_ms,
            status_str,
            &[],
        ) {
            eprintln!("Failed to write audit log: {}", e);
        }
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_client_ip_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "10.0.0.1".parse().unwrap());
        let ip = extract_client_ip(&headers);
        assert_eq!(ip, Some("10.0.0.1".to_string()));
    }

    #[test]
    fn test_extract_client_ip_x_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "10.0.0.2".parse().unwrap());
        let ip = extract_client_ip(&headers);
        assert_eq!(ip, Some("10.0.0.2".to_string()));
    }

    #[test]
    fn test_extract_client_ip_x_real_ip_preferred() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "10.0.0.1".parse().unwrap());
        headers.insert("x-forwarded-for", "10.0.0.2".parse().unwrap());
        let ip = extract_client_ip(&headers);
        assert_eq!(ip, Some("10.0.0.1".to_string()));
    }

    #[test]
    fn test_extract_client_ip_none() {
        let headers = HeaderMap::new();
        let ip = extract_client_ip(&headers);
        assert!(ip.is_none());
    }
}
