//! End-to-end regression coverage for `pdftract serve` over the real
//! binary (bead pdftract-4e3b62b5).
//!
//! The defect this file pins down: every request — including
//! `GET /health` — returned HTTP 500 while the production server handed
//! a bare `Router` to `axum::serve`, supplying no
//! `ConnectInfo<SocketAddr>` for the audit/metrics middleware to
//! extract ("Missing request extension" before any handler ran). Fixed
//! by serving through `into_make_service_with_connect_info` (commit
//! 2566480f, bead pdftract-b60ae586).
//!
//! The in-module `serve::tests` cannot catch a regression of that
//! shape: they build the router themselves and serve it through the
//! ConnectInfo make service by hand, so the wiring in `run()` under
//! test here never executes. These tests spawn the shipped binary
//! instead, so the real startup path — argument parsing, listener bind,
//! banner, make-service wiring — is what answers the requests.
//!
//! Test hygiene (repo rules): the child is killed from an RAII guard
//! whose `Drop` reaps with a bounded wait; stderr is drained on a
//! background thread so the child can never block on a full pipe
//! (stdout and stdin are null); the port is chosen by the server itself
//! with `--bind 127.0.0.1:0` and parsed from the startup banner, so
//! reruns never collide on a fixed port still held by a leaked process.

use std::fs;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;

/// The startup banner `serve::run` prints with the address actually
/// bound (port 0 in `--bind` lets the OS choose the real port).
const START_BANNER: &str = "pdftract serve is starting on http://";

/// The tracked repo fixture every serve test uploads (same file the
/// in-module tests use).
fn test_pdf_bytes() -> Vec<u8> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/test-minimal.pdf"
    );
    fs::read(path).unwrap_or_else(|err| panic!("read fixture {path}: {err}"))
}

/// A spawned `pdftract serve` process with its stderr drained.
///
/// `Drop` kills the child and reaps it with a bounded wait, so a
/// failing assertion can never leak a server process to wedge later
/// runs.
struct Server {
    child: Child,
    stderr: Arc<Mutex<String>>,
}

impl Server {
    /// Spawn `pdftract serve <args...>` with null stdin/stdout and
    /// stderr drained into a shared buffer (banners are read while the
    /// child runs, so lines are appended as they arrive, not at EOF).
    fn spawn(args: &[&str]) -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_pdftract"))
            .arg("serve")
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn pdftract serve");
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

    /// The main listener's base URL, parsed from the startup banner
    /// after waiting (bounded) for it. The banner carries the address
    /// actually bound, so `--bind 127.0.0.1:0` yields the OS-chosen
    /// port here — the property that makes `:0` binds usable for test
    /// harnesses at all.
    fn base_url(&self) -> String {
        assert!(
            wait_for(Duration::from_secs(15), || self
                .stderr
                .lock()
                .expect("stderr lock")
                .contains(START_BANNER)),
            "startup banner did not appear within 15s; stderr:\n{}",
            self.stderr_dump()
        );
        let text = self.stderr.lock().expect("stderr lock");
        let start = text.find(START_BANNER).expect("banner present") + START_BANNER.len();
        let rest = &text[start..];
        let end = rest
            .find(|c: char| c.is_whitespace())
            .expect("banner ends the address at whitespace/newline");
        format!("http://{}", &rest[..end])
    }

    fn stderr_dump(&self) -> String {
        self.stderr.lock().expect("stderr lock").clone()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        // Kill first, then reap with a bounded wait (never a bare
        // `wait()`): a panicking test must not leak the server, and a
        // server that ignores the termination request must not wedge
        // the test binary.
        let _ = self.child.kill();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            match self.child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => thread::sleep(Duration::from_millis(20)),
                Err(_) => break,
            }
        }
        // SIGKILL should have reaped the short-lived test server. If
        // the OS still refuses to report exit, return from Drop rather
        // than blocking indefinitely; the bounded cleanup is the
        // important test contract.
    }
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

/// Wait (bounded) until the main port answers `GET /health` with a
/// success status, so single-shot request assertions never race the
/// (sub-millisecond) window between banner and accept loop.
fn wait_until_healthy(client: &Client, base: &str, server: &Server) {
    let url = format!("{base}/health");
    assert!(
        wait_for(Duration::from_secs(15), || client
            .get(&url)
            .send()
            .map_or(false, |r| r.status().is_success())),
        "server did not become healthy on {url} within 15s; stderr:\n{}",
        server.stderr_dump()
    );
}

/// GET /health over the real binary answers 200 — the minimal
/// assertion that was false for every endpoint before the ConnectInfo
/// fix (500 "Missing request extension").
#[test]
fn health_answers_200_over_real_binary() {
    let server = Server::spawn(&["--bind", "127.0.0.1:0"]);
    let base = server.base_url();

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("build HTTP client");
    wait_until_healthy(&client, &base, &server);

    let response = client
        .get(format!("{base}/health"))
        .send()
        .unwrap_or_else(|err| panic!("GET /health: {err}; stderr:\n{}", server.stderr_dump()));
    assert_eq!(
        response.status(),
        reqwest::StatusCode::OK,
        "stderr:\n{}",
        server.stderr_dump()
    );

    let body: serde_json::Value = response.json().expect("/health JSON body");
    assert_eq!(body["status"], "ok", "health body: {body}");
    assert!(
        body["version"].as_str().map_or(false, |v| !v.is_empty()),
        "health body names a version: {body}"
    );
}

/// One full extraction round trip over the real binary: multipart
/// upload of the tracked fixture to `POST /extract` answers 200 with
/// the document schema (non-empty fingerprint, at least one page).
/// Before the ConnectInfo fix this — like every request — died with a
/// 500 before the handler ran.
#[test]
fn extract_answers_200_over_real_binary() {
    let server = Server::spawn(&["--bind", "127.0.0.1:0"]);
    let base = server.base_url();

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("build HTTP client");
    wait_until_healthy(&client, &base, &server);

    let form = reqwest::blocking::multipart::Form::new().part(
        "pdf",
        reqwest::blocking::multipart::Part::bytes(test_pdf_bytes())
            .file_name("test-minimal.pdf")
            .mime_str("application/pdf")
            .expect("static mime"),
    );
    let response = client
        .post(format!("{base}/extract"))
        .multipart(form)
        .send()
        .unwrap_or_else(|err| panic!("POST /extract: {err}; stderr:\n{}", server.stderr_dump()));
    assert_eq!(
        response.status(),
        reqwest::StatusCode::OK,
        "extraction failed; stderr:\n{}",
        server.stderr_dump()
    );

    let body: serde_json::Value = response.json().expect("/extract JSON body");
    assert!(
        body["fingerprint"]
            .as_str()
            .map_or(false, |f| !f.is_empty()),
        "fingerprint present: {body}"
    );
    assert!(
        body["pages"].as_array().map_or(false, |p| !p.is_empty()),
        "at least one page: {body}"
    );
}
