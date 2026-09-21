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
use crate::middleware::audit::RequestMetadata;
use crate::middleware::{audit_middleware, AuditState};
use anyhow::{Context, Result};
use axum::{
    extract::{DefaultBodyLimit, Extension, Request as AxumRequest, State},
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

    /// Metrics registry shared by this server's handlers (`metrics`
    /// feature). Created at startup, so `pdftract_build_info` is
    /// populated for the process's whole lifetime.
    #[cfg(feature = "metrics")]
    metrics: crate::metrics::Registry,
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
            #[cfg(feature = "metrics")]
            metrics: crate::metrics::Registry::new(),
        }
    }

    /// The metrics registry handle (`metrics` feature).
    #[cfg(feature = "metrics")]
    pub fn metrics(&self) -> &crate::metrics::Registry {
        &self.metrics
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
/// * `metrics_port` - Optional port for a SECOND listener serving
///   `GET /metrics` (OpenMetrics v1.0) on --bind's interface; never on
///   the main port. Bound before the main listener, so a port that is
///   already in use fails startup cleanly.
/// * `auth_token` - Optional bearer token for authentication
/// * `max_upload_mb` - Optional max upload size in MB (default 256)
/// * `root` - Optional root directory for path-traversal protection
///
/// # Returns
/// * Ok(()) when the server shuts down cleanly
/// * Err if the server fails to start or crashes
pub async fn run_server(
    bind_addr: String,
    metrics_port: Option<u16>,
    auth_token: Option<SecretString>,
    max_upload_mb: Option<usize>,
    root: Option<&std::path::Path>,
    audit_log: Option<std::path::PathBuf>,
) -> Result<()> {
    // Create audit log writer if specified
    let audit_writer = if let Some(ref path) = audit_log {
        Some(
            AuditLogWriter::open(path)
                .context(format!("Failed to open audit log: {}", path.display()))?,
        )
    } else {
        None
    };

    // Create the shared server state
    let state = McpServerState::new(
        auth_token,
        max_upload_mb,
        root.map(|p| p.to_path_buf()),
        audit_writer,
    );

    // Sample `pdftract_rayon_pool_utilization` periodically for the whole
    // lifetime of the server (`metrics` feature).
    #[cfg(feature = "metrics")]
    {
        let sampler_registry = state.metrics().clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(std::time::Duration::from_secs(5));
            loop {
                ticker.tick().await;
                sampler_registry
                    .set_rayon_pool_utilization(crate::metrics::sampler::sample_utilization());
            }
        });
    }

    // Bind the `--metrics PORT` exposition listener BEFORE the main
    // listener (`metrics` feature): a port that cannot be bound must fail
    // startup cleanly here — with a clean error and a nonzero exit —
    // before anything is served. `/metrics` exists only on that second
    // listener; the main router below gains no metrics route (plan
    // "Monitoring and Alerting" endpoint policy).
    #[cfg(feature = "metrics")]
    if let Some(port) = metrics_port {
        let metrics_addr = crate::metrics::listener_addr(&bind_addr, port)?;
        let registry = state.metrics().clone();
        crate::metrics::bind_and_spawn(&metrics_addr, registry).await?;
    }
    #[cfg(not(feature = "metrics"))]
    if metrics_port.is_some() {
        anyhow::bail!(
            "--metrics requires a build with the `metrics` cargo feature \
             (rebuild with --features metrics)"
        );
    }

    let app = build_router(state);

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

    // Run the server. The router stack (audit log + metrics middleware)
    // extracts `ConnectInfo<SocketAddr>`, so the make service must supply
    // it — a bare Router serve 500s every request with "Missing request
    // extension".
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .context("Server error")?;

    Ok(())
}

/// Build the MCP HTTP+SSE application router (shared by [`run_server`]
/// and tests).
///
/// Layer order, outermost first: metrics counting (`metrics` feature),
/// request logging, body limit, audit. The metrics layer sits outermost
/// so it observes the response actually sent.
fn build_router(state: McpServerState) -> Router {
    #[cfg(feature = "metrics")]
    let metrics_registry = state.metrics().clone();

    // Note: Set DefaultBodyLimit to a very high value (256 MB) so our handler
    // can catch oversized requests and return a proper JSON error response.
    // Our custom check in handle_post_request enforces the actual limit.
    let router = Router::new()
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

    // Count every response in `pdftract_http_requests_total`. Observation
    // only — no new route.
    #[cfg(feature = "metrics")]
    let router = router.layer(axum::middleware::from_fn_with_state(
        metrics_registry,
        metrics_http_middleware,
    ));

    router
}

/// Map a request path to its registered route template for the `endpoint`
/// label. Unregistered paths collapse to `"unmatched"` — the label never
/// carries a raw request path (plan cardinality policy).
#[cfg(feature = "metrics")]
fn endpoint_label(path: &str) -> &'static str {
    match path {
        "/" => "/",
        "/health" => "/health",
        "/sse" => "/sse",
        _ => "unmatched",
    }
}

/// Count one HTTP response in `pdftract_http_requests_total`
/// (`metrics` feature).
#[cfg(feature = "metrics")]
async fn metrics_http_middleware(
    State(registry): State<crate::metrics::Registry>,
    req: AxumRequest,
    next: axum::middleware::Next,
) -> AxumResponse {
    let endpoint = endpoint_label(req.uri().path());
    let response = next.run(req).await;
    registry.inc_http_request(endpoint, response.status().as_u16());
    response
}

/// The tool name a `tools/call` request invokes, if well-formed.
///
/// Used for `pdftract_mcp_requests_total`; the invocation is counted by
/// its requested name, including names absent from the registry (the
/// client did invoke that tool; it gets a method-not-found error back).
#[cfg(feature = "metrics")]
fn tool_call_name(request: &Request) -> Option<String> {
    if request.method != "tools/call" {
        return None;
    }
    request
        .params
        .as_ref()?
        .get("name")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

/// Count the diagnostics an mcp tool result reports (`metrics` feature).
///
/// Extraction tools embed the document's structured diagnostics in their
/// result at `metadata.diagnostics_detailed`; each entry feeds
/// `pdftract_diagnostic_emitted_total` with the same code/severity
/// labels the serve path's `record_extraction` uses. Observation only:
/// error responses, results without the array, and unparseable entries
/// count nothing, and the response is never modified.
#[cfg(feature = "metrics")]
fn record_response_diagnostics(metrics: &crate::metrics::Registry, response: &Response) {
    let Some(result) = response.get_result() else {
        return;
    };
    let Some(entries) = result
        .get("metadata")
        .and_then(|metadata| metadata.get("diagnostics_detailed"))
    else {
        return;
    };
    let Ok(diagnostics) =
        serde_json::from_value::<Vec<pdftract_core::schema::DiagnosticJson>>(entries.clone())
    else {
        return;
    };
    for diagnostic in &diagnostics {
        metrics.inc_diagnostic(&diagnostic.code, &diagnostic.severity);
    }
}

/// POST / handler - process JSON-RPC requests.
///
/// Accepts both single requests and batch arrays.
/// Returns a single response or batch response array.
async fn handle_post_request(
    State(state): State<McpServerState>,
    Extension(metadata): Extension<RequestMetadata>,
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
        // Count the tool invocation (`metrics` feature).
        #[cfg(feature = "metrics")]
        if let Some(tool_name) = tool_call_name(&request) {
            state.metrics.inc_mcp_request(&tool_name);
        }
        let response = handle_request(request, registry, root);
        // Count diagnostics the tool reported (`metrics` feature).
        #[cfg(feature = "metrics")]
        record_response_diagnostics(&state.metrics, &response);
        responses.push(response);
    }

    // Write audit log if configured
    if let Some(ref writer) = state.audit.writer {
        let duration_ms = metadata.start_time.elapsed().as_millis() as u64;

        // For batch requests, we log the batch as a single entry
        // For single requests, we log one entry
        // The tool name is the first request's method (or "mcp.batch" for batches)
        let tool_name = if responses.len() == 1 {
            // For single request, get the method from the response if it's a tools/call
            // Otherwise use the metadata tool from the URL path
            metadata.tool.clone()
        } else {
            "mcp.batch".to_string()
        };

        // Determine status: 200 if all responses are success, 500 if any error
        let status = if responses.iter().all(|r| r.is_success()) {
            200
        } else {
            500
        };

        // Collect diagnostics from all error responses
        let diagnostics: Vec<String> = responses
            .iter()
            .filter_map(|r| r.get_error())
            .map(|e| e.code.to_string())
            .collect();

        let _ = writer.log(
            &tool_name,
            metadata.client_ip.as_deref(),
            None, // No fingerprint available at MCP layer (PDF bytes not directly exposed)
            duration_ms,
            status,
            &diagnostics,
        );
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

/// Redact sensitive headers from a HeaderMap for logging.
///
/// Returns a comma-separated string of header names with "[REDACTED]" placeholders
/// for sensitive headers (Authorization, Cookie, Proxy-Authorization).
fn redact_headers_for_log(headers: &HeaderMap) -> String {
    let mut redacted = Vec::new();

    for (name, _) in headers.iter() {
        let name_str = name.as_str();
        match name_str {
            "authorization" | "cookie" | "proxy-authorization" => {
                redacted.push(format!("{}=[REDACTED]", name_str));
            }
            _ => {
                redacted.push(format!("{}=[...]", name_str));
            }
        }
    }

    redacted.join(", ")
}

/// Logging middleware for all HTTP requests.
///
/// Logs the method, path, response status, and headers (with sensitive values redacted).
async fn logging_middleware(
    req: AxumRequest,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let headers = req.headers().clone();
    let redacted_headers = redact_headers_for_log(&headers);

    let response = next.run(req).await;

    let status = response.status();
    tracing::info!(
        "{} {} -> {} | Headers: {}",
        method,
        uri,
        status,
        redacted_headers
    );

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

    /// Metrics: a `tools/call` driven through the HTTP router counts in
    /// `pdftract_mcp_requests_total` with the tool label, and the HTTP
    /// response counts under the "/" endpoint label.
    #[cfg(feature = "metrics")]
    #[tokio::test]
    async fn test_metrics_registry_counts_tool_call_and_http() {
        use axum::body::Body;
        use tower::ServiceExt;

        let state = McpServerState::new(None, None, None, None);
        let metrics = state.metrics().clone();
        // The audit middleware extracts ConnectInfo, which a bare router
        // has no source for under `oneshot` (every extractor rejection
        // surfaces as a 500 before any handler runs), so the peer
        // address the live server would provide is mocked here.
        // Test-only — production routing is untouched.
        let app = build_router(state).layer(axum::extract::connect_info::MockConnectInfo(
            std::net::SocketAddr::from(([127, 0, 0, 1], 4242)),
        ));

        let pdf_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/test-minimal.pdf"
        );
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "hash",
                "arguments": {"path": pdf_path}
            }
        });
        let request = AxumRequest::builder()
            .method("POST")
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let text = metrics.render();
        assert!(
            text.contains("pdftract_mcp_requests_total{tool=\"hash\"} 1\n"),
            "missing mcp tool counter:\n{text}"
        );
        assert!(
            text.contains("pdftract_http_requests_total{endpoint=\"/\",status=\"200\"} 1\n"),
            "missing mcp http counter:\n{text}"
        );
        assert!(
            text.contains("pdftract_build_info{version=\""),
            "build_info missing:\n{text}"
        );
    }

    /// Metrics: the diagnostics an mcp tool result reports at
    /// `metadata.diagnostics_detailed` feed
    /// `pdftract_diagnostic_emitted_total` with code/severity labels;
    /// error responses and results without the array count nothing.
    #[cfg(feature = "metrics")]
    #[test]
    fn test_metrics_record_response_diagnostics_counts_tool_result() {
        let registry = crate::metrics::Registry::new();

        // A successful extract-shaped result with two diagnostics.
        let success = Response::success(
            Id::Number(1),
            json!({
                "metadata": {
                    "diagnostics_detailed": [
                        {"code": "STREAM_BOMB", "message": "m", "severity": "error"},
                        {"code": "STRUCT_MISSING_KEY", "message": "m", "severity": "warn"}
                    ]
                }
            }),
        );
        record_response_diagnostics(&registry, &success);

        // An error response and a result without the array: no counts.
        record_response_diagnostics(
            &registry,
            &Response::error(Id::Number(2), ErrorObject::invalid_params()),
        );
        record_response_diagnostics(
            &registry,
            &Response::success(Id::Number(3), json!({"text": "no diagnostics here"})),
        );

        let text = registry.render();
        assert!(
            text.contains(
                "pdftract_diagnostic_emitted_total{code=\"STREAM_BOMB\",severity=\"error\"} 1\n"
            ),
            "missing STREAM_BOMB counter:\n{text}"
        );
        assert!(
            text.contains(
                "pdftract_diagnostic_emitted_total{code=\"STRUCT_MISSING_KEY\",severity=\"warn\"} 1\n"
            ),
            "missing STRUCT_MISSING_KEY counter:\n{text}"
        );
    }

    /// Metrics: a `tools/call` to `extract` driven through the HTTP
    /// router feeds `pdftract_diagnostic_emitted_total` on the live
    /// path — the rendered counters must mirror exactly what the
    /// response body reports.
    ///
    /// At the current HEAD no fixture emits `diagnostics_detailed`
    /// through extraction (the engine's diagnostics pipeline is in a
    /// known degraded window; every probed fixture either errors or
    /// extracts with zero diagnostics), so the mirrored count is zero
    /// today. The unit test above pins the non-zero label rendering;
    /// this test pins that the real `handle_post_request` loop feeds
    /// the helper with the real response: the loop runs (tool counter),
    /// nothing phantom is counted (no non-zero diagnostic lines), and
    /// the rendered total agrees with the body.
    #[cfg(feature = "metrics")]
    #[tokio::test]
    async fn test_metrics_tool_call_diagnostics_counted_from_extract_result() {
        use axum::body::Body;
        use tower::ServiceExt;

        let state = McpServerState::new(None, None, None, None);
        let metrics = state.metrics().clone();
        let app = build_router(state).layer(axum::extract::connect_info::MockConnectInfo(
            std::net::SocketAddr::from(([127, 0, 0, 1], 4242)),
        ));

        // Extracts successfully at HEAD (see the serve-path metrics
        // tests, which assert pages from this same fixture).
        let pdf_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/test-minimal.pdf"
        );
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "extract",
                "arguments": {"path": pdf_path}
            }
        });
        let request = AxumRequest::builder()
            .method("POST")
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1 << 20)
            .await
            .unwrap();
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();

        // What the client received, as (code, severity) -> count.
        let mut reported: std::collections::BTreeMap<(String, String), usize> =
            std::collections::BTreeMap::new();
        if let Some(entries) = response["result"]["metadata"]["diagnostics_detailed"].as_array() {
            for diagnostic in entries {
                let key = (
                    diagnostic["code"].as_str().unwrap().to_string(),
                    diagnostic["severity"].as_str().unwrap().to_string(),
                );
                *reported.entry(key).or_insert(0) += 1;
            }
        }

        let text = metrics.render();

        // The tool loop ran and the invocation is counted.
        assert!(
            text.contains("pdftract_mcp_requests_total{tool=\"extract\"} 1\n"),
            "extract tool call not counted:\n{text}"
        );

        // Every (code, severity) the response reported is rendered with
        // exactly the reported count, and nothing else was counted.
        let rendered_nonzero = text
            .lines()
            .filter(|l| l.starts_with("pdftract_diagnostic_emitted_total{"))
            .filter(|l| !l.ends_with(" 0"))
            .collect::<Vec<_>>();
        if reported.is_empty() {
            assert!(
                rendered_nonzero.is_empty(),
                "diagnostics counted that the response did not report: {rendered_nonzero:?}"
            );
        } else {
            for ((code, severity), count) in &reported {
                let line = format!(
                    "pdftract_diagnostic_emitted_total{{code=\"{code}\",severity=\"{severity}\"}} {count}\n"
                );
                assert!(
                    text.contains(&line),
                    "missing or mismatched counter line {line}; rendered:\n{text}"
                );
            }
            assert_eq!(
                rendered_nonzero.len(),
                reported.len(),
                "phantom diagnostic counters; rendered:\n{text}"
            );
        }
    }
}
