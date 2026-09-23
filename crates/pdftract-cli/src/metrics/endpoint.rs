//! The `--metrics PORT` exposition listener shared by `pdftract serve`
//! and `pdftract mcp --bind`.
//!
//! `--metrics PORT` opens a SECOND listener, separate from the main
//! serve/MCP port, whose routes are `GET /metrics` and `GET /ready`:
//! the [`Registry`](super::Registry)'s rendered OpenMetrics v1.0 text
//! served with [`CONTENT_TYPE`](super::CONTENT_TYPE), and the readiness
//! probe. Plan policy ("Monitoring and Alerting"): `/metrics` MUST bind
//! only on the `--metrics` listener, never on the main serve or mcp
//! port, so scraping reachability can differ from production traffic —
//! the main router gains no route.
//!
//! # Liveness vs readiness
//!
//! The plan's "Health and readiness endpoints" split is deliberate:
//!
//! - `GET /health` on the MAIN port is liveness: always 200 while the
//!   process is up, even with a saturated pool or a broken cache. A
//!   liveness probe that failed under load would restart exactly the
//!   server that is merely busy.
//! - `GET /ready` on THIS listener is readiness: 200 only while the
//!   server is accepting work — `pdftract_rayon_pool_utilization` at or
//!   below [`SATURATION_THRESHOLD`] (0.90) AND the cache location (when
//!   the cache is enabled) writable. 503 otherwise, with a body naming
//!   every failed condition (`pool_saturated`, `cache_unwritable`) so
//!   an operator can tell them apart without re-testing. Routing layers
//!   SHOULD pull a node out of rotation on 503.
//!
//! Both readiness inputs are injectable ([`Readiness::with_inputs`]) so
//! tests can force each failure condition without manufacturing real
//! extraction load or read-only mounts.
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
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::{from_fn_with_state, Next},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::Registry;

/// Pool utilization at or below which the server counts as accepting
/// work; strictly above it, `/ready` reports `pool_saturated` (plan:
/// "utilization is below 90%" is ready, so exactly 90% is still ready).
pub const SATURATION_THRESHOLD: f64 = 0.90;

/// The two inputs behind `GET /ready`.
///
/// Each is a plain closure so tests can force a failure condition
/// directly; the production wiring is [`Readiness::production`].
#[derive(Clone)]
pub struct Readiness {
    pool_utilization: Arc<dyn Fn() -> f64 + Send + Sync>,
    cache_writable: Arc<dyn Fn() -> bool + Send + Sync>,
}

impl Readiness {
    /// Production readiness inputs: the registry's sampled
    /// `pdftract_rayon_pool_utilization` gauge, and a create-and-remove
    /// probe file in `cache_dir`. `None` (cache disabled or absent, as
    /// in `pdftract mcp`) makes the cache condition trivially pass —
    /// the plan makes the check conditional on the cache being enabled.
    pub fn production(registry: Registry, cache_dir: Option<PathBuf>) -> Self {
        Readiness {
            pool_utilization: Arc::new(move || registry.rayon_pool_utilization()),
            cache_writable: Arc::new(move || match &cache_dir {
                Some(dir) => probe_cache_writable(dir),
                None => true,
            }),
        }
    }

    /// Arbitrary readiness inputs, for tests that force one failure
    /// condition at a time without real load or read-only mounts.
    pub fn with_inputs(
        pool_utilization: impl Fn() -> f64 + Send + Sync + 'static,
        cache_writable: impl Fn() -> bool + Send + Sync + 'static,
    ) -> Self {
        Readiness {
            pool_utilization: Arc::new(pool_utilization),
            cache_writable: Arc::new(cache_writable),
        }
    }

    /// Evaluate both readiness conditions now.
    pub fn evaluate(&self) -> ReadinessReport {
        let pool_utilization = (self.pool_utilization)();
        let cache_writable = (self.cache_writable)();
        ReadinessReport {
            pool_utilization,
            pool_saturated: pool_utilization > SATURATION_THRESHOLD,
            cache_writable,
        }
    }
}

/// The result of evaluating `GET /ready`'s two conditions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReadinessReport {
    /// Observed pool utilization (the gauge value or injected sample).
    pub pool_utilization: f64,
    /// Whether utilization strictly exceeded [`SATURATION_THRESHOLD`].
    pub pool_saturated: bool,
    /// Whether the enabled cache location accepted the write probe.
    pub cache_writable: bool,
}

impl ReadinessReport {
    /// Machine-readable names of every failed condition, in a stable
    /// order. Empty exactly when the server is ready.
    pub fn failed_conditions(&self) -> Vec<&'static str> {
        let mut failed = Vec::new();
        if self.pool_saturated {
            failed.push("pool_saturated");
        }
        if !self.cache_writable {
            failed.push("cache_unwritable");
        }
        failed
    }

    /// Whether the server is accepting work.
    pub fn is_ready(&self) -> bool {
        !self.pool_saturated && self.cache_writable
    }
}

/// Probe `dir` for writability by creating, writing and removing a
/// uniquely named probe file in it.
///
/// Plain filesystem only: this is the local extraction cache (plan
/// Phase 6.9), so the check is an ordinary directory write — no
/// secret-store path is consulted, created, or required.
///
/// The probe file is removed again on every path where it was created
/// (including when the write itself fails), so a probe leaves nothing
/// behind in `dir`. A directory that accepts the create but not the
/// remove is reported unwritable — a cache directory that won't let the
/// process unlink cannot function anyway.
pub(crate) fn probe_cache_writable(dir: &Path) -> bool {
    if !dir.is_dir() {
        return false;
    }
    let probe = dir.join(probe_file_name());
    let Ok(mut file) = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    else {
        return false;
    };
    let wrote = file.write_all(b"pdftract /ready writability probe").is_ok();
    let removed = std::fs::remove_file(&probe).is_ok();
    wrote && removed
}

/// Unique probe file name: process id plus a nanosecond timestamp, so
/// concurrent probes (and stale probes from a previous run) never
/// collide.
fn probe_file_name() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|age| age.as_nanos())
        .unwrap_or(0);
    format!(".pdftract-ready-probe-{}-{nanos}", std::process::id())
}

/// State for the metrics listener's two routes.
#[derive(Clone)]
struct MetricsState {
    registry: Registry,
    readiness: Readiness,
}

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

/// Bind the metrics listener and serve `GET /metrics` and `GET /ready`
/// from `registry`/`readiness` on it until the process exits. Returns
/// the address actually bound (the caller may have passed port 0).
///
/// Binding happens on the caller's task — before the main listener is
/// bound — so a port that is already in use fails startup with a clean
/// error (nonzero exit) instead of a panic, and before any traffic is
/// served; only the serve loop itself runs in the background.
pub async fn bind_and_spawn(
    metrics_addr: &str,
    registry: Registry,
    readiness: Readiness,
) -> Result<SocketAddr> {
    let listener = tokio::net::TcpListener::bind(metrics_addr)
        .await
        .with_context(|| format!("Failed to bind metrics listener to {}", metrics_addr))?;
    let bound = listener
        .local_addr()
        .context("Failed to read the metrics listener's address")?;
    eprintln!("Metrics endpoint: http://{}/metrics", bound);

    tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, metrics_router(registry, readiness)).await {
            eprintln!("Metrics listener error: {}", error);
        }
    });
    Ok(bound)
}

/// The metrics listener's router: `GET /metrics` and `GET /ready`, and
/// nothing else.
fn metrics_router(registry: Registry, readiness: Readiness) -> Router {
    Router::new()
        .route("/metrics", get(exposition))
        .route("/ready", get(ready))
        // Count the monitoring listener itself in the shared HTTP counter.
        // This is deliberately outside the two handlers so 404s on unknown
        // monitoring paths are counted as `endpoint="unmatched"` too.
        .layer(from_fn_with_state(
            registry.clone(),
            metrics_http_middleware,
        ))
        .with_state(MetricsState {
            registry,
            readiness,
        })
}

/// `GET /metrics` — the registry rendered as OpenMetrics v1.0 text
/// (terminates in `# EOF`).
async fn exposition(State(state): State<MetricsState>) -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, super::CONTENT_TYPE)],
        state.registry.render(),
    )
}

/// `GET /ready` — readiness probe.
///
/// 200 with `{"status":"ready",...}` while the server is accepting work;
/// 503 with `{"status":"unready","unready":[<failed conditions>],...}`
/// otherwise. The body names every failed condition, so an operator can
/// distinguish pool saturation from an unwritable cache without
/// re-testing (plan "Health and readiness endpoints").
async fn ready(State(state): State<MetricsState>) -> Response {
    let report = state.readiness.evaluate();
    let unready = report.failed_conditions();
    let status = if unready.is_empty() {
        "ready"
    } else {
        "unready"
    };
    let body = json!({
        "status": status,
        "unready": unready,
        "pool_utilization": report.pool_utilization,
        "cache_writable": report.cache_writable,
    });
    if unready.is_empty() {
        (StatusCode::OK, Json(body)).into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(body)).into_response()
    }
}

/// Map a metrics-listener path to its bounded endpoint label.
fn endpoint_label(path: &str) -> &'static str {
    match path {
        "/metrics" => "/metrics",
        "/ready" => "/ready",
        _ => "unmatched",
    }
}

/// Count one response served by the separate metrics/readiness listener.
async fn metrics_http_middleware(
    State(registry): State<Registry>,
    request: Request,
    next: Next,
) -> Response {
    let endpoint = endpoint_label(request.uri().path());
    let response = next.run(request).await;
    registry.inc_http_request(endpoint, response.status().as_u16());
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listener_addr_keeps_the_host_and_swaps_the_port() {
        assert_eq!(
            listener_addr("127.0.0.1:8080", 9100).unwrap(),
            "127.0.0.1:9100"
        );
        assert_eq!(listener_addr("0.0.0.0:3000", 9090).unwrap(), "0.0.0.0:9090");
        assert_eq!(listener_addr("[::1]:9000", 9100).unwrap(), "[::1]:9100");
        assert_eq!(
            listener_addr("localhost:8080", 9100).unwrap(),
            "localhost:9100"
        );
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

        let result = bind_and_spawn(
            &format!("127.0.0.1:{port}"),
            Registry::new(),
            Readiness::production(Registry::new(), None),
        )
        .await;

        match result {
            Ok(_) => panic!("binding a held port must fail"),
            Err(error) => assert!(
                error
                    .to_string()
                    .contains("Failed to bind metrics listener"),
                "unexpected error: {error:#}"
            ),
        }
    }

    #[tokio::test]
    async fn served_route_returns_openmetrics_with_the_exact_content_type() {
        let bound = bind_and_spawn(
            "127.0.0.1:0",
            Registry::new(),
            Readiness::production(Registry::new(), None),
        )
        .await
        .expect("metrics listener binds on :0");

        let raw = http_get(bound, "/metrics").await;

        assert!(raw.starts_with("HTTP/1.1 200 OK\r\n"), "got: {raw}");
        assert!(
            raw.to_ascii_lowercase()
                .contains(&format!("content-type: {}", crate::metrics::CONTENT_TYPE)),
            "content-type must be exactly the OpenMetrics type, got: {raw}"
        );
        assert!(
            raw.ends_with("# EOF\n"),
            "response must end in # EOF:\\n, got: {raw}"
        );
        let body = raw.split("\r\n\r\n").nth(1).unwrap_or_default();
        assert!(
            body.contains("pdftract_build_info"),
            "body must render the registry, got: {raw}"
        );
    }

    /// Issue one `GET path` with `Connection: close` and return the raw
    /// HTTP/1.1 response text.
    async fn http_get(bound: SocketAddr, path: &str) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = tokio::net::TcpStream::connect(bound)
            .await
            .expect("connect");
        stream
            .write_all(
                format!("GET {path} HTTP/1.1\r\nHost: metrics\r\nConnection: close\r\n\r\n")
                    .as_bytes(),
            )
            .await
            .expect("write request");
        let mut raw = String::new();
        stream
            .read_to_string(&mut raw)
            .await
            .expect("read response");
        raw
    }

    /// Body of a raw response, parsed as JSON.
    async fn http_get_json(bound: SocketAddr, path: &str) -> serde_json::Value {
        let raw = http_get(bound, path).await;
        let body = raw.split("\r\n\r\n").nth(1).unwrap_or_default();
        serde_json::from_str(body).unwrap_or_else(|error| panic!("body was {raw:?}: {error}"))
    }

    /// Criterion 1: a healthy server returns 200 from GET /ready.
    #[tokio::test]
    async fn ready_is_200_on_a_healthy_server() {
        let bound = bind_and_spawn(
            "127.0.0.1:0",
            Registry::new(),
            Readiness::with_inputs(|| 0.0, || true),
        )
        .await
        .expect("metrics listener binds on :0");

        let raw = http_get(bound, "/ready").await;
        assert!(raw.starts_with("HTTP/1.1 200 OK\r\n"), "got: {raw}");

        let body = http_get_json(bound, "/ready").await;
        assert_eq!(body["status"], "ready", "got: {body}");
        assert_eq!(body["unready"], serde_json::json!([]), "got: {body}");
    }

    /// Criterion 2: injected utilization above 0.90 → 503 naming the
    /// pool, and only the pool.
    #[tokio::test]
    async fn ready_is_503_naming_the_pool_when_utilization_exceeds_the_threshold() {
        let bound = bind_and_spawn(
            "127.0.0.1:0",
            Registry::new(),
            Readiness::with_inputs(|| 0.95, || true),
        )
        .await
        .expect("metrics listener binds on :0");

        let raw = http_get(bound, "/ready").await;
        assert!(raw.starts_with("HTTP/1.1 503"), "got: {raw}");

        let body = http_get_json(bound, "/ready").await;
        assert_eq!(body["status"], "unready", "got: {body}");
        assert_eq!(
            body["unready"],
            serde_json::json!(["pool_saturated"]),
            "body must name pool saturation and nothing else: {body}"
        );
    }

    /// Exactly at the threshold the pool is NOT saturated: the bead pins
    /// the 503 to utilization *exceeding* 0.90, so 0.90 itself is ready.
    #[tokio::test]
    async fn ready_is_200_at_exactly_the_threshold() {
        let bound = bind_and_spawn(
            "127.0.0.1:0",
            Registry::new(),
            Readiness::with_inputs(|| SATURATION_THRESHOLD, || true),
        )
        .await
        .expect("metrics listener binds on :0");

        let raw = http_get(bound, "/ready").await;
        assert!(raw.starts_with("HTTP/1.1 200 OK\r\n"), "got: {raw}");
    }

    /// Criterion 3's forced-failure twin: an unwritable cache → 503
    /// naming the cache, and only the cache. (The real unwritable path
    /// is exercised end-to-end by the serve-level tests via the default
    /// prober; here the same input seam is forced directly.)
    #[tokio::test]
    async fn ready_is_503_naming_the_cache_when_the_cache_probe_fails() {
        let bound = bind_and_spawn(
            "127.0.0.1:0",
            Registry::new(),
            Readiness::with_inputs(|| 0.0, || false),
        )
        .await
        .expect("metrics listener binds on :0");

        let raw = http_get(bound, "/ready").await;
        assert!(raw.starts_with("HTTP/1.1 503"), "got: {raw}");

        let body = http_get_json(bound, "/ready").await;
        assert_eq!(body["status"], "unready", "got: {body}");
        assert_eq!(
            body["unready"],
            serde_json::json!(["cache_unwritable"]),
            "body must name the cache and nothing else: {body}"
        );
    }

    /// A 503 must name BOTH failed conditions at once, so an operator
    /// can tell them apart without re-testing.
    #[tokio::test]
    async fn ready_names_every_failed_condition_when_both_fail() {
        let bound = bind_and_spawn(
            "127.0.0.1:0",
            Registry::new(),
            Readiness::with_inputs(|| 0.99, || false),
        )
        .await
        .expect("metrics listener binds on :0");

        let body = http_get_json(bound, "/ready").await;
        assert_eq!(body["status"], "unready", "got: {body}");
        assert_eq!(
            body["unready"],
            serde_json::json!(["pool_saturated", "cache_unwritable"]),
            "body must name both conditions: {body}"
        );
    }

    /// The production readiness input reads the registry gauge the
    /// sampler publishes.
    #[test]
    fn production_readiness_reads_the_registry_gauge() {
        let registry = Registry::new();
        let readiness = Readiness::production(registry.clone(), None);
        assert!(readiness.evaluate().is_ready());

        registry.set_rayon_pool_utilization(0.91);
        let report = readiness.evaluate();
        assert!(!report.is_ready());
        assert_eq!(report.failed_conditions(), vec!["pool_saturated"]);
    }

    /// Criterion 5: the writability probe accepts a writable directory
    /// and cleans up its probe file — nothing left behind.
    #[test]
    fn probe_accepts_a_writable_directory_and_leaves_nothing_behind() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(probe_cache_writable(dir.path()));
        let leftovers: Vec<_> = std::fs::read_dir(dir.path()).expect("read_dir").collect();
        assert!(
            leftovers.is_empty(),
            "probe must not leave files behind: {leftovers:?}"
        );
    }

    /// Criterion 3's filesystem half: a nonexistent cache location is
    /// unwritable.
    #[test]
    fn probe_rejects_a_nonexistent_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("does-not-exist");
        assert!(!probe_cache_writable(&missing));
    }

    /// A read-only directory is unwritable, and the failed probe leaves
    /// nothing behind. Skips when permission bits are advisory (root).
    #[test]
    fn probe_rejects_a_readonly_directory_and_leaves_nothing_behind() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let mut permissions = std::fs::metadata(dir.path())
            .expect("metadata")
            .permissions();
        permissions.set_mode(0o555);
        std::fs::set_permissions(dir.path(), permissions.clone()).expect("chmod");

        let writable = probe_cache_writable(dir.path());

        // Restore writability first so the tempdir can be removed, then
        // assert: if the probe still succeeded (running as root), the
        // directory is empty anyway and there is nothing to assert about
        // rejection.
        let mut restored = std::fs::metadata(dir.path())
            .expect("metadata")
            .permissions();
        restored.set_mode(0o755);
        std::fs::set_permissions(dir.path(), restored).expect("chmod back");

        if writable {
            // Permission bits are advisory here (uid 0); nothing to prove.
            return;
        }
        assert!(!writable);
        let leftovers: Vec<_> = std::fs::read_dir(dir.path()).expect("read_dir").collect();
        assert!(
            leftovers.is_empty(),
            "failed probe must not leave files behind: {leftovers:?}"
        );
    }
}
