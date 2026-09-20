//! Registration of `pdftract_remote_bytes_downloaded_total` on the remote
//! fetch path (`metrics` + `remote` features).
//!
//! `pdftract-core`'s `HttpRangeSource` sits below the cli-owned
//! [`crate::metrics::Registry`] and cannot reference cli types, so the
//! source exposes an observation-only bytes-downloaded hook
//! (`pdftract_core::source::BytesDownloadedHook`) and this module builds
//! the registry-backed callback that the cli registers where it builds a
//! remote source (`main.rs`'s URL-extract path,
//! `hash::compute_fingerprint_from_url`, and the remote branch of
//! `grep::worker::worker_run`). Observation only: the hook never
//! influences range reads, retries, or error classification, and there
//! is no endpoint, no CLI flag, and no listener here.

use std::sync::Arc;

use pdftract_core::source::{BytesDownloadedHook, HttpRangeSource};

/// Build the registry-backed hook handed to a remote source.
///
/// Every invocation adds the reported byte count to
/// `pdftract_remote_bytes_downloaded_total` on the shared registry.
pub(crate) fn bytes_downloaded_hook(registry: &crate::metrics::Registry) -> BytesDownloadedHook {
    let registry = registry.clone();
    Arc::new(move |bytes| registry.add_remote_bytes_downloaded(bytes))
}

/// Register a command-scoped metrics registry's bytes-downloaded hook on
/// a remote source built by a cli command, returning the registry.
///
/// Standalone commands have no exposition surface yet (the plan's
/// `/metrics` listener belongs to `serve`/`mcp`), so the counter is fed
/// for the command's lifetime and becomes observable once a surface that
/// owns a registry also fetches remote bytes. Without this registration
/// the counter is dead code.
pub(crate) fn register_bytes_downloaded_hook(
    source: &mut HttpRangeSource,
) -> crate::metrics::Registry {
    let registry = crate::metrics::Registry::new();
    source.set_bytes_downloaded_hook(Some(bytes_downloaded_hook(&registry)));
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc as StdArc;
    use std::time::Duration;

    /// Minimal loopback HTTP server with Range support, bound on `:0`.
    ///
    /// The accept loop polls a shutdown flag while non-blocking, so the
    /// server thread is gone within a poll interval of [`Drop::drop`] —
    /// the drop waits (bounded) for the thread's exit flag instead of an
    /// unkillable join.
    struct LoopbackRangeServer {
        _shutdown: StdArc<AtomicBool>,
        _finished: StdArc<AtomicBool>,
        _listener: TcpListener,
        url: String,
    }

    impl LoopbackRangeServer {
        fn spawn(pdf_data: Vec<u8>) -> std::io::Result<Self> {
            Self::spawn_with_range_support(pdf_data, true)
        }

        /// A server that advertises no Range support: HEAD omits
        /// `Accept-Ranges` and GET answers `200 OK` with the full body —
        /// the shape that drives `open_remote` into the
        /// download-to-temp fallback.
        fn spawn_without_range_support(pdf_data: Vec<u8>) -> std::io::Result<Self> {
            Self::spawn_with_range_support(pdf_data, false)
        }

        fn spawn_with_range_support(
            pdf_data: Vec<u8>,
            supports_range: bool,
        ) -> std::io::Result<Self> {
            let listener = TcpListener::bind("127.0.0.1:0")?;
            listener.set_nonblocking(true)?;
            let url = format!("http://{}/doc.pdf", listener.local_addr()?);

            let pdf_data = StdArc::new(pdf_data);
            let shutdown = StdArc::new(AtomicBool::new(false));
            let finished = StdArc::new(AtomicBool::new(false));

            let thread_listener = listener.try_clone()?;
            let thread_pdf = pdf_data;
            let thread_shutdown = shutdown.clone();
            let thread_finished = finished.clone();

            std::thread::spawn(move || {
                loop {
                    if thread_shutdown.load(Ordering::Relaxed) {
                        break;
                    }
                    match thread_listener.accept() {
                        Ok((mut stream, _)) => {
                            // Accepted sockets inherit the listener's
                            // non-blocking mode on Linux; request
                            // handling below needs blocking reads.
                            let _ = stream.set_nonblocking(false);
                            let _ = handle_connection(&mut stream, &thread_pdf, supports_range);
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(2));
                        }
                        Err(_) => break,
                    }
                }
                thread_finished.store(true, Ordering::Relaxed);
            });

            Ok(Self {
                _shutdown: shutdown,
                _finished: finished,
                _listener: listener,
                url,
            })
        }
    }

    impl Drop for LoopbackRangeServer {
        fn drop(&mut self) {
            self._shutdown.store(true, Ordering::Relaxed);
            // Bounded wait for the accept loop to observe the flag; the
            // non-blocking poll interval is 2ms, so this returns long
            // before the bound.
            let deadline = std::time::Instant::now() + Duration::from_millis(200);
            while !self._finished.load(Ordering::Relaxed) && std::time::Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(2));
            }
        }
    }

    /// Serve one request: HEAD returns size (+ `Accept-Ranges: bytes`
    /// when `supports_range`), GET with a Range returns 206 with the
    /// (clamped) slice; without Range support GET returns `200 OK` with
    /// the whole body, like a real no-range server.
    fn handle_connection(
        stream: &mut TcpStream,
        pdf_data: &[u8],
        supports_range: bool,
    ) -> std::io::Result<()> {
        let mut buffer = [0u8; 8192];
        let mut head = Vec::new();
        while !head.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = stream.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            head.extend_from_slice(&buffer[..n]);
            if head.len() > 64 * 1024 {
                break;
            }
        }
        let request = String::from_utf8_lossy(&head);
        let mut lines = request.lines();
        let first_line = lines.next().unwrap_or("");
        let mut parts = first_line.split_whitespace();
        let method = parts.next().unwrap_or("");

        let mut response = Vec::new();
        if method == "HEAD" {
            response.extend_from_slice(b"HTTP/1.1 200 OK\r\n");
            response.extend_from_slice(b"Content-Length: ");
            response.extend_from_slice(pdf_data.len().to_string().as_bytes());
            response.extend_from_slice(b"\r\n");
            if supports_range {
                response.extend_from_slice(b"Accept-Ranges: bytes\r\n");
            }
            response.extend_from_slice(b"Connection: close\r\n\r\n");
        } else if method == "GET" {
            let range = request
                .lines()
                .find_map(|l| l.strip_prefix("Range: bytes="))
                .map(str::trim)
                .and_then(|v| v.split_once('-'))
                .and_then(|(s, e)| Some(s.parse::<usize>().ok()?, e.parse::<usize>().ok()?));

            let (start, end) = match range {
                Some((start, end)) => (start, end.min(pdf_data.len() - 1)),
                None => (0, pdf_data.len() - 1),
            };
            let data = &pdf_data[start..=end];

            if !supports_range {
                // No Range support: ignore the header, serve everything
                // with a plain 200 (the source aborts and falls back).
                response.extend_from_slice(b"HTTP/1.1 200 OK\r\n");
                response.extend_from_slice(b"Content-Length: ");
                response.extend_from_slice(pdf_data.len().to_string().as_bytes());
                response.extend_from_slice(b"\r\nConnection: close\r\n\r\n");
                response.extend_from_slice(pdf_data);
            } else {
                response.extend_from_slice(b"HTTP/1.1 206 Partial Content\r\n");
                response.extend_from_slice(
                    format!(
                        "Content-Range: bytes {}-{}/{}\r\n",
                        start,
                        end,
                        pdf_data.len()
                    )
                    .as_bytes(),
                );
                response.extend_from_slice(b"Content-Length: ");
                response.extend_from_slice(data.len().to_string().as_bytes());
                response.extend_from_slice(b"\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n");
                response.extend_from_slice(data);
            }
        } else {
            response.extend_from_slice(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n");
        }

        stream.write_all(&response)?;
        stream.flush()
    }

    /// The whole wiring under test: a loopback server on `:0`, a source
    /// opened through the hook seam, and the registry it feeds.
    fn open_wired_source(
        server: &LoopbackRangeServer,
    ) -> (crate::metrics::Registry, HttpRangeSource) {
        let registry = crate::metrics::Registry::new();
        let hook = bytes_downloaded_hook(&registry);
        let source = HttpRangeSource::with_headers_and_hook(&server.url, Vec::new(), Some(hook))
            .expect("open remote source against loopback server");
        (registry, source)
    }

    fn fixture_pdf() -> Vec<u8> {
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/test-minimal.pdf"
        ))
        .expect("tests/fixtures/test-minimal.pdf fixture")
    }

    #[test]
    fn test_metrics_remote_bytes_downloaded_incremented_by_body_size() {
        let pdf = fixture_pdf();

        let server = LoopbackRangeServer::spawn(pdf.clone()).expect("bind loopback server on :0");
        let (registry, mut source) = open_wired_source(&server);
        assert!(source.supports_range());

        // One remote fetch covers the whole (single-block) document.
        let got = source.read_range(0, pdf.len()).expect("range read");
        assert_eq!(got.as_ref(), pdf.as_slice());

        let expected = format!("pdftract_remote_bytes_downloaded_total {}\n", pdf.len());
        assert!(
            registry.render().contains(&expected),
            "counter not incremented by body size; rendered:\n{}",
            registry.render()
        );

        // A cached re-read must not re-count: the hook observes fetches,
        // not reads (observation only, no double counting).
        let again = source.read_range(0, pdf.len()).expect("cached re-read");
        assert_eq!(again.as_ref(), pdf.as_slice());
        assert!(
            registry.render().contains(&expected),
            "cached re-read re-counted bytes; rendered:\n{}",
            registry.render()
        );
    }

    #[test]
    fn test_metrics_remote_bytes_downloaded_counts_fetched_bytes_not_requested() {
        let pdf = fixture_pdf();

        let server = LoopbackRangeServer::spawn(pdf.clone()).expect("bind loopback server on :0");
        let (registry, mut source) = open_wired_source(&server);

        // Two reads at discontiguous offsets request 150 bytes together,
        // but both fall inside the document's single block: the first
        // read fetches one whole-body block and the second is a cache
        // hit. The counter must reflect the bytes actually downloaded
        // (one full body), not the bytes requested.
        let front = source.read_range(0, 100).expect("front read");
        assert_eq!(front.as_ref(), &pdf[..100]);
        let back_start = pdf.len() - 50;
        let back = source.read_range(back_start as u64, 50).expect("back read");
        assert_eq!(back.as_ref(), &pdf[back_start..]);

        let expected = format!("pdftract_remote_bytes_downloaded_total {}\n", pdf.len());
        assert!(
            registry.render().contains(&expected),
            "counter should equal the one fetched body; rendered:\n{}",
            registry.render()
        );
    }

    #[test]
    fn test_metrics_register_bytes_downloaded_hook_wires_command_registry() {
        let pdf = b"%PDF-1.4 tiny body".to_vec();
        let server = LoopbackRangeServer::spawn(pdf.clone()).expect("bind loopback server on :0");

        // The cli registration path used by main.rs: register the hook on
        // a plain source and hold the returned command-scoped registry.
        let mut wired =
            HttpRangeSource::with_headers(&server.url, Vec::new()).expect("open source");
        let command_registry = register_bytes_downloaded_hook(&mut wired);
        let _ = wired.read_range(0, pdf.len()).expect("wired read");

        let expected = format!("pdftract_remote_bytes_downloaded_total {}\n", pdf.len());
        assert!(
            command_registry.render().contains(&expected),
            "command-scoped registry not fed by the fetch; rendered:\n{}",
            command_registry.render()
        );
    }

    #[test]
    fn test_metrics_remote_bytes_downloaded_counts_no_range_fallback_download() {
        use std::io::Read as _;

        let pdf = fixture_pdf();
        let server =
            LoopbackRangeServer::spawn_without_range_support(pdf.clone())
                .expect("bind loopback server on :0");

        // The second seam branch: a server without Range support drives
        // `open_remote` into the download-to-temp fallback, which must
        // feed the same counter through the RemoteOpts-carried hook.
        let registry = crate::metrics::Registry::new();
        let opts = pdftract_core::source::RemoteOpts::new()
            .with_bytes_downloaded_hook(bytes_downloaded_hook(&registry));
        let mut source = pdftract_core::source::open_remote(&server.url, &opts, None)
            .expect("open remote source against a no-range server");
        assert_eq!(
            source.len(),
            pdf.len() as u64,
            "fallback source lost the HEAD-probed content length"
        );

        let mut downloaded = Vec::new();
        source
            .read_to_end(&mut downloaded)
            .expect("read the full fallback-downloaded body");
        assert_eq!(downloaded.as_slice(), pdf.as_slice(), "body mismatch");

        let expected = format!("pdftract_remote_bytes_downloaded_total {}\n", pdf.len());
        assert!(
            registry.render().contains(&expected),
            "fallback download did not feed the counter; rendered:\n{}",
            registry.render()
        );
    }
}
