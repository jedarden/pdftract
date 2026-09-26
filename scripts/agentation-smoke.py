#!/usr/bin/env python3
"""Browser smoke test for Agentation on every workspace HTML entry point."""

from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import sys
from threading import Thread

try:
    from playwright.sync_api import sync_playwright
except ImportError as error:  # pragma: no cover - depends on the verification host
    raise SystemExit("agentation-smoke.py requires the Playwright Python package") from error


ROOT = Path(__file__).resolve().parents[1]
PAGES = (
    "/crates/pdftract-cli/src/inspect/demo-json-tree-navigation.html",
    "/crates/pdftract-cli/src/inspect/frontend/index.html",
    "/crates/pdftract-inspector-ui/static/index.html",
)


class RepositoryHandler(SimpleHTTPRequestHandler):
    """Serve the source pages and the CLI's absolute /static/ asset paths."""

    STATIC_ASSETS = {
        "/static/app.js": ROOT / "crates/pdftract-cli/src/inspect/frontend/app.js",
        "/static/style.css": ROOT / "crates/pdftract-cli/src/inspect/frontend/style.css",
        "/static/agentation.js": ROOT / "crates/pdftract-cli/src/inspect/agentation.js",
    }

    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=ROOT, **kwargs)

    def do_GET(self):  # noqa: N802 - required by BaseHTTPRequestHandler
        asset = self.STATIC_ASSETS.get(self.path)
        if asset is None:
            return super().do_GET()

        body = asset.read_bytes()
        self.send_response(200)
        self.send_header("Content-Type", "application/javascript; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def end_headers(self):
        if self.path == "/crates/pdftract-cli/src/inspect/frontend/index.html":
            self.send_header(
                "Content-Security-Policy",
                "default-src 'self'; script-src 'self' https://esm.sh; "
                "style-src 'self' 'unsafe-inline'",
            )
        super().end_headers()

    def log_message(self, format, *args):  # noqa: A002 - inherited signature
        del format, args


def main() -> int:
    server = ThreadingHTTPServer(("127.0.0.1", 0), RepositoryHandler)
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()
    base_url = f"http://127.0.0.1:{server.server_port}"

    try:
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(headless=True)
            try:
                for page_path in PAGES:
                    page = browser.new_page()
                    page.goto(base_url + page_path, wait_until="load")
                    page.wait_for_function(
                        "() => !!document.getElementById('agentation-root')",
                        timeout=15_000,
                    )
                    mounted = page.evaluate(
                        "() => !!document.getElementById('agentation-root')"
                    )
                    if not mounted:
                        raise AssertionError(f"Agentation did not mount on {page_path}")
                    print(f"Agentation mounted: {page_path}")
                    page.close()
            finally:
                browser.close()
    finally:
        server.shutdown()
        thread.join()
        server.server_close()

    return 0


if __name__ == "__main__":
    sys.exit(main())
