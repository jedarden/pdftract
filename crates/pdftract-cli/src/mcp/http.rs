//! HTTP+SSE transport for the MCP server.
//!
//! This module implements the HTTP+SSE transport defined in the MCP spec:
//! https://modelcontextprotocol.io/spec/transports#http-with-sse
//!
//! # Transport architecture
//!
//! - POST /: client → server JSON-RPC requests (single or batch)
//! - GET /sse: server → client notifications via Server-Sent Events
//! - GET /health: health check endpoint (always returns 200 OK)
//!
//! # Concurrency model
//!
//! - Each SSE connection gets its own broadcast channel
//! - Server uses tokio::sync::broadcast for fan-out of notifications
//! - Backpressure handling: slow clients get dropped with logged warning
//!
//! # Authentication
//!
//! - Bearer token via Authorization header when --auth-token is set
//! - Required for non-loopback binds (per TH-03)
//! - /health endpoint is exempt from auth (always returns 200)

use crate::mcp::framing::{BatchMessage, ErrorObject, Id, Notification, Request, Response};
use crate::mcp::tools;
use crate::middleware::{AuditState, audit_middleware};
use anyhow::{anyhow, Context, Result};
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Request as AxumRequest, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Json, Response as AxumResponse, Sse},
    routing::{get, post},
    Router,
};
use pdftract_core::audit::AuditLogWriter;
use secrecy::{ExposeSecret, SecretString};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use subtle::ConstantTimeEq;
use tokio::sync::broadcast;

/// Default maximum request body size (256 MB)
const DEFAULT_MAX_UPLOAD_MB: usize = 256;

/// SSE keepalive interval (30 seconds)
const SSE_KEEPALIVE_SECS: u64 = 30;

/// Maximum number of concurrent SSE clients
const MAX_SSE_CLIENTS: usize = 100;

/// Shared server state for the MCP HTTP+SSE transport.
#[derive(Clone)]
pub struct McpServerState {
    /// Bearer token for authentication (if set)
    auth_token: Option<SecretString>,

    /// Broadcast channel for server-initiated notifications
    notify_tx: broadcast::Sender<Notification>,

    /// Maximum request body size in bytes
    max_body_bytes: usize,

    /// Active SSE client count (for diagnostics)
    client_count: Arc<AtomicUsize>,

    /// Tool registry for tools/list and tools/call
    tool_registry: Arc<tools::ToolRegistry>,

    /// Root directory for path-traversal protection (canonicalized at startup)
    root: Option<PathBuf>,

    /// Audit log state
    pub audit: AuditState,
}

impl McpServerState {
    /// Create a new MCP server state.
    pub fn new(
        auth_token: Option<SecretString>,
        max_upload_mb: Option<usize>,
        root: Option<PathBuf>,
        audit_writer: Option<AuditLogWriter>,
    ) -> Self {
        let max_body_bytes = max_upload_mb.unwrap_or(DEFAULT_MAX_UPLOAD_MB) * 1024 * 1024;
        let notify_tx = broadcast::channel(100).0; // Channel size 100 for buffered notifications

        Self {
            auth_token,
            notify_tx,
            max_body_bytes,
            client_count: Arc::new(AtomicUsize::new(0)),
            tool_registry: Arc::new(tools::all_tools()),
            root,
            audit: AuditState::new(audit_writer),
        }
    }

    /// Broadcast a notification to all connected SSE clients.
    ///
    /// Returns the number of clients the notification was sent to.
    /// If no clients are connected, returns 0.
    pub fn broadcast_notification(&self, notification: Notification) -> usize {
        // recv_count is the number of receivers that got the message
        // (before it was dropped due to channel overflow or lag)
        self.notify_tx
            .send(notification)
            .map_or(0, |recv_count| recv_count)
    }

    /// Get the current number of active SSE clients.
    pub fn client_count(&self) -> usize {
        self.client_count.load(Ordering::Relaxed)
    }
}

/// Start the MCP HTTP+SSE server.
///
/// This function:
/// 1. Creates the axum router with POST /, GET /sse, GET /health
/// 2. Applies middleware (auth, compression, etc.)
/// 3. Binds to the specified address
/// 4. Runs the server until shutdown
///
/// # Arguments
/// * `bind_addr` - The bind address (e.g., "127.0.0.1:8080")
/// * `auth_token` - Optional bearer token for authentication
/// * `max_upload_mb` - Optional max upload size in MB (default 256)
/// * `root` - Optional root directory for path-traversal protection
///
/// # Returns
/// * Ok(()) when the server shuts down cleanly
/// * Err if the server fails to start or crashes
pub async fn run_server(
    bind_addr: String,
    auth_token: Option<SecretString>,
    max_upload_mb: Option<usize>,
    root: Option<&std::path::Path>,
    audit_log: Option<std::path::PathBuf>,
) -> Result<()> {
    // Create audit log writer if specified
    let audit_writer = if let Some(ref path) = audit_log {
        Some(AuditLogWriter::open(path).context(format!(
            "Failed to open audit log: {}",
            path.display()
        ))?)
    } else {
        None
    };

    // Create the shared server state
    let state = McpServerState::new(auth_token, max_upload_mb, root.map(|p| p.to_path_buf()), audit_writer);
    let max_body_bytes = state.max_body_bytes;

    // Build the router
    // Note: Set DefaultBodyLimit to a very high value (256 MB) so our handler
    // can catch oversized requests and return a proper JSON error response.
    // Our custom check in handle_post_request enforces the actual limit.
    let app = Router::new()
        .route("/", post(handle_post_request))
        .route("/sse", get(handle_sse))
        .route("/health", get(handle_health))
        .layer(axum::middleware::from_fn_with_state(
            state.audit.clone(),
            audit_middleware,
        ))
        .with_state(state)
        .layer(DefaultBodyLimit::max(256 * 1024 * 1024)) // 256 MB hard limit
        .layer(axum::middleware::from_fn(logging_middleware));

    // Resolve the bind address
    let addr = bind_addr
        .parse::<SocketAddr>()
        .with_context(|| format!("Invalid bind address: {}", bind_addr))?;

    // Create the TCP listener
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind to {}", bind_addr))?;

    eprintln!("MCP HTTP+SSE server listening on {}", bind_addr);
    eprintln!("Endpoints:");
    eprintln!("  POST /        - JSON-RPC requests");
    eprintln!("  GET  /sse     - Server-Sent Events");
    eprintln!("  GET  /health  - Health check");
    eprintln!();

    // Run the server
    axum::serve(listener, app).await.context("Server error")?;

    Ok(())
}

/// POST / handler - process JSON-RPC requests.
///
/// Accepts both single requests and batch arrays.
/// Returns a single response or batch response array.
async fn handle_post_request(
    State(state): State<McpServerState>,
    headers: HeaderMap,
    body: String,
) -> AxumResponse {
    // Check authentication first
    match check_auth(&state, &headers) {
        Ok(()) => {}
        Err(resp) => return resp,
    }

    // Check request body size via Content-Length header
    if let Some(content_length) = headers.get("content-length").and_then(|v| v.to_str().ok()) {
        if let Ok(length) = content_length.parse::<usize>() {
            if length > state.max_body_bytes {
                return payload_too_large_response(state.max_body_bytes);
            }
        }
    } else {
        // If no Content-Length header, check the actual body size
        if body.len() > state.max_body_bytes {
            return payload_too_large_response(state.max_body_bytes);
        }
    }

    // Parse the request body as either a single Request or a Batch
    let batch_result: std::result::Result<BatchMessage, _> = serde_json::from_str(&body);

    let batch = match batch_result {
        Ok(batch) => batch,
        Err(_) => {
            return error_response(StatusCode::BAD_REQUEST, ErrorObject::invalid_request());
        }
    };

    // Process each request and collect responses
    let requests = batch.into_requests();
    let mut responses = Vec::with_capacity(requests.len());
    let registry = state.tool_registry.as_ref();
    let root = state.root.as_deref();

    for request in requests {
        let response = handle_request(request, registry, root);
        responses.push(response);
    }

    // Return the response(s)
    // If it was a single request, return a single response
    // If it was a batch, return a batch response
    if responses.len() == 1 {
        Json(responses.into_iter().next().unwrap()).into_response()
    } else {
        Json(responses).into_response()
    }
}

/// GET /sse handler - server-sent events stream.
///
/// Returns a long-lived SSE connection that receives server notifications.
/// Sends a keepalive comment every 30 seconds.
async fn handle_sse(State(state): State<McpServerState>, headers: HeaderMap) -> AxumResponse {
    // Check authentication first
    match check_auth(&state, &headers) {
        Ok(()) => {}
        Err(resp) => return resp,
    }

    // Check client limit
    let client_count = state.client_count.fetch_add(1, Ordering::Relaxed) + 1;
    if client_count > MAX_SSE_CLIENTS {
        state.client_count.fetch_sub(1, Ordering::Relaxed);
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Maximum concurrent clients exceeded",
                "limit": MAX_SSE_CLIENTS,
            })),
        )
            .into_response();
    }

    // Subscribe to the broadcast channel
    let mut rx = state.notify_tx.subscribe();
    let client_count_clone = state.client_count.clone();

    // Create a stream using tokio_stream
    let stream = async_stream::stream! {
        // Send initial connection message
        yield Ok::<_, axum::Error>(axum::response::sse::Event::default()
            .comment("connected"));

        // Create a keepalive timer
        let mut keepalive = tokio::time::interval(Duration::from_secs(SSE_KEEPALIVE_SECS));

        loop {
            tokio::select! {
                // Incoming notification
                result = rx.recv() => {
                    match result {
                        Ok(notification) => {
                            // Serialize the notification as SSE data
                            let json = match serde_json::to_string(&notification) {
                                Ok(j) => j,
                                Err(e) => {
                                    tracing::error!("Failed to serialize notification: {}", e);
                                    // Send error comment and continue
                                    yield Ok::<_, axum::Error>(axum::response::sse::Event::default()
                                        .comment(&format!("serialization error: {e}")));
                                    continue;
                                }
                            };

                            yield Ok::<_, axum::Error>(axum::response::sse::Event::default()
                                .data(json));
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            // Backpressure: client couldn't keep up
                            tracing::warn!("SSE client lagged, dropped {} notifications", n);
                            yield Ok::<_, axum::Error>(axum::response::sse::Event::default()
                                .comment(&format!("lagged: dropped {n} notifications")));
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            // Channel closed (server shutting down)
                            yield Ok::<_, axum::Error>(axum::response::sse::Event::default()
                                .comment("server shutdown"));
                            break;
                        }
                    }
                }
                // Keepalive tick
                _ = keepalive.tick() => {
                    yield Ok::<_, axum::Error>(axum::response::sse::Event::default()
                        .comment("keepalive"));
                }
            }
        }

        // Decrement client count on disconnect
        client_count_clone.fetch_sub(1, Ordering::Relaxed);
    };

    // Return SSE response with appropriate headers
    Sse::new(stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(SSE_KEEPALIVE_SECS))
                .text("keepalive"),
        )
        .into_response()
}

/// GET /health handler - health check endpoint.
///
/// Always returns 200 OK with version info.
/// This endpoint is exempt from authentication.
async fn handle_health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Verify a bearer token against the configured token using constant-time comparison.
///
/// Returns true if the tokens match, false otherwise.
/// This function is pure computation with no side effects, making it suitable
/// for timing-attack resistance testing.
///
/// The comparison is constant-time with respect to both content and length:
/// - All bytes are always compared
/// - No short-circuiting on length mismatch
/// - Timing does not reveal where the first mismatch occurred
fn verify_token(provided: &str, configured: &str) -> bool {
    use subtle::Choice;

    let provided_bytes = provided.as_bytes();
    let configured_bytes = configured.as_bytes();

    // To achieve true constant-time comparison regardless of length,
    // we need to always compare the same number of bytes.
    // We use the maximum length and pad the shorter slice with zeros.
    let max_len = provided_bytes.len().max(configured_bytes.len());

    // Create extended arrays that are both the same length (max_len)
    // The shorter slice is padded with zeros at the end
    let mut provided_ext = Vec::with_capacity(max_len);
    let mut configured_ext = Vec::with_capacity(max_len);

    provided_ext.extend_from_slice(provided_bytes);
    provided_ext.resize(max_len, 0);

    configured_ext.extend_from_slice(configured_bytes);
    configured_ext.resize(max_len, 0);

    // Constant-time compare the extended arrays
    let bytes_match = provided_ext.ct_eq(&configured_ext);

    // For the tokens to be truly equal, we also need the lengths to match.
    // We compute this in constant-time using Choice.
    let lengths_match = Choice::from(u8::from(provided_bytes.len() == configured_bytes.len()));

    // Both bytes AND lengths must match
    (bytes_match & lengths_match).into()
}

/// Check bearer token authentication.
///
/// Returns Err(response) if auth fails, Ok(()) if auth passes.
/// If no auth token is configured, all requests are allowed.
///
/// Token comparison uses constant-time comparison to prevent timing attacks.
fn check_auth(
    state: &McpServerState,
    headers: &HeaderMap,
) -> std::result::Result<(), AxumResponse> {
    if let Some(token) = &state.auth_token {
        let auth_header = headers.get("Authorization").and_then(|v| v.to_str().ok());

        match auth_header {
            Some(header) if header.starts_with("Bearer ") => {
                let provided_token = &header[7..]; // Strip "Bearer "
                let configured_token = token.expose_secret();

                // Use constant-time comparison to prevent timing attacks
                if verify_token(provided_token, configured_token) {
                    Ok(())
                } else {
                    let mut response = (
                        StatusCode::UNAUTHORIZED,
                        Json(Response::error(
                            Id::Null,
                            ErrorObject::new(-32001, "Invalid authentication token"),
                        )),
                    )
                        .into_response();
                    response.headers_mut().insert(
                        "WWW-Authenticate",
                        HeaderValue::from_static("Bearer realm=\"pdftract\""),
                    );
                    Err(response)
                }
            }
            _ => {
                let mut response = (
                    StatusCode::UNAUTHORIZED,
                    Json(Response::error(
                        Id::Null,
                        ErrorObject::new(-32001, "Missing authentication token"),
                    )),
                )
                    .into_response();
                response.headers_mut().insert(
                    "WWW-Authenticate",
                    HeaderValue::from_static("Bearer realm=\"pdftract\""),
                );
                Err(response)
            }
        }
    } else {
        Ok(())
    }
}

/// Handle a single JSON-RPC request and return a response.
fn handle_request(
    request: Request,
    registry: &tools::ToolRegistry,
    root: Option<&std::path::Path>,
) -> Response {
    let id = request.request_id();

    match request.method.as_str() {
        "tools/list" => {
            let tools = registry.tools_list();
            Response::success(id, tools)
        }
        "initialize" => {
            let result = serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {},
                    "resources": {},
                    "prompts": {}
                },
                "serverInfo": {
                    "name": "pdftract",
                    "version": env!("CARGO_PKG_VERSION")
                }
            });
            Response::success(id, result)
        }
        "tools/call" => {
            // Extract tool name and arguments from params
            let params = match request.params {
                Some(p) => p,
                None => {
                    return Response::error(
                        id,
                        ErrorObject::invalid_params()
                            .with_data(json!({"reason": "Missing params"})),
                    );
                }
            };

            let tool_name = match params.get("name").and_then(|v| v.as_str()) {
                Some(name) => name,
                None => {
                    return Response::error(
                        id,
                        ErrorObject::invalid_params()
                            .with_data(json!({"reason": "Missing or invalid 'name' field"})),
                    );
                }
            };

            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or(Value::Object(serde_json::Map::new()));

            // Look up the tool in the registry
            let tool = match registry.get(tool_name) {
                Some(t) => t,
                None => {
                    return Response::error(id, ErrorObject::method_not_found(tool_name));
                }
            };

            // Execute the tool with observability logging
            let start = std::time::Instant::now();
            let log_path = arguments
                .get("path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let result = tool.execute(arguments, log_path.as_deref(), root);

            let duration_ms = start.elapsed().as_millis();
            let response_size = result
                .as_ref()
                .ok()
                .map(|v| serde_json::to_vec(v).unwrap_or_default().len())
                .unwrap_or(0);

            // Emit structured log line to stderr
            // Format: timestamp, tool_name, path (or hash), duration_ms, response_size_bytes, error_code
            let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            let path_or_hash = log_path.unwrap_or_else(|| "<unknown>".to_string());
            let error_code = result.as_ref().err().map(|e| e.code.to_string());

            eprintln!(
                "{} tool={} path={} duration_ms={} response_size_bytes={} error_code={:?}",
                timestamp, tool_name, path_or_hash, duration_ms, response_size, error_code,
            );

            match result {
                Ok(value) => Response::success(id, value),
                Err(error) => Response::error(id, error),
            }
        }
        _ => {
            tracing::warn!("Unknown MCP method: {}", request.method);
            Response::error(id, ErrorObject::method_not_found(&request.method))
        }
    }
}

/// Create an error response with the given status code and error object.
fn error_response(status: StatusCode, error: ErrorObject) -> AxumResponse {
    (status, Json(Response::error(Id::Null, error))).into_response()
}

/// Create a 413 Payload Too Large response with custom JSON body.
fn payload_too_large_response(max_bytes: usize) -> AxumResponse {
    let max_mb = max_bytes / (1024 * 1024);
    let error_json = serde_json::json!({
        "jsonrpc": "2.0",
        "error": {
            "code": -32002,
            "message": format!("Request body too large (maximum {} MB)", max_mb),
            "data": {
                "limit_bytes": max_bytes,
                "limit_mb": max_mb
            }
        },
        "id": null
    });
    (StatusCode::PAYLOAD_TOO_LARGE, Json(error_json)).into_response()
}

/// Logging middleware for all HTTP requests.
///
/// Logs the method, path, and response status for each request.
async fn logging_middleware(
    req: AxumRequest,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = req.method().clone();
    let uri = req.uri().clone();

    let response = next.run(req).await;

    let status = response.status();
    tracing::info!("{} {} -> {}", method, uri, status);

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_server_state_creation() {
        let token = SecretString::new("test-token".into());
        let state = McpServerState::new(Some(token), Some(10), None, None);

        assert_eq!(state.max_body_bytes, 10 * 1024 * 1024);
        assert_eq!(state.client_count(), 0);
        assert!(state.auth_token.is_some());
    }

    #[test]
    fn test_mcp_server_state_no_token() {
        let state = McpServerState::new(None, None, None, None);

        assert_eq!(state.max_body_bytes, DEFAULT_MAX_UPLOAD_MB * 1024 * 1024);
        assert_eq!(state.client_count(), 0);
        assert!(state.auth_token.is_none());
    }

    #[test]
    fn test_mcp_server_state_broadcast() {
        let state = McpServerState::new(None, None, None, None);
        let notification = Notification::new("test/notification", None);

        // Broadcast with no clients should return 0
        let count = state.broadcast_notification(notification);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_handle_request_tools_list() {
        let registry = tools::all_tools();
        let request = Request::new("tools/list", None, Some(Id::Number(1)));
        let response = handle_request(request, &registry, None);

        assert!(response.is_success());
        assert!(response.get_result().is_some());
    }

    #[test]
    fn test_handle_request_initialize() {
        let registry = tools::all_tools();
        let request = Request::new("initialize", None, Some(Id::Number(1)));
        let response = handle_request(request, &registry, None);

        assert!(response.is_success());
        let result = response.get_result().unwrap();
        assert!(result.get("protocolVersion").is_some());
        assert!(result.get("serverInfo").is_some());
    }

    #[test]
    fn test_handle_request_unknown_method() {
        let registry = tools::all_tools();
        let request = Request::new("unknown/method", None, Some(Id::Number(1)));
        let response = handle_request(request, &registry, None);

        assert!(response.is_error());
        let error = response.get_error().unwrap();
        assert_eq!(error.code, -32601);
    }

    #[test]
    fn test_error_response() {
        let error = ErrorObject::invalid_params();
        let response = error_response(StatusCode::BAD_REQUEST, error);

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_check_auth_no_token_configured() {
        let state = McpServerState::new(None, None, None, None);
        let mut headers = HeaderMap::new();

        // No token configured, so any headers should pass
        assert!(check_auth(&state, &headers).is_ok());

        headers.insert(
            "Authorization",
            HeaderValue::from_static("Bearer irrelevant"),
        );
        assert!(check_auth(&state, &headers).is_ok());
    }

    #[test]
    fn test_check_auth_valid_token() {
        let token = SecretString::new("correct-token".into());
        let state = McpServerState::new(Some(token), None, None, None);
        let mut headers = HeaderMap::new();

        headers.insert(
            "Authorization",
            HeaderValue::from_static("Bearer correct-token"),
        );
        assert!(check_auth(&state, &headers).is_ok());
    }

    #[test]
    fn test_check_auth_invalid_token() {
        let token = SecretString::new("correct-token".into());
        let state = McpServerState::new(Some(token), None, None, None);
        let mut headers = HeaderMap::new();

        headers.insert(
            "Authorization",
            HeaderValue::from_static("Bearer wrong-token"),
        );
        let result = check_auth(&state, &headers);
        assert!(result.is_err());
        if let Err(resp) = result {
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }
    }

    #[test]
    fn test_check_auth_missing_token() {
        let token = SecretString::new("correct-token".into());
        let state = McpServerState::new(Some(token), None, None, None);
        let headers = HeaderMap::new();

        let result = check_auth(&state, &headers);
        assert!(result.is_err());
        if let Err(resp) = result {
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
            assert!(resp.headers().get("WWW-Authenticate").is_some());
        }
    }

    #[test]
    fn test_check_auth_malformed_header() {
        let token = SecretString::new("correct-token".into());
        let state = McpServerState::new(Some(token), None, None, None);
        let mut headers = HeaderMap::new();

        // Missing "Bearer " prefix
        headers.insert("Authorization", HeaderValue::from_static("correct-token"));
        let result = check_auth(&state, &headers);
        assert!(result.is_err());
    }

    /// Timing-attack test: verifies that token comparison is constant-time.
    ///
    /// This test makes many comparisons with different token lengths and compares
    /// the timing variance. A non-constant-time comparison would show a
    /// significant difference in timing between tokens that mismatch early
    /// versus tokens that mismatch late.
    ///
    /// This is a statistical test that may occasionally fail due to system
    /// noise, so we use a relatively loose threshold (5x variance allowed).
    #[test]
    fn test_check_auth_constant_time() {
        use std::time::Instant;

        let correct_token = "correct-token-32-bytes-long!";

        // Test 1: Token that mismatches at the first character
        let token_early = "Xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";

        // Test 2: Token that mismatches at the last character
        let token_late = "correct-token-32-bytes-long?";

        // Test 3: Correct token (should return true)
        let token_correct = "correct-token-32-bytes-long!";

        let iterations = 1000;
        let mut times_early = Vec::with_capacity(iterations);
        let mut times_late = Vec::with_capacity(iterations);
        let mut times_correct = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            let start = Instant::now();
            let _ = verify_token(token_early, correct_token);
            times_early.push(start.elapsed());

            let start = Instant::now();
            let _ = verify_token(token_late, correct_token);
            times_late.push(start.elapsed());

            let start = Instant::now();
            let _ = verify_token(token_correct, correct_token);
            times_correct.push(start.elapsed());
        }

        // Calculate median times to reduce noise impact
        let mut sorted_early = times_early.clone();
        let mut sorted_late = times_late.clone();
        let mut sorted_correct = times_correct.clone();

        sorted_early.sort();
        sorted_late.sort();
        sorted_correct.sort();

        let median_early = sorted_early[iterations / 2];
        let median_late = sorted_late[iterations / 2];
        let median_correct = sorted_correct[iterations / 2];

        // For constant-time comparison, all three should have similar timing
        // We allow up to 5x variance to account for system noise
        let max_time = median_early.max(median_late).max(median_correct);
        let min_time = median_early.min(median_late).min(median_correct);

        let ratio = if min_time.as_nanos() > 0 {
            max_time.as_nanos() / min_time.as_nanos()
        } else {
            1 // Both are essentially zero
        };

        // Assert that timing variance is within acceptable bounds
        // If this fails, the comparison is likely not constant-time
        assert!(
            ratio <= 5,
            "Token comparison appears to be non-constant-time: \
             early mismatch={:?}, late mismatch={:?}, correct={:?}, ratio={}",
            median_early,
            median_late,
            median_correct,
            ratio
        );

        // Also verify that the correct token actually returns true
        assert!(verify_token(token_correct, correct_token));
        assert!(!verify_token(token_early, correct_token));
        assert!(!verify_token(token_late, correct_token));
    }

    /// Test that tokens of different lengths have constant-time comparison.
    ///
    /// A naive string comparison would short-circuit on length mismatch,
    /// which is a timing leak. This test verifies that our implementation
    /// does not have this leak.
    #[test]
    fn test_check_auth_constant_time_different_lengths() {
        use std::time::Instant;

        let token = SecretString::new("correct-token-32-bytes-long!".into());
        let state = McpServerState::new(Some(token), None, None, None);

        // Test 1: Token that is much shorter
        let mut headers_short = HeaderMap::new();
        headers_short.insert("Authorization", HeaderValue::from_static("Bearer short"));

        // Test 2: Token that is much longer
        let mut headers_long = HeaderMap::new();
        headers_long.insert(
            "Authorization",
            HeaderValue::from_static("Bearer this-token-is-much-longer-than-the-correct-one"),
        );

        let iterations = 1000;
        let mut times_short = Vec::with_capacity(iterations);
        let mut times_long = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            let start = Instant::now();
            let _ = check_auth(&state, &headers_short);
            times_short.push(start.elapsed());

            let start = Instant::now();
            let _ = check_auth(&state, &headers_long);
            times_long.push(start.elapsed());
        }

        // Calculate median times
        let mut sorted_short = times_short.clone();
        let mut sorted_long = times_long.clone();
        sorted_short.sort();
        sorted_long.sort();

        let median_short = sorted_short[iterations / 2];
        let median_long = sorted_long[iterations / 2];

        // For constant-time comparison, different lengths should have similar timing
        // We allow up to 3x variance for length differences (implementation-dependent)
        let ratio = if median_short.as_nanos() > 0 && median_long.as_nanos() > 0 {
            let max = median_short.max(median_long);
            let min = median_short.min(median_long);
            max.as_nanos() / min.as_nanos()
        } else {
            1
        };

        assert!(
            ratio <= 3,
            "Token comparison appears to leak length information: \
             short={:?}, long={:?}, ratio={}",
            median_short,
            median_long,
            ratio
        );
    }
}
