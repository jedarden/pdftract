//! Content Security Policy middleware for the inspector.
//!
//! Implements TH-09 XSS mitigation by adding strict CSP headers to all
//! inspector responses. The policy permits only same-origin scripts and
//! default sources, preventing execution of any injected content.

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

/// CSP header value for inspector responses.
///
/// Per TH-09 (plan line 898), the inspector MUST set:
/// - `default-src 'self'` - only allow resources from same origin
/// - `script-src 'self'` - only allow scripts from same origin
/// - No `unsafe-inline` or external sources
const CSP_HEADER_VALUE: &str = "default-src 'self'; script-src 'self'";

/// CSP middleware that adds security headers to all responses.
///
/// This is a defense-in-depth measure for TH-09 XSS mitigation. The primary
/// defense is that the inspector renders extracted text as SVG `<text>` nodes
/// (not innerHTML), but CSP ensures that even if a regression introduces
/// HTML rendering, injected scripts cannot execute.
pub async fn csp_middleware(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;

    // Add CSP header to all responses
    response.headers_mut().insert(
        "Content-Security-Policy",
        CSP_HEADER_VALUE.parse().unwrap(),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use axum::http::StatusCode;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_csp_header_added() {
        async fn handler() -> &'static str {
            "Hello"
        }

        let app = Router::new()
            .route("/", get(handler))
            .layer(axum::middleware::from_fn(csp_middleware));

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()["Content-Security-Policy"],
            CSP_HEADER_VALUE
        );
    }
}
