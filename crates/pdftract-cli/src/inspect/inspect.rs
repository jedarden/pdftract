//! Inspector web debug viewer implementation.
//!
//! Implements Phase 7.9.1: inspect subcommand with extraction pipeline,
//! axum server, and browser launcher.
//!
//! Phase 7.9.3: Frontend bundle served via include_bytes!.

use super::api;
use super::args::InspectArgs;
use crate::middleware::{audit_middleware, csp_middleware, AuditState};
use anyhow::{Context, Result};
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use pdftract_core::audit::AuditLogWriter;
use pdftract_core::extract::{extract_pdf, result_to_json};
use pdftract_core::options::ExtractionOptions;
use serde_json::Value as JsonValue;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Cached extraction result for the inspector.
#[derive(Clone)]
pub struct InspectorState {
    /// Extraction result for the primary document
    pub document_a: JsonValue,
    /// Extraction result for the comparison document (if any)
    pub document_b: Option<JsonValue>,
    /// Authentication token for non-loopback binds
    pub auth_token: Option<String>,
    /// Audit log state
    pub audit: AuditState,
}

/// Run the inspector subcommand.
///
/// # Steps
///
/// 1. Validate arguments
/// 2. Run extraction pipeline on the input file
/// 3. (Optionally) Run extraction on the compare file
/// 4. Build axum router with inspector state
/// 5. Start HTTP server
/// 6. Launch browser (unless --no-open)
/// 7. Wait for Ctrl-C
///
/// # Errors
///
/// Returns an error if:
/// - Argument validation fails
/// - PDF extraction fails
/// - Server fails to bind
pub async fn run(args: InspectArgs) -> Result<()> {
    // Step 1: Validate arguments
    args.validate().context("Invalid inspect arguments")?;

    // Step 2: Extract the primary document
    let document_a = extract_document(&args.file).context(format!(
        "Failed to extract document: {}",
        args.file.display()
    ))?;

    // Step 3: Extract the compare document if provided
    let document_b = if let Some(ref compare_path) = args.compare {
        Some(extract_document(compare_path).context(format!(
            "Failed to extract compare document: {}",
            compare_path.display()
        ))?)
    } else {
        None
    };

    // Create audit log writer if specified
    let audit_writer = if let Some(ref path) = args.audit_log {
        Some(
            AuditLogWriter::open(path)
                .context(format!("Failed to open audit log: {}", path.display()))?,
        )
    } else {
        None
    };

    // Step 4: Build inspector state
    let state = InspectorState {
        document_a,
        document_b,
        auth_token: args.auth_token.clone(),
        audit: AuditState::new(audit_writer),
    };

    // Step 5: Build axum router with audit middleware
    let app = create_router_with_audit(state);

    // Step 6: Start server
    let bind_addr = args.parse_bind()?;
    let addr = (bind_addr, args.port);
    let server_url = args.server_url();

    eprintln!("Inspector running at {}", server_url);
    eprintln!("Press Ctrl-C to stop");
    if let Some(ref path) = args.audit_log {
        eprintln!("Audit log: {}", path.display());
    }

    // Spawn the server task
    let server_handle = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .context(format!("Failed to bind to {}", addr.0))?;

        axum::serve(listener, app).await.context("Server error")?;

        Ok::<(), anyhow::Error>(())
    });

    // Step 7: Launch browser (unless --no-open)
    if !args.no_open {
        launch_browser(&server_url);
    }

    // Wait for Ctrl-C
    tokio::select! {
        result = server_handle => {
            result??;
        }
        _ = tokio::signal::ctrl_c() => {
            eprintln!("\nShutting down inspector...");
        }
    }

    Ok(())
}

/// Extract a PDF document and return the JSON result.
fn extract_document(path: &Path) -> Result<JsonValue> {
    // Run extraction with default options
    let options = ExtractionOptions::default();
    let result = extract_pdf(path, &options).context(format!(
        "Extraction pipeline failed for: {}",
        path.display()
    ))?;

    // Convert to JSON
    let json = result_to_json(&result);

    Ok(json)
}

/// Create the axum router for the inspector with audit middleware.
fn create_router_with_audit(state: InspectorState) -> Router {
    let audit_state = state.audit.clone();
    let state_arc = Arc::new(Mutex::new(state));

    Router::new()
        // Index page (Phase 7.9.3)
        .route("/", get(index_handler))
        // Static assets (Phase 7.9.3)
        .route("/static/style.css", get(static_style_handler))
        .route("/static/app.js", get(static_app_handler))
        // API endpoints (Phase 7.9.2)
        .route("/api/document", get(api::api_document))
        .route("/api/page/:i", get(api::api_page))
        .route("/api/page/:i/svg", get(api::api_page_svg))
        .route("/api/page/:i/thumbnail", get(api::api_page_thumbnail))
        .route("/api/raster/:i.png", get(api::api_raster))
        .route("/api/search", get(api::api_search))
        // Comparison mode endpoints (Phase 7.9.8)
        .route("/api/compare/document", get(api::api_compare_document))
        .route("/api/compare/page/:i", get(api::api_compare_page))
        .route("/api/compare/page/:i/svg/:side", get(api::api_compare_page_svg))
        // CSP middleware (TH-09 XSS mitigation)
        .layer(axum::middleware::from_fn(csp_middleware))
        // Audit middleware
        .layer(axum::middleware::from_fn_with_state(
            audit_state,
            audit_middleware,
        ))
        .with_state(state_arc)
}

/// Handler for the index page (Phase 7.9.3).
async fn index_handler(State(_state): State<Arc<Mutex<InspectorState>>>) -> Html<String> {
    Html(String::from_utf8(include_bytes!("frontend/index.html").to_vec()).unwrap())
}

/// Handler for static style.css (Phase 7.9.3).
async fn static_style_handler() -> impl IntoResponse {
    let css = String::from_utf8(include_bytes!("frontend/style.css").to_vec()).unwrap();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/css; charset=utf-8")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(axum::body::Body::from(css))
        .unwrap()
}

/// Handler for static app.js (Phase 7.9.3).
async fn static_app_handler() -> impl IntoResponse {
    let js = String::from_utf8(include_bytes!("frontend/app.js").to_vec()).unwrap();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/javascript; charset=utf-8")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(axum::body::Body::from(js))
        .unwrap()
}

/// Launch the OS default browser to the given URL.
///
/// This function attempts to open the URL in the user's default browser:
/// - Linux: `xdg-open`
/// - macOS: `open`
/// - Windows: `cmd /c start`
///
/// If the browser launch fails (e.g., no $DISPLAY on Linux), we print the URL
/// instead of failing. This allows CI environments to work gracefully.
fn launch_browser(url: &str) {
    let (program, args) = if cfg!(target_os = "linux") {
        ("xdg-open", vec![url])
    } else if cfg!(target_os = "macos") {
        ("open", vec![url])
    } else if cfg!(target_os = "windows") {
        ("cmd", vec!["/c", "start", url])
    } else {
        // Unknown OS; just print the URL
        eprintln!("Open this URL in your browser: {}", url);
        return;
    };

    match std::process::Command::new(program).args(&args).spawn() {
        Ok(_) => {}
        Err(e) => {
            // Browser launch failed (e.g., no $DISPLAY on Linux)
            eprintln!("Failed to launch browser: {}", e);
            eprintln!("Open this URL in your browser: {}", url);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_launch_browser_doesnt_crash() {
        // This should not crash even if there's no display
        launch_browser("http://127.0.0.1:7676/");
    }
}