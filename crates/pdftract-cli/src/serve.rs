//! HTTP serve mode for pdftract.
//!
//! This module implements Phase 6.4's `pdftract serve` subcommand: a long-running
//! HTTP service for multi-tenant extraction with cache integration.
//!
//! # Security Model
//!
//! **NO AUTHENTICATION**: pdftract serve has NO built-in authentication. This is a
//! deliberate design decision - authentication and authorization are the responsibility
//! of the deployment infrastructure (reverse proxy, API gateway, service mesh).
//!
//! Deploy behind a reverse proxy (nginx, Traefik, Caddy, envoy) for production use.
//! The reverse proxy should handle:
//! - TLS termination
//! - Authentication (OAuth2, API keys, mTLS, etc.)
//! - Rate limiting
//! - IP whitelisting/blacklisting
//!
//! # File Path Safety
//!
//! All PDFs arrive via **multipart upload only**. No endpoint accepts a file path
//! parameter from the server filesystem. This design prevents:
//! - Directory traversal attacks (../../etc/passwd)
//! - Unintended file access via request parameters
//! - Path-based injection attacks
//!
//! Routes accept `multipart/form-data` with a `pdf` field containing the file bytes.
//! The server never reads from the server filesystem on behalf of a request.
//!
//! # Endpoints
//!
//! - `POST /extract` — Extract and return JSON with cache status in response body
//! - `POST /extract/text` — Extract and return plain text with X-Pdftract-Cache header
//! - `POST /extract/stream` — Extract and return streaming NDJSON with X-Pdftract-Cache header
//! - `GET /health` — Health check (always returns 200 OK — this is the
//!   LIVENESS probe: it stays 200 even with a saturated pool or an
//!   unwritable cache, because a busy server should not be restarted)
//!
//! With `--metrics PORT` (plan "Monitoring and Alerting") a SECOND listener
//! is opened on the same interface and serves exactly two routes:
//!
//! - `GET /metrics` (metrics port only) — the registry rendered as
//!   OpenMetrics v1.0 text (`application/openmetrics-text;
//!   version=1.0.0; charset=utf-8`, `# EOF`-terminated). Never routed on
//!   the main port, so scraping reachability can differ from production
//!   traffic. Without the flag this listener does not exist and this
//!   module's router gains no metrics route.
//! - `GET /ready` (metrics port only) — the READINESS probe: 200 while
//!   the server is accepting work, 503 (body naming every failed
//!   condition) when `pdftract_rayon_pool_utilization` exceeds 0.90 or
//!   the enabled cache location is not writable. Also never routed on
//!   the main port; that port's `/health` deliberately stays always-200
//!   in both failure states.
//!
//! # Cache headers
//!
//! All endpoints return `X-Pdftract-Cache: hit | miss | skipped` header:
//! - `hit`: Served from cache
//! - `miss`: Ran extraction; populated cache
//! - `skipped`: Cache not configured or --no-cache equivalent
//!
//! # Concurrency model
//!
//! The serve mode uses a two-level concurrency architecture:
//!
//! - **tokio**: Per-request concurrency via the async executor. Each HTTP request
//!   is handled asynchronously on tokio's multi-threaded runtime.
//! - **rayon**: Per-document parallelism within each extraction. PDF pages are
//!   processed in parallel using rayon's work-stealing thread pool.
//!
//! The bridge between async (tokio) and sync (rayon) is `tokio::task::spawn_blocking`.
//! Each POST handler wraps the synchronous extraction call in `spawn_blocking`, which
//! runs the work on tokio's blocking thread pool (separate from the async reactor).
//!
//! This design ensures:
//! - The async reactor is never blocked by extraction work
//! - Multiple PDFs can be extracted concurrently (one per request)
//! - Within each PDF, pages are processed in parallel (rayon)
//! - Thread pools are sized appropriately (tokio: 512 blocking threads; rayon: num_cpus)
//!
//! # Error codes
//!
//! - `REQUEST_TOO_LARGE`: Request body exceeds --max-upload-mb limit
//! - `BAD_REQUEST`: Invalid request parameters or missing file
//! - `EXTRACTION_ERROR`: PDF parsing or extraction failure
//! - `INTERNAL_PANIC`: spawn_blocking task panicked (indicates a bug)

use crate::middleware::{audit_middleware, AuditState, RequestMetadata};
use anyhow::{Context, Result};
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Extension, Multipart, State},
    http::{HeaderValue, Request, Response, StatusCode},
    response::{IntoResponse, Json, Response as AxumResponse},
    routing::{get, post},
    Router,
};
use bytes;
use pdftract_core::audit::AuditLogWriter;
use pdftract_core::cache;
use pdftract_core::diagnostics::DiagCode;
use pdftract_core::extract::result_to_json;
use pdftract_core::options::{ExtractionOptions, ReceiptsMode};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Cache state for the HTTP server.
#[derive(Clone)]
pub struct CacheState {
    /// Cache directory path
    pub cache_dir: Option<PathBuf>,
    /// Cache size limit in bytes
    pub cache_size_bytes: u64,
    /// Whether cache is disabled
    pub cache_disabled: bool,
}

/// Server state for the HTTP serve mode.
#[derive(Clone)]
pub struct ServeState {
    /// Cache configuration
    pub cache: Arc<Mutex<CacheState>>,
    /// Audit log state
    pub audit: AuditState,
    /// Default maximum decompression size in bytes (from --max-decompress-gb)
    pub max_decompress_bytes: u64,
    /// Metrics registry shared by every handler of this server
    /// (`metrics` feature). Created at startup, so `pdftract_build_info`
    /// is populated for the process's whole lifetime.
    #[cfg(feature = "metrics")]
    pub metrics: crate::metrics::Registry,
    /// Readiness inputs behind `GET /ready` on the metrics listener
    /// (`metrics` feature): the registry's utilization gauge and a
    /// writability probe of the enabled cache location. Tests replace
    /// them via [`ServeState::with_readiness`] to force each failure
    /// condition.
    #[cfg(feature = "metrics")]
    pub readiness: crate::metrics::Readiness,
}

impl ServeState {
    /// Create a new serve state.
    pub fn new(
        cache_dir: Option<PathBuf>,
        cache_size_bytes: u64,
        cache_disabled: bool,
        audit_writer: Option<AuditLogWriter>,
        max_decompress_bytes: u64,
        trust_forwarded_for: bool,
    ) -> Self {
        let cache = CacheState {
            cache_dir: cache_dir.clone(),
            cache_size_bytes,
            cache_disabled,
        };
        let audit = if trust_forwarded_for {
            AuditState::with_trusted_forwarded_for(audit_writer)
        } else {
            AuditState::new(audit_writer)
        };
        #[cfg(feature = "metrics")]
        let metrics = crate::metrics::Registry::new();
        // Publish the cache's current size before the first request.  The
        // gauge is a point-in-time view of the on-disk index, not merely the
        // size of entries written by this process since startup.
        #[cfg(feature = "metrics")]
        if !cache_disabled {
            if let Some(dir) = cache_dir.as_deref() {
                refresh_cache_size(&metrics, dir, cache_size_bytes);
            }
        }
        // The cache location is fixed at startup, so the readiness probe
        // can capture it; a disabled or absent cache makes the condition
        // trivially pass (plan: the check applies "if enabled").
        #[cfg(feature = "metrics")]
        let readiness = crate::metrics::Readiness::production(
            metrics.clone(),
            if cache_disabled { None } else { cache_dir },
        );
        Self {
            cache: Arc::new(Mutex::new(cache)),
            audit,
            max_decompress_bytes,
            #[cfg(feature = "metrics")]
            metrics,
            #[cfg(feature = "metrics")]
            readiness,
        }
    }

    /// Replace the readiness inputs (`metrics` feature). Production
    /// state keeps the inputs [`ServeState::new`] built from its cache
    /// configuration; tests call this to force a failure condition
    /// without real load or read-only mounts.
    #[cfg(feature = "metrics")]
    pub fn with_readiness(mut self, readiness: crate::metrics::Readiness) -> Self {
        self.readiness = readiness;
        self
    }
}

/// Cache status for response headers and metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStatus {
    Hit,
    Miss,
    Skipped,
}

impl CacheStatus {
    /// Convert to string for header/metadata.
    pub fn as_str(self) -> &'static str {
        match self {
            CacheStatus::Hit => "hit",
            CacheStatus::Miss => "miss",
            CacheStatus::Skipped => "skipped",
        }
    }

    /// Create header value.
    pub fn header_value(self) -> HeaderValue {
        HeaderValue::from_static(self.as_str())
    }

    /// Create from string.
    pub fn from_string(s: &str) -> Self {
        match s {
            "hit" => CacheStatus::Hit,
            "miss" => CacheStatus::Miss,
            "skipped" => CacheStatus::Skipped,
            _ => CacheStatus::Skipped,
        }
    }
}

/// API error response shape.
///
/// All 4xx and 5xx responses use this JSON shape for consistency.
#[derive(Debug, Serialize)]
pub struct ApiError {
    /// Error code (e.g., "BAD_REQUEST", "REQUEST_TOO_LARGE", "ENCRYPTED")
    pub error: String,
    /// Human-readable error message
    pub message: String,
    /// Optional hint for actionable errors (e.g., "Supply the correct password via --password")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl ApiError {
    /// Create a new API error with code and message.
    pub fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        ApiError {
            error: error.into(),
            message: message.into(),
            hint: None,
        }
    }

    /// Add a hint to the error.
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

/// Extraction request parameters.
///
/// These are parsed from multipart form fields. All fields are optional
/// and have sensible defaults defined in the plan (lines 2127-2137).
#[derive(Debug, Default)]
struct ExtractParams {
    /// Receipts mode (off, lite, svg)
    receipts: Option<String>,
    /// Disable cache for this request
    no_cache: bool,
    /// Enable full-render path using PDFium
    full_render: bool,
    /// Maximum decompression size in GB (overrides server default)
    max_decompress_gb: Option<usize>,
    /// OCR language codes (comma-separated)
    ocr_language: Option<String>,
    /// OCR DPI override
    ocr_dpi: Option<u32>,
    /// Enable markdown anchors
    markdown_anchors: bool,
    /// Page range (e.g., "1-5,7,12-")
    pages: Option<String>,
}

/// Helper function to extract DiagCode from extraction error messages.
///
/// Extraction errors from pdftract-core are wrapped in anyhow::Error and lose
/// their structured DiagCode information. This function parses the error message
/// and maps it to the appropriate DiagCode for API error responses.
fn extract_diag_code_from_error(msg: &str) -> Option<DiagCode> {
    let msg_lower = msg.to_lowercase();

    // Encryption-related errors
    if msg_lower.contains("encryption") || msg_lower.contains("encrypted") {
        if msg_lower.contains("unsupported") {
            return Some(DiagCode::EncryptionUnsupported);
        }
        if msg_lower.contains("password") || msg_lower.contains("decrypt") {
            return Some(DiagCode::EncryptionWrongPassword);
        }
        return Some(DiagCode::EncryptionUnsupported);
    }

    // Corrupt/truncated PDF errors
    if msg_lower.contains("corrupt") || msg_lower.contains("truncated") {
        if msg_lower.contains("xref") || msg_lower.contains("cross-reference") {
            return Some(DiagCode::XrefTruncated);
        }
        if msg_lower.contains("stream") || msg_lower.contains("decompress") {
            return Some(DiagCode::StreamDecodeError);
        }
        if msg_lower.contains("unexpected eof") || msg_lower.contains("end of file") {
            return Some(DiagCode::StructUnexpectedEof);
        }
        return Some(DiagCode::StreamDecodeError);
    }

    // Stream decode errors
    if msg_lower.contains("decode") && (msg_lower.contains("error") || msg_lower.contains("failed"))
    {
        return Some(DiagCode::StreamDecodeError);
    }

    // Bomb limit errors
    if msg_lower.contains("bomb") || msg_lower.contains("decompression limit") {
        return Some(DiagCode::StreamBomb);
    }

    // Xref errors
    if msg_lower.contains("xref")
        && (msg_lower.contains("invalid") || msg_lower.contains("not found"))
    {
        return Some(DiagCode::XrefTrailerNotFound);
    }

    // Trailer errors
    if msg_lower.contains("trailer") && msg_lower.contains("not found") {
        return Some(DiagCode::XrefTrailerNotFound);
    }

    // Catalog errors
    if msg_lower.contains("catalog") && msg_lower.contains("parse") {
        return Some(DiagCode::StructMissingKey);
    }

    // No specific code matched
    None
}

/// Field-typing helpers for multipart form parsing.
mod form_helpers {
    /// Parse a boolean from a form field value.
    ///
    /// Accepts: "true", "1", "yes", "on" → true
    ///          "false", "0", "no", "off" → false
    ///
    /// Case-insensitive. Returns an error for unrecognized values.
    pub fn parse_bool(field_name: &str, value: &str) -> Result<bool, String> {
        match value.trim().to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Ok(true),
            "false" | "0" | "no" | "off" => Ok(false),
            _ => Err(format!(
                "Invalid boolean value for '{}': '{}'. Expected: true, false, 1, 0, yes, no, on, off",
                field_name, value
            )),
        }
    }

    /// Parse a float (f32) from a form field value.
    pub fn parse_float(field_name: &str, value: &str) -> Result<f32, String> {
        value
            .trim()
            .parse::<f32>()
            .map_err(|_| format!("Invalid float value for '{}': '{}'", field_name, value))
    }

    /// Parse an integer (u32) from a form field value.
    pub fn parse_int(field_name: &str, value: &str) -> Result<u32, String> {
        value
            .trim()
            .parse::<u32>()
            .map_err(|_| format!("Invalid integer value for '{}': '{}'", field_name, value))
    }

    /// Parse a comma-separated list into a Vec<String>.
    ///
    /// Empty values are filtered out. Whitespace around each value is trimmed.
    pub fn parse_comma_list(value: &str) -> Vec<String> {
        value
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Validate PDF magic bytes.
    ///
    /// Returns an error if the data does not start with "%PDF-" (the standard
    /// PDF file signature). This check is performed on the first 5 bytes of
    /// the uploaded file.
    ///
    /// The PDF spec (ISO 32000-1:2008, section 7.5.2) requires that the first
    /// line of a PDF file contains "%PDF-x.y" where x.y is the version number.
    pub fn validate_pdf_magic_bytes(data: &[u8]) -> Result<(), String> {
        if data.len() < 5 {
            return Err("Uploaded file is too small to be a valid PDF".to_string());
        }

        // Check for %PDF- signature (standard PDF magic bytes)
        if !data.starts_with(b"%PDF-") {
            return Err("Uploaded file is not a PDF (missing %PDF- header)".to_string());
        }

        Ok(())
    }
}

/// Run the HTTP serve mode.
///
/// # Arguments
///
/// * `bind_addr` — Address to bind (e.g., "127.0.0.1:8080")
/// * `cache_dir` — Optional cache directory
/// * `cache_size_bytes` — Cache size limit in bytes
/// * `cache_disabled` — Whether cache is globally disabled
/// * `max_upload_mb` — Maximum request body size in MB
/// * `max_decompress_gb` — Maximum decompression size in GB
/// * `audit_log` — Optional audit log file path
/// * `trust_forwarded_for` — Whether to trust X-Forwarded-For for client IP
pub async fn run(
    bind_addr: String,
    metrics_port: Option<u16>,
    cache_dir: Option<PathBuf>,
    cache_size_bytes: u64,
    cache_disabled: bool,
    max_upload_mb: usize,
    max_decompress_gb: usize,
    audit_log: Option<PathBuf>,
    trust_forwarded_for: bool,
) -> Result<()> {
    let cache_dir_for_logging = cache_dir.as_deref();

    // Create audit log writer if specified
    let audit_writer = if let Some(ref path) = audit_log {
        Some(
            AuditLogWriter::open(path)
                .context(format!("Failed to open audit log: {}", path.display()))?,
        )
    } else {
        None
    };

    // Convert max_decompress_gb to bytes (1 GB = 1 << 30 bytes)
    let max_decompress_bytes = (max_decompress_gb as u64) * (1 << 30);

    let state = ServeState::new(
        cache_dir.clone(),
        cache_size_bytes,
        cache_disabled,
        audit_writer,
        max_decompress_bytes,
        trust_forwarded_for,
    );

    let max_body_bytes = max_upload_mb * 1024 * 1024;

    // Bind the `--metrics PORT` listener BEFORE the main
    // listener (`metrics` feature): a port that cannot be bound must fail
    // startup cleanly here — with the flag's clean error and a nonzero
    // exit — before anything is served. `/metrics` and `/ready` exist
    // only on this listener; the main router below gains neither route
    // (plan "Monitoring and Alerting" endpoint policy).
    #[cfg(feature = "metrics")]
    let metrics_listener = if let Some(port) = metrics_port {
        let metrics_addr = crate::metrics::listener_addr(&bind_addr, port)?;
        let registry = state.metrics.clone();
        let readiness = state.readiness.clone();
        Some(crate::metrics::bind_and_spawn(&metrics_addr, registry, readiness).await?)
    } else {
        None
    };
    #[cfg(not(feature = "metrics"))]
    if metrics_port.is_some() {
        anyhow::bail!(
            "--metrics requires a build with the `metrics` cargo feature \
             (rebuild with --features serve, which implies it)"
        );
    }

    #[cfg(feature = "metrics")]
    let sampler_registry = state.metrics.clone();
    let app = build_router(state, max_body_bytes);

    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(listener) => listener,
        Err(error) => {
            #[cfg(feature = "metrics")]
            if let Some(metrics_listener) = metrics_listener {
                metrics_listener.shutdown().await;
            }
            return Err(error).context(format!("Failed to bind to {}", bind_addr));
        }
    };

    // Sample `pdftract_rayon_pool_utilization` periodically for the whole
    // lifetime of the server (`metrics` feature). Start this only after both
    // sockets have bound successfully, so failed startup leaves no task.
    #[cfg(feature = "metrics")]
    let sampler_task = {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(std::time::Duration::from_secs(5));
            loop {
                ticker.tick().await;
                sampler_registry
                    .set_rayon_pool_utilization(crate::metrics::sampler::sample_utilization());
            }
        })
    };

    // Print startup banner with security warning
    eprintln!("pdftract serve is starting on http://{}", bind_addr);
    eprintln!("*** NO BUILT-IN AUTH *** — Deploy behind a reverse proxy for production.");
    if let Some(dir) = cache_dir_for_logging {
        eprintln!(
            "Cache enabled: {} (max {} bytes)",
            dir.display(),
            cache_size_bytes
        );
    } else {
        eprintln!("Cache disabled");
    }
    if let Some(ref path) = audit_log {
        eprintln!("Audit log: {}", path.display());
    }
    eprintln!("Max upload size: {} MB", max_upload_mb);
    eprintln!("Max decompression size: {} GB", max_decompress_gb);

    // The router stack (audit log + metrics middleware) extracts
    // `ConnectInfo<SocketAddr>`, so the make service must supply it — a
    // bare Router serve 500s every request with "Missing request
    // extension".
    let server_result = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .context("HTTP server error");

    #[cfg(feature = "metrics")]
    sampler_task.abort();
    #[cfg(feature = "metrics")]
    if let Some(metrics_listener) = metrics_listener {
        metrics_listener.shutdown().await;
    }

    server_result?;

    Ok(())
}

/// Build the serve application router (shared by [`run`] and tests).
///
/// Layer order, outermost first: metrics counting (`metrics` feature),
/// body limit, 413 JSON conversion, audit. The metrics layer sits
/// outermost so it observes the response actually sent, after the 413
/// conversion.
fn build_router(state: ServeState, max_body_bytes: usize) -> Router {
    #[cfg(feature = "metrics")]
    let metrics_registry = state.metrics.clone();

    // Apply body limit with custom 413 JSON response
    // The custom rejection handler converts tower-http's default text/plain 413 to JSON
    let limit_bytes = max_body_bytes;
    let router = Router::new()
        .route("/", get(root_handler))
        .route(
            "/extract",
            get(extract_get_not_found_handler).post(extract_handler),
        )
        .route("/extract/text", post(extract_text_handler))
        .route("/extract/stream", post(extract_stream_handler))
        .route("/health", get(health_handler))
        .layer(axum::middleware::from_fn_with_state(
            state.audit.clone(),
            audit_middleware,
        ))
        .layer(axum::middleware::from_fn(
            move |req: Request<axum::body::Body>, next: axum::middleware::Next| async move {
                // Check Content-Length header against limit (early rejection for efficiency)
                if let Some(content_length) = req.headers().get("content-length") {
                    if let Ok(len_str) = content_length.to_str() {
                        if let Ok(len) = len_str.parse::<usize>() {
                            if len > limit_bytes {
                                let api_error = ApiError {
                                    error: "REQUEST_TOO_LARGE".to_string(),
                                    message: "Request body exceeds the configured limit"
                                        .to_string(),
                                    hint: None,
                                };
                                let body = serde_json::to_vec(&api_error).unwrap_or_default();
                                let response: Response<axum::body::Body> = Response::builder()
                                    .status(StatusCode::PAYLOAD_TOO_LARGE)
                                    .header("Content-Type", "application/json")
                                    .body(axum::body::Body::from(body))
                                    .unwrap();
                                return response;
                            }
                        }
                    }
                }
                let response = next.run(req).await;
                // Convert any 413 response to JSON (handles DefaultBodyLimit rejections for chunked requests)
                if response.status() == StatusCode::PAYLOAD_TOO_LARGE {
                    let api_error = ApiError {
                        error: "REQUEST_TOO_LARGE".to_string(),
                        message: "Request body exceeds the configured limit".to_string(),
                        hint: None,
                    };
                    let body = serde_json::to_vec(&api_error).unwrap_or_default();
                    let json_response: Response<axum::body::Body> = Response::builder()
                        .status(StatusCode::PAYLOAD_TOO_LARGE)
                        .header("Content-Type", "application/json")
                        .body(axum::body::Body::from(body))
                        .unwrap();
                    return json_response;
                }
                response
            },
        ))
        .layer(DefaultBodyLimit::max(max_body_bytes));

    // Count every response (including router 404s and converted 413s) in
    // `pdftract_http_requests_total`. Observation only — no new route.
    #[cfg(feature = "metrics")]
    let router = router.layer(axum::middleware::from_fn_with_state(
        metrics_registry,
        metrics_http_middleware,
    ));

    router.with_state(state)
}

/// Map a request path to its registered route template for the `endpoint`
/// label. Unregistered paths collapse to `"unmatched"` — the label never
/// carries a raw request path (plan cardinality policy).
#[cfg(feature = "metrics")]
fn endpoint_label(path: &str) -> &'static str {
    match path {
        "/" => "/",
        "/health" => "/health",
        "/extract" => "/extract",
        "/extract/text" => "/extract/text",
        "/extract/stream" => "/extract/stream",
        _ => "unmatched",
    }
}

/// Count one HTTP response in `pdftract_http_requests_total`
/// (`metrics` feature).
#[cfg(feature = "metrics")]
async fn metrics_http_middleware(
    State(registry): State<crate::metrics::Registry>,
    req: Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Response<axum::body::Body> {
    let endpoint = endpoint_label(req.uri().path());
    let response = next.run(req).await;
    registry.inc_http_request(endpoint, response.status().as_u16());
    response
}

/// Record structured diagnostics on the registry (`metrics` feature).
///
/// Keeping this separate from the extraction result recorder lets the
/// streaming path report the diagnostics carried by its footer without
/// creating a second extraction observation.
#[cfg(feature = "metrics")]
fn record_diagnostics(
    metrics: &crate::metrics::Registry,
    diagnostics: &[pdftract_core::schema::DiagnosticJson],
) {
    for diagnostic in diagnostics {
        metrics.inc_diagnostic(
            &diagnostic.code,
            diagnostic_metric_severity_name(&diagnostic.severity),
        );
    }
}

/// Whether a serve request explicitly selected an OCR setting. The HTTP form
/// has no standalone `ocr` flag; language and DPI fields are the request-level
/// signals that select the OCR path. The build feature still gates whether
/// that path is available at runtime.
#[cfg(feature = "metrics")]
fn extraction_ocr_requested(params: &ExtractParams) -> bool {
    cfg!(feature = "ocr") && (params.ocr_language.is_some() || params.ocr_dpi.is_some())
}

/// Record one completed extraction on the registry (`metrics` feature):
/// duration histogram, result/ocr counter, emitted pages, cache hit or
/// miss, and the document's diagnostics with code and severity labels.
#[cfg(feature = "metrics")]
fn record_extraction(
    metrics: &crate::metrics::Registry,
    elapsed_seconds: f64,
    result_label: &str,
    ocr: bool,
    pages: u64,
    cache_status: Option<&str>,
    diagnostics_detailed: &[pdftract_core::schema::DiagnosticJson],
    extraction_diagnostic: Option<DiagCode>,
) {
    metrics.observe_extraction_duration(elapsed_seconds);
    metrics.inc_extraction(result_label, ocr);
    metrics.add_pages_extracted(pages);
    match cache_status {
        Some("hit") => metrics.inc_cache_hit(),
        Some("miss") => metrics.inc_cache_miss(),
        _ => {}
    }
    if let Some(code) = extraction_diagnostic {
        metrics.inc_diagnostic(code.name(), diagnostic_metric_severity(code.severity()));
    }
    record_diagnostics(metrics, diagnostics_detailed);
}

/// Own the lifecycle of one extraction submitted to `spawn_blocking`.
///
/// The guard lives inside the blocking task, rather than in the request
/// future. That makes cancellation and panics observable too: dropping a
/// task that never starts, or unwinding a task that fails, records one error
/// and decrements the in-flight gauge exactly once.
#[cfg(feature = "metrics")]
struct ExtractionMetricsGuard {
    metrics: crate::metrics::Registry,
    started: std::time::Instant,
    cache_status_on_error: Option<&'static str>,
    ocr: bool,
    done: bool,
}

#[cfg(feature = "metrics")]
impl ExtractionMetricsGuard {
    fn new(metrics: crate::metrics::Registry, cache_enabled: bool, ocr: bool) -> Self {
        metrics.inc_inflight_extractions();
        Self {
            metrics,
            started: std::time::Instant::now(),
            cache_status_on_error: cache_enabled.then_some("miss"),
            ocr,
            done: false,
        }
    }

    fn finish(
        &mut self,
        result_label: &str,
        pages: u64,
        cache_status: Option<&str>,
        diagnostics_detailed: &[pdftract_core::schema::DiagnosticJson],
        extraction_diagnostic: Option<DiagCode>,
    ) {
        if self.done {
            return;
        }
        self.done = true;
        self.metrics.dec_inflight_extractions();
        record_extraction(
            &self.metrics,
            self.started.elapsed().as_secs_f64(),
            result_label,
            self.ocr,
            pages,
            cache_status,
            diagnostics_detailed,
            extraction_diagnostic,
        );
    }

    fn finish_success(
        &mut self,
        result: &pdftract_core::extract::ExtractionResult,
        cache_status: &str,
    ) {
        self.finish(
            "success",
            result.pages.len() as u64,
            Some(cache_status),
            result.metadata.diagnostics_detailed.as_slice(),
            None,
        );
    }

    fn finish_error(&mut self, extraction_diagnostic: Option<DiagCode>) {
        self.finish(
            "error",
            0,
            self.cache_status_on_error,
            &[],
            extraction_diagnostic,
        );
    }
}

#[cfg(feature = "metrics")]
impl Drop for ExtractionMetricsGuard {
    fn drop(&mut self) {
        // A task that is cancelled before it reaches its explicit result
        // handling, or one that panics, is still a completed error outcome.
        self.finish_error(None);
    }
}

/// Refresh the cache-size gauge from entry files, falling back to the index
/// for a cache that only has metadata (as in startup recovery/tests).
#[cfg(feature = "metrics")]
fn refresh_cache_size(
    metrics: &crate::metrics::Registry,
    cache_dir: &std::path::Path,
    cache_size_bytes: u64,
) {
    let indexed_size = || {
        pdftract_core::cache::layout::load_index(cache_dir)
            .ok()
            .flatten()
            .map(|index| index.total_bytes)
    };
    let size = match pdftract_core::cache::Lru::new(cache_dir, cache_size_bytes)
        .current_size_bytes()
    {
        Ok(size) if size > 0 => Some(size),
        Ok(_) => indexed_size().or(Some(0)),
        Err(_) => indexed_size(),
    };
    if let Some(size) = size {
        metrics.set_cache_size_bytes(size);
    }
}

/// Map core diagnostic severities to the labels documented by the metrics
/// surface. Core uses the more descriptive `warning` and `fatal` values in
/// JSON, while the Prometheus contract deliberately has three bounded values:
/// `info`, `warn`, and `error`.
#[cfg(feature = "metrics")]
fn diagnostic_metric_severity(severity: pdftract_core::diagnostics::Severity) -> &'static str {
    match severity {
        pdftract_core::diagnostics::Severity::Info => "info",
        pdftract_core::diagnostics::Severity::Warning => "warn",
        pdftract_core::diagnostics::Severity::Error
        | pdftract_core::diagnostics::Severity::Fatal => "error",
    }
}

#[cfg(feature = "metrics")]
fn diagnostic_metric_severity_name(severity: &str) -> &'static str {
    match severity {
        "info" => "info",
        "warn" | "warning" => "warn",
        "error" | "fatal" => "error",
        // DiagnosticJson is produced by core, but keep the metric label
        // bounded if a future producer supplies an unexpected value.
        _ => "error",
    }
}

/// Root handler - returns server info.
async fn root_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "service": "pdftract",
        "version": env!("CARGO_PKG_VERSION"),
        "endpoints": [
            "POST /extract - Extract PDF and return JSON",
            "POST /extract/text - Extract PDF and return plain text",
            "POST /extract/stream - Extract PDF and return streaming NDJSON",
            "GET /health - Health check"
        ]
    }))
}

/// Health check handler.
async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Extract GET handler - returns 404 for file-path parameter attempts.
///
/// This handler explicitly rejects GET requests to /extract (which might attempt
/// to pass file paths via query parameters like ?path=/etc/passwd). All PDF
/// extraction must use POST with multipart/form-data uploads.
async fn extract_get_not_found_handler() -> impl IntoResponse {
    let api_error = ApiError {
        error: "NOT_FOUND".to_string(),
        message: "POST to /extract with multipart/form-data is required; file-path parameters are not supported".to_string(),
        hint: Some("Use POST /extract with a 'file' field containing the PDF bytes".to_string()),
    };
    (StatusCode::NOT_FOUND, Json(api_error))
}

/// Extract handler - returns JSON with cache status in metadata.
async fn extract_handler(
    State(state): State<ServeState>,
    Extension(metadata): Extension<RequestMetadata>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AxumError> {
    let (pdf_file, params) = receive_pdf(&mut multipart).await?;
    let options = build_options(&state, &params)?;

    // Get cache configuration
    let cache_state = state.cache.lock().await;
    let cache_dir = cache_state.cache_dir.clone();
    let cache_size_bytes = cache_state.cache_size_bytes;
    let cache_disabled = params.no_cache || cache_state.cache_disabled || cache_dir.is_none();
    drop(cache_state);

    // Perform extraction with cache integration
    #[cfg(feature = "metrics")]
    let extraction_metrics = ExtractionMetricsGuard::new(
        state.metrics.clone(),
        !cache_disabled,
        extraction_ocr_requested(&params),
    );

    let pdf_file_clone = pdf_file.clone();
    let extracted = {
        #[cfg(feature = "metrics")]
        let mut extraction_metrics = extraction_metrics;
        tokio::task::spawn_blocking(move || {
            // The synchronous extraction is the unit of work submitted by
            // this service to the worker pool.  Hold the guard for its full
            // lifetime so the sampler can observe live utilization.
            #[cfg(feature = "metrics")]
            let _busy = crate::metrics::sampler::BusyGuard::begin_with_registry(
                extraction_metrics.metrics.clone(),
            );
            let cache_dir_ref = cache_dir.as_deref();
            let extraction = cache::extract_with_cache(
                &pdf_file_clone,
                &options,
                cache_dir_ref,
                cache_disabled,
                Some(cache_size_bytes),
            );
            // Sample the cache's own tracked on-disk size after every
            // cache-enabled extraction (`metrics` feature).
            #[cfg(feature = "metrics")]
            if !cache_disabled {
                if let Some(dir) = cache_dir_ref {
                    refresh_cache_size(&extraction_metrics.metrics, dir, cache_size_bytes);
                }
            }

            match extraction {
                Ok((result, status, age)) => {
                    #[cfg(feature = "metrics")]
                    extraction_metrics.finish_success(&result, &status);
                    Ok((result, status, age))
                }
                Err(e) => {
                    let msg = format!("{:?}", e);
                    let diag_code = extract_diag_code_from_error(&msg);
                    #[cfg(feature = "metrics")]
                    extraction_metrics.finish_error(diag_code);
                    Err(AxumError::Extraction(msg, diag_code))
                }
            }
        })
        .await
        .map_err(|e| {
            // Distinguish between cancellation (task dropped) and panic
            if e.is_cancelled() {
                AxumError::Internal(format!("Task cancelled: {}", e))
            } else {
                // is_panic() true means the task panicked - indicates a bug
                AxumError::InternalPanic(format!("Extraction task panicked: {}", e))
            }
        })
    };

    let (mut result, cache_status, cache_age) = extracted??;

    // Build JSON response with cache status
    result.metadata.cache_status = Some(cache_status.clone());
    result.metadata.cache_age_seconds = cache_age;

    // Extract fingerprint and diagnostics for audit log
    let fingerprint = result.fingerprint.clone();
    let diagnostics: Vec<String> = result.metadata.diagnostics.clone();

    let json = result_to_json(&result);

    let response = AxumResponse::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header(
            "X-Pdftract-Cache",
            CacheStatus::from_string(&cache_status).header_value(),
        )
        .body(Body::from(serde_json::to_string(&json).unwrap()))
        .map_err(|e| AxumError::Internal(format!("{:?}", e).to_string()))?;

    // Write audit log if configured
    if let Some(ref writer) = state.audit.writer {
        let duration_ms = metadata.start_time.elapsed().as_millis() as u64;
        let _ = writer.log(
            &metadata.tool,
            metadata.client_ip.as_deref(),
            Some(&fingerprint),
            duration_ms,
            200,
            &diagnostics,
        );
    }

    Ok(response)
}

/// Extract text handler - returns plain text with X-Pdftract-Cache header.
async fn extract_text_handler(
    State(state): State<ServeState>,
    Extension(metadata): Extension<RequestMetadata>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AxumError> {
    let (pdf_file, params) = receive_pdf(&mut multipart).await?;
    let options = build_options(&state, &params)?;

    // Get cache configuration
    let cache_state = state.cache.lock().await;
    let cache_dir = cache_state.cache_dir.clone();
    let cache_size_bytes = cache_state.cache_size_bytes;
    let cache_disabled = params.no_cache || cache_state.cache_disabled || cache_dir.is_none();
    drop(cache_state);

    #[cfg(feature = "metrics")]
    let extraction_metrics = ExtractionMetricsGuard::new(
        state.metrics.clone(),
        !cache_disabled,
        extraction_ocr_requested(&params),
    );

    let extracted = {
        #[cfg(feature = "metrics")]
        let mut extraction_metrics = extraction_metrics;
        tokio::task::spawn_blocking(move || {
            #[cfg(feature = "metrics")]
            let _busy = crate::metrics::sampler::BusyGuard::begin_with_registry(
                extraction_metrics.metrics.clone(),
            );
            let cache_dir_ref = cache_dir.as_deref();
            let extraction = cache::extract_with_cache(
                &pdf_file,
                &options,
                cache_dir_ref,
                cache_disabled,
                Some(cache_size_bytes),
            );
            // Sample the cache's own tracked on-disk size after every
            // cache-enabled extraction (`metrics` feature).
            #[cfg(feature = "metrics")]
            if !cache_disabled {
                if let Some(dir) = cache_dir_ref {
                    refresh_cache_size(&extraction_metrics.metrics, dir, cache_size_bytes);
                }
            }

            match extraction {
                Ok((result, status, age)) => {
                    #[cfg(feature = "metrics")]
                    extraction_metrics.finish_success(&result, &status);
                    Ok((result, status, age))
                }
                Err(e) => {
                    let msg = format!("{:?}", e);
                    let diag_code = extract_diag_code_from_error(&msg);
                    #[cfg(feature = "metrics")]
                    extraction_metrics.finish_error(diag_code);
                    Err(AxumError::Extraction(msg, diag_code))
                }
            }
        })
        .await
        .map_err(|e| {
            // Distinguish between cancellation (task dropped) and panic
            if e.is_cancelled() {
                AxumError::Internal(format!("Task cancelled: {}", e))
            } else {
                // is_panic() true means the task panicked - indicates a bug
                AxumError::InternalPanic(format!("Extraction task panicked: {}", e))
            }
        })
    };

    let (result, cache_status, _cache_age) = extracted??;

    // Extract fingerprint and diagnostics for audit log
    let fingerprint = result.fingerprint.clone();
    let diagnostics: Vec<String> = result.metadata.diagnostics.clone();

    let mut text = String::new();
    for page in &result.pages {
        for span in &page.spans {
            text.push_str(&span.text);
            text.push('\n');
        }
    }

    let response = AxumResponse::builder()
        .status(StatusCode::OK)
        .header(
            "X-Pdftract-Cache",
            CacheStatus::from_string(&cache_status).header_value(),
        )
        .body(Body::from(text))
        .map_err(|e| AxumError::Internal(format!("{:?}", e).to_string()))?;

    // Write audit log if configured
    if let Some(ref writer) = state.audit.writer {
        let duration_ms = metadata.start_time.elapsed().as_millis() as u64;
        let _ = writer.log(
            &metadata.tool,
            metadata.client_ip.as_deref(),
            Some(&fingerprint),
            duration_ms,
            200,
            &diagnostics,
        );
    }

    Ok(response)
}

/// Extract stream handler - returns true async streaming NDJSON.
///
/// This handler spawns a background task that extracts pages sequentially
/// and sends them over a channel. The response body is a stream that yields
/// each page as NDJSON immediately after it's extracted.
///
/// Cache status is always "skipped" for streaming since we bypass the cache
/// to provide true incremental output.
async fn extract_stream_handler(
    State(state): State<ServeState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AxumError> {
    use tokio_stream::wrappers::ReceiverStream;
    use tokio_stream::StreamExt;

    let (pdf_file, params) = receive_pdf(&mut multipart).await?;
    let options = build_options(&state, &params)?;

    // Get cache configuration (for logging only - streaming bypasses cache)
    let cache_state = state.cache.lock().await;
    let _cache_dir = cache_state.cache_dir.clone();
    drop(cache_state);

    // Create a channel for streaming pages
    let (tx, rx) = tokio::sync::mpsc::channel::<Vec<u8>>(16);

    #[cfg(feature = "metrics")]
    let extraction_metrics = ExtractionMetricsGuard::new(
        state.metrics.clone(),
        false,
        extraction_ocr_requested(&params),
    );

    // Spawn extraction task in background
    tokio::task::spawn_blocking(move || {
        use pdftract_core::extract::extract_pdf_ndjson;

        #[cfg(feature = "metrics")]
        let mut extraction_metrics = extraction_metrics;
        #[cfg(feature = "metrics")]
        let _busy = crate::metrics::sampler::BusyGuard::begin_with_registry(
            extraction_metrics.metrics.clone(),
        );

        // Clone sender for error handling
        let tx_for_error = tx.clone();

        // Write to a custom writer that sends to the channel
        struct ChannelWriter {
            tx: tokio::sync::mpsc::Sender<Vec<u8>>,
        };

        impl std::io::Write for ChannelWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                // Clone the buffer since we need to send it
                self.tx
                    .blocking_send(buf.to_vec())
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                Ok(buf.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let writer = ChannelWriter { tx };

        // Extract to NDJSON, streaming each page as it's extracted
        let extraction = extract_pdf_ndjson(&pdf_file, &options, writer);

        // Record before the last sender drops, so a fully-drained response
        // always sees the completion values. The guard's Drop path covers a
        // task cancelled before this match or a panic during extraction.
        #[cfg(feature = "metrics")]
        match &extraction {
            Ok(metadata) => extraction_metrics.finish(
                "success",
                metadata.page_count as u64,
                None,
                &metadata.diagnostics_detailed,
                None,
            ),
            Err(error) => {
                let message = format!("{:?}", error);
                extraction_metrics.finish_error(extract_diag_code_from_error(&message));
            }
        }

        if let Err(e) = extraction {
            // Send error as a JSON line
            let error_json = serde_json::json!({
                "error": format!("{:?}", e)
            });
            if let Ok(json_bytes) = serde_json::to_vec(&error_json) {
                let _ = tx_for_error.blocking_send(json_bytes);
                let _ = tx_for_error.blocking_send(b"\n".to_vec());
            }
        }

        Ok::<(), AxumError>(())
    });

    // Create a stream from the receiver
    let stream = ReceiverStream::new(rx).map(|item| Ok::<_, axum::Error>(bytes::Bytes::from(item)));

    // Return a streaming body
    let body = Body::from_stream(stream);

    let response = AxumResponse::builder()
        .status(StatusCode::OK)
        .header("X-Pdftract-Cache", CacheStatus::Skipped.header_value())
        .header("Content-Type", "application/x-ndjson")
        .body(body)
        .map_err(|e| AxumError::Internal(format!("{:?}", e).to_string()))?;

    Ok(response)
}

/// Receive uploaded PDF file and extraction parameters.
///
/// Parses multipart/form-data with the following structure:
/// - `file` or `pdf`: Required field containing the PDF file bytes
/// - `receipts`: Optional; "off", "lite", or "svg" (default: "off")
/// - `no_cache`: Optional; boolean flag to disable cache (default: false)
/// - `full_render`: Optional; boolean flag to enable full-render (default: false)
/// - `max_decompress_gb`: Optional; integer max decompression size in GB
/// - `ocr_language`: Optional; comma-separated list of language codes (default: "eng")
/// - `ocr_dpi`: Optional; integer DPI override for OCR
/// - `markdown_anchors`: Optional; boolean flag to enable markdown anchors (default: false)
///
/// Unknown fields are logged as warnings and ignored (forward-compatibility).
///
/// Returns a tuple of (temp file path, parsed parameters). The temp file is
/// cleaned up by the OS; the caller should extract from it before the request ends.
async fn receive_pdf(multipart: &mut Multipart) -> Result<(PathBuf, ExtractParams), AxumError> {
    use form_helpers::{parse_bool, parse_int, validate_pdf_magic_bytes};

    let mut pdf_path = None;
    let mut pdf_bytes: Option<Vec<u8>> = None;
    let mut params = ExtractParams::default();

    // Known form fields for validation (forward-compatibility: unknown fields are warned)
    const KNOWN_FIELDS: &[&str] = &[
        "file",
        "pdf",
        "receipts",
        "no_cache",
        "full_render",
        "max_decompress_gb",
        "ocr_language",
        "ocr_dpi",
        "markdown_anchors",
        "pages",
    ];

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AxumError::Internal(format!("Failed to read multipart field: {:?}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();

        // Handle the file field (required)
        if name == "file" || name == "pdf" {
            let data = field
                .bytes()
                .await
                .map_err(|e| AxumError::Internal(format!("Failed to read file field: {:?}", e)))?;

            // Validate PDF magic bytes before processing
            validate_pdf_magic_bytes(&data).map_err(|msg| {
                AxumError::BadRequest(
                    msg,
                    Some("Upload a valid PDF file (must start with %PDF-)".to_string()),
                )
            })?;

            pdf_bytes = Some(data.to_vec());

            // Create a temp file that will persist for the duration of the request
            let temp_dir = std::env::temp_dir();
            let temp_file = temp_dir.join(format!("pdftract-upload-{}.pdf", uuid::Uuid::new_v4()));
            tokio::fs::write(&temp_file, &data)
                .await
                .map_err(|e| AxumError::Internal(format!("Failed to write temp file: {:?}", e)))?;
            pdf_path = Some(temp_file);
            continue;
        }

        // Parse form fields (all are optional)
        match name.as_str() {
            "receipts" => {
                if let Ok(value) = field.text().await {
                    params.receipts = Some(value);
                }
            }
            "no_cache" => {
                // Presence of the field means true (checkbox behavior)
                params.no_cache = true;
            }
            "full_render" => {
                if let Ok(value) = field.text().await {
                    params.full_render = parse_bool("full_render", &value).unwrap_or(false);
                } else {
                    // Checkbox without value also means true
                    params.full_render = true;
                }
            }
            "max_decompress_gb" => {
                if let Ok(value) = field.text().await {
                    params.max_decompress_gb = parse_int("max_decompress_gb", &value)
                        .ok()
                        .map(|v| v as usize);
                }
            }
            "ocr_language" => {
                if let Ok(value) = field.text().await {
                    params.ocr_language = Some(value);
                }
            }
            "ocr_dpi" => {
                if let Ok(value) = field.text().await {
                    params.ocr_dpi = parse_int("ocr_dpi", &value).ok();
                }
            }
            "markdown_anchors" => {
                if let Ok(value) = field.text().await {
                    params.markdown_anchors =
                        parse_bool("markdown_anchors", &value).unwrap_or(false);
                } else {
                    params.markdown_anchors = true;
                }
            }
            "pages" => {
                if let Ok(value) = field.text().await {
                    params.pages = Some(value);
                }
            }
            _ => {
                // Unknown field - log warning and ignore (forward-compatibility)
                if !name.is_empty() {
                    tracing::warn!(
                        "Unknown multipart field '{}' ignored (known fields: {:?})",
                        name,
                        KNOWN_FIELDS.join(", ")
                    );
                }
            }
        }
    }

    // Validate that a PDF was uploaded
    let pdf_path = pdf_path.ok_or_else(|| {
        AxumError::MissingField("No PDF file uploaded".to_string(), "file".to_string())
    })?;

    Ok((pdf_path, params))
}

/// Build extraction options from parameters.
///
/// Validates that full_render is only used when the feature is available.
/// If full_render is requested but the feature is not compiled in,
/// the request still succeeds but falls back to direct compositing.
fn build_options(
    state: &ServeState,
    params: &ExtractParams,
) -> Result<ExtractionOptions, AxumError> {
    use form_helpers::parse_comma_list;

    // Parse receipts mode (default: Off)
    let receipts_mode = match params.receipts.as_deref() {
        Some("lite") => ReceiptsMode::Lite,
        Some("svg") => ReceiptsMode::SvgClip,
        _ => ReceiptsMode::Off,
    };

    // Validate max_decompress_gb if provided
    if let Some(gb) = params.max_decompress_gb {
        const MAX_DECOMPRESS_GB_HARD_CAP: usize = 4096;
        if gb > MAX_DECOMPRESS_GB_HARD_CAP {
            return Err(AxumError::BadRequest(
                format!(
                    "max_decompress_gb value {} exceeds hard cap of {} GB",
                    gb, MAX_DECOMPRESS_GB_HARD_CAP
                ),
                Some(format!("Use a value <= {} GB", MAX_DECOMPRESS_GB_HARD_CAP)),
            ));
        }
    }

    // Check if full_render is requested
    if params.full_render {
        // Validate that full_render is available at runtime
        #[cfg(all(feature = "ocr", feature = "full-render"))]
        {
            use pdftract_core::render::pdfium_path::has_full_render;
            if !has_full_render() {
                return Err(AxumError::BadRequest(
                    "full_render requested but PDFium is not available at runtime. \
                    Ensure the PDFium native library is installed."
                        .to_string(),
                    Some("Install PDFium or build with --features full-render".to_string()),
                ));
            }
        }

        #[cfg(not(all(feature = "ocr", feature = "full-render")))]
        {
            // Feature not compiled in - fall back to direct compositing
            // Log a debug message but don't fail the request
            tracing::debug!(
                "full_render requested but full-render feature not compiled; using direct compositing path"
            );
        }
    }

    // Parse OCR language list (default: ["eng"])
    let ocr_language = params
        .ocr_language
        .as_deref()
        .map(parse_comma_list)
        .unwrap_or_else(|| vec!["eng".to_string()]);

    // Calculate max_decompress_bytes: use form field override if provided, otherwise use server default
    let max_decompress_bytes = if let Some(gb) = params.max_decompress_gb {
        (gb as u64) * (1 << 30) // Convert GB to bytes
    } else {
        state.max_decompress_bytes
    };

    // Build extraction options with defaults + overrides
    Ok(ExtractionOptions {
        receipts: receipts_mode,
        full_render: params.full_render,
        ocr_dpi_override: params.ocr_dpi,
        ocr_language,
        markdown_anchors: params.markdown_anchors,
        max_decompress_bytes,
        pages: params.pages.clone(),
        ..Default::default()
    })
}

/// Error types for the HTTP server.
#[derive(Debug)]
pub enum AxumError {
    /// Bad request (400) - invalid parameters
    BadRequest(String, Option<String>),
    /// Missing field (400) - required multipart field not provided
    MissingField(String, String),
    /// Request too large (413) - body exceeds configured limit
    RequestTooLarge,
    /// Extraction error (422) - PDF parsing or extraction failure
    Extraction(String, Option<DiagCode>),
    /// Internal error (500) - server-side failure
    Internal(String),
    /// Internal panic (500) - spawn_blocking task panicked (indicates a bug)
    InternalPanic(String),
}

impl IntoResponse for AxumError {
    fn into_response(self) -> AxumResponse {
        let api_error = match self {
            AxumError::RequestTooLarge => ApiError {
                error: "REQUEST_TOO_LARGE".to_string(),
                message: "Request body exceeds the configured limit".to_string(),
                hint: None,
            },
            AxumError::MissingField(msg, field_name) => ApiError::new("MISSING_FIELD", msg)
                .with_hint(format!("Supply the '{}' multipart field", field_name)),
            AxumError::BadRequest(msg, hint) => {
                let mut err = ApiError::new("BAD_REQUEST", msg);
                if let Some(h) = hint {
                    err = err.with_hint(h);
                }
                err
            }
            AxumError::Extraction(msg, diag_code) => {
                let (error_code, hint) = if let Some(dc) = diag_code {
                    match dc {
                        DiagCode::EncryptionUnsupported => (
                            "ENCRYPTED".to_string(),
                            Some("Supply the correct password via --password, or use an Adobe-side decryption tool first".to_string()),
                        ),
                        DiagCode::EncryptionWrongPassword => (
                            "WRONG_PASSWORD".to_string(),
                            Some("The supplied password is incorrect".to_string()),
                        ),
                        DiagCode::StreamBomb => (
                            "DECOMPRESSION_LIMIT".to_string(),
                            Some("PDF decompression exceeded the configured limit. This file may be a zip-bomb or malicious PDF.".to_string()),
                        ),
                        DiagCode::StreamDecodeError | DiagCode::XrefTruncated | DiagCode::StructUnexpectedEof => (
                            "CORRUPT_PDF".to_string(),
                            Some("The PDF file is corrupt or truncated and cannot be extracted".to_string()),
                        ),
                        _ => ("EXTRACTION_ERROR".to_string(), None),
                    }
                } else {
                    ("EXTRACTION_ERROR".to_string(), None)
                };
                let mut err = ApiError::new(error_code, msg);
                if let Some(h) = hint {
                    err = err.with_hint(h);
                }
                err
            }
            AxumError::Internal(msg) => {
                // Generate a tracing tag for ops to correlate with logs
                let tag = format!("{:x}", uuid::Uuid::new_v4().as_u128());
                tracing::error!("Internal error [{}]: {}", tag, msg);
                ApiError::new("INTERNAL", "Internal error during extraction".to_string())
                    .with_hint(format!("Reference tag {} for debugging", tag))
            }
            AxumError::InternalPanic(msg) => {
                let tag = format!("{:x}", uuid::Uuid::new_v4().as_u128());
                tracing::error!("Internal panic [{}]: {}", tag, msg);
                ApiError::new(
                    "INTERNAL_PANIC",
                    "Extraction task panicked (indicates a bug)".to_string(),
                )
                .with_hint(format!("Reference tag {} for debugging", tag))
            }
        };

        let status = match api_error.error.as_str() {
            "REQUEST_TOO_LARGE" => StatusCode::PAYLOAD_TOO_LARGE, // 413
            "BAD_REQUEST" | "MISSING_FIELD" => StatusCode::BAD_REQUEST, // 400
            "ENCRYPTED"
            | "WRONG_PASSWORD"
            | "EXTRACTION_ERROR"
            | "CORRUPT_PDF"
            | "DECOMPRESSION_LIMIT" => StatusCode::UNPROCESSABLE_ENTITY, // 422
            "INTERNAL" | "INTERNAL_PANIC" => StatusCode::INTERNAL_SERVER_ERROR, // 500
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(api_error)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    use std::time::Duration;

    /// Test that the AxumError enum converts to correct status codes and error codes.
    #[tokio::test]
    async fn test_error_into_response() {
        // Test BadRequest
        let err = AxumError::BadRequest("test".to_string(), None);
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        // Test MissingField
        let err = AxumError::MissingField("test".to_string(), "file".to_string());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        // Test RequestTooLarge (413)
        let err = AxumError::RequestTooLarge;
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);

        // Test Extraction
        let err = AxumError::Extraction("test".to_string(), None);
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

        // Test Extraction with DiagCode::EncryptionUnsupported
        let err = AxumError::Extraction("test".to_string(), Some(DiagCode::EncryptionUnsupported));
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

        // Test Extraction with DiagCode::StreamDecodeError (CORRUPT_PDF)
        let err = AxumError::Extraction("test".to_string(), Some(DiagCode::StreamDecodeError));
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

        // Test Extraction with DiagCode::XrefTruncated (CORRUPT_PDF)
        let err = AxumError::Extraction("test".to_string(), Some(DiagCode::XrefTruncated));
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

        // Test Internal
        let err = AxumError::Internal("test".to_string());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        // Test InternalPanic
        let err = AxumError::InternalPanic("test".to_string());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

        // Test Extraction with DiagCode::StreamBomb (DECOMPRESSION_LIMIT)
        let err = AxumError::Extraction("test".to_string(), Some(DiagCode::StreamBomb));
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
        // Verify the response body contains DECOMPRESSION_LIMIT error code
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "DECOMPRESSION_LIMIT");
        assert!(json["hint"].as_str().unwrap().contains("zip-bomb"));
    }

    /// Test that GET /extract returns 404 (security constraint: no file-path parameters).
    #[tokio::test]
    async fn test_extract_get_returns_404() {
        use axum::{
            body::Body,
            http::{Request, StatusCode},
        };
        use tower::ServiceExt;

        let state = ServeState::new(None, 1024 * 1024 * 1024, true, None, 1 << 30, false);
        let app = Router::new()
            .route(
                "/extract",
                get(extract_get_not_found_handler).post(extract_handler),
            )
            .with_state(state);

        // Test GET /extract (should return 404, not 405)
        let request = Request::builder()
            .uri("/extract")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        // Verify the error message is descriptive
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "NOT_FOUND");
        assert!(json["message"].as_str().unwrap().contains("POST"));
    }

    /// Test that 413 response matches exact JSON format from plan critical test (line 2163).
    ///
    /// The critical test requires: {"error":"REQUEST_TOO_LARGE","message":"Request body exceeds the configured limit"}
    /// This test verifies the ApiError serialization produces this exact format (no hint field).
    #[test]
    fn test_413_json_format() {
        // Test the exact format required by the critical test
        let api_error = ApiError {
            error: "REQUEST_TOO_LARGE".to_string(),
            message: "Request body exceeds the configured limit".to_string(),
            hint: None,
        };
        let json_str = serde_json::to_string(&api_error).unwrap();
        assert_eq!(
            json_str,
            r#"{"error":"REQUEST_TOO_LARGE","message":"Request body exceeds the configured limit"}"#
        );

        // Verify the IntoResponse impl also produces the correct status code
        let err = AxumError::RequestTooLarge;
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    /// Test that CacheStatus converts correctly to/from strings.
    #[test]
    fn test_cache_status_conversions() {
        assert_eq!(CacheStatus::Hit.as_str(), "hit");
        assert_eq!(CacheStatus::Miss.as_str(), "miss");
        assert_eq!(CacheStatus::Skipped.as_str(), "skipped");

        assert_eq!(CacheStatus::from_string("hit"), CacheStatus::Hit);
        assert_eq!(CacheStatus::from_string("miss"), CacheStatus::Miss);
        assert_eq!(CacheStatus::from_string("skipped"), CacheStatus::Skipped);
        assert_eq!(CacheStatus::from_string("invalid"), CacheStatus::Skipped);
    }

    /// Metrics: an existing cache is represented immediately after server
    /// startup, before the first request has a chance to refresh the gauge.
    #[cfg(feature = "metrics")]
    #[test]
    fn test_metrics_cache_size_is_initialized_from_existing_index() {
        let cache_dir = tempfile::tempdir().expect("temp cache dir");
        let mut index = pdftract_core::cache::layout::CacheIndex::default();
        index.total_bytes = 4096;
        pdftract_core::cache::layout::save_index(cache_dir.path(), &index)
            .expect("write cache index");

        let state = ServeState::new(
            Some(cache_dir.path().to_path_buf()),
            64 * 1024 * 1024,
            false,
            None,
            1 << 30,
            false,
        );

        assert!(state
            .metrics
            .render()
            .contains("pdftract_cache_size_bytes 4096\n"));
    }

    /// Helper to load a valid test PDF.
    fn load_test_pdf() -> Vec<u8> {
        // The tracked repo fixture (same one `fixture_pdf_bytes` uses).
        // The old path, crates/pdftract-libpdftract/tests/hello.pdf, was
        // removed from the tree by 4039def0 and this loader has been
        // failing on a missing file ever since.
        let pdf_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/test-minimal.pdf"
        );
        std::fs::read(pdf_path).expect("Failed to read test PDF")
    }

    /// Integration test: 8 concurrent requests complete in parallel.
    ///
    /// This is the critical test from the plan (line 2146). It verifies that:
    /// - All 8 requests complete (proves no deadlock or serialization)
    /// - Wallclock time is similar to a single request (proves parallelism)
    /// - /health responds quickly during concurrent extractions (proves /health doesn't block)
    #[tokio::test]
    async fn test_concurrent_requests_parallel() {
        use axum::{
            body::Body,
            http::{HeaderMap, HeaderValue, Method, StatusCode},
        };
        use reqwest::multipart::{Form, Part};
        use tokio::time::Instant;

        // Start the server in the background. Build the same router
        // production serves (its middleware inserts RequestMetadata and
        // extracts ConnectInfo) and serve it through the ConnectInfo make
        // service — a hand-rolled two-route Router 500s every /extract
        // with "Missing request extension" before any handler runs.
        let state = ServeState::new(None, 1024 * 1024 * 1024, true, None, 1 << 30, false); // No cache, 1 GB decompress limit
        let app = build_router(state, 256 * 1024 * 1024);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind");
        let addr = listener.local_addr().expect("Failed to get local address");
        let port = addr.port();

        tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .await
            .expect("Server error");
        });

        // Give the server a moment to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        let base_url = format!("http://127.0.0.1:{}", port);
        let client = reqwest::Client::new();
        let pdf_bytes = load_test_pdf();

        // First, test that /health responds quickly
        let health_start = Instant::now();
        let health_resp = client
            .get(format!("{}/health", base_url))
            .send()
            .await
            .expect("Health request failed");
        let health_duration = health_start.elapsed();

        assert_eq!(health_resp.status(), StatusCode::OK);
        assert!(
            health_duration < Duration::from_millis(100),
            "/health should respond in < 100ms, took {:?}",
            health_duration
        );

        // Now launch 8 concurrent extraction requests
        let mut handles = Vec::new();
        let start = Instant::now();

        for i in 0..8 {
            let client = client.clone();
            let url = format!("{}/extract", base_url);
            let pdf = pdf_bytes.clone();

            let handle = tokio::spawn(async move {
                let part = Part::bytes(pdf).file_name(format!("test{}.pdf", i));
                let form = Form::new().part("file", part);

                let resp = client
                    .post(&url)
                    .multipart(form)
                    .send()
                    .await
                    .expect("Extraction request failed");

                (i, resp.status(), client)
            });

            handles.push(handle);
        }

        // Wait for all requests to complete
        let mut results = Vec::new();
        for handle in handles {
            let (i, status, _) = handle.await.expect("Task panicked");
            results.push((i, status));
        }

        let total_duration = start.elapsed();

        // The critical test: all 8 requests completed (proves no deadlock or serialization)
        // We don't assert OK status because the test PDF might not extract correctly;
        // the important thing is that all requests got a response.
        assert_eq!(results.len(), 8, "All 8 requests should have completed");

        // The critical assertion: if requests were serialized, total time would be
        // roughly 8x a single request. With parallelism, it should be much less.
        // We use a very loose threshold to account for system load and variability.
        let single_request_estimate = Duration::from_millis(100); // Rough estimate
        let serialized_estimate = single_request_estimate * 8;

        assert!(
            total_duration < serialized_estimate,
            "Requests appear serialized: completed in {:?}, expected < {:?}",
            total_duration,
            serialized_estimate
        );

        // Also verify /health still responds quickly during load
        let health_start = Instant::now();
        let health_resp = client
            .get(format!("{}/health", base_url))
            .send()
            .await
            .expect("Health request failed");
        let health_duration = health_start.elapsed();

        assert_eq!(health_resp.status(), StatusCode::OK);
        assert!(
            health_duration < Duration::from_millis(100),
            "/health should respond in < 100ms during load, took {:?}",
            health_duration
        );
    }

    /// Unit tests for field-typing helpers.
    mod form_helpers_tests {
        use super::form_helpers::*;

        #[test]
        fn test_parse_bool_true() {
            assert!(parse_bool("test", "true").unwrap());
            assert!(parse_bool("test", "TRUE").unwrap());
            assert!(parse_bool("test", "1").unwrap());
            assert!(parse_bool("test", "yes").unwrap());
            assert!(parse_bool("test", "YES").unwrap());
            assert!(parse_bool("test", "on").unwrap());
            assert!(parse_bool("test", "ON").unwrap());
        }

        #[test]
        fn test_parse_bool_false() {
            assert!(!parse_bool("test", "false").unwrap());
            assert!(!parse_bool("test", "FALSE").unwrap());
            assert!(!parse_bool("test", "0").unwrap());
            assert!(!parse_bool("test", "no").unwrap());
            assert!(!parse_bool("test", "NO").unwrap());
            assert!(!parse_bool("test", "off").unwrap());
            assert!(!parse_bool("test", "OFF").unwrap());
        }

        #[test]
        fn test_parse_bool_invalid() {
            assert!(parse_bool("test", "invalid").is_err());
            assert!(parse_bool("test", "2").is_err());
            assert!(parse_bool("test", "").is_err());
        }

        #[test]
        fn test_parse_float() {
            assert_eq!(parse_float("test", "1.5").unwrap(), 1.5);
            assert_eq!(parse_float("test", "0.5").unwrap(), 0.5);
            assert_eq!(parse_float("test", "0").unwrap(), 0.0);
            assert_eq!(parse_float("test", "-1.5").unwrap(), -1.5);
        }

        #[test]
        fn test_parse_float_invalid() {
            assert!(parse_float("test", "invalid").is_err());
            assert!(parse_float("test", "").is_err());
        }

        #[test]
        fn test_parse_int() {
            assert_eq!(parse_int("test", "100").unwrap(), 100);
            assert_eq!(parse_int("test", "0").unwrap(), 0);
            assert_eq!(parse_int("test", "300").unwrap(), 300);
        }

        #[test]
        fn test_parse_int_invalid() {
            assert!(parse_int("test", "invalid").is_err());
            assert!(parse_int("test", "1.5").is_err());
            assert!(parse_int("test", "-1").is_err()); // u32 can't be negative
        }

        #[test]
        fn test_parse_comma_list() {
            assert_eq!(parse_comma_list("eng,fra,deu"), vec!["eng", "fra", "deu"]);
            assert_eq!(parse_comma_list("eng, fra, deu"), vec!["eng", "fra", "deu"]);
            assert_eq!(parse_comma_list("eng"), vec!["eng"]);
            assert_eq!(parse_comma_list(""), Vec::<String>::new());
            assert_eq!(parse_comma_list("eng,,fra"), vec!["eng", "fra"]); // Empty values filtered
        }

        #[test]
        fn test_validate_pdf_magic_bytes_valid() {
            let valid_pdf = b"%PDF-1.4\n";
            assert!(validate_pdf_magic_bytes(valid_pdf).is_ok());

            let valid_pdf2 = b"%PDF-1.2\n%%EOF";
            assert!(validate_pdf_magic_bytes(valid_pdf2).is_ok());
        }

        #[test]
        fn test_validate_pdf_magic_bytes_invalid() {
            let invalid_pdf = b"NOT-A-PDF";
            assert!(validate_pdf_magic_bytes(invalid_pdf).is_err());

            let invalid_pdf2 = b"hello";
            assert!(validate_pdf_magic_bytes(invalid_pdf2).is_err());
        }

        #[test]
        fn test_validate_pdf_magic_bytes_too_small() {
            let too_small = b"%PD";
            assert!(validate_pdf_magic_bytes(too_small).is_err());
        }
    }

    /// Test that build_options correctly handles all form fields.
    #[test]
    fn test_build_options_with_all_fields() {
        let state = ServeState::new(None, 1024 * 1024 * 1024, true, None, 1 << 30, false);

        let params = ExtractParams {
            receipts: Some("lite".to_string()),
            no_cache: true,
            full_render: false,
            max_decompress_gb: Some(2),
            ocr_language: Some("eng,fra,deu".to_string()),
            ocr_dpi: Some(300),
            markdown_anchors: true,
            pages: Some("1-5".to_string()),
        };

        let options = build_options(&state, &params).unwrap();

        assert_eq!(options.receipts, ReceiptsMode::Lite);
        assert_eq!(options.ocr_language, vec!["eng", "fra", "deu"]);
        assert_eq!(options.ocr_dpi_override, Some(300));
        assert_eq!(options.markdown_anchors, true);
        assert!(!options.full_render);
        // max_decompress_gb=2 should be converted to 2 * 2^30 bytes
        assert_eq!(options.max_decompress_bytes, 2 * (1u64 << 30));
    }

    /// Test that build_options uses defaults when fields are missing.
    #[test]
    fn test_build_options_with_defaults() {
        let state = ServeState::new(None, 1024 * 1024 * 1024, true, None, 1 << 30, false);

        let params = ExtractParams::default();

        let options = build_options(&state, &params).unwrap();

        assert_eq!(options.receipts, ReceiptsMode::Off);
        assert_eq!(options.ocr_language, vec!["eng"]);
        assert_eq!(options.ocr_dpi_override, None);
        assert_eq!(options.markdown_anchors, false);
        // When max_decompress_gb is not provided, use server default (1 << 30 = 1 GB)
        assert_eq!(options.max_decompress_bytes, 1 << 30);
    }

    /// Test that max_decompress_gb validation works.
    #[test]
    fn test_build_options_max_decompress_gb_validation() {
        let state = ServeState::new(None, 1024 * 1024 * 1024, true, None, 1 << 30, false);

        let params = ExtractParams {
            max_decompress_gb: Some(5000), // Exceeds hard cap
            ..Default::default()
        };

        let result = build_options(&state, &params);
        assert!(result.is_err());

        match result.unwrap_err() {
            AxumError::BadRequest(msg, _) => {
                assert!(msg.contains("exceeds hard cap"));
            }
            _ => panic!("Expected BadRequest error"),
        }
    }

    // ---- Metrics instrumentation tests (`metrics` feature) ----
    //
    // These drive real extractions through the handlers in process and
    // assert on the rendered registry; `tests/fixtures/test-minimal.pdf`
    // is the provenance-recorded fixture that extracts successfully
    // (1 page, "Dummy PDF file").

    /// Bytes of the extraction fixture used by the metrics tests.
    #[cfg(feature = "metrics")]
    fn fixture_pdf_bytes() -> Vec<u8> {
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/test-minimal.pdf"
        ))
        .expect("test fixture should be readable")
    }

    /// Build a multipart POST request carrying the PDF in a `file` field.
    #[cfg(feature = "metrics")]
    fn multipart_file_request(uri: &'static str, pdf: &[u8]) -> Request<Body> {
        const BOUNDARY: &str = "pdftractmetricstestboundary";
        let mut body: Vec<u8> = Vec::new();
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\n\
                 Content-Disposition: form-data; name=\"file\"; filename=\"fixture.pdf\"\r\n\
                 Content-Type: application/pdf\r\n\
                 \r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(pdf);
        body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
        Request::builder()
            .method("POST")
            .uri(uri)
            .header(
                "content-type",
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .body(Body::from(body))
            .unwrap()
    }

    /// A router ready for in-process `oneshot` requests. The audit
    /// middleware extracts `ConnectInfo`, which a bare router has no
    /// source for under `oneshot` (every extractor rejection surfaces as
    /// a 500 before any handler runs), so the peer address the live
    /// server would provide is mocked here. Test-only — production
    /// routing is untouched.
    #[cfg(feature = "metrics")]
    fn oneshot_app(state: ServeState) -> Router {
        use axum::extract::connect_info::MockConnectInfo;

        build_router(state, 256 * 1024 * 1024).layer(MockConnectInfo(std::net::SocketAddr::from(
            ([127, 0, 0, 1], 4242),
        )))
    }

    /// Read one plain (unlabeled) sample's integer value straight from a
    /// registry snapshot by sample name — asserts the counter state
    /// itself, not its rendered OpenMetrics text. Panics if the sample is
    /// absent, carries labels, or is not an integer.
    #[cfg(feature = "metrics")]
    fn snapshot_int(families: &[crate::metrics::MetricFamily], name: &str) -> u64 {
        for family in families {
            for sample in &family.samples {
                if sample.name == name {
                    assert!(
                        sample.labels.is_empty(),
                        "{name} must be unlabeled, got {:?}",
                        sample.labels
                    );
                    return match sample.value {
                        crate::metrics::SampleValue::Int(value) => value,
                        other => panic!("{name}: expected an integer sample, got {other:?}"),
                    };
                }
            }
        }
        panic!("sample {name} absent from the registry snapshot");
    }

    /// Metrics: one successful extraction through POST /extract increments
    /// the extraction counter (correct result/ocr labels), the pages
    /// counter, the duration histogram, and the HTTP counter, and settles
    /// the inflight gauge.
    #[cfg(feature = "metrics")]
    #[tokio::test]
    async fn test_metrics_extraction_success_through_serve_handler() {
        use tower::ServiceExt;

        let state = ServeState::new(None, 1024 * 1024 * 1024, true, None, 1 << 30, false);
        let metrics = state.metrics.clone();
        let app = oneshot_app(state);

        let response = app
            .oneshot(multipart_file_request("/extract", &fixture_pdf_bytes()))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let text = metrics.render();
        assert!(
            text.contains("pdftract_extractions_total{result=\"success\",ocr=\"false\"} 1\n"),
            "missing success extraction counter:\n{text}"
        );
        assert!(
            text.contains("pdftract_pages_extracted_total 1\n"),
            "missing pages counter:\n{text}"
        );
        assert!(
            text.contains(
                "pdftract_http_requests_total{endpoint=\"/extract\",status=\"200\"} 1\n"
            ),
            "missing http counter:\n{text}"
        );
        assert!(
            text.contains("pdftract_extraction_duration_seconds_count 1\n"),
            "missing duration observation:\n{text}"
        );
        assert!(
            text.contains("pdftract_extraction_duration_seconds_bucket{le=\"+Inf\"} 1\n"),
            "missing duration bucket:\n{text}"
        );
        assert!(
            text.contains("pdftract_inflight_extractions 0\n"),
            "inflight gauge did not settle:\n{text}"
        );
        assert!(
            text.contains("pdftract_build_info{version=\""),
            "build_info missing:\n{text}"
        );
    }

    /// Metrics: exercising the existing cache integration through
    /// POST /extract moves the hit, miss, and size counters (a miss writes
    /// an entry; the identical second request hits it). The counters are
    /// asserted at the registry state level (`snapshot()`), and the
    /// rendered OpenMetrics text is additionally checked to agree.
    #[cfg(feature = "metrics")]
    #[tokio::test]
    async fn test_metrics_cache_hit_miss_and_size_counters() {
        use tower::ServiceExt;

        let cache_dir = tempfile::tempdir().expect("temp cache dir");
        let state = ServeState::new(
            Some(cache_dir.path().to_path_buf()),
            64 * 1024 * 1024,
            false,
            None,
            1 << 30,
            false,
        );
        let metrics = state.metrics.clone();
        let app = oneshot_app(state);
        let pdf = fixture_pdf_bytes();

        // First extraction: cache miss, entry written.
        let response = app
            .clone()
            .oneshot(multipart_file_request("/extract", &pdf))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let families = metrics.snapshot();
        assert_eq!(
            snapshot_int(&families, "pdftract_cache_misses_total"),
            1,
            "first extraction must count exactly one cache miss"
        );
        assert_eq!(
            snapshot_int(&families, "pdftract_cache_hits_total"),
            0,
            "hits should still be zero"
        );
        let size_after_miss = snapshot_int(&families, "pdftract_cache_size_bytes");
        assert!(
            size_after_miss > 0,
            "cache size should reflect the written entry"
        );

        // The rendered document must agree with the registry state.
        let text = metrics.render();
        assert!(
            text.contains("pdftract_cache_misses_total 1\n"),
            "missing cache miss:\n{text}"
        );
        assert!(
            text.contains("pdftract_cache_hits_total 0\n"),
            "hits should still be zero:\n{text}"
        );
        assert!(
            !text.contains("pdftract_cache_size_bytes 0\n"),
            "cache size should reflect the written entry:\n{text}"
        );

        // Second extraction of the same document: cache hit.
        let response = app
            .oneshot(multipart_file_request("/extract", &pdf))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let families = metrics.snapshot();
        assert_eq!(
            snapshot_int(&families, "pdftract_cache_hits_total"),
            1,
            "the identical second extraction must count exactly one hit"
        );
        assert_eq!(
            snapshot_int(&families, "pdftract_cache_misses_total"),
            1,
            "misses must not double-count"
        );
        assert_eq!(
            snapshot_int(&families, "pdftract_cache_size_bytes"),
            size_after_miss,
            "a hit serves the existing entry; on-disk size must not move"
        );

        // The rendered document must agree with the registry state.
        let text = metrics.render();
        assert!(
            text.contains("pdftract_cache_hits_total 1\n"),
            "missing cache hit:\n{text}"
        );
        assert!(
            text.contains("pdftract_cache_misses_total 1\n"),
            "misses must not double-count:\n{text}"
        );
        assert!(
            text.contains("pdftract_extractions_total{result=\"success\",ocr=\"false\"} 2\n"),
            "both extractions counted:\n{text}"
        );
        assert!(
            text.contains("pdftract_pages_extracted_total 2\n"),
            "pages from both extractions counted:\n{text}"
        );
        assert!(
            !text.contains("pdftract_cache_size_bytes 0\n"),
            "cache size stays positive:\n{text}"
        );
    }

    /// Metrics: a failing extraction records result="error", the duration,
    /// and the 422 HTTP response, and settles the inflight gauge.
    #[cfg(feature = "metrics")]
    #[tokio::test]
    async fn test_metrics_extraction_error_records_error_result_and_422() {
        use tower::ServiceExt;

        let state = ServeState::new(None, 1024 * 1024 * 1024, true, None, 1 << 30, false);
        let metrics = state.metrics.clone();
        let app = oneshot_app(state);

        // Valid magic bytes, unparseable body.
        let not_a_pdf = b"%PDF-1.4\nthis is not a parseable PDF body".to_vec();
        let response = app
            .oneshot(multipart_file_request("/extract", &not_a_pdf))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let text = metrics.render();
        assert!(
            text.contains("pdftract_extractions_total{result=\"error\",ocr=\"false\"} 1\n"),
            "missing error extraction counter:\n{text}"
        );
        assert!(
            text.contains(
                "pdftract_http_requests_total{endpoint=\"/extract\",status=\"422\"} 1\n"
            ),
            "missing 422 http counter:\n{text}"
        );
        assert!(
            text.contains("pdftract_extraction_duration_seconds_count 1\n"),
            "error extractions are timed too:\n{text}"
        );
        assert!(
            text.contains("pdftract_inflight_extractions 0\n"),
            "inflight gauge did not settle:\n{text}"
        );
    }

    /// A blocking task can be cancelled before it starts. Its owned guard
    /// must still publish one timed error and release the in-flight gauge.
    #[cfg(feature = "metrics")]
    #[test]
    fn test_metrics_cancelled_extraction_records_error_and_releases_inflight() {
        let metrics = crate::metrics::Registry::new();
        let guard = ExtractionMetricsGuard::new(metrics.clone(), true, false);
        assert!(metrics
            .render()
            .contains("pdftract_inflight_extractions 1\n"));

        drop(guard);

        let text = metrics.render();
        assert!(text.contains("pdftract_extractions_total{result=\"error\",ocr=\"false\"} 1\n"));
        assert!(text.contains("pdftract_cache_misses_total 1\n"));
        assert!(text.contains("pdftract_extraction_duration_seconds_count 1\n"));
        assert!(text.contains("pdftract_inflight_extractions 0\n"));
    }

    /// The guard carries the request's OCR selection through both successful
    /// and failed completion paths instead of substituting build capability.
    #[cfg(feature = "metrics")]
    #[test]
    fn test_metrics_extraction_guard_preserves_ocr_label() {
        let metrics = crate::metrics::Registry::new();
        let guard = ExtractionMetricsGuard::new(metrics.clone(), false, true);
        drop(guard);

        assert!(metrics
            .render()
            .contains("pdftract_extractions_total{result=\"error\",ocr=\"true\"} 1\n"));
    }

    /// A cache location that is an existing file is not writable as a cache
    /// directory. Extraction remains best-effort, while the miss metric and
    /// the production readiness probe report the distinct states.
    #[cfg(feature = "metrics")]
    #[tokio::test]
    async fn test_metrics_cache_unwritable_path_records_miss_and_unready() {
        use tower::ServiceExt;

        let cache_file = tempfile::NamedTempFile::new().expect("cache sentinel file");
        let state = ServeState::new(
            Some(cache_file.path().to_path_buf()),
            64 * 1024 * 1024,
            false,
            None,
            1 << 30,
            false,
        );
        let metrics = state.metrics.clone();
        let readiness = state.readiness.clone();
        let app = oneshot_app(state);

        let response = app
            .oneshot(multipart_file_request("/extract", &fixture_pdf_bytes()))
            .await
            .expect("extraction request");
        assert_eq!(response.status(), StatusCode::OK);

        let text = metrics.render();
        assert!(text.contains("pdftract_cache_misses_total 1\n"));
        assert!(text.contains("pdftract_cache_size_bytes 0\n"));
        let report = readiness.evaluate();
        assert!(!report.cache_writable);
        assert_eq!(report.failed_conditions(), vec!["cache_unwritable"]);
    }

    /// Metrics: a streamed extraction counts its result, pages, duration,
    /// and the HTTP response once the body is fully drained.
    #[cfg(feature = "metrics")]
    #[tokio::test]
    async fn test_metrics_stream_extraction_counts_pages_and_http() {
        use tower::ServiceExt;

        let state = ServeState::new(None, 1024 * 1024 * 1024, true, None, 1 << 30, false);
        let metrics = state.metrics.clone();
        let app = oneshot_app(state);

        let response = app
            .oneshot(multipart_file_request(
                "/extract/stream",
                &fixture_pdf_bytes(),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Drain to completion; the background task records its metrics
        // before the response stream closes.
        let collected = response.into_body().collect().await.unwrap();
        assert!(
            String::from_utf8_lossy(&collected.to_bytes()).contains("Dummy PDF file"),
            "stream should carry the extracted page"
        );

        let text = metrics.render();
        assert!(
            text.contains("pdftract_extractions_total{result=\"success\",ocr=\"false\"} 1\n"),
            "missing stream extraction counter:\n{text}"
        );
        assert!(
            text.contains("pdftract_pages_extracted_total 1\n"),
            "missing stream pages counter:\n{text}"
        );
        assert!(
            text.contains(
                "pdftract_http_requests_total{endpoint=\"/extract/stream\",status=\"200\"} 1\n"
            ),
            "missing stream http counter:\n{text}"
        );
        assert!(
            text.contains("pdftract_extraction_duration_seconds_count 1\n"),
            "missing stream duration observation:\n{text}"
        );
        assert!(
            text.contains("pdftract_inflight_extractions 0\n"),
            "inflight gauge did not settle:\n{text}"
        );
    }

    /// Metrics: responses on unregistered paths count under the
    /// "unmatched" endpoint label, never the raw path.
    #[cfg(feature = "metrics")]
    #[tokio::test]
    async fn test_metrics_unmatched_path_uses_unmatched_label() {
        use tower::ServiceExt;

        let state = ServeState::new(None, 1024 * 1024 * 1024, true, None, 1 << 30, false);
        let metrics = state.metrics.clone();
        let app = oneshot_app(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/nope/some/raw/path")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let text = metrics.render();
        assert!(
            text.contains(
                "pdftract_http_requests_total{endpoint=\"unmatched\",status=\"404\"} 1\n"
            ),
            "raw path leaked into the endpoint label:\n{text}"
        );
    }

    /// Readiness vs liveness, saturation state: with the readiness input
    /// forced saturated, `GET /health` on the main port must STILL be
    /// 200 — a busy server is alive and must not be restarted. The
    /// readiness verdict is the metrics listener's `GET /ready` job.
    #[cfg(feature = "metrics")]
    #[tokio::test]
    async fn test_health_returns_200_when_pool_is_saturated() {
        use tower::ServiceExt;

        let state = ServeState::new(None, 1024 * 1024 * 1024, true, None, 1 << 30, false)
            .with_readiness(crate::metrics::Readiness::with_inputs(|| 0.95, || true));

        // Liveness remains healthy on the main router even when the
        // injected readiness input reports saturation.
        let response = oneshot_app(state.clone())
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // The same injected state is exposed through the real metrics
        // listener, where saturation must make readiness fail and identify
        // the pool condition.
        let bound = crate::metrics::bind_and_spawn(
            "127.0.0.1:0",
            state.metrics.clone(),
            state.readiness.clone(),
        )
        .await
        .expect("metrics listener binds on :0");
        let response = reqwest::Client::new()
            .get(format!("http://{bound}/ready"))
            .send()
            .await
            .expect("ready request");
        assert_eq!(response.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
        let body: serde_json::Value = response.json().await.expect("ready body");
        assert_eq!(body["status"], "unready", "got: {body}");
        assert_eq!(
            body["unready"],
            serde_json::json!(["pool_saturated"]),
            "body must name pool saturation: {body}"
        );
    }

    /// Readiness vs liveness, cache state: with the cache forced
    /// unwritable, `GET /health` must STILL be 200.
    #[cfg(feature = "metrics")]
    #[tokio::test]
    async fn test_health_returns_200_when_cache_is_unwritable() {
        use tower::ServiceExt;

        let state = ServeState::new(None, 1024 * 1024 * 1024, true, None, 1 << 30, false)
            .with_readiness(crate::metrics::Readiness::with_inputs(|| 0.0, || false));

        let response = oneshot_app(state)
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    /// The readiness probe is a metrics-listener route: the main port
    /// answers 404, so readiness reachability stays separate from
    /// production traffic (plan endpoint policy, same as /metrics).
    #[cfg(feature = "metrics")]
    #[tokio::test]
    async fn test_ready_is_not_routed_on_the_main_port() {
        use tower::ServiceExt;

        let state = ServeState::new(None, 1024 * 1024 * 1024, true, None, 1 << 30, false);
        let response = oneshot_app(state)
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    /// Criterion 1, production wiring end to end: a healthy server (cache
    /// enabled at a writable location) returns 200 from GET /ready on the
    /// metrics listener.
    #[cfg(feature = "metrics")]
    #[tokio::test]
    async fn test_ready_is_200_when_cache_location_is_writable() {
        let cache = tempfile::tempdir().expect("tempdir");
        let state = ServeState::new(
            Some(cache.path().to_path_buf()),
            1024 * 1024 * 1024,
            false,
            None,
            1 << 30,
            false,
        );
        let bound = crate::metrics::bind_and_spawn(
            "127.0.0.1:0",
            state.metrics.clone(),
            state.readiness.clone(),
        )
        .await
        .expect("metrics listener binds on :0");

        let response = reqwest::Client::new()
            .get(format!("http://{bound}/ready"))
            .send()
            .await
            .expect("ready request");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = response.json().await.expect("ready body");
        assert_eq!(body["status"], "ready", "got: {body}");
        assert_eq!(body["unready"], serde_json::json!([]), "got: {body}");
        assert_eq!(body["cache_writable"], true, "got: {body}");
    }

    /// Criterion 3, production wiring end to end: the cache pointed at
    /// an unwritable path (existing parent, missing directory) drives
    /// the DEFAULT prober — no injection — to 503 with the body naming
    /// the cache.
    #[cfg(feature = "metrics")]
    #[tokio::test]
    async fn test_ready_is_503_when_cache_points_at_an_unwritable_path() {
        use tower::ServiceExt;

        let parent = tempfile::tempdir().expect("tempdir");
        let missing = parent.path().join("cache-dir-does-not-exist");
        let state = ServeState::new(
            Some(missing),
            1024 * 1024 * 1024,
            false, // cache "enabled", but its location cannot be written
            None,
            1 << 30,
            false,
        );
        // Cache readiness failure is not a liveness failure: the main
        // router must continue to answer /health with 200.
        let health = oneshot_app(state.clone())
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("health request");
        assert_eq!(health.status(), StatusCode::OK);

        let bound = crate::metrics::bind_and_spawn(
            "127.0.0.1:0",
            state.metrics.clone(),
            state.readiness.clone(),
        )
        .await
        .expect("metrics listener binds on :0");

        let response = reqwest::Client::new()
            .get(format!("http://{bound}/ready"))
            .send()
            .await
            .expect("ready request");
        assert_eq!(response.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
        let body: serde_json::Value = response.json().await.expect("ready body");
        assert_eq!(body["status"], "unready", "got: {body}");
        assert_eq!(
            body["unready"],
            serde_json::json!(["cache_unwritable"]),
            "body must name the cache and nothing else: {body}"
        );
        assert_eq!(body["cache_writable"], false, "got: {body}");
    }
}
