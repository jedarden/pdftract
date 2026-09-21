//! The `--metrics PORT` exposition listener shared by `pdftract serve`
//! and `pdftract mcp --bind`.
//!
//! `--metrics PORT` opens a SECOND listener, separate from the main
//! serve/MCP port, whose only route is `GET /metrics`: the
//! [`Registry`](super::Registry)'s rendered OpenMetrics v1.0 text served
//! with [`CONTENT_TYPE`](super::CONTENT_TYPE). Plan policy ("Monitoring
//! and Alerting"): `/metrics` MUST bind only on the `--metrics` listener,
//! never on the main serve or mcp port, so scraping reachability can
//! differ from production traffic — the main router gains no route.
//!
//! The flag names a port only; the listener shares the main listener's
//! interface (the host part of the bind address), so the operator's
//! interface selection and network policy cover both ports while each
//! stays independently firewallable.
//!
//! The listener is unauthenticated by default; like the rest of the HTTP
//! surface, restriction is the deployment's job (firewall, K8s
//! NetworkPolicy).

use anyhow::{Context, Result};
use axum::{extract::State, response::IntoResponse, routing::get, Router};
use std::net::SocketAddr;

use super::Registry;

/// The bind address of the metrics listener for a server bound to
/// `main_bind`: the host part is kept, the port replaced
/// (`("127.0.0.1:8080", 9100)` → `"127.0.0.1:9100"`).
///
/// Splitting after the last `:` covers the IPv4, IPv6 (`"[::1]:9000"`)
/// and hostname (`"localhost:8080"`) forms of `--bind` alike.
pub fn listener_addr(main_bind: &str, port: u16) -> Result<String> {
    let Some(host) = main_bind.rsplit_once(':').map(|(host, _)| host) else {
        anyhow::bail!(
            "--metrics needs a --bind of the form HOST:PORT, got {:?}",
            main_bind
        );
    };
    Ok(format!("{host}:{port}"))
}

/// Bind the metrics listener and serve `GET /metrics` from `registry` on
/// it until the process exits. Returns the address actually bound (the
/// caller may have passed port 0).
///
/// Binding happens on the caller's task — before the main listener is
/// bound — so a port that is already in use fails startup with a clean
/// error (nonzero exit) instead of a panic, and before any traffic is
/// served; only the serve loop itself runs in the background.
pub async fn bind_and_spawn(metrics_addr: &str, registry: Registry) -> Result<SocketAddr> {
    let listener = tokio::net::TcpListener::bind(metrics_addr)
        .await
        .with_context(|| format!("Failed to bind metrics listener to {}", metrics_addr))?;
    let bound = listener
        .local_addr()
        .context("Failed to read the metrics listener's address")?;
    eprintln!("Metrics endpoint: http://{}/metrics", bound);

    tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, metrics_router(registry)).await {
            eprintln!("Metrics listener error: {}", error);
        }
    });
    Ok(bound)
}

/// The metrics listener's router: `GET /metrics` and nothing else.
fn metrics_router(registry: Registry) -> Router {
    Router::new()
        .route("/metrics", get(exposition))
        .with_state(registry)
}

/// `GET /metrics` — the registry rendered as OpenMetrics v1.0 text
/// (terminates in `# EOF`).
async fn exposition(State(registry): State<Registry>) -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, super::CONTENT_TYPE)],
        registry.render(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listener_addr_keeps_the_host_and_swaps_the_port() {
        assert_eq!(listener_addr("127.0.0.1:8080", 9100).unwrap(), "127.0.0.1:9100");
        assert_eq!(listener_addr("0.0.0.0:3000", 9090).unwrap(), "0.0.0.0:9090");
        assert_eq!(listener_addr("[::1]:9000", 9100).unwrap(), "[::1]:9100");
        assert_eq!(listener_addr("localhost:8080", 9100).unwrap(), "localhost:9100");
    }

    #[test]
    fn listener_addr_rejects_a_bind_without_a_port() {
        assert!(listener_addr("127.0.0.1", 9100).is_err());
        assert!(listener_addr("", 9100).is_err());
    }

    #[tokio::test]
    async fn bind_failure_is_a_clean_error() {
        let squatter = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("squatter binds on :0");
        let port = squatter.local_addr().unwrap().port();

        let result = bind_and_spawn(&format!("127.0.0.1:{port}"), Registry::new()).await;

        match result {
            Ok(_) => panic!("binding a held port must fail"),
            Err(error) => assert!(
                error.to_string().contains("Failed to bind metrics listener"),
                "unexpected error: {error:#}"
            ),
        }
    }

    #[tokio::test]
    async fn served_route_returns_openmetrics_with_the_exact_content_type() {
        let bound = bind_and_spawn("127.0.0.1:0", Registry::new())
            .await
            .expect("metrics listener binds on :0");

        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = tokio::net::TcpStream::connect(bound).await.expect("connect");
        stream
            .write_all(b"GET /metrics HTTP/1.1\r\nHost: metrics\r\nConnection: close\r\n\r\n")
            .await
            .expect("write request");
        let mut raw = String::new();
        stream.read_to_string(&mut raw).await.expect("read response");

        assert!(raw.starts_with("HTTP/1.1 200 OK\r\n"), "got: {raw}");
        assert!(
            raw.to_ascii_lowercase()
                .contains(&format!("content-type: {}", crate::metrics::CONTENT_TYPE)),
            "content-type must be exactly the OpenMetrics type, got: {raw}"
        );
        assert!(raw.ends_with("# EOF\n"), "response must end in # EOF:\\n, got: {raw}");
        let body = raw.split("\r\n\r\n").nth(1).unwrap_or_default();
        assert!(
            body.contains("pdftract_build_info"),
            "body must render the registry, got: {raw}"
        );
    }
}
