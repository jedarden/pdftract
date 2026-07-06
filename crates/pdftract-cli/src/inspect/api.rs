//! API handlers for the inspector debug viewer.
//!
//! This module implements Phase 7.9.2's HTTP API endpoints:
//! - GET /api/document - Document-level metadata
//! - GET /api/page/{i} - Per-page JSON with spans/blocks/columns
//! - GET /api/page/{i}/svg - Full SVG render with overlays
//! - GET /api/page/{i}/thumbnail - Thumbnail SVG for sidebar
//! - GET /api/raster/{i}.png - Base64 PNG for scanned pages
//! - GET /api/search?q=... - Search across spans
//!
//! Phase 7.9.8: Comparison mode endpoints:
//! - GET /api/compare/document - Diff summary for both documents
//! - GET /api/compare/page/{i} - Side-by-side page data with diff
//! - GET /api/compare/page/{i}/svg/{side} - SVG for one side (a or b)

use super::inspect::InspectorState;
use super::render::anchors;
use super::render::blocks;
use super::render::columns;
use super::render::confidence_heatmap;
use super::render::mcid;
use super::render::ocr_regions;
use super::render::reading_order;
use super::render::spans;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response as AxumResponse},
};
use pdftract_core::schema::BlockJson;
use pdftract_core::schema::SpanJson;
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

/// Diff summary for comparison mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    /// Number of pages added in B
    pub pages_added: usize,
    /// Number of pages removed from A
    pub pages_removed: usize,
    /// Number of blocks added in B
    pub blocks_added: usize,
    /// Number of blocks removed from A
    pub blocks_removed: usize,
    /// Number of blocks changed
    pub blocks_changed: usize,
    /// Number of spans added in B
    pub spans_added: usize,
    /// Number of spans removed from A
    pub spans_removed: usize,
    /// Number of spans changed
    pub spans_changed: usize,
    /// Whether reading order changed on any page
    pub reading_order_changed: bool,
}

/// Comparison document metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareDocumentMeta {
    /// Document A metadata
    pub a: JsonValue,
    /// Document B metadata (null if not in comparison mode)
    pub b: Option<JsonValue>,
    /// Diff summary (null if not in comparison mode)
    pub diff_summary: Option<DiffSummary>,
}

/// Page diff information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageDiff {
    /// Block IDs that changed (yellow)
    pub changed_blocks: Vec<usize>,
    /// Block IDs only in A (red)
    pub removed_blocks: Vec<usize>,
    /// Block IDs only in B (green)
    pub added_blocks: Vec<usize>,
    /// Span indices that changed
    pub changed_spans: Vec<usize>,
    /// Span indices only in A
    pub removed_spans: Vec<usize>,
    /// Span indices only in B
    pub added_spans: Vec<usize>,
    /// Whether reading order changed on this page
    pub reading_order_changed: bool,
}

/// Comparison page data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparePageData {
    /// Page A data (null if page doesn't exist in A)
    pub a: Option<JsonValue>,
    /// Page B data (null if page doesn't exist in B)
    pub b: Option<JsonValue>,
    /// Diff information (null if not in comparison mode or page missing from one side)
    pub diff: Option<PageDiff>,
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

/// Compute page diff between two pages.
fn compute_page_diff(page_a: &JsonValue, page_b: &JsonValue) -> PageDiff {
    let blocks_a = page_a.get("blocks").and_then(|b| b.as_array());
    let blocks_b = page_b.get("blocks").and_then(|b| b.as_array());
    let spans_a = page_a.get("spans").and_then(|s| s.as_array());
    let spans_b = page_b.get("spans").and_then(|s| s.as_array());

    let mut diff = PageDiff {
        changed_blocks: Vec::new(),
        removed_blocks: Vec::new(),
        added_blocks: Vec::new(),
        changed_spans: Vec::new(),
        removed_spans: Vec::new(),
        added_spans: Vec::new(),
        reading_order_changed: false,
    };

    // Match blocks between A and B
    let blocks_a_vec: Vec<BlockJson> = blocks_a
        .map(|arr| {
            arr.iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect()
        })
        .unwrap_or_default();

    let blocks_b_vec: Vec<BlockJson> = blocks_b
        .map(|arr| {
            arr.iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect()
        })
        .unwrap_or_default();

    let mut matched_a = vec![false; blocks_a_vec.len()];
    let mut matched_b = vec![false; blocks_b_vec.len()];

    // Match blocks by bbox overlap and text similarity
    for (i, block_a) in blocks_a_vec.iter().enumerate() {
        let mut best_match = None;
        let mut best_score = 0.0;

        for (j, block_b) in blocks_b_vec.iter().enumerate() {
            if matched_b[j] {
                continue;
            }

            let score = block_match_score(block_a, block_b);
            if score > 0.5 && score > best_score {
                best_match = Some(j);
                best_score = score;
            }
        }

        if let Some(j) = best_match {
            matched_a[i] = true;
            matched_b[j] = true;

            // Check if block changed
            if blocks_changed(block_a, &blocks_b_vec[j]) {
                diff.changed_blocks.push(i);
            }
        } else {
            diff.removed_blocks.push(i);
        }
    }

    // Find added blocks (in B but not matched)
    for (j, matched) in matched_b.iter().enumerate() {
        if !*matched {
            diff.added_blocks.push(j);
        }
    }

    // Match spans between A and B
    let spans_a_vec: Vec<SpanJson> = spans_a
        .map(|arr| {
            arr.iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect()
        })
        .unwrap_or_default();

    let spans_b_vec: Vec<SpanJson> = spans_b
        .map(|arr| {
            arr.iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect()
        })
        .unwrap_or_default();

    let mut span_matched_a = vec![false; spans_a_vec.len()];
    let mut span_matched_b = vec![false; spans_b_vec.len()];

    // Match spans by bbox overlap and text similarity
    for (i, span_a) in spans_a_vec.iter().enumerate() {
        let mut best_match = None;
        let mut best_score = 0.0;

        for (j, span_b) in spans_b_vec.iter().enumerate() {
            if span_matched_b[j] {
                continue;
            }

            let score = span_match_score(span_a, span_b);
            if score > 0.5 && score > best_score {
                best_match = Some(j);
                best_score = score;
            }
        }

        if let Some(j) = best_match {
            span_matched_a[i] = true;
            span_matched_b[j] = true;

            // Check if span changed
            if spans_changed(span_a, &spans_b_vec[j]) {
                diff.changed_spans.push(i);
            }
        } else {
            diff.removed_spans.push(i);
        }
    }

    // Find added spans (in B but not matched)
    for (j, matched) in span_matched_b.iter().enumerate() {
        if !*matched {
            diff.added_spans.push(j);
        }
    }

    // Check reading order (compare block sequences)
    if blocks_a_vec.len() != blocks_b_vec.len() {
        diff.reading_order_changed = true;
    }

    diff
}

/// Compute diff summary for two documents.
fn compute_diff_summary(doc_a: &JsonValue, doc_b: &JsonValue) -> DiffSummary {
    let pages_a = doc_a.get("pages").and_then(|p| p.as_array());
    let pages_b = doc_b.get("pages").and_then(|p| p.as_array());

    let mut summary = DiffSummary {
        pages_added: 0,
        pages_removed: 0,
        blocks_added: 0,
        blocks_removed: 0,
        blocks_changed: 0,
        spans_added: 0,
        spans_removed: 0,
        spans_changed: 0,
        reading_order_changed: false,
    };

    if let (Some(pages_a), Some(pages_b)) = (pages_a, pages_b) {
        // Count page differences
        summary.pages_added = pages_b.len().saturating_sub(pages_a.len());
        summary.pages_removed = pages_a.len().saturating_sub(pages_b.len());

        let max_pages = pages_a.len().max(pages_b.len());

        for i in 0..max_pages {
            let page_a = pages_a.get(i);
            let page_b = pages_b.get(i);

            if let (Some(pa), Some(pb)) = (page_a, page_b) {
                let diff = compute_page_diff(pa, pb);

                summary.blocks_added += diff.added_blocks.len();
                summary.blocks_removed += diff.removed_blocks.len();
                summary.blocks_changed += diff.changed_blocks.len();
                summary.spans_added += diff.added_spans.len();
                summary.spans_removed += diff.removed_spans.len();
                summary.spans_changed += diff.changed_spans.len();

                if diff.reading_order_changed {
                    summary.reading_order_changed = true;
                }
            }
        }
    }

    summary
}

/// Compute match score between two blocks (0.0 to 1.0).
pub fn block_match_score(a: &BlockJson, b: &BlockJson) -> f64 {
    let bbox_score = bbox_overlap_score(&a.bbox, &b.bbox);
    let text_score = text_similarity_score(&a.text, &b.text);

    // Weighted average: bbox is more important than text for blocks
    0.7 * bbox_score + 0.3 * text_score
}

/// Compute match score between two spans (0.0 to 1.0).
pub fn span_match_score(a: &SpanJson, b: &SpanJson) -> f64 {
    let bbox_score = bbox_overlap_score(&a.bbox, &b.bbox);
    let text_score = text_similarity_score(&a.text, &b.text);

    // Equal weight for spans
    0.5 * bbox_score + 0.5 * text_score
}

/// Compute bbox overlap score (0.0 to 1.0).
pub fn bbox_overlap_score(bbox_a: &[f64; 4], bbox_b: &[f64; 4]) -> f64 {
    let [ax0, ay0, ax1, ay1] = *bbox_a;
    let [bx0, by0, bx1, by1] = *bbox_b;

    // Compute intersection
    let ix0 = ax0.max(bx0);
    let iy0 = ay0.max(by0);
    let ix1 = ax1.min(bx1);
    let iy1 = ay1.min(by1);

    // No intersection
    if ix0 >= ix1 || iy0 >= iy1 {
        return 0.0;
    }

    let intersection_area = (ix1 - ix0) * (iy1 - iy0);
    let area_a = (ax1 - ax0) * (ay1 - ay0);
    let area_b = (bx1 - bx0) * (by1 - by0);

    // IoU (Intersection over Union)
    let union_area = area_a + area_b - intersection_area;
    if union_area > 0.0 {
        intersection_area / union_area
    } else {
        0.0
    }
}

/// Compute text similarity score using normalized Levenshtein distance (0.0 to 1.0).
pub fn text_similarity_score(text_a: &str, text_b: &str) -> f64 {
    if text_a == text_b {
        return 1.0;
    }

    let len_a = text_a.chars().count();
    let len_b = text_b.chars().count();

    if len_a == 0 && len_b == 0 {
        return 1.0;
    }

    if len_a == 0 || len_b == 0 {
        return 0.0;
    }

    let distance = levenshtein_distance(text_a, text_b);
    let max_len = len_a.max(len_b);

    // Convert to similarity score (1.0 = identical, 0.0 = completely different)
    let similarity = 1.0 - (distance as f64 / max_len as f64);
    similarity
}

/// Compute Levenshtein distance between two strings.
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let len_a = a_chars.len();
    let len_b = b_chars.len();

    let mut matrix = vec![vec![0; len_b + 1]; len_a + 1];

    for i in 0..=len_a {
        matrix[i][0] = i;
    }

    for j in 0..=len_b {
        matrix[0][j] = j;
    }

    for i in 1..=len_a {
        for j in 1..=len_b {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };

            matrix[i][j] = *[
                matrix[i - 1][j] + 1,        // deletion
                matrix[i][j - 1] + 1,        // insertion
                matrix[i - 1][j - 1] + cost, // substitution
            ]
            .iter()
            .min()
            .unwrap();
        }
    }

    matrix[len_a][len_b]
}

/// Check if two blocks are different.
fn blocks_changed(a: &BlockJson, b: &BlockJson) -> bool {
    // Check if text or bbox differ significantly
    let text_sim = text_similarity_score(&a.text, &b.text);
    let bbox_sim = bbox_overlap_score(&a.bbox, &b.bbox);

    // Consider changed if either text or bbox differs significantly
    text_sim < 0.9 || bbox_sim < 0.9
}

/// Check if two spans are different.
fn spans_changed(a: &SpanJson, b: &SpanJson) -> bool {
    // Check if text or bbox differ significantly
    let text_sim = text_similarity_score(&a.text, &b.text);
    let bbox_sim = bbox_overlap_score(&a.bbox, &b.bbox);

    // Consider changed if either text or bbox differs significantly
    text_sim < 0.9 || bbox_sim < 0.9
}

/// Handler for GET /api/compare/document - returns comparison metadata.
pub async fn api_compare_document(
    State(state): State<Arc<tokio::sync::Mutex<InspectorState>>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    check_auth(&state, &headers)?;

    let state_guard = state.lock().await;

    let document_a = state_guard.document_a.clone();
    let document_b = state_guard.document_b.clone();

    let diff_summary = if let Some(ref doc_b) = document_b {
        Some(compute_diff_summary(&document_a, doc_b))
    } else {
        None
    };

    let meta = CompareDocumentMeta {
        a: document_a,
        b: document_b,
        diff_summary,
    };

    Ok(Json(meta))
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

/// Handler for GET /api/compare/page/{i} - returns comparison page data.
pub async fn api_compare_page(
    State(state): State<Arc<tokio::sync::Mutex<InspectorState>>>,
    Path(page_index): Path<usize>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    check_auth(&state, &headers)?;

    let state_guard = state.lock().await;

    // Get pages from document_a
    let pages_a = state_guard
        .document_a
        .get("pages")
        .and_then(|p| p.as_array())
        .ok_or_else(|| ApiError {
            error: "INTERNAL_ERROR".to_string(),
            message: "No pages in document".to_string(),
        })?;

    // Get page A (null if out of range)
    let page_a = if page_index < pages_a.len() {
        Some(pages_a[page_index].clone())
    } else {
        None
    };

    // Get page B (null if not in comparison mode or out of range)
    let page_b = if let Some(ref doc_b) = state_guard.document_b {
        let pages_b = doc_b.get("pages").and_then(|p| p.as_array());
        if let Some(pages_b) = pages_b {
            if page_index < pages_b.len() {
                Some(pages_b[page_index].clone())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // Compute diff if both pages exist
    let diff = match (&page_a, &page_b) {
        (Some(a), Some(b)) => Some(compute_page_diff(a, b)),
        _ => None,
    };

    let data = ComparePageData {
        a: page_a,
        b: page_b,
        diff,
    };

    Ok(Json(data))
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

/// Handler for GET /api/compare/page/{i}/svg/{side} - returns SVG for one side.
pub async fn api_compare_page_svg(
    State(state): State<Arc<tokio::sync::Mutex<InspectorState>>>,
    Path((page_index, side)): Path<(usize, String)>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    check_auth(&state, &headers)?;

    let state_guard = state.lock().await;

    // Validate side parameter
    if side != "a" && side != "b" {
        return Err(ApiError {
            error: "BAD_REQUEST".to_string(),
            message: "Side must be 'a' or 'b'".to_string(),
        });
    }

    // Get pages from the appropriate document
    let pages = if side == "a" {
        state_guard
            .document_a
            .get("pages")
            .and_then(|p| p.as_array())
    } else if let Some(ref doc_b) = state_guard.document_b {
        doc_b.get("pages").and_then(|p| p.as_array())
    } else {
        None
    };

    let pages = pages.ok_or_else(|| ApiError {
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
///
/// This function generates a complete SVG document containing:
/// - Background layer (white background, glyph paths in full version)
/// - 8 toggleable overlay layers (spans, blocks, columns, reading_order, confidence_heatmap, ocr, mcid, anchors)
/// - Selection layer (invisible <text> elements for browser text selection)
///
/// # Arguments
///
/// * `page` - Page JSON data from document_a
/// * `width` - Page width in points
/// * `height` - Page height in points
/// * `thumbnail` - If true, renders simplified version (200px wide, fewer layers)
///
/// # Returns
///
/// A complete SVG document string.
fn render_page_svg(page: &JsonValue, width: f64, height: f64, thumbnail: bool) -> String {
    // Parse page data into structs
    let spans_json = page.get("spans").and_then(|s| s.as_array());
    let blocks_json = page.get("blocks").and_then(|b| b.as_array());

    // Parse spans and blocks from JSON
    let spans: Vec<SpanJson> = spans_json
        .map(|arr| {
            arr.iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect()
        })
        .unwrap_or_default();

    let blocks: Vec<BlockJson> = blocks_json
        .map(|arr| {
            arr.iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect()
        })
        .unwrap_or_default();

    // Get page index and page number
    let page_index = page.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
    let page_number = page.get("number").and_then(|n| n.as_u64()).unwrap_or(1) as u32;

    let mut svg_layers = Vec::new();

    // 1. Background layer - white background with glyph paths (full version only)
    // Note: Full glyph path rendering requires font data which isn't available in JSON
    // For now, we render a simple white background. This can be extended later
    // to include actual glyph paths via ttf-parser when font data is available.
    svg_layers.push(
        r#"<g class="background"><rect width="100%" height="100%" fill="white"/></g>"#.to_string(),
    );

    // 2. Selection layer - invisible <text> elements for browser text selection
    // This layer is always rendered (even in thumbnails) to enable text selection
    if !spans.is_empty() {
        let selection_elements = render_selection_layer(&spans, height);
        svg_layers.push(format!(
            r#"<g class="selection" style="pointer-events: none;">{}</g>"#,
            selection_elements.join("")
        ));
    }

    // Overlay layers (only in full version, not thumbnails)
    if !thumbnail {
        // 3. Spans layer - thin outline rectangles per span, color-coded by confidence
        if !spans.is_empty() {
            let span_elements = spans::render_spans(&spans, &blocks);
            svg_layers.push(format!(
                r#"<g class="layer-spans" style="display: none;">{}</g>"#,
                span_elements.join("")
            ));
        }

        // 4. Blocks layer - translucent block rects, color-coded by kind
        if !blocks.is_empty() {
            let block_elements = blocks::render_blocks(&blocks);
            svg_layers.push(format!(
                r#"<g class="layer-blocks" style="display: none;">{}</g>"#,
                block_elements.join("")
            ));
        }

        // 5. Columns layer - dashed vertical lines at column boundaries
        // Extract column information from spans
        let page_height_f32 = height as f32;
        let detected_columns = extract_columns_from_spans(&spans, page_height_f32);
        if !detected_columns.is_empty() {
            let column_elements = columns::render_columns(&detected_columns, page_height_f32);
            svg_layers.push(format!(
                r#"<g class="layer-columns" style="display: none;">{}</g>"#,
                column_elements.join("")
            ));
        }

        // 6. Reading order layer - curved arrows with numeric labels
        if blocks.len() > 1 {
            // Use natural block order for reading order (0, 1, 2, ...)
            let order: Vec<usize> = (0..blocks.len()).collect();
            let reading_order_elements = reading_order::render_reading_order(&blocks, &order);
            if !reading_order_elements.is_empty() {
                svg_layers.push(format!(
                    r#"<g class="layer-reading-order" style="display: none;">{}</g>"#,
                    reading_order_elements.join("")
                ));
            }
        }

        // 7. Confidence heatmap layer - per-glyph color cells
        if !spans.is_empty() {
            let heatmap_elements = confidence_heatmap::render_confidence_heatmap(&spans);
            if !heatmap_elements.is_empty() {
                svg_layers.push(format!(
                    r#"<g class="layer-confidence-heatmap" style="display: none;">{}</g>"#,
                    heatmap_elements.join("")
                ));
            }
        }

        // 8. OCR layer - cyan diagonal-stripe overlay on OCR'd regions
        let ocr_elements = ocr_regions::render_ocr_regions(&spans);
        if !ocr_elements.is_empty() {
            svg_layers.push(format!(
                r#"<g class="layer-ocr" style="display: none;">{}</g>"#,
                ocr_elements.join("")
            ));
        }

        // 9. MCID layer - numeric MCID labels for marked-content blocks
        // Note: MCID tracking requires page metadata (mcid_map) which may not be present
        // in all JSON documents. This is a placeholder for future Phase 3.4 integration.
        svg_layers.push(r#"<g class="layer-mcid" style="display: none;"></g>"#.to_string());

        // 10. Anchors layer - block-ID labels at top-left of each block
        if !blocks.is_empty() {
            let anchor_elements = anchors::render_anchors(page_index, page_number, &blocks);
            svg_layers.push(format!(
                r#"<g class="layer-anchors" style="display: none;">{}</g>"#,
                anchor_elements.join("")
            ));
        }
    }

    let layers_html = svg_layers.join("\n");

    // Create SVG with arrowhead marker definition for reading order arrows
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="{}" height="{}">
<defs>
  <marker id="arrowhead" markerWidth="10" markerHeight="10" refX="9" refY="3" orient="auto">
    <path d="M0,0 L0,6 L9,3 z" fill="#3b82f6" />
  </marker>
</defs>
<style>
  .layer-spans, .layer-blocks, .layer-columns, .layer-reading-order, .layer-confidence-heatmap, .layer-ocr, .layer-mcid, .layer-anchors {{
    display: none;
  }}
</style>
{}
</svg>"##,
        width, height, width, height, layers_html
    )
}

/// Render invisible <text> elements for browser text selection.
///
/// These elements are positioned over the text content but have opacity 0,
/// making them invisible to the user but selectable by the browser.
/// This enables users to copy-paste text from the inspector.
fn render_selection_layer(spans: &[SpanJson], page_height: f64) -> Vec<String> {
    spans.iter().map(|span| {
        let [x0, y0, x1, y1] = span.bbox;

        // Flip Y coordinate for SVG (PDF y-up, SVG y-down)
        let svg_y = page_height - y1;
        let font_size = span.size;

        // Escape text content for XML
        let text_escaped = escape_xml_text(&span.text);

        format!(
            r#"<text x="{:.2}" y="{:.2}" font-size="{:.2}" fill="black" opacity="0" style="cursor: text;">{}</text>"#,
            x0, svg_y, font_size, text_escaped
        )
    }).collect()
}

/// Render OCR layer with cyan diagonal-stripe overlay.
///
/// Spans with confidence_source containing "ocr" get a translucent cyan
/// overlay with diagonal stripes to indicate they were OCR-extracted.
fn render_ocr_layer(spans: &[SpanJson]) -> Vec<String> {
    spans.iter().filter(|span| {
        span.confidence_source.as_ref()
            .map(|s| s.contains("ocr"))
            .unwrap_or(false)
    }).map(|span| {
        let [x0, y0, x1, y1] = span.bbox;
        let width = x1 - x0;
        let height = y1 - y0;

        format!(
            r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="cyan" fill-opacity="0.1" stroke="cyan" stroke-width="1" stroke-dasharray="4,2" class="ocr-overlay" />"#,
            x0, y0, width, height
        )
    }).collect()
}

/// Extract column information from spans.
///
/// Groups spans by their column field and creates Column objects
/// for rendering column boundaries.
fn extract_columns_from_spans(
    spans: &[SpanJson],
    _page_height: f32,
) -> Vec<pdftract_core::layout::columns::Column> {
    use pdftract_core::layout::columns::Column;
    use std::collections::HashMap;

    // Group spans by column
    let mut column_spans: HashMap<u32, Vec<&SpanJson>> = HashMap::new();

    for span in spans {
        if let Some(col) = span.column {
            column_spans.entry(col).or_default().push(span);
        }
    }

    // Create Column objects from grouped spans
    column_spans
        .into_iter()
        .map(|(col_index, col_spans)| {
            // Find the x-range for this column
            let x0 = col_spans
                .iter()
                .map(|s| s.bbox[0])
                .fold(f64::INFINITY, f64::min);
            let x1 = col_spans
                .iter()
                .map(|s| s.bbox[2])
                .fold(f64::NEG_INFINITY, f64::max);

            Column {
                index: col_index,
                x_range: [x0 as f32, x1 as f32],
            }
        })
        .collect()
}

/// Escape text content for XML.
///
/// Replaces special XML characters with their entity references.
fn escape_xml_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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

    #[test]
    fn test_render_page_svg_basic() {
        // Create a minimal page JSON
        let page_json = serde_json::json!({
            "index": 0,
            "number": 1,
            "width": 612.0,
            "height": 792.0,
            "spans": [
                {
                    "text": "Hello World",
                    "bbox": [100.0, 200.0, 300.0, 220.0],
                    "font": "Helvetica",
                    "size": 12.0,
                    "color": "#000000",
                }
            ],
            "blocks": [
                {
                    "kind": "paragraph",
                    "text": "Hello World",
                    "bbox": [100.0, 200.0, 300.0, 220.0],
                }
            ],
        });

        let svg = render_page_svg(&page_json, 612.0, 792.0, false);

        // Verify basic SVG structure
        assert!(svg.contains("<svg"));
        assert!(svg.contains("viewBox=\"0 0 612 792\""));
        assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));

        // Verify arrowhead marker is present
        assert!(svg.contains("<marker id=\"arrowhead\""));
        assert!(svg.contains("<path d=\"M0,0 L0,6 L9,3 z\""));

        // Verify all layer groups are present
        assert!(svg.contains("<g class=\"background\">"));
        assert!(svg.contains("<g class=\"selection\""));
        assert!(svg.contains("<g class=\"layer-spans\""));
        assert!(svg.contains("<g class=\"layer-blocks\""));
        assert!(svg.contains("<g class=\"layer-columns\""));
        assert!(svg.contains("<g class=\"layer-reading-order\""));
        assert!(svg.contains("<g class=\"layer-confidence-heatmap\""));
        assert!(svg.contains("<g class=\"layer-ocr\""));
        assert!(svg.contains("<g class=\"layer-mcid\""));
        assert!(svg.contains("<g class=\"layer-anchors\""));

        // Verify text selection element is present
        assert!(svg.contains("Hello World"));
        assert!(svg.contains("opacity=\"0\""));

        // Verify style block is present
        assert!(svg.contains("<style>"));
        assert!(svg.contains("display: none;"));
    }

    #[test]
    fn test_render_page_svg_thumbnail() {
        // Create a minimal page JSON
        let page_json = serde_json::json!({
            "index": 0,
            "number": 1,
            "width": 612.0,
            "height": 792.0,
            "spans": [
                {
                    "text": "Hello",
                    "bbox": [100.0, 200.0, 200.0, 220.0],
                    "font": "Helvetica",
                    "size": 12.0,
                }
            ],
        });

        let svg = render_page_svg(&page_json, 200.0, 258.8, true);

        // Verify thumbnail SVG structure
        assert!(svg.contains("viewBox=\"0 0 200 258.8\""));

        // Verify background and selection layers are present
        assert!(svg.contains("<g class=\"background\">"));
        assert!(svg.contains("<g class=\"selection\""));

        // Verify overlay layers are NOT present in thumbnail
        assert!(!svg.contains("<g class=\"layer-spans\""));
        assert!(!svg.contains("<g class=\"layer-blocks\""));
    }

    #[test]
    fn test_render_page_svg_empty_page() {
        // Create an empty page JSON
        let page_json = serde_json::json!({
            "index": 0,
            "number": 1,
            "width": 612.0,
            "height": 792.0,
            "spans": [],
            "blocks": [],
        });

        let svg = render_page_svg(&page_json, 612.0, 792.0, false);

        // Verify SVG is still generated
        assert!(svg.contains("<svg"));
        assert!(svg.contains("<g class=\"background\">"));

        // Verify selection layer is present but empty
        assert!(svg.contains("<g class=\"selection\""));
        assert!(svg.contains("</g>"));
    }

    #[test]
    fn test_escape_xml_text() {
        assert_eq!(escape_xml_text("hello"), "hello");
        assert_eq!(escape_xml_text("a&b"), "a&amp;b");
        assert_eq!(escape_xml_text("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml_text("\"quote\""), "&quot;quote&quot;");
        assert_eq!(escape_xml_text("'apos'"), "&apos;apos&apos;");
        assert_eq!(
            escape_xml_text("All & <special> \"chars'"),
            "All &amp; &lt;special&gt; &quot;chars&apos;"
        );
    }

    #[test]
    fn test_render_ocr_layer() {
        let spans = vec![
            SpanJson {
                text: "OCR text".to_string(),
                bbox: [100.0, 200.0, 300.0, 220.0],
                font: "Helvetica".to_string(),
                size: 12.0,
                color: None,
                rendering_mode: None,
                confidence: Some(0.85),
                confidence_source: Some("ocr".to_string()),
                lang: None,
                flags: vec![],
                receipt: None,
                column: None,
            },
            SpanJson {
                text: "Vector text".to_string(),
                bbox: [100.0, 230.0, 300.0, 250.0],
                font: "Helvetica".to_string(),
                size: 12.0,
                color: None,
                rendering_mode: None,
                confidence: Some(0.95),
                confidence_source: Some("vector".to_string()),
                lang: None,
                flags: vec![],
                receipt: None,
                column: None,
            },
        ];

        let ocr_elements = render_ocr_layer(&spans);

        // Only OCR span should have an overlay
        assert_eq!(ocr_elements.len(), 1);
        assert!(ocr_elements[0].contains("class=\"ocr-overlay\""));
        assert!(ocr_elements[0].contains("fill=\"cyan\""));
    }

    #[test]
    fn test_extract_columns_from_spans() {
        let spans = vec![
            SpanJson {
                text: "Column 1".to_string(),
                bbox: [50.0, 100.0, 200.0, 120.0],
                font: "Helvetica".to_string(),
                size: 12.0,
                color: None,
                rendering_mode: None,
                confidence: None,
                confidence_source: None,
                lang: None,
                flags: vec![],
                receipt: None,
                column: Some(0),
            },
            SpanJson {
                text: "Column 2".to_string(),
                bbox: [250.0, 100.0, 400.0, 120.0],
                font: "Helvetica".to_string(),
                size: 12.0,
                color: None,
                rendering_mode: None,
                confidence: None,
                confidence_source: None,
                lang: None,
                flags: vec![],
                receipt: None,
                column: Some(1),
            },
        ];

        let columns = extract_columns_from_spans(&spans, 792.0);

        assert_eq!(columns.len(), 2);
        assert_eq!(columns[0].index, 0);
        assert_eq!(columns[1].index, 1);
        // Check x-ranges are approximately correct
        assert!((columns[0].x_range[0] - 50.0).abs() < 0.1);
        assert!((columns[0].x_range[1] - 200.0).abs() < 0.1);
        assert!((columns[1].x_range[0] - 250.0).abs() < 0.1);
        assert!((columns[1].x_range[1] - 400.0).abs() < 0.1);
    }
}
