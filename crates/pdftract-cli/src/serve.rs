//! HTTP serve mode for pdftract.
//!
//! This module implements Phase 6.4's `pdftract serve` subcommand: a long-running
//! HTTP service for multi-tenant extraction with cache integration.
//!
//! # Endpoints
//!
//! - `POST /extract` — Extract and return JSON with cache status in response body
//! - `POST /extract/text` — Extract and return plain text with X-Pdftract-Cache header
//! - `POST /extract/stream` — Extract and return streaming NDJSON with X-Pdftract-Cache header
//!
//! # Cache headers
//!
//! All endpoints return `X-Pdftract-Cache: hit | miss | skipped` header:
//! - `hit`: Served from cache
//! - `miss`: Ran extraction; populated cache
//! - `skipped`: Cache not configured or --no-cache equivalent

use anyhow::{Context, Result};
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Multipart, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Json, Response as AxumResponse},
    routing::{get, post},
    Router,
};
use pdftract_core::options::{ExtractionOptions, ReceiptsMode};
use pdftract_core::extract::{extract_pdf, result_to_json};
use pdftract_core::cache;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::limit::RequestBodyLimitLayer;

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
}

impl ServeState {
    /// Create a new serve state.
    pub fn new(cache_dir: Option<PathBuf>, cache_size_bytes: u64, cache_disabled: bool) -> Self {
        let cache = CacheState {
            cache_dir,
            cache_size_bytes,
            cache_disabled,
        };
        Self {
            cache: Arc::new(Mutex::new(cache)),
        }
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

/// Extraction request parameters.
#[derive(Debug, Deserialize)]
struct ExtractParams {
    /// Receipts mode (off, lite, svg)
    #[serde(default)]
    receipts: String,
    /// Disable cache for this request
    #[serde(default)]
    no_cache: bool,
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
pub async fn run(
    bind_addr: String,
    cache_dir: Option<PathBuf>,
    cache_size_bytes: u64,
    cache_disabled: bool,
    max_upload_mb: usize,
) -> Result<()> {
    let cache_dir_for_logging = cache_dir.as_deref();
    let state = ServeState::new(cache_dir.clone(), cache_size_bytes, cache_disabled);

    let max_body_bytes = max_upload_mb * 1024 * 1024;

    let app = Router::new()
        .route("/", get(root_handler))
        .route("/extract", post(extract_handler))
        .route("/extract/text", post(extract_text_handler))
        .route("/extract/stream", post(extract_stream_handler))
        .route("/health", get(health_handler))
        .layer(DefaultBodyLimit::max(max_body_bytes))
        .layer(RequestBodyLimitLayer::new(max_body_bytes))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await
        .context(format!("Failed to bind to {}", bind_addr))?;

    eprintln!("pdftract serve listening on http://{}", bind_addr);
    if let Some(dir) = cache_dir_for_logging {
        eprintln!("Cache enabled: {} (max {} bytes)", dir.display(), cache_size_bytes);
    } else {
        eprintln!("Cache disabled");
    }

    axum::serve(listener, app).await
        .context("HTTP server error")?;

    Ok(())
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
    (StatusCode::OK, "OK")
}

/// Extract handler - returns JSON with cache status in metadata.
async fn extract_handler(
    State(state): State<ServeState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AxumError> {
    let (pdf_file, params) = receive_pdf(&mut multipart).await?;
    let options = build_options(&params);

    // Get cache configuration
    let cache_state = state.cache.lock().await;
    let cache_dir = cache_state.cache_dir.clone();
    let cache_size_bytes = cache_state.cache_size_bytes;
    let cache_disabled = params.no_cache || cache_state.cache_disabled || cache_dir.is_none();
    drop(cache_state);

    // Perform extraction with cache integration
    let pdf_file_clone = pdf_file.clone();
    let (result, cache_status, cache_age) = tokio::task::spawn_blocking(move || {
        let cache_dir_ref = cache_dir.as_deref();
        cache::extract_with_cache(&pdf_file_clone, &options, cache_dir_ref, cache_disabled, Some(cache_size_bytes))
            .map_err(|e| AxumError::Extraction(format!("{:?}", e)))
    })
    .await
    .map_err(|e| AxumError::Internal(format!("{:?}", e)))?
    .map_err(|e| AxumError::Extraction(format!("{:?}", e)))?;

    // Build JSON response with cache status
    let mut result = result;
    result.metadata.cache_status = Some(cache_status.clone());
    result.metadata.cache_age_seconds = cache_age;

    let json = result_to_json(&result);

    let response = AxumResponse::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("X-Pdftract-Cache", CacheStatus::from_string(&cache_status).header_value())
        .body(Body::from(serde_json::to_string(&json).unwrap()))
        .map_err(|e| AxumError::Internal(format!("{:?}", e)))?;

    Ok(response)
}

/// Extract text handler - returns plain text with X-Pdftract-Cache header.
async fn extract_text_handler(
    State(state): State<ServeState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AxumError> {
    let (pdf_file, params) = receive_pdf(&mut multipart).await?;
    let options = build_options(&params);

    // Get cache configuration
    let cache_state = state.cache.lock().await;
    let cache_dir = cache_state.cache_dir.clone();
    let cache_size_bytes = cache_state.cache_size_bytes;
    let cache_disabled = params.no_cache || cache_state.cache_disabled || cache_dir.is_none();
    drop(cache_state);

    let (result, cache_status, _cache_age) = tokio::task::spawn_blocking(move || {
        let cache_dir_ref = cache_dir.as_deref();
        cache::extract_with_cache(&pdf_file, &options, cache_dir_ref, cache_disabled, Some(cache_size_bytes))
            .map_err(|e| AxumError::Extraction(format!("{:?}", e)))
    })
    .await
    .map_err(|e| AxumError::Internal(format!("{:?}", e)))?
    .map_err(|e| AxumError::Extraction(format!("{:?}", e)))?;

    let mut text = String::new();
    for page in &result.pages {
        for span in &page.spans {
            text.push_str(&span.text);
            text.push('\n');
        }
    }

    let response = AxumResponse::builder()
        .status(StatusCode::OK)
        .header("X-Pdftract-Cache", CacheStatus::from_string(&cache_status).header_value())
        .body(Body::from(text))
        .map_err(|e| AxumError::Internal(format!("{:?}", e)))?;

    Ok(response)
}

/// Extract stream handler - returns NDJSON with X-Pdftract-Cache header (always "skipped" for streaming).
async fn extract_stream_handler(
    State(state): State<ServeState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AxumError> {
    let (pdf_file, params) = receive_pdf(&mut multipart).await?;
    let options = build_options(&params);

    // Get cache configuration
    let cache_state = state.cache.lock().await;
    let cache_dir = cache_state.cache_dir.clone();
    let cache_size_bytes = cache_state.cache_size_bytes;
    let cache_disabled = params.no_cache || cache_state.cache_disabled || cache_dir.is_none();
    drop(cache_state);

    let (result, _cache_status, _cache_age) = tokio::task::spawn_blocking(move || {
        let cache_dir_ref = cache_dir.as_deref();
        cache::extract_with_cache(&pdf_file, &options, cache_dir_ref, cache_disabled, Some(cache_size_bytes))
            .map_err(|e| AxumError::Extraction(format!("{:?}", e)))
    })
    .await
    .map_err(|e| AxumError::Internal(format!("{:?}", e)))?
    .map_err(|e| AxumError::Extraction(format!("{:?}", e)))?;

    // Build NDJSON output
    let mut ndjson = String::new();
    for page in &result.pages {
        let page_json = serde_json::json!({
            "index": page.index,
            "spans": page.spans,
            "blocks": page.blocks,
        });
        ndjson.push_str(&serde_json::to_string(&page_json).unwrap());
        ndjson.push('\n');
    }

    let response = AxumResponse::builder()
        .status(StatusCode::OK)
        .header("X-Pdftract-Cache", CacheStatus::Skipped.header_value())
        .header("Content-Type", "application/x-ndjson")
        .body(Body::from(ndjson))
        .map_err(|e| AxumError::Internal(format!("{:?}", e)))?;

    Ok(response)
}

/// Receive uploaded PDF file and extraction parameters.
async fn receive_pdf(multipart: &mut Multipart) -> Result<(PathBuf, ExtractParams), AxumError> {
    let mut pdf_path = None;
    let mut params = ExtractParams {
        receipts: "off".to_string(),
        no_cache: false,
    };

    while let Some(field) = multipart.next_field().await
        .map_err(|e| AxumError::Internal(format!("{:?}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" || name == "pdf" {
            let data = field.bytes().await
                .map_err(|e| AxumError::Internal(format!("{:?}", e)))?;

            // Create a temp file that will persist for the duration of the request
            let temp_dir = std::env::temp_dir();
            let temp_file = temp_dir.join(format!("pdftract-upload-{}.pdf", uuid::Uuid::new_v4()));
            tokio::fs::write(&temp_file, &data).await
                .map_err(|e| AxumError::Internal(format!("{:?}", e)))?;
            pdf_path = Some(temp_file);
        } else if name == "receipts" {
            if let Ok(value) = field.text().await {
                params.receipts = value;
            }
        } else if name == "no_cache" {
            params.no_cache = true;
        }
    }

    let pdf_path = pdf_path.ok_or_else(|| AxumError::BadRequest("No PDF file uploaded".to_string()))?;

    Ok((pdf_path, params))
}

/// Build extraction options from parameters.

/// Build extraction options from parameters.
fn build_options(params: &ExtractParams) -> ExtractionOptions {
    let receipts_mode = match params.receipts.as_str() {
        "lite" => ReceiptsMode::Lite,
        "svg" => ReceiptsMode::SvgClip,
        _ => ReceiptsMode::Off,
    };
    ExtractionOptions::with_receipts(receipts_mode)
}

/// Error types for the HTTP server.
#[derive(Debug)]
pub enum AxumError {
    BadRequest(String),
    Extraction(String),
    Internal(String),
}

impl IntoResponse for AxumError {
    fn into_response(self) -> AxumResponse {
        let (status, message) = match self {
            AxumError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AxumError::Extraction(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
            AxumError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = serde_json::json!({
            "error": message,
        });

        (status, Json(body)).into_response()
    }
}
