//! Integration tests for the `--metrics PORT` exposition listener shared
//! by `pdftract serve` and `pdftract mcp --bind` (plan "Monitoring and
//! Alerting", bead pdftract-c20c44c1).
//!
//! These spawn the real binary so the second listener is observed on the
//! wire, not just in-process:
//!
//! - with `--metrics`, a SECOND listener serves `GET /metrics` with the
//!   exact OpenMetrics content type, all 13 documented `pdftract_*`
//!   families, and `# EOF` termination — while the main port does NOT
//!   serve `/metrics`;
//! - without the flag, no metrics listener exists at all;
//! - a `--metrics` port that cannot be bound fails startup cleanly
//!   (error message, nonzero exit), never a panic.
//!
//! Test hygiene (repo rules): children are killed from an RAII guard
//! whose `Drop` kills and then reaps with a bounded wait; stderr is
//! drained on a background thread so the child can never block on a full
//! pipe (stdout and stdin are null); the main bind port is chosen by
//! binding `:0` and releasing, and the metrics port is chosen by the
//! server itself with `--metrics 0` (parsed from its stderr banner), so
//! nothing collides on a fixed port across runs.

#![cfg(feature = "metrics")]

use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;

/// The exact wire content type of the exposition (openmetrics::CONTENT_TYPE).
const OPENMETRICS_CONTENT_TYPE: &str = "application/openmetrics-text; version=1.0.0; charset=utf-8";

/// The stderr banner `metrics::endpoint::bind_and_spawn` prints with the
/// bound address (`--metrics 0` lets the OS choose the port).
const METRICS_BANNER: &str = "Metrics endpoint: http://";

/// The 13 documented `pdftract_*` metric names (plan "Monitoring and
/// Alerting" table), paired with their OpenMetrics family name — the name
/// in the `# HELP`/`# TYPE` metadata. Counter *samples* carry the
/// `_total` suffix but counter family *metadata* does not, and a labeled
/// family with no observations yet exposes metadata only, so "the body
/// contains the documented name" is asserted as: the name appears as a
/// sample line OR its family appears in a `# TYPE` line.
const DOCUMENTED_FAMILIES: &[(&str, &str)] = &[
    ("pdftract_extractions_total", "pdftract_extractions"),
    (
        "pdftract_extraction_duration_seconds",
        "pdftract_extraction_duration_seconds",
    ),
    ("pdftract_pages_extracted_total", "pdftract_pages_extracted"),
    ("pdftract_cache_hits_total", "pdftract_cache_hits"),
    ("pdftract_cache_misses_total", "pdftract_cache_misses"),
    ("pdftract_cache_size_bytes", "pdftract_cache_size_bytes"),
    ("pdftract_mcp_requests_total", "pdftract_mcp_requests"),
    ("pdftract_http_requests_total", "pdftract_http_requests"),
    (
        "pdftract_remote_bytes_downloaded_total",
        "pdftract_remote_bytes_downloaded",
    ),
    (
        "pdftract_diagnostic_emitted_total",
        "pdftract_diagnostic_emitted",
    ),
    ("pdftract_inflight_extractions", "pdftract_inflight_extractions"),
    (
        "pdftract_rayon_pool_utilization",
        "pdftract_rayon_pool_utilization",
    ),
    ("pdftract_build_info", "pdftract_build_info"),
];

/// A spawned pdftract server process with its stderr drained.
///
/// `Drop` kills the child and reaps it with a bounded wait, so a failing
/// assertion can never leak a server process to wedge later runs.
struct Server {
    child: Child,
    stderr: Arc<Mutex<String>>,
}

impl Server {
    /// Spawn `pdftract <args...>` with null stdin/stdout and stderr
    /// drained into a shared buffer (banners are read while the child
    /// runs, so lines are appended as they arrive, not at EOF).
    fn spawn(args: &[&str]) -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_pdftract"))
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn pdftract server");
        let stderr = child.stderr.take().expect("stderr was piped");
        let buffer = Arc::new(Mutex::new(String::new()));
        let shared = Arc::clone(&buffer);
        thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => shared.lock().expect("stderr lock").push_str(&line),
                }
            }
        });
        Server { child, stderr: buffer }
    }

    /// The bound metrics address from the stderr banner, waiting (bounded)
    /// for the listener to come up first.
    fn metrics_url(&self) -> String {
        assert!(
            wait_for(Duration::from_secs(15), || self
                .stderr
                .lock()
                .expect("stderr lock")
                .contains(METRICS_BANNER)),
            "metrics listener banner did not appear within 15s; stderr:\n{}",
            self.stderr_dump()
        );
        let text = self.stderr.lock().expect("stderr lock");
        let start = text.find(METRICS_BANNER).expect("banner present") + METRICS_BANNER.len();
        let rest = &text[start..];
        let end = rest.find("/metrics").expect("banner names the /metrics route");
        format!("http://{}", &rest[..end])
    }

    fn stderr_dump(&self) -> String {
        self.stderr.lock().expect("stderr lock").clone()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            match self.child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => thread::sleep(Duration::from_millis(20)),
                Err(_) => break,
            }
        }
        // SIGKILL plus a 5s grace has always reaped by here; this is the
        // backstop that guarantees no zombie survives the test process.
        let _ = self.child.wait();
    }
}

/// A free loopback port, chosen by binding `:0` and immediately
/// releasing it (the standard grab-and-release; the server binds it next).
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind 127.0.0.1:0")
        .local_addr()
        .expect("local addr")
        .port()
}

/// Poll `cond` (bounded); one final check after the deadline so a slow
/// but successful condition is not lost to scheduling.
fn wait_for(timeout: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if cond() {
            return true;
        }
        thread::sleep(Duration::from_millis(25));
    }
    cond()
}

fn client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("build HTTP client")
}

/// Wait (bounded) until the main port answers `/health`, proving the
/// server is up before assertions that must not race startup.
fn wait_until_healthy(client: &Client, main: u16) {
    let url = format!("http://127.0.0.1:{main}/health");
    assert!(
        wait_for(Duration::from_secs(15), || client
            .get(&url)
            .send()
            .map_or(false, |r| r.status().is_success())),
        "server did not become healthy on :{main} within 15s"
    );
}

/// `GET <metrics_url>/metrics` must return 200 with the exact OpenMetrics
/// content type, `# EOF` termination, and all 13 documented families.
fn assert_exposition(client: &Client, metrics_url: &str) -> String {
    let response = client
        .get(format!("{metrics_url}/metrics"))
        .send()
        .expect("GET /metrics");
    assert_eq!(
        response.status(),
        reqwest::StatusCode::OK,
        "/metrics must return 200"
    );
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .expect("content-type header")
            .to_str()
            .expect("content-type is ASCII"),
        OPENMETRICS_CONTENT_TYPE,
        "content type must be exactly the OpenMetrics v1.0 type"
    );
    let body = response.text().expect("exposition body");

    assert_eq!(
        body.matches("# TYPE ").count(),
        13,
        "plan defines exactly 13 metric families; body:\n{body}"
    );
    for (documented, family) in DOCUMENTED_FAMILIES {
        let as_sample = body.contains(&format!("\n{documented} "));
        let as_metadata = body.contains(&format!("# TYPE {family} "));
        assert!(
            as_sample || as_metadata,
            "documented metric {documented} absent from the exposition:\n{body}"
        );
    }
    assert!(
        body.ends_with("# EOF\n"),
        "OpenMetrics documents terminate in # EOF:\\n; body tail: {:?}",
        &body[body.len().saturating_sub(80)..]
    );
    body
}

/// `pdftract serve --metrics 0` opens a second listener that serves the
/// full 13-family OpenMetrics exposition while the main port serves no
/// `/metrics` route — and after real traffic the counters move from
/// metadata-only to sample lines.
#[test]
fn serve_metrics_listener_serves_openmetrics_on_a_second_port() {
    let main = free_port();
    let server = Server::spawn(&[
        "serve",
        "--bind",
        &format!("127.0.0.1:{main}"),
        "--metrics",
        "0",
    ]);
    let client = client();

    wait_until_healthy(&client, main);
    let metrics_url = server.metrics_url();

    // The exposition on the second listener: full surface, right type.
    assert_exposition(&client, &metrics_url);
    let ready = client
        .get(format!("{metrics_url}/ready"))
        .send()
        .expect("GET /ready on the metrics listener");
    assert_eq!(
        ready.status(),
        reqwest::StatusCode::OK,
        "a serve instance without a cache is ready while the pool is idle"
    );
    let ready_body: serde_json::Value = ready.json().expect("/ready JSON body");
    assert_eq!(ready_body["status"], "ready", "got: {ready_body}");
    assert_eq!(ready_body["unready"], serde_json::json!([]));
    // The main port must NOT serve /metrics (endpoint policy), while
    // still serving its own routes.
    let main_metrics = client.get(format!("http://127.0.0.1:{main}/metrics")).send();
    match main_metrics {
        Ok(response) => assert_eq!(
            response.status(),
            reqwest::StatusCode::NOT_FOUND,
            "/metrics must never be routed on the main serve port"
        ),
        Err(error) => panic!("main port should still be serving: {error}"),
    }

    // Real traffic: bytes that pass the %PDF- magic check but cannot
    // parse complete the request path and record result="error" —
    // moving the labeled `pdftract_extractions_total` family from
    // metadata-only to samples. (Bytes without the magic are rejected
    // by validate_pdf_magic_bytes before the extraction path runs.)
    let mut not_a_pdf = b"%PDF-1.4\n".to_vec();
    not_a_pdf.extend_from_slice(b"this file has no xref, no pages, and no trailer");
    let form = reqwest::blocking::multipart::Form::new().part(
        "pdf",
        reqwest::blocking::multipart::Part::bytes(not_a_pdf).file_name("not-a-pdf.pdf"),
    );
    let extract = client
        .post(format!("http://127.0.0.1:{main}/extract"))
        .multipart(form)
        .send()
        .expect("POST /extract with malformed pdf");
    assert!(
        !extract.status().is_success(),
        "garbage input must not extract successfully"
    );

    let body = assert_exposition(&client, &metrics_url);
    assert!(
        body.contains("pdftract_extractions_total{result=\"error\""),
        "a failed extraction must record pdftract_extractions_total{{result=\"error\"}}; body:\n{body}"
    );
    assert!(
        body.contains("\npdftract_http_requests_total{"),
        "served requests must record pdftract_http_requests_total samples; body:\n{body}"
    );
}

/// `pdftract mcp --bind ... --metrics 0` gets the same second listener:
/// the full exposition on the metrics port, nothing on the main port.
#[test]
fn mcp_metrics_listener_serves_openmetrics_on_a_second_port() {
    let main = free_port();
    let server = Server::spawn(&[
        "mcp",
        "--bind",
        &format!("127.0.0.1:{main}"),
        "--metrics",
        "0",
    ]);
    let client = client();

    wait_until_healthy(&client, main);
    let metrics_url = server.metrics_url();

    assert_exposition(&client, &metrics_url);

    // The second listener serves /metrics and nothing else.
    let response = client
        .get(format!("{metrics_url}/health"))
        .send()
        .expect("GET /health on the metrics listener");
    assert_eq!(
        response.status(),
        reqwest::StatusCode::NOT_FOUND,
        "the metrics listener must serve only /metrics"
    );

    // And the main MCP port must NOT serve /metrics (endpoint policy).
    let response = client
        .get(format!("http://127.0.0.1:{main}/metrics"))
        .send()
        .expect("GET /metrics on the main mcp port");
    assert_eq!(
        response.status(),
        reqwest::StatusCode::NOT_FOUND,
        "/metrics must never be routed on the main mcp port"
    );
}

/// Without `--metrics`, no second listener exists: the main ports answer
/// their own routes but return 404 for `/metrics`, and no metrics banner
/// is ever printed.
#[test]
fn without_the_flag_there_is_no_metrics_listener_and_no_metrics_route() {
    let client = client();

    let serve_port = free_port();
    let serve = Server::spawn(&["serve", "--bind", &format!("127.0.0.1:{serve_port}")]);
    wait_until_healthy(&client, serve_port);
    let response = client
        .get(format!("http://127.0.0.1:{serve_port}/metrics"))
        .send()
        .expect("GET /metrics on serve without --metrics");
    assert_eq!(
        response.status(),
        reqwest::StatusCode::NOT_FOUND,
        "serve without --metrics must not gain a /metrics route"
    );
    assert!(
        !serve.stderr_dump().contains(METRICS_BANNER),
        "no metrics listener may open without --metrics; stderr:\n{}",
        serve.stderr_dump()
    );

    let mcp_port = free_port();
    let mcp = Server::spawn(&["mcp", "--bind", &format!("127.0.0.1:{mcp_port}")]);
    wait_until_healthy(&client, mcp_port);
    let response = client
        .get(format!("http://127.0.0.1:{mcp_port}/metrics"))
        .send()
        .expect("GET /metrics on mcp without --metrics");
    assert_eq!(
        response.status(),
        reqwest::StatusCode::NOT_FOUND,
        "mcp without --metrics must not gain a /metrics route"
    );
    assert!(
        !mcp.stderr_dump().contains(METRICS_BANNER),
        "no metrics listener may open without --metrics; stderr:\n{}",
        mcp.stderr_dump()
    );
}

/// A `--metrics` port that cannot be bound (already in use) fails
/// startup cleanly: an error message naming the failure and a nonzero
/// exit — never a panic.
#[test]
fn an_already_bound_metrics_port_fails_startup_cleanly() {
    // Squatter holds the port for the whole test, so the server's bind
    // of the same port must fail.
    let squatter = TcpListener::bind("127.0.0.1:0").expect("squatter binds on :0");
    let port = squatter.local_addr().expect("squatter local addr").port();

    let main = free_port();
    let mut server = Server::spawn(&[
        "serve",
        "--bind",
        &format!("127.0.0.1:{main}"),
        "--metrics",
        &port.to_string(),
    ]);

    // Bounded wait for the process to exit on its own (clean failure).
    let mut exited = None;
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if let Ok(Some(status)) = server.child.try_wait() {
            exited = Some(status);
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    let status = exited.unwrap_or_else(|| {
        panic!(
            "server kept running with an unbindable --metrics port; stderr:\n{}",
            server.stderr_dump()
        )
    });
    assert!(
        !status.success(),
        "an unbindable --metrics port must exit nonzero, got {status}"
    );

    // The failure is a clean startup error, not a panic.
    wait_for(Duration::from_secs(5), || {
        server.stderr_dump().contains("Failed to bind metrics listener")
    });
    let stderr = server.stderr_dump();
    assert!(
        stderr.contains("Failed to bind metrics listener"),
        "stderr must name the metrics bind failure; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Error:"),
        "stderr must carry the top-level Error line; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("panicked at"),
        "bind failure must not panic; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains(METRICS_BANNER),
        "a failed metrics bind must not print the listener banner; stderr:\n{stderr}"
    );
}

/// `--metrics` is an HTTP-transport flag: under `--stdio` it is rejected
/// as a usage error before any listener opens.
#[test]
fn metrics_with_stdio_transport_is_rejected_as_a_usage_error() {
    let mut server = Server::spawn(&["mcp", "--stdio", "--metrics", "0"]);

    let mut exited = None;
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if let Ok(Some(status)) = server.child.try_wait() {
            exited = Some(status);
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }
    let status = exited.unwrap_or_else(|| {
        panic!(
            "mcp --stdio --metrics kept running; stderr:\n{}",
            server.stderr_dump()
        )
    });
    assert_eq!(status.code(), Some(2), "usage errors exit 2: {status}");
    let stderr = server.stderr_dump();
    assert!(
        stderr.contains("--metrics requires HTTP transport"),
        "stderr must explain the rejection; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains(METRICS_BANNER),
        "no listener may open in stdio mode; stderr:\n{stderr}"
    );
}

/// The metrics listener shares the server process lifetime: stopping the
/// primary server must also release the ephemeral metrics port.
#[test]
fn metrics_listener_shuts_down_with_the_server_process() {
    let main = free_port();
    let server = Server::spawn(&[
        "serve",
        "--bind",
        &format!("127.0.0.1:{main}"),
        "--metrics",
        "0",
    ]);
    let client = client();

    wait_until_healthy(&client, main);
    let metrics_url = server.metrics_url();
    assert_exposition(&client, &metrics_url);

    drop(server);

    assert!(
        wait_for(Duration::from_secs(5), || client
            .get(format!("{metrics_url}/metrics"))
            .send()
            .is_err()),
        "metrics listener should stop when the server process exits"
    );
}
