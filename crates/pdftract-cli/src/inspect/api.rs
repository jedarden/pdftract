//! API handlers for the inspector debug viewer.
//!
//! This module implements Phase 7.9.2's HTTP API endpoints:
//! - GET /api/document - Document-level metadata
//! - GET /api/page/{i} - Per-page JSON with spans/blocks/columns
//! - GET /api/page/{i}/svg - Full SVG render with overlays
//! - GET /api/page/{i}/thumbnail - Thumbnail SVG for sidebar
//! - GET /api/raster/{i}.png - Base64 PNG for scanned pages
//! - GET /api/search?q=... - Search across spans

use super::inspect::InspectorState;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response as AxumResponse},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

/// Query parameters for the search endpoint.
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    /// Search query string
    q: Option<String>,
}

/// Search result match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
    /// Page index containing the match
    pub page_index: usize,
    /// Span index within the page
    pub span_index: usize,
    /// Bounding box of the matching span
    pub bbox: [f64; 4],
    /// The matched text
    pub text: String,
}

/// API error response.
#[derive(Debug, Serialize)]
pub struct ApiError {
    /// Error code
    pub error: String,
    /// Human-readable message
    pub message: String,
}

/// Handler for GET /api/document - returns document-level metadata.
pub async fn api_document(
    State(state): State<Arc<tokio::sync::Mutex<InspectorState>>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    check_auth(&state, &headers)?;

    let state_guard = state.lock().await;
    Ok(Json(state_guard.document_a.clone()))
}

/// Handler for GET /api/page/{i} - returns per-page JSON.
pub async fn api_page(
    State(state): State<Arc<tokio::sync::Mutex<InspectorState>>>,
    Path(page_index): Path<usize>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    check_auth(&state, &headers)?;

    let state_guard = state.lock().await;

    // Get pages from document_a
    let pages = state_guard
        .document_a
        .get("pages")
        .and_then(|p| p.as_array())
        .ok_or_else(|| ApiError {
            error: "INTERNAL_ERROR".to_string(),
            message: "No pages in document".to_string(),
        })?;

    // Validate page index
    if page_index >= pages.len() {
        return Err(ApiError {
            error: "NOT_FOUND".to_string(),
            message: format!(
                "Page {} not found (document has {} pages)",
                page_index,
                pages.len()
            ),
        });
    }

    Ok(Json(pages[page_index].clone()))
}

/// Handler for GET /api/page/{i}/svg - returns SVG render with overlays.
pub async fn api_page_svg(
    State(state): State<Arc<tokio::sync::Mutex<InspectorState>>>,
    Path(page_index): Path<usize>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    check_auth(&state, &headers)?;

    let state_guard = state.lock().await;

    // Get pages from document_a
    let pages = state_guard
        .document_a
        .get("pages")
        .and_then(|p| p.as_array())
        .ok_or_else(|| ApiError {
            error: "INTERNAL_ERROR".to_string(),
            message: "No pages in document".to_string(),
        })?;

    // Validate page index
    if page_index >= pages.len() {
        return Err(ApiError {
            error: "NOT_FOUND".to_string(),
            message: format!("Page {} not found", page_index),
        });
    }

    // Get page dimensions
    let page = &pages[page_index];
    let width = page.get("width").and_then(|w| w.as_f64()).unwrap_or(612.0);
    let height = page.get("height").and_then(|h| h.as_f64()).unwrap_or(792.0);

    // Render SVG with all overlay layers
    let svg = render_page_svg(page, width, height, false);

    let response = AxumResponse::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "image/svg+xml")
        .body(axum::body::Body::from(svg))
        .map_err(|e| ApiError {
            error: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to build response: {}", e),
        })?;

    Ok(response)
}

/// Handler for GET /api/page/{i}/thumbnail - returns thumbnail SVG.
pub async fn api_page_thumbnail(
    State(state): State<Arc<tokio::sync::Mutex<InspectorState>>>,
    Path(page_index): Path<usize>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    check_auth(&state, &headers)?;

    let state_guard = state.lock().await;

    // Get pages from document_a
    let pages = state_guard
        .document_a
        .get("pages")
        .and_then(|p| p.as_array())
        .ok_or_else(|| ApiError {
            error: "INTERNAL_ERROR".to_string(),
            message: "No pages in document".to_string(),
        })?;

    // Validate page index
    if page_index >= pages.len() {
        return Err(ApiError {
            error: "NOT_FOUND".to_string(),
            message: format!("Page {} not found", page_index),
        });
    }

    // Get page dimensions
    let page = &pages[page_index];
    let width = page.get("width").and_then(|w| w.as_f64()).unwrap_or(612.0);
    let height = page.get("height").and_then(|h| h.as_f64()).unwrap_or(792.0);

    // Render thumbnail SVG (200px wide, reduced detail)
    let scale = 200.0 / width;
    let thumb_width = 200.0;
    let thumb_height = height * scale;
    let svg = render_page_svg(page, thumb_width, thumb_height, true);

    let response = AxumResponse::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "image/svg+xml")
        .body(axum::body::Body::from(svg))
        .map_err(|e| ApiError {
            error: "INTERNAL_ERROR".to_string(),
            message: format!("Failed to build response: {}", e),
        })?;

    Ok(response)
}

/// Handler for GET /api/raster/{i}.png - returns base64 PNG for scanned pages.
pub async fn api_raster(
    State(state): State<Arc<tokio::sync::Mutex<InspectorState>>>,
    Path(page_index): Path<usize>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    check_auth(&state, &headers)?;

    let state_guard = state.lock().await;

    // Get pages from document_a
    let pages = state_guard
        .document_a
        .get("pages")
        .and_then(|p| p.as_array())
        .ok_or_else(|| ApiError {
            error: "INTERNAL_ERROR".to_string(),
            message: "No pages in document".to_string(),
        })?;

    // Validate page index
    if page_index >= pages.len() {
        return Err(ApiError {
            error: "NOT_FOUND".to_string(),
            message: format!("Page {} not found", page_index),
        });
    }

    // Check if page has raster (scanned content)
    let page = &pages[page_index];
    let raster = page.get("raster").and_then(|r| r.as_str());

    if let Some(base64_png) = raster {
        // Return the base64 PNG data
        let png_data = base64_decode_to_bytes(base64_png);
        let response = AxumResponse::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "image/png")
            .body(axum::body::Body::from(png_data))
            .map_err(|e| ApiError {
                error: "INTERNAL_ERROR".to_string(),
                message: format!("Failed to build response: {}", e),
            })?;
        Ok(response)
    } else {
        // No raster on this page (vector page)
        Err(ApiError {
            error: "NOT_FOUND".to_string(),
            message: "Page is vector (no raster content)".to_string(),
        })
    }
}

/// Handler for GET /api/search?q=... - search across spans.
pub async fn api_search(
    State(state): State<Arc<tokio::sync::Mutex<InspectorState>>>,
    Query(params): Query<SearchQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    check_auth(&state, &headers)?;

    let query = params.q.unwrap_or_default();
    if query.is_empty() {
        return Ok(Json(Vec::<SearchMatch>::new()));
    }

    let state_guard = state.lock().await;

    // Get pages from document_a
    let pages = state_guard
        .document_a
        .get("pages")
        .and_then(|p| p.as_array())
        .ok_or_else(|| ApiError {
            error: "INTERNAL_ERROR".to_string(),
            message: "No pages in document".to_string(),
        })?;

    let mut matches = Vec::new();
    let query_lower = query.to_lowercase();

    // Search through all pages
    for (page_index, page) in pages.iter().enumerate() {
        let spans = page.get("spans").and_then(|s| s.as_array());

        if let Some(spans) = spans {
            for (span_index, span) in spans.iter().enumerate() {
                let text = span.get("text").and_then(|t| t.as_str()).unwrap_or("");
                let bbox = span.get("bbox").and_then(|b| {
                    b.as_array().and_then(|arr| {
                        let nums: Vec<Option<f64>> = arr.iter().map(|v| v.as_f64()).collect();
                        if nums.len() == 4 && nums.iter().all(|o| o.is_some()) {
                            Some([
                                nums[0].unwrap(),
                                nums[1].unwrap(),
                                nums[2].unwrap(),
                                nums[3].unwrap(),
                            ])
                        } else {
                            None
                        }
                    })
                });

                // Case-insensitive substring match
                if text.to_lowercase().contains(&query_lower) {
                    if let Some(bbox) = bbox {
                        matches.push(SearchMatch {
                            page_index,
                            span_index,
                            bbox,
                            text: text.to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok(Json(matches))
}

/// Check authentication if token is configured.
fn check_auth(
    state: &tokio::sync::Mutex<InspectorState>,
    headers: &HeaderMap,
) -> Result<(), ApiError> {
    // Get auth token from state (requires lock)
    // Note: This is a synchronous check, so we use try_lock to avoid deadlock
    let state_guard = state.try_lock().map_err(|_| ApiError {
        error: "INTERNAL_ERROR".to_string(),
        message: "State lock contention".to_string(),
    })?;

    if let Some(ref token) = state_guard.auth_token {
        let auth_header = headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| ApiError {
                error: "UNAUTHORIZED".to_string(),
                message: "Missing Authorization header".to_string(),
            })?;

        // Check Bearer token format
        if !auth_header.starts_with("Bearer ") {
            return Err(ApiError {
                error: "UNAUTHORIZED".to_string(),
                message: "Invalid Authorization header format (expected 'Bearer <token>')"
                    .to_string(),
            });
        }

        let provided_token = &auth_header[7..]; // Skip "Bearer "
        if provided_token != token {
            return Err(ApiError {
                error: "UNAUTHORIZED".to_string(),
                message: "Invalid token".to_string(),
            });
        }
    }

    Ok(())
}

/// Render a page as SVG with all overlay layers.
fn render_page_svg(page: &JsonValue, width: f64, height: f64, thumbnail: bool) -> String {
    // Get page data
    let spans = page.get("spans").and_then(|s| s.as_array());
    let blocks = page.get("blocks").and_then(|b| b.as_array());

    let mut svg_layers = Vec::new();

    // Render each layer (these functions are defined in the render modules)
    // For now, we'll create a basic SVG structure
    // The full implementation will call the render functions from the render/ modules

    // Spans layer
    if let Some(spans_array) = spans {
        // TODO: call render::spans::render_spans()
        // For now, placeholder
        if !thumbnail {
            svg_layers.push(r#"<g class="layer-spans"></g>"#.to_string());
        }
    }

    // Blocks layer
    if let Some(blocks_array) = blocks {
        // TODO: call render::blocks::render_blocks()
        if !thumbnail {
            svg_layers.push(r#"<g class="layer-blocks"></g>"#.to_string());
        }
    }

    // Other layers (columns, reading_order, confidence_heatmap, ocr, mcid, anchors)
    // TODO: add remaining layers

    let layers_html = svg_layers.join("\n");

    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="{}" height="{}">
<rect width="100%" height="100%" fill="white"/>
{}
</svg>"#,
        width, height, width, height, layers_html
    )
}

/// Decode a base64 string to bytes.
fn base64_decode_to_bytes(input: &str) -> Vec<u8> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(input)
        .unwrap_or_default()
}

impl IntoResponse for ApiError {
    fn into_response(self) -> AxumResponse {
        let status = match self.error.as_str() {
            "UNAUTHORIZED" => StatusCode::UNAUTHORIZED,
            "NOT_FOUND" => StatusCode::NOT_FOUND,
            "BAD_REQUEST" => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(self)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_match_serialization() {
        let m = SearchMatch {
            page_index: 0,
            span_index: 5,
            bbox: [100.0, 200.0, 300.0, 250.0],
            text: "hello world".to_string(),
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("hello world"));
    }

    #[test]
    fn test_base64_decode() {
        let input = "SGVsbG8gV29ybGQ="; // "Hello World" in base64
        let bytes = base64_decode_to_bytes(input);
        assert_eq!(String::from_utf8(bytes).unwrap(), "Hello World");
    }
}
