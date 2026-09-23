//! Integration tests for the `--metrics PORT` exposition listener shared
//! by `pdftract serve` and `pdftract mcp --bind` (plan "Monitoring and
//! Alerting", bead pdftract-c2447bd8).
//!
//! These spawn the real binary so the second listener is observed on the
//! wire, not just in-process:
//!
//! - with `--metrics`, a SECOND listener serves `GET /metrics` with the
//!   exact OpenMetrics content type, all 13 documented `pdftract_*`
//!   families, and `# EOF` termination — while the main port does NOT
//!   serve `/metrics`;
//! - `GET /ready` on that listener is 200 while the server accepts work
//!   (idle pool, writable or absent cache) and 503 naming the failed
//!   condition when the cache location cannot be written — with
//!   `GET /health` on the main port staying 200 in that unready state
//!   (liveness and readiness are separate concerns);
//! - every metric name and label selector the shipped alert rules
//!   (`docs/operations/prometheus-rules.yaml`) reference is served by
//!   the live exposition after real traffic;
//! - readiness probes leave no probe files behind in the cache
//!   directory;
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

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
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
    (
        "pdftract_inflight_extractions",
        "pdftract_inflight_extractions",
    ),
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
        Server {
            child,
            stderr: buffer,
        }
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
        let end = rest
            .find("/metrics")
            .expect("banner names the /metrics route");
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

    assert_valid_openmetrics(&body);

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

/// Return the first numeric sample for a metric line prefix. This is used
/// for before/after assertions so a request must move a counter, not merely
/// cause the formatter to emit a family that was already present.
fn sample_value(body: &str, line_prefix: &str) -> Option<f64> {
    body.lines()
        .find(|line| line.starts_with(line_prefix))
        .and_then(|line| line.split_whitespace().last())
        .and_then(|value| value.parse::<f64>().ok())
}

/// The monitoring section is the source of truth for the public metric
/// family list. Keep the integration test coupled to that table instead of
/// letting a renamed or removed documented family silently pass.
fn documented_families_from_plan() -> BTreeSet<String> {
    const PLAN: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/plan/plan.md"
    ));
    let (_, rest) = PLAN
        .split_once("## Monitoring and Alerting")
        .expect("plan must contain a Monitoring and Alerting section");
    let section = rest
        .split_once("\n## ")
        .map_or(rest, |(section, _)| section);

    section
        .lines()
        .filter(|line| line.trim_start().starts_with("| `pdftract_"))
        .filter_map(|line| {
            let metric = line.split('`').nth(1)?;
            metric
                .strip_prefix("pdftract_")
                .map(|_| family_of(metric).to_string())
        })
        .collect()
}

/// Validate the wire body as OpenMetrics text, not merely as a collection of
/// expected substrings. This deliberately covers the subset emitted by the
/// hand-rolled formatter: HELP/TYPE metadata, samples, labels, numeric
/// values, and the required final EOF marker.
fn assert_valid_openmetrics(body: &str) {
    let mut types = BTreeMap::new();
    let mut saw_eof = false;

    for (line_number, line) in body.lines().enumerate() {
        assert!(
            !saw_eof,
            "OpenMetrics has content after # EOF at line {line_number}"
        );
        if line == "# EOF" {
            saw_eof = true;
            continue;
        }
        if line.starts_with("# HELP ") {
            let mut parts = line[7..].splitn(2, ' ');
            let name = parts.next().unwrap_or_default();
            let help = parts.next().unwrap_or_default();
            assert_metric_name(name, line_number);
            assert!(!help.is_empty(), "HELP text is empty at line {line_number}");
            continue;
        }
        if line.starts_with("# TYPE ") {
            let mut parts = line[7..].split_whitespace();
            let name = parts.next().unwrap_or_default();
            let kind = parts.next().unwrap_or_default();
            assert_metric_name(name, line_number);
            assert!(
                matches!(kind, "counter" | "gauge" | "histogram"),
                "unsupported OpenMetrics type {kind:?} at line {line_number}"
            );
            assert!(
                types.insert(name.to_string(), kind).is_none(),
                "duplicate TYPE metadata for {name}"
            );
            continue;
        }
        assert!(
            !line.starts_with('#'),
            "unknown OpenMetrics comment at line {line_number}: {line}"
        );

        let (name, labels, value) = parse_sample(line, line_number);
        let family = family_of(name);
        let kind = types
            .get(family)
            .unwrap_or_else(|| panic!("sample {name} appears before its TYPE metadata"));
        match (
            *kind,
            name.strip_suffix("_total"),
            name.strip_suffix("_bucket"),
        ) {
            ("counter", Some(_), _) => {}
            ("histogram", _, Some(_)) => {
                assert!(labels.contains("le="), "histogram bucket lacks le label")
            }
            ("histogram", _, None) if name.ends_with("_sum") || name.ends_with("_count") => {}
            ("gauge", _, _) => {}
            _ => panic!("sample {name} does not match TYPE {family} {kind}"),
        }
        let _ = value;
    }

    assert!(saw_eof, "OpenMetrics exposition is missing # EOF");
    assert_eq!(
        types.len(),
        DOCUMENTED_FAMILIES.len(),
        "TYPE metadata count changed; update the documented metric surface"
    );
    let metadata_families: BTreeSet<_> = types.keys().cloned().collect();
    let expected_families: BTreeSet<_> = DOCUMENTED_FAMILIES
        .iter()
        .map(|(_, family)| (*family).to_string())
        .collect();
    assert_eq!(metadata_families, expected_families);
    assert_eq!(documented_families_from_plan(), expected_families);
}

fn assert_metric_name(name: &str, line_number: usize) {
    assert!(!name.is_empty(), "empty metric name at line {line_number}");
    assert!(
        name.bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':')),
        "invalid metric name {name:?} at line {line_number}"
    );
}

/// Return `(sample name, label source, numeric value)` after validating the
/// sample's name, optional label set, and value. The label source is retained
/// as text because the family-level assertions only need to verify that the
/// histogram has its required `le` label.
fn parse_sample(line: &str, line_number: usize) -> (&str, &str, &str) {
    let separator = line
        .find(|character: char| character.is_ascii_whitespace())
        .unwrap_or_else(|| panic!("sample has no value at line {line_number}: {line}"));
    let head = &line[..separator];
    let value = line[separator..].trim_start();
    let mut value_parts = value.split_whitespace();
    let numeric = value_parts.next().unwrap_or_default();
    assert!(
        numeric.parse::<f64>().is_ok() || matches!(numeric, "NaN" | "+Inf" | "-Inf"),
        "invalid sample value {numeric:?} at line {line_number}"
    );
    assert!(
        value_parts.next().is_none(),
        "unexpected extra sample fields at line {line_number}: {line}"
    );

    let (name, labels) = match head.split_once('{') {
        Some((name, labels)) => {
            assert!(
                labels.ends_with('}'),
                "unterminated labels at line {line_number}: {line}"
            );
            let labels = &labels[..labels.len() - 1];
            assert_labels(labels, line_number);
            (name, labels)
        }
        None => (head, ""),
    };
    assert_metric_name(name, line_number);
    (name, labels, numeric)
}

fn assert_labels(labels: &str, line_number: usize) {
    if labels.is_empty() {
        return;
    }
    let bytes = labels.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        let name_start = at;
        while at < bytes.len() && bytes[at] != b'=' {
            at += 1;
        }
        assert!(at > name_start, "empty label name at line {line_number}");
        assert_metric_name(&labels[name_start..at], line_number);
        assert!(
            at + 1 < bytes.len() && bytes[at + 1] == b'"',
            "label value must start with a quote at line {line_number}"
        );
        at += 2;
        let mut closed = false;
        while at < bytes.len() {
            match bytes[at] {
                b'\\' => {
                    at += 1;
                    assert!(
                        at < bytes.len(),
                        "trailing label escape at line {line_number}"
                    );
                    assert!(
                        matches!(bytes[at], b'\\' | b'n' | b'"'),
                        "invalid label escape at line {line_number}"
                    );
                    at += 1;
                }
                b'"' => {
                    at += 1;
                    closed = true;
                    break;
                }
                _ => at += 1,
            }
        }
        assert!(closed, "unterminated label value at line {line_number}");
        if at < bytes.len() {
            assert_eq!(
                bytes[at], b',',
                "labels must be comma-separated at line {line_number}"
            );
            at += 1;
        }
    }
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
    let before = assert_exposition(&client, &metrics_url);
    let before_failed_extractions =
        sample_value(&before, "pdftract_extractions_total{result=\"error\"").unwrap_or(0.0);
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
    let main_metrics = client
        .get(format!("http://127.0.0.1:{main}/metrics"))
        .send();
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
    let extract_status = extract.status().as_u16();
    assert!(
        !extract.status().is_success(),
        "garbage input must not extract successfully"
    );

    let body = assert_exposition(&client, &metrics_url);
    let after_failed_extractions =
        sample_value(&body, "pdftract_extractions_total{result=\"error\"")
            .expect("the real request must create an extraction counter sample");
    assert!(
        after_failed_extractions > before_failed_extractions,
        "the real extraction request must move the error counter: before={before_failed_extractions}, after={after_failed_extractions}\n{body}"
    );
    assert!(
        body.contains("pdftract_extractions_total{result=\"error\""),
        "a failed extraction must record pdftract_extractions_total{{result=\"error\"}}; body:\n{body}"
    );
    assert!(
        body.contains("\npdftract_http_requests_total{"),
        "served requests must record pdftract_http_requests_total samples; body:\n{body}"
    );
    assert!(
        body.contains(&format!(
            "pdftract_http_requests_total{{endpoint=\"/extract\",status=\"{extract_status}\"}}"
        )),
        "the malformed service request must carry its registered endpoint and status labels; body:\n{body}"
    );
    assert!(
        body.contains("pdftract_http_requests_total{endpoint=\"/health\",status=\"200\"}"),
        "the liveness request must carry its endpoint and status labels; body:\n{body}"
    );
    assert!(
        body.contains("pdftract_http_requests_total{endpoint=\"/metrics\",status=\"200\"}"),
        "metrics-listener traffic must carry the /metrics endpoint and status labels; body:\n{body}"
    );
    assert!(
        body.contains("pdftract_http_requests_total{endpoint=\"/ready\",status=\"200\"}"),
        "readiness traffic must carry the /ready endpoint and status labels; body:\n{body}"
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

    let before = assert_exposition(&client, &metrics_url);
    let before_mcp_requests =
        sample_value(&before, "pdftract_mcp_requests_total{tool=\"classify\"}").unwrap_or(0.0);

    // A real HTTP MCP tools/call request must be counted by its tool name,
    // even when the selected Phase 5.6 stub returns an application error.
    let call = client
        .post(format!("http://127.0.0.1:{main}/"))
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "classify",
                "arguments": {"path": "unused.pdf"}
            }
        }))
        .send()
        .expect("POST MCP tools/call");
    assert_eq!(call.status(), reqwest::StatusCode::OK);
    let call_body: serde_json::Value = call.json().expect("MCP response JSON");
    assert_eq!(call_body["jsonrpc"], "2.0");
    assert_eq!(call_body["id"], 1);

    // Both the metrics listener and the main MCP listener count unmatched
    // paths with a bounded label rather than exposing a raw request path.
    let response = client
        .get(format!("{metrics_url}/health"))
        .send()
        .expect("GET /health on the metrics listener");
    assert_eq!(
        response.status(),
        reqwest::StatusCode::NOT_FOUND,
        "the metrics listener must not serve the main liveness route"
    );
    let response = client
        .get(format!("http://127.0.0.1:{main}/metrics"))
        .send()
        .expect("GET /metrics on the main mcp port");
    assert_eq!(
        response.status(),
        reqwest::StatusCode::NOT_FOUND,
        "the main MCP listener must not serve /metrics"
    );

    let body = assert_exposition(&client, &metrics_url);
    let after_mcp_requests = sample_value(&body, "pdftract_mcp_requests_total{tool=\"classify\"}")
        .expect("tools/call must create a labeled MCP request sample");
    assert!(
        after_mcp_requests > before_mcp_requests,
        "MCP tool counter must increase: before={before_mcp_requests}, after={after_mcp_requests}\n{body}"
    );
    assert!(
        body.contains("pdftract_http_requests_total{endpoint=\"/\",status=\"200\"}"),
        "the MCP service request must carry endpoint and status labels; body:\n{body}"
    );
    assert!(
        body.contains("pdftract_http_requests_total{endpoint=\"/metrics\",status=\"200\"}"),
        "metrics-listener traffic must carry the /metrics endpoint and status labels; body:\n{body}"
    );
    assert!(
        body.contains("pdftract_http_requests_total{endpoint=\"unmatched\",status=\"404\"}"),
        "the metrics listener's unmatched route traffic must be bounded and labeled; body:\n{body}"
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
        server
            .stderr_dump()
            .contains("Failed to bind metrics listener")
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

// ---------------------------------------------------------------------------
// Alert-rule coverage: every name and label selector the shipped rules
// reference must be served by the live exposition.
// ---------------------------------------------------------------------------

/// The shipped alert rules, relative to this crate's manifest dir (the
/// file cargo runs test binaries from), so the check works regardless
/// of the caller's working directory.
fn alert_rules_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/operations/prometheus-rules.yaml")
}

/// Collect the value of every `expr` key anywhere in the parsed rules
/// document — the PromQL of the rules. Group names (`name:
/// pdftract_serve`) and annotation prose never reach the metric-name
/// scan, and YAML comments — like the note about metrics that are
/// deliberately NOT shipped — are dropped by the parser entirely.
fn expr_strings(value: &serde_yaml::Value, out: &mut Vec<String>) {
    match value {
        serde_yaml::Value::Mapping(map) => {
            for (key, item) in map {
                if key.as_str() == Some("expr") {
                    if let serde_yaml::Value::String(text) = item {
                        out.push(text.clone());
                    }
                }
                expr_strings(item, out);
            }
        }
        serde_yaml::Value::Sequence(items) => {
            for item in items {
                expr_strings(item, out);
            }
        }
        _ => {}
    }
}

/// Every `pdftract_*` metric name in `text`: scan for the prefix and
/// take the longest following run of `[a-z0-9_]`. PromQL identifiers
/// end at `{`, `[`, whitespace or operators, so this yields exactly the
/// series names an expression references.
fn pdftract_names_in(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let bytes = text.as_bytes();
    let mut searched = 0;
    while let Some(found) = text[searched..].find("pdftract_") {
        let start = searched + found;
        let mut end = start + "pdftract_".len();
        while end < bytes.len()
            && (bytes[end].is_ascii_lowercase()
                || bytes[end].is_ascii_digit()
                || bytes[end] == b'_')
        {
            end += 1;
        }
        names.push(text[start..end].to_string());
        searched = end;
    }
    names
}

/// Label names `name` is selected by in `text`: for every `{...}`
/// selector attached to the metric, the identifiers directly followed
/// by `="` or `=~"` (`pdftract_http_requests_total{status=~"5.."}` →
/// `["status"]`). `by (le, ...)` grouping carries no `=`, so it never
/// matches — grouping labels are Prometheus-side, not exposition-side.
fn selector_labels_in(text: &str, name: &str) -> Vec<String> {
    let mut labels = Vec::new();
    let prefix = format!("{name}{{");
    let mut searched = 0;
    while let Some(found) = text[searched..].find(&prefix) {
        let open = searched + found + prefix.len() - 1; // at the '{'
        let close = text[open..]
            .find('}')
            .map(|c| open + c)
            .unwrap_or(text.len());
        let selector = &text[open..close];
        let bytes = selector.as_bytes();
        let mut at = 0;
        while at < bytes.len() {
            if bytes[at].is_ascii_lowercase() || bytes[at] == b'_' {
                let start = at;
                while at < bytes.len()
                    && (bytes[at].is_ascii_lowercase()
                        || bytes[at].is_ascii_digit()
                        || bytes[at] == b'_')
                {
                    at += 1;
                }
                let rest = &selector[at..];
                if rest.starts_with("=~\"") || rest.starts_with("=\"") {
                    labels.push(selector[start..at].to_string());
                }
            } else {
                at += 1;
            }
        }
        searched = close;
    }
    labels
}

/// The family behind a rules-file series name: counters are referenced
/// with their `_total` sample suffix and histogram series as
/// `_bucket`/`_sum`/`_count`, while `# TYPE` metadata carries the bare
/// family name.
fn family_of(name: &str) -> &str {
    for suffix in ["_total", "_bucket", "_sum", "_count"] {
        if let Some(base) = name.strip_suffix(suffix) {
            return base;
        }
    }
    name
}

/// Whether the exposition serves `name` — as a sample line (labeled or
/// plain) or as its family's `# TYPE` metadata.
fn exposition_has_name(body: &str, name: &str) -> bool {
    body.contains(&format!("\n{name} "))
        || body.contains(&format!("\n{name}{{"))
        || body.contains(&format!("\n# TYPE {} ", family_of(name)))
}

/// Whether the exposition carries at least one sample of `name` whose
/// label set includes `label` — what a rules-file selector like
/// `severity="error"` actually matches against.
fn exposition_serves_label(body: &str, name: &str, label: &str) -> bool {
    let marker = format!("{label}=\"");
    body.lines().any(|line| {
        line.starts_with(name)
            && matches!(
                line.get(name.len()..).and_then(|rest| rest.chars().next()),
                Some('{') | Some(' ')
            )
            && line.contains(&marker)
    })
}

/// POST one multipart upload of bytes that pass the %PDF- magic check
/// but cannot parse; the request completes the extraction path and
/// records labeled `error` samples (extractions, http requests,
/// diagnostics). Returns the response status code.
fn post_unparseable_pdf(client: &Client, main: u16) -> reqwest::StatusCode {
    let mut not_a_pdf = b"%PDF-1.4\n".to_vec();
    not_a_pdf.extend_from_slice(b"this file has no xref, no pages, and no trailer");
    let form = reqwest::blocking::multipart::Form::new().part(
        "pdf",
        reqwest::blocking::multipart::Part::bytes(not_a_pdf).file_name("not-a-pdf.pdf"),
    );
    let response = client
        .post(format!("http://127.0.0.1:{main}/extract"))
        .multipart(form)
        .send()
        .expect("POST /extract with malformed pdf");
    let status = response.status();
    assert!(
        !status.is_success(),
        "garbage input must not extract successfully"
    );
    status
}

/// The names and label selectors the shipped alert rules reference are
/// the exposition's real contract: a rule naming a metric the listener
/// does not serve could never fire. This test parses
/// `docs/operations/prometheus-rules.yaml` (not a hard-coded copy) and
/// checks every referenced `pdftract_*` series — and every label
/// selected on it — against the live exposition of a serve instance
/// after real traffic, so the check tracks the rules file as it
/// evolves.
#[test]
fn exposition_serves_every_metric_and_label_the_alert_rules_use() {
    // Derive the contract from the shipped rules file.
    let rules_path = alert_rules_path();
    let rules_text = fs::read_to_string(&rules_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", rules_path.display()));
    let parsed: serde_yaml::Value =
        serde_yaml::from_str(&rules_text).unwrap_or_else(|error| panic!("parse rules: {error}"));
    let mut exprs_parts = Vec::new();
    expr_strings(&parsed, &mut exprs_parts);
    let exprs = exprs_parts.join("\n");
    assert!(
        !exprs.is_empty(),
        "no expr strings found in {} — the file's shape changed",
        rules_path.display()
    );

    let names: BTreeSet<String> = pdftract_names_in(&exprs).into_iter().collect();
    // Guard against a silent vacuous pass: these series are what the
    // current rules reference, so a file restructure that hides them
    // from the scan must fail here, not pass.
    for expected in [
        "pdftract_extraction_duration_seconds_bucket",
        "pdftract_http_requests_total",
        "pdftract_cache_hits_total",
        "pdftract_cache_misses_total",
        "pdftract_rayon_pool_utilization",
        "pdftract_cache_size_bytes",
        "pdftract_diagnostic_emitted_total",
    ] {
        assert!(
            names.contains(expected),
            "alert rules no longer reference {expected}; update this test's expected list"
        );
    }
    let labeled: Vec<(String, String)> = names
        .iter()
        .flat_map(|name| {
            selector_labels_in(&exprs, name)
                .into_iter()
                .map(|label| (name.clone(), label))
        })
        .collect();
    // The current rules select `status` on http requests and
    // `severity` on diagnostics; both need real samples after traffic.
    assert!(
        labeled.contains(&(
            "pdftract_http_requests_total".to_string(),
            "status".to_string()
        )),
        "rules no longer select a status label; update this test"
    );
    assert!(
        labeled.contains(&(
            "pdftract_diagnostic_emitted_total".to_string(),
            "severity".to_string()
        )),
        "rules no longer select a severity label; update this test"
    );

    // A serve instance WITH a cache, so the healthy readiness path also
    // runs its real writability probe against a live directory.
    let cache = tempfile::tempdir().expect("cache tempdir");
    let main = free_port();
    let server = Server::spawn(&[
        "serve",
        "--bind",
        &format!("127.0.0.1:{main}"),
        "--cache-dir",
        cache.path().to_str().expect("cache path is UTF-8"),
        "--metrics",
        "0",
    ]);
    let client = client();

    wait_until_healthy(&client, main);
    let metrics_url = server.metrics_url();

    // Healthy readiness with a real (writable) cache location.
    let ready = client
        .get(format!("{metrics_url}/ready"))
        .send()
        .expect("GET /ready");
    assert_eq!(
        ready.status(),
        reqwest::StatusCode::OK,
        "writable cache + idle pool is ready"
    );

    // Real traffic so labeled families gain samples.
    let failure_status = post_unparseable_pdf(&client, main);
    assert!(
        failure_status.as_u16() >= 400,
        "garbage upload must fail 4xx/5xx, got {failure_status}"
    );

    let body = assert_exposition(&client, &metrics_url);

    // Every referenced series is served, as samples or family metadata.
    for name in &names {
        assert!(
            exposition_has_name(&body, name),
            "alert rule references {name} but the exposition does not serve it:\n{body}"
        );
    }
    // Every selected label has a matching labeled sample.
    for (name, label) in &labeled {
        assert!(
            exposition_serves_label(&body, name, label),
            "alert rule selects {label} on {name} but no sample carries it:\n{body}"
        );
    }
    // histogram_quantile needs the bucket series with le labels,
    // including the mandatory +Inf bucket.
    assert!(
        exposition_serves_label(&body, "pdftract_extraction_duration_seconds_bucket", "le"),
        "histogram buckets must carry le labels:\n{body}"
    );
    assert!(
        body.contains("pdftract_extraction_duration_seconds_bucket{le=\"+Inf\"}"),
        "the mandatory le=\"+Inf\" bucket is missing:\n{body}"
    );
    // The error-rate rule divides by all http traffic and matches
    // status=~"5..": the status label values must be bare 3-digit codes.
    let statuses: Vec<&str> = body
        .lines()
        .filter(|line| line.starts_with("pdftract_http_requests_total{"))
        .filter_map(|line| line.split("status=\"").nth(1))
        .map(|rest| &rest[..rest.find('"').unwrap_or(rest.len())])
        .collect();
    assert!(
        !statuses.is_empty(),
        "no http request samples after real traffic:\n{body}"
    );
    for status in statuses {
        assert_eq!(
            status.len(),
            3,
            "status label values must be bare 3-digit codes for the status=~\"5..\" matcher, got {status:?}:\n{body}"
        );
        assert!(
            status.chars().all(|c| c.is_ascii_digit()),
            "status label values must be numeric, got {status:?}:\n{body}"
        );
    }

    // Readiness probing leaves nothing behind: repeated probes (the
    // ready GET above) must not leave probe files in the cache dir.
    let leftovers: Vec<_> = fs::read_dir(cache.path())
        .expect("read cache dir")
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|name| name.starts_with(".pdftract-ready-probe"))
                .unwrap_or(false)
        })
        .collect();
    assert!(
        leftovers.is_empty(),
        "readiness probes must not leave files in the cache dir: {leftovers:?}"
    );
}

/// Restores a directory's mode when dropped, so a panic between the
/// chmod and the end of the test cannot leave the tempdir read-only.
struct RestoreMode {
    path: PathBuf,
    mode: u32,
}

impl Drop for RestoreMode {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(self.mode));
    }
}

/// End-to-end readiness failure: a serve instance whose cache location
/// exists but cannot be written (mode 0555) reports 503 from GET /ready
/// with the body naming the cache — while GET /health on the main port
/// stays 200, because an unready server is still alive and must not be
/// restarted by a liveness probe.
///
/// Skips when permission bits are advisory (running as root), exactly
/// like the in-process `probe_rejects_a_readonly_directory...` test.
#[test]
fn ready_is_503_with_an_unwritable_cache_while_health_stays_200() {
    let cache = tempfile::tempdir().expect("cache tempdir");
    let original = fs::metadata(cache.path())
        .expect("cache dir metadata")
        .permissions()
        .mode();
    // Declared after `cache`, dropped before it: the mode is restored
    // while the directory is still there, then TempDir removes it.
    let _restore = RestoreMode {
        path: cache.path().to_path_buf(),
        mode: original,
    };
    fs::set_permissions(cache.path(), fs::Permissions::from_mode(0o555))
        .expect("chmod 0555 the cache dir");

    // The write attempt must actually fail for this uid, or there is
    // nothing to observe (root ignores the mode bits).
    let advisory = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(cache.path().join("advisory-check"))
        .is_ok();
    if advisory {
        fs::remove_file(cache.path().join("advisory-check")).expect("remove advisory check");
        eprintln!("skipping: permission bits are advisory here (uid 0)");
        return;
    }

    let main = free_port();
    let server = Server::spawn(&[
        "serve",
        "--bind",
        &format!("127.0.0.1:{main}"),
        "--cache-dir",
        cache.path().to_str().expect("cache path is UTF-8"),
        "--metrics",
        "0",
    ]);
    let client = client();

    // The main port comes up and reports healthy in the unready state —
    // liveness first, and the wait itself asserts /health answers 200.
    wait_until_healthy(&client, main);
    let metrics_url = server.metrics_url();

    let ready = client
        .get(format!("{metrics_url}/ready"))
        .send()
        .expect("GET /ready with an unwritable cache");
    assert_eq!(
        ready.status(),
        reqwest::StatusCode::SERVICE_UNAVAILABLE,
        "an unwritable cache location must make /ready 503"
    );
    let body: serde_json::Value = ready.json().expect("/ready JSON body");
    assert_eq!(body["status"], "unready", "got: {body}");
    assert_eq!(
        body["unready"],
        serde_json::json!(["cache_unwritable"]),
        "the body must name the cache and nothing else: {body}"
    );
    assert_eq!(body["cache_writable"], false, "got: {body}");

    // And /health stays 200 — the failure case must not turn a busy or
    // degraded-but-alive server into a restart.
    let health = client
        .get(format!("http://127.0.0.1:{main}/health"))
        .send()
        .expect("GET /health while unready");
    assert_eq!(
        health.status(),
        reqwest::StatusCode::OK,
        "/health is liveness and stays 200 while /ready is 503"
    );
}
