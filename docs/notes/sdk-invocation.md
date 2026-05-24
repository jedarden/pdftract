# pdftract SDK Invocation Guide

How to invoke the `pdftract` binary from various languages, both via subprocess and via the HTTP server mode.

## Binary Modes Reference

```
pdftract extract <file.pdf>                      # JSON to stdout
pdftract extract <file.pdf> --text               # plain text to stdout
pdftract extract <file.pdf> --output out.json    # JSON to file
pdftract serve --port 8080                       # HTTP server: POST /extract → JSON
pdftract mcp --bind 127.0.0.1:0 --auth-token-file token.txt  # MCP server over HTTP or stdio
```

---

## Subprocess Contract

Every SDK invoking pdftract via subprocess MUST follow this contract. The contract defines the wire protocol between the SDK and the binary: argument layout, stream discipline, exit codes, and environment variable handling.

### argv Layout

The canonical form an SDK SHOULD construct:

```
pdftract <SUBCOMMAND> [GLOBAL_OPTIONS] <POSITIONAL_ARGS> [SUBCOMMAND_OPTIONS]
```

- **SUBCOMMAND**: `extract`, `serve`, `mcp`, `verify-receipt`, `inspect`
- **GLOBAL_OPTIONS**: Flags that apply to all subcommands (`--help`, `--version`, `--config PATH`)
- **POSITIONAL_ARGS**: Subcommand-specific arguments (e.g., PDF file path for `extract`)
- **SUBCOMMAND_OPTIONS**: Flags specific to the subcommand (e.g., `--text`, `--json`, `--output PATH`)

**Rules:**
1. Multi-value flags (e.g., `--profile NAME`) may be repeated; order is preserved.
2. Flag arguments MUST use `--flag=value` or `--flag value` syntax (both are accepted).
3. The PDF path is the first positional argument to `extract`. Use `-` to read PDF bytes from stdin (for remote sources or in-memory PDFs).
4. `--json` is implicit for `extract` when neither `--text` nor `--output PATH` is specified.
5. `--output PATH` writes JSON to a file; stdout contains only the path to that file on success.

**Examples:**
```bash
# Basic extraction (JSON to stdout)
pdftract extract document.pdf

# Plain text output
pdftract extract document.pdf --text

# JSON to file (stdout contains only the file path on success)
pdftract extract document.pdf --output /tmp/result.json

# With profile and cache
pdftract extract document.pdf --profile scientific_paper --cache-dir /var/cache/pdftract

# Remote source (PDF bytes fetched via HTTP, piped to stdin)
curl -s https://example.com/doc.pdf | pdftract extract -

# Multi-format output (JSON + Markdown + plain text)
pdftract extract document.pdf --json --md --text --output-dir /tmp/outputs
```

### stdin Discipline

stdin is used for two purposes: password ingress and PDF bytes.

**Password ingress (`--password-stdin`):**
- When `--password-stdin` is present, pdftract reads **exactly one line** from stdin and uses it as the PDF password.
- The line is stripped of the trailing newline but NOT whitespace-trimmed.
- After reading the password, stdin is NOT consumed further; the PDF must be provided via a positional argument (not stdin).
- The password value is NEVER logged, appears in no diagnostic output, and is redacted from `--capture-diagnostics` archives.
- **TH-07**: `--password VALUE` on the command line is REJECTED unless `PDFTRACT_INSECURE_CLI_PASSWORD=1` is set. SDKs MUST use `--password-stdin` or `PDFTRACT_PASSWORD` instead.

**PDF bytes from stdin:**
- When the PDF path is `-`, pdftract reads the entire PDF byte stream from stdin.
- This is the canonical way to handle remote sources (HTTP-fetched PDFs) or in-memory PDFs without writing to disk.
- stdin is read to EOF; the binary does NOT prompt or interact.
- When `-` is used as the path, `--password-stdin` cannot be used simultaneously (both would consume stdin). Use `PDFTRACT_PASSWORD` instead.

**Example:**
```bash
# Password via stdin
echo "secret123" | pdftract extract --password-stdin encrypted.pdf

# Remote PDF fetched via curl, piped to pdftract
curl -s https://example.com/doc.pdf | pdftract extract -

# DO NOT DO THIS (TH-07 violation -- rejected unless opt-in):
pdftract extract encrypted.pdf --password secret123
```

### stdout Discipline

stdout carries ONLY the extraction output in structured form. NOTHING else may be written to stdout.

**`extract` subcommand:**
- In `--json` mode (default): a single JSON object conforming to `docs/schema/v1.0/pdftract.schema.json`. No trailing newlines beyond the JSON structure.
- In `--text` mode: plain text, UTF-8 encoded. Lines are separated by `\n`. No trailing metadata.
- In `--output PATH` mode: the absolute path to the output file is written to stdout on success. On error, stderr contains the diagnostic and stdout is empty.
- **Critical**: SDKs that mix log lines into stdout break JSON parsing. The binary MUST keep stdout clean.

**`serve` / `mcp --bind` modes:**
- stdout is NOT used for request responses. HTTP responses go to the socket; MCP JSON-RPC frames go to the transport (stdio for MCP stdio mode, HTTP for MCP `--bind` mode).
- Log lines are routed to stderr via the `log` crate (see stderr discipline).

**INV-9 (MCP stdio mode):** In MCP stdio mode, stdout MUST contain ONLY JSON-RPC frames. Any non-JSON-RPC byte breaks the protocol.

### stderr Discipline

stderr carries human-readable logs, progress events, and diagnostics. The format is NOT machine-parseable (except for `--progress-json` mode, see below).

**Log levels (controlled by `RUST_LOG`):**
- `error`: Fatal errors that prevent extraction (e.g., "cannot open input file").
- `warn`: Non-fatal issues (e.g., "cache miss, extracting from PDF").
- `info` (default): High-level progress (e.g., "extracting page 5 of 10", "profile matched: scientific_paper").
- `debug`: Per-phase timing, resolved options (passwords redacted), per-page glyph/span counts.
- `trace`: Detailed phase internals (cache key derivation steps, etc.).

**Progress events (when `--progress-json` is set):**
- Each event is emitted as a single-line JSON object on stderr, newline-delimited (ndjson format).
- See `--progress-json` schema below.

**NEVER logged at any level:**
- Password values (PDF, MCP, inspector) — redacted as `<redacted>`
- Bearer-token values — redacted as `<redacted>`
- PDF byte contents — only the SHA-256 fingerprint is logged
- Full extracted text — only span/page counts
- `Cookie`, `Authorization`, or `Proxy-Authorization` HTTP headers

### Exit Code Taxonomy

pdftract follows the sysexits(3) convention. Every exit code below 64 is reserved; codes 64–78 are application-specific.

| Exit Code | Name | Meaning | TH Reference |
|-----------|------|---------|--------------|
| 0 | SUCCESS | Extraction completed successfully. | — |
| 64 | USAGE_ERROR | Invalid command-line arguments, unknown flags, conflicting options. | — |
| 65 | DATA_ERROR | Malformed PDF (cannot parse xref, trailer, or page tree). | — |
| 66 | PASSWORD_MISSING | PDF is encrypted but no password was provided. | TH-07 |
| 67 | CANNOT_OPEN_INPUT | File not found or permission denied. | — |
| 70 | INTERNAL_ERROR | Unexpected panic or bug (should never happen in production). | INV-8 |
| 73 | CANNOT_CREATE_OUTPUT | Cannot write to `--output PATH` (permission denied, disk full, etc.). | — |
| 74 | IO_ERROR | Generic I/O error (read failure, network timeout for remote source). | — |
| 75 | TEMP_FAILURE | Temporary failure; retry may succeed (e.g., remote source returned 503). | — |
| 77 | PERMISSION_DENIED | Insufficient permissions (e.g., `--root DIR` traversal blocked). | TH-02 |
| 78 | CONFIG_ERROR | Configuration error (invalid profile YAML, missing required `--auth-token` on public MCP bind). | TH-03 (line 874) |

**TH-03 (exit 78):** `pdftract mcp --bind 0.0.0.0:PORT` without `--auth-token` or `PDFTRACT_MCP_TOKEN` aborts with exit code 78 and a stderr message explaining the risk. Loopback binds (`127.0.0.1`, `::1`) are exempt.

**TH-07 (password handling):** Using `--password VALUE` without `PDFTRACT_INSECURE_CLI_PASSWORD=1` exits with code 64 (USAGE_ERROR) and a stderr hint to use `--password-stdin` or `PDFTRACT_PASSWORD` instead.

### Environment Variable Pass-Through

The following environment variables are recognized by pdftract. SDKs SHOULD set them explicitly when the corresponding behavior is desired.

| Variable | Purpose | Secret? |
|----------|---------|---------|
| `PDFTRACT_PASSWORD` | PDF decryption password. | YES — never logged |
| `PDFTRACT_MCP_TOKEN` | MCP server bearer token (for `--auth-token`). | YES — never logged |
| `PDFTRACT_INSECURE_CLI_PASSWORD` | Set to `1` to allow `--password VALUE` (TH-07 opt-out). | NO |
| `PDFTRACT_INSECURE_CLI_TOKEN` | Set to `1` to allow `--auth-token VALUE`. | NO |
| `RUST_LOG` | Log level filter (e.g., `pdftract=debug`). | NO |
| `NO_COLOR` | Disable ANSI colors in stderr output. | NO |
| `XDG_CONFIG_HOME` | Base directory for profile search (overrides `~/.config`). | NO |
| `PDFTRACT_CONFIG_DIR` | Explicit profile directory path (overrides XDG default). | NO |

**Secret handling:**
- Secret-bearing variables (`PDFTRACT_PASSWORD`, `PDFTRACT_MCP_TOKEN`) are NEVER emitted in logs, diagnostics, or `--capture-diagnostics` archives.
- They are held in `secrecy::SecretString` to prevent accidental `Debug` prints.

### `--progress-json` Event Schema

When `--progress-json` is passed, pdftract emits newline-delimited JSON objects to stderr, one per event. This allows SDKs to parse progress without scraping human-readable logs.

**Event types:**

```jsonc
// Extraction started
{"event":"open","fingerprint":"pdftract-v1:abcd...","path":"document.pdf","version":"1.0.0"}

// Page processing started
{"event":"page_started","page":5,"total":10}

// Page processing completed
{"event":"page_completed","page":5,"span_count":123,"block_count":12}

// OCR started (Phase 5.4)
{"event":"ocr_started","page":3,"engine":"tesseract","lang":"eng"}

// OCR completed
{"event":"ocr_completed","page":3,"duration_ms":1234}

// Profile matched (Phase 7.10)
{"event":"profile_matched","profile":"scientific_paper","priority":100}

// Password received (TH-07 — NEVER includes the password value)
{"event":"password_received","source":"stdin"}  // or "env", "mcp_body", "form_field"

// Extraction completed successfully
{"event":"completed","duration_ms":5678,"page_count":10}

// Fatal error (extraction aborted)
{"event":"error","code":"PASSWORD_WRONG","message":"Incorrect password","exit_code":66}
```

**Parsing:**
- Each line is valid JSON. SDKs read stderr line-by-line and `JSON.parse()` each line.
- The `event` field discriminates the type; additional fields are event-specific.
- Human-readable log lines are still emitted to stderr intermixed with JSON lines. SDKs should filter by attempting JSON parse first; lines that fail to parse are human logs.

### `--capture-diagnostics` Archive Layout

When `--capture-diagnostics PATH` is passed, pdftract creates a diagnostic archive on error or when explicitly requested. The archive is attached to bug reports for reproduction.

**Archive formats:**
- `.zip` (default) — Use when `zip` command is available.
- `.tar.gz` — Fallback when `zip` is not available.

**Contained files:**

```
diagnostics-20260516-123456.zip
├── manifest.json              # Archive metadata (version, timestamp, exit code)
├── runtime_config.json        # Extraction options with secrets REDACTED
├── stderr.log                 # Captured stderr (passwords REDACTED)
├── pdf_fingerprint.txt        # SHA-256 fingerprint of the input PDF
├── pdf_source_sanitized.pdf   # PDF with all text content replaced by placeholders
└── version.txt                # `pdftract --version` output
```

**`manifest.json` schema:**
```json
{
  "captured_at": "2026-05-16T12:34:56Z",
  "pdftract_version": "1.0.0",
  "exit_code": 65,
  "exit_reason": "DATA_ERROR",
  "diagnostic_codes": ["XREF_REPAIRED", "STREAM_BOMB"],
  "pdf_fingerprint": "pdftract-v1:abcd...",
  "options_redacted": true
}
```

**`runtime_config.json` schema:**
```json
{
  "subcommand": "extract",
  "args": ["document.pdf", "--profile", "scientific_paper"],
  "env": {
    "RUST_LOG": "pdftract=info",
    "PDFTRACT_PASSWORD": "<redacted>",
    "PDFTRACT_MCP_TOKEN": "<redacted>"
  }
}
```

**Secret scrubbing (TH-08):**
- `PDFTRACT_PASSWORD` value → `"<redacted>"`
- `PDFTRACT_MCP_TOKEN` value → `"<redacted>"`
- Full extracted text → NOT included (only span counts in stderr.log)
- PDF source → `pdf_source_sanitized.pdf` replaces all text content with placeholder glyphs (`[` / `]`) but preserves structure

**Rotation:** Archives are NOT auto-rotated. Operators MUST manage disk space manually.

---

## 1. Python

## JSON Output Schema

```json
{
  "pages": [
    {
      "page": 1,
      "spans": [
        {
          "text": "Hello world",
          "bbox": [x0, y0, x1, y1],
          "font": "Helvetica",
          "size": 12.0,
          "confidence": 0.98
        }
      ],
      "blocks": [
        {
          "kind": "paragraph",
          "text": "Hello world",
          "bbox": [x0, y0, x1, y1]
        }
      ]
    }
  ],
  "metadata": {
    "title": "...",
    "author": "...",
    "page_count": 10
  }
}
```

---

## 1. Python

> **When to prefer subprocess:** one-off scripts, CLI pipelines, or when starting the server is not worth the overhead.
> **When to prefer HTTP:** long-running services, parallel extraction across many files, or when sharing a single pdftract instance across multiple workers.

### Subprocess

```python
import subprocess
import json
import os


def extract_pdf_subprocess(pdf_path: str, password: str | None = None) -> dict:
    """Extract text from a PDF via subprocess and return the parsed JSON result.
    
    Args:
        pdf_path: Path to the PDF file.
        password: Optional PDF password. Passed via env var (TH-07 compliant).
    
    Returns:
        Parsed JSON output from pdftract.
    
    Raises:
        RuntimeError: If pdftract exits with a non-zero code.
    """
    env = os.environ.copy()
    if password:
        # TH-07: Pass password via env var, NOT via --password flag.
        # Using --password VALUE is rejected unless PDFTRACT_INSECURE_CLI_PASSWORD=1.
        env["PDFTRACT_PASSWORD"] = password
    
    result = subprocess.run(
        ["pdftract", "extract", pdf_path],
        capture_output=True,
        text=True,
        env=env,
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"pdftract failed (exit {result.returncode}): {result.stderr.strip()}"
        )
    return json.loads(result.stdout)


def extract_pdf_password_stdin(pdf_path: str, password: str) -> dict:
    """Extract with password via --password-stdin (TH-07 compliant).
    
    This is the recommended method when you cannot use env vars (e.g., in
    restricted environments where env injection is not possible).
    """
    result = subprocess.run(
        ["pdftract", "extract", "--password-stdin", pdf_path],
        input=password + "\n",  # stdin: one line containing the password
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"pdftract failed (exit {result.returncode}): {result.stderr.strip()}"
        )
    return json.loads(result.stdout)


def extract_pdf_from_bytes(pdf_bytes: bytes, password: str | None = None) -> dict:
    """Extract from in-memory PDF bytes (avoids writing to disk).
    
    The PDF is piped to pdftract via stdin using the special '-' path.
    When using stdin for the PDF, --password-stdin cannot be used simultaneously;
    use PDFTRACT_PASSWORD env var instead.
    """
    env = os.environ.copy()
    if password:
        env["PDFTRACT_PASSWORD"] = password
    
    result = subprocess.run(
        ["pdftract", "extract", "-"],  # '-' means read PDF from stdin
        input=pdf_bytes,
        capture_output=True,
        env=env,
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"pdftract failed (exit {result.returncode}): {result.stderr.strip()}"
        )
    return json.loads(result.stdout)



def full_text(data: dict) -> str:
    """Concatenate all block text across every page."""
    parts = []
    for page in data["pages"]:
        for block in page["blocks"]:
            parts.append(block["text"])
    return "\n".join(parts)


def page_text(data: dict, page_number: int) -> str:
    """Return concatenated block text for a single page (1-indexed)."""
    for page in data["pages"]:
        if page["page"] == page_number:
            return "\n".join(block["text"] for block in page["blocks"])
    raise ValueError(f"Page {page_number} not found")


if __name__ == "__main__":
    import sys

    pdf = sys.argv[1]
    # Example: extract with password
    # data = extract_pdf_subprocess(pdf, password="secret123")
    data = extract_pdf_subprocess(pdf)

    print(f"Title   : {data['metadata'].get('title', '(none)')}")
    print(f"Pages   : {data['metadata']['page_count']}")
    print()
    print("--- Full text ---")
    print(full_text(data))
    print()
    print("--- Page 1 text ---")
    print(page_text(data, 1))
```

### HTTP (requests / httpx)

```python
# pip install requests
# pip install httpx   # async alternative shown below

import requests
import json


PDFTRACT_URL = "http://localhost:8080"


def extract_pdf_http(pdf_path: str, password: str | None = None) -> dict:
    """POST a PDF file to pdftract serve and return the parsed JSON result.
    
    Args:
        pdf_path: Path to the PDF file.
        password: Optional PDF password (sent as multipart form field).
    
    Raises:
        requests.HTTPError: If the HTTP request fails.
    """
    with open(pdf_path, "rb") as f:
        files = {"file": (pdf_path, f, "application/pdf")}
        data: dict[str, str] = {}
        if password:
            # TH-07: Password via form field is allowed (not exposed in ps/process list).
            data["password"] = password
        
        response = requests.post(
            f"{PDFTRACT_URL}/extract",
            files=files,
            data=data,
            timeout=60,
        )
    response.raise_for_status()
    return response.json()


def full_text(data: dict) -> str:
    parts = []
    for page in data["pages"]:
        for block in page["blocks"]:
            parts.append(block["text"])
    return "\n".join(parts)


def page_text(data: dict, page_number: int) -> str:
    for page in data["pages"]:
        if page["page"] == page_number:
            return "\n".join(block["text"] for block in page["blocks"])
    raise ValueError(f"Page {page_number} not found")


# --- Async variant with httpx ---
import asyncio
import httpx


async def extract_pdf_async(pdf_path: str) -> dict:
    async with httpx.AsyncClient(timeout=60) as client:
        with open(pdf_path, "rb") as f:
            response = await client.post(
                f"{PDFTRACT_URL}/extract",
                files={"file": (pdf_path, f, "application/pdf")},
            )
        response.raise_for_status()
        return response.json()


if __name__ == "__main__":
    import sys

    pdf = sys.argv[1]

    # Synchronous
    data = extract_pdf_http(pdf)
    print(full_text(data))

    # Asynchronous
    data = asyncio.run(extract_pdf_async(pdf))
    print(full_text(data))
```

---

## 2. Node.js / JavaScript

> **When to prefer subprocess:** build scripts, one-off tooling, or serverless functions where spinning up a child process is acceptable.
> **When to prefer HTTP:** Express/Fastify services, or when pdftract is deployed as a sidecar or shared microservice.

### Subprocess (child_process)

```js
// Node.js 18+ (ESM)
import { execFile } from "node:child_process";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

/**
 * Extract text from a PDF via subprocess.
 * @param {string} pdfPath
 * @param {string} [password] Optional PDF password (TH-07: passed via env)
 * @returns {Promise<object>} Parsed pdftract JSON
 */
async function extractPdfSubprocess(pdfPath, password) {
  const env = { ...process.env };
  if (password) {
    // TH-07: Pass password via env var, NOT via --password flag.
    // Using --password VALUE is rejected unless PDFTRACT_INSECURE_CLI_PASSWORD=1.
    env.PDFTRACT_PASSWORD = password;
  }

  const { stdout, stderr } = await execFileAsync("pdftract", ["extract", pdfPath], {
    env,
  }).catch((err) => {
    throw new Error(`pdftract failed (exit ${err.code}): ${err.stderr}`);
  });

  return JSON.parse(stdout);
}

/**
 * Extract with password via --password-stdin (TH-07 compliant).
 * @param {string} pdfPath
 * @param {string} password
 * @returns {Promise<object>}
 */
async function extractPdfPasswordStdin(pdfPath, password) {
  const { execFile } = require("node:child_process");
  
  return new Promise((resolve, reject) => {
    const proc = execFile("pdftract", ["extract", "--password-stdin", pdfPath]);
    
    let stdout = "";
    let stderr = "";
    
    proc.stdout.on("data", (data) => { stdout += data; });
    proc.stderr.on("data", (data) => { stderr += data; });
    
    proc.on("close", (code) => {
      if (code !== 0) {
        reject(new Error(`pdftract failed (exit ${code}): ${stderr}`));
      } else {
        resolve(JSON.parse(stdout));
      }
    });
    
    // Write password to stdin, followed by newline
    proc.stdin.write(password + "\n");
    proc.stdin.end();
  });
}

/** Concatenate all block text across every page. */
function fullText(data) {
  return data.pages
    .flatMap((page) => page.blocks.map((b) => b.text))
    .join("\n");
}

/** Return concatenated block text for a single page (1-indexed). */
function pageText(data, pageNumber) {
  const page = data.pages.find((p) => p.page === pageNumber);
  if (!page) throw new Error(`Page ${pageNumber} not found`);
  return page.blocks.map((b) => b.text).join("\n");
}

// Usage
const data = await extractPdfSubprocess(process.argv[2]);
console.log("Title  :", data.metadata.title ?? "(none)");
console.log("Pages  :", data.metadata.page_count);
console.log("\n--- Full text ---");
console.log(fullText(data));
console.log("\n--- Page 1 ---");
console.log(pageText(data, 1));
```

### HTTP (native fetch)

```js
// Node.js 18+ — fetch is available globally; no extra dependencies required.
import { readFile } from "node:fs/promises";

const PDFTRACT_URL = "http://localhost:8080";

/**
 * POST a PDF to pdftract serve.
 * @param {string} pdfPath
 * @param {string} [password] Optional PDF password (sent as form field)
 * @returns {Promise<object>} Parsed pdftract JSON
 */
async function extractPdfHttp(pdfPath, password) {
  const bytes = await readFile(pdfPath);
  const blob = new Blob([bytes], { type: "application/pdf" });

  const form = new FormData();
  form.append("file", blob, pdfPath);
  if (password) {
    // TH-07: Password via form field is allowed.
    form.append("password", password);
  }

  const res = await fetch(`${PDFTRACT_URL}/extract`, {
    method: "POST",
    body: form,
  });

  if (!res.ok) {
    const body = await res.text();
    throw new Error(`pdftract HTTP ${res.status}: ${body}`);
  }

  return res.json();
}

function fullText(data) {
  return data.pages
    .flatMap((page) => page.blocks.map((b) => b.text))
    .join("\n");
}

function pageText(data, pageNumber) {
  const page = data.pages.find((p) => p.page === pageNumber);
  if (!page) throw new Error(`Page ${pageNumber} not found`);
  return page.blocks.map((b) => b.text).join("\n");
}

// Usage
const data = await extractPdfHttp(process.argv[2]);
console.log(fullText(data));
```

---

## 3. Go

> **When to prefer subprocess:** CLI utilities or single-binary deployments where you want zero network overhead.
> **When to prefer HTTP:** Go services handling concurrent requests — spin up pdftract serve once and hit it from multiple goroutines.

### Subprocess (os/exec)

```go
package main

import (
	"encoding/json"
	"fmt"
	"log"
	"os"
	"os/exec"
	"strings"
)

// extractSubprocess runs `pdftract extract <path>` and returns the parsed result.
// If password is non-empty, it is passed via PDFTRACT_PASSWORD env var (TH-07 compliant).
func extractSubprocess(pdfPath string, password string) (*PDFTractResult, error) {
	cmd := exec.Command("pdftract", "extract", pdfPath)
	
	if password != "" {
		// TH-07: Pass password via env var, NOT via --password flag.
		cmd.Env = append(os.Environ(), "PDFTRACT_PASSWORD="+password)
	}
	
	out, err := cmd.Output()
	if err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			return nil, fmt.Errorf("pdftract failed: %s", string(exitErr.Stderr))
		}
		return nil, fmt.Errorf("exec error: %w", err)
	}

	var result PDFTractResult
	if err := json.Unmarshal(out, &result); err != nil {
		return nil, fmt.Errorf("json parse error: %w", err)
	}
	return &result, nil
}

type Span struct {
	Text       string    `json:"text"`
	BBox       [4]float64 `json:"bbox"`
	Font       string    `json:"font"`
	Size       float64   `json:"size"`
	Confidence float64   `json:"confidence"`
}

type Block struct {
	Kind string    `json:"kind"`
	Text string    `json:"text"`
	BBox [4]float64 `json:"bbox"`
}

type Page struct {
	Page   int     `json:"page"`
	Spans  []Span  `json:"spans"`
	Blocks []Block `json:"blocks"`
}

type Metadata struct {
	Title     string `json:"title"`
	Author    string `json:"author"`
	PageCount int    `json:"page_count"`
}

type PDFTractResult struct {
	Pages    []Page   `json:"pages"`
	Metadata Metadata `json:"metadata"`
}

// extractSubprocess runs `pdftract extract <path>` and returns the parsed result.
func extractSubprocess(pdfPath string) (*PDFTractResult, error) {
	out, err := exec.Command("pdftract", "extract", pdfPath).Output()
	if err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			return nil, fmt.Errorf("pdftract failed: %s", string(exitErr.Stderr))
		}
		return nil, fmt.Errorf("exec error: %w", err)
	}

	var result PDFTractResult
	if err := json.Unmarshal(out, &result); err != nil {
		return nil, fmt.Errorf("json parse error: %w", err)
	}
	return &result, nil
}

// FullText concatenates all block text across every page.
func (r *PDFTractResult) FullText() string {
	var sb strings.Builder
	for _, page := range r.Pages {
		for _, block := range page.Blocks {
			sb.WriteString(block.Text)
			sb.WriteByte('\n')
		}
	}
	return sb.String()
}

// PageText returns concatenated block text for a single page (1-indexed).
func (r *PDFTractResult) PageText(pageNumber int) (string, error) {
	for _, page := range r.Pages {
		if page.Page == pageNumber {
			var sb strings.Builder
			for _, block := range page.Blocks {
				sb.WriteString(block.Text)
				sb.WriteByte('\n')
			}
			return sb.String(), nil
		}
	}
	return "", fmt.Errorf("page %d not found", pageNumber)
}

func main() {
	if len(os.Args) < 2 {
		log.Fatal("usage: program <file.pdf>")
	}

	result, err := extractSubprocess(os.Args[1])
	if err != nil {
		log.Fatalf("extraction failed: %v", err)
	}

	fmt.Printf("Title : %s\n", result.Metadata.Title)
	fmt.Printf("Pages : %d\n", result.Metadata.PageCount)
	fmt.Println("\n--- Full text ---")
	fmt.Println(result.FullText())

	p1, err := result.PageText(1)
	if err != nil {
		log.Printf("page 1: %v", err)
	} else {
		fmt.Println("--- Page 1 ---")
		fmt.Println(p1)
	}
}
```

### HTTP (net/http)

```go
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"mime/multipart"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
)

const pdftractURL = "http://localhost:8080"

// extractHTTP POSTs a PDF file to pdftract serve.
// If password is non-empty, it is sent as a multipart form field (TH-07 compliant).
func extractHTTP(pdfPath string, password string) (*PDFTractResult, error) {
	f, err := os.Open(pdfPath)
	if err != nil {
		return nil, fmt.Errorf("open file: %w", err)
	}
	defer f.Close()

	var buf bytes.Buffer
	mw := multipart.NewWriter(&buf)

	part, err := mw.CreateFormFile("file", filepath.Base(pdfPath))
	if err != nil {
		return nil, fmt.Errorf("create form file: %w", err)
	}
	if _, err := io.Copy(part, f); err != nil {
		return nil, fmt.Errorf("copy file: %w", err)
	}

	if password != "" {
		// TH-07: Password via form field is allowed.
		err = mw.WriteField("password", password)
		if err != nil {
			return nil, fmt.Errorf("write password field: %w", err)
		}
	}

	mw.Close()

	resp, err := http.Post(
		pdftractURL+"/extract",
		mw.FormDataContentType(),
		&buf,
	)
	if err != nil {
		return nil, fmt.Errorf("http post: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("pdftract HTTP %d: %s", resp.StatusCode, body)
	}

	var result PDFTractResult
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, fmt.Errorf("json decode: %w", err)
	}
	return &result, nil
}

func main() {
	if len(os.Args) < 2 {
		log.Fatal("usage: program <file.pdf>")
	}

	result, err := extractHTTP(os.Args[1])
	if err != nil {
		log.Fatalf("extraction failed: %v", err)
	}

	fmt.Println(result.FullText())
}
```

---

## 4. Ruby

> **When to prefer subprocess:** Rake tasks, standalone scripts, or Rails background jobs without a persistent pdftract process.
> **When to prefer HTTP:** Sidekiq workers or Rails requests — keep pdftract serve running as a separate process and hit it over loopback.

### Subprocess (Open3)

```ruby
require "open3"
require "json"

# Extract text from a PDF via subprocess.
# Returns a Hash parsed from pdftract's JSON output.
# If password is provided, it is passed via env var (TH-07 compliant).
def extract_pdf_subprocess(pdf_path, password: nil)
  env = {}
  env["PDFTRACT_PASSWORD"] = password if password

  stdout, stderr, status = Open3.capture3(
    env,
    "pdftract", "extract", pdf_path
  )

  unless status.success?
    raise "pdftract failed (exit #{status.exitstatus}): #{stderr.strip}"
  end

  JSON.parse(stdout)
end

# Extract with password via --password-stdin (TH-07 compliant).
def extract_pdf_password_stdin(pdf_path, password)
  require "open3"
  require "json"

  # Pass password via stdin; Open3 with :stdin_data is the cleanest way.
  stdout, stderr, status = Open3.capture3(
    "pdftract", "extract", "--password-stdin", pdf_path,
    stdin_data: password + "\n"
  )

  unless status.success?
    raise "pdftract failed (exit #{status.exitstatus}): #{stderr.strip}"
  end

  JSON.parse(stdout)
end

# Concatenate all block text across every page.
def full_text(data)
  data["pages"]
    .flat_map { |page| page["blocks"].map { |b| b["text"] } }
    .join("\n")
end

# Return concatenated block text for a single page (1-indexed).
def page_text(data, page_number)
  page = data["pages"].find { |p| p["page"] == page_number }
  raise "Page #{page_number} not found" unless page

  page["blocks"].map { |b| b["text"] }.join("\n")
end

# Usage
pdf_path = ARGV[0] || raise("Usage: ruby extract.rb <file.pdf>")
data = extract_pdf_subprocess(pdf_path)

puts "Title : #{data.dig("metadata", "title") || "(none)"}"
puts "Pages : #{data.dig("metadata", "page_count")}"
puts
puts "--- Full text ---"
puts full_text(data)
puts
puts "--- Page 1 ---"
puts page_text(data, 1)
```

### HTTP (net/http)

```ruby
require "net/http"
require "json"

PDFTRACT_URL = URI("http://localhost:8080/extract")

# POST a PDF file to pdftract serve.
# If password is provided, it is sent as a multipart form field (TH-07 compliant).
def extract_pdf_http(pdf_path, password: nil)
  boundary = "----pdftract#{rand(0xFFFFFF).to_s(16)}"
  body = build_multipart(pdf_path, boundary, password:)

  http = Net::HTTP.new(PDFTRACT_URL.host, PDFTRACT_URL.port)
  http.read_timeout = 60

  request = Net::HTTP::Post.new(PDFTRACT_URL.path)
  request["Content-Type"] = "multipart/form-data; boundary=#{boundary}"
  request.body = body

  response = http.request(request)
  raise "pdftract HTTP #{response.code}: #{response.body}" unless response.is_a?(Net::HTTPSuccess)

  JSON.parse(response.body)
end

def build_multipart(pdf_path, boundary, password: nil)
  crlf = "\r\n"
  pdf_data = File.binread(pdf_path)
  filename = File.basename(pdf_path)

  parts = [
    "--#{boundary}#{crlf}",
    "Content-Disposition: form-data; name=\"file\"; filename=\"#{filename}\"#{crlf}",
    "Content-Type: application/pdf#{crlf}",
    crlf,
    pdf_data,
  ]

  if password
    # TH-07: Password via form field is allowed.
    parts.concat([
      "#{crlf}--#{boundary}#{crlf}",
      "Content-Disposition: form-data; name=\"password\"#{crlf}",
      crlf,
      password,
    ])
  end

  parts.concat([
    "#{crlf}--#{boundary}--#{crlf}",
  ])

  parts.join
end

def full_text(data)
  data["pages"]
    .flat_map { |page| page["blocks"].map { |b| b["text"] } }
    .join("\n")
end

def page_text(data, page_number)
  page = data["pages"].find { |p| p["page"] == page_number }
  raise "Page #{page_number} not found" unless page

  page["blocks"].map { |b| b["text"] }.join("\n")
end

# Usage
pdf_path = ARGV[0] || raise("Usage: ruby extract_http.rb <file.pdf>")
data = extract_pdf_http(pdf_path)

puts full_text(data)
```

---

## 5. Java

> **When to prefer subprocess:** batch jobs or standalone utilities. ProcessBuilder is simple and avoids a network stack.
> **When to prefer HTTP:** Spring Boot services or multi-threaded apps — pdftract serve handles concurrent requests, while subprocess creates a new process per call.

Requires Java 11+. No external dependencies — uses only the standard library.

### Subprocess (ProcessBuilder)

```java
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;

import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;

/**
 * Invokes pdftract via subprocess and parses the JSON result.
 *
 * Dependency (Maven):
 *   <dependency>
 *     <groupId>com.fasterxml.jackson.core</groupId>
 *     <artifactId>jackson-databind</artifactId>
 *     <version>2.17.0</version>
 *   </dependency>
 *
 * If you prefer no dependencies, replace ObjectMapper with org.json or
 * a manual string parse — the structure is straightforward.
 */
public class PdftractSubprocess {

    private static final ObjectMapper MAPPER = new ObjectMapper();

    /**
     * Extract text from a PDF.
     * @param pdfPath Path to the PDF file.
     * @param password Optional PDF password (TH-07: passed via env var).
     */
    public static JsonNode extract(String pdfPath, String password) throws IOException, InterruptedException {
        ProcessBuilder pb = new ProcessBuilder("pdftract", "extract", pdfPath);
        pb.redirectErrorStream(false); // keep stderr separate
        
        if (password != null && !password.isEmpty()) {
            // TH-07: Pass password via env var, NOT via --password flag.
            Map<String, String> env = pb.environment();
            env.put("PDFTRACT_PASSWORD", password);
        }
        
        Process process = pb.start();

        byte[] stdout = process.getInputStream().readAllBytes();
        byte[] stderr = process.getErrorStream().readAllBytes();

        int exit = process.waitFor();
        if (exit != 0) {
            throw new IOException(
                "pdftract failed (exit " + exit + "): " + new String(stderr).strip()
            );
        }

        return MAPPER.readTree(stdout);
    }

    /** Concatenate all block text across every page. */
    public static String fullText(JsonNode data) {
        List<String> parts = new ArrayList<>();
        for (JsonNode page : data.get("pages")) {
            for (JsonNode block : page.get("blocks")) {
                parts.add(block.get("text").asText());
            }
        }
        return String.join("\n", parts);
    }

    /** Return concatenated block text for a single page (1-indexed). */
    public static String pageText(JsonNode data, int pageNumber) {
        for (JsonNode page : data.get("pages")) {
            if (page.get("page").asInt() == pageNumber) {
                List<String> parts = new ArrayList<>();
                for (JsonNode block : page.get("blocks")) {
                    parts.add(block.get("text").asText());
                }
                return String.join("\n", parts);
            }
        }
        throw new IllegalArgumentException("Page " + pageNumber + " not found");
    }

    public static void main(String[] args) throws Exception {
        if (args.length < 1) {
            System.err.println("Usage: PdftractSubprocess <file.pdf>");
            System.exit(1);
        }

        JsonNode data = extract(args[0]);

        JsonNode meta = data.get("metadata");
        System.out.println("Title : " + meta.path("title").asText("(none)"));
        System.out.println("Pages : " + meta.get("page_count").asInt());
        System.out.println("\n--- Full text ---");
        System.out.println(fullText(data));
        System.out.println("\n--- Page 1 ---");
        System.out.println(pageText(data, 1));
    }
}
```

### HTTP (java.net.http.HttpClient, Java 11+)

```java
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;

import java.io.IOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;

public class PdftractHttp {

    private static final String PDFTRACT_URL = "http://localhost:8080";
    private static final ObjectMapper MAPPER = new ObjectMapper();
    private static final HttpClient CLIENT = HttpClient.newBuilder()
        .connectTimeout(Duration.ofSeconds(10))
        .build();

    /**
     * Extract text from a PDF via HTTP.
     * @param pdfPath Path to the PDF file.
     * @param password Optional PDF password (TH-07: sent as form field).
     */
    public static JsonNode extract(String pdfPath, String password) throws IOException, InterruptedException {
        Path path = Path.of(pdfPath);
        byte[] pdfBytes = Files.readAllBytes(path);
        String filename = path.getFileName().toString();
        String boundary = UUID.randomUUID().toString().replace("-", "");

        // Build multipart/form-data body manually (no external library needed)
        String crlf = "\r\n";
        StringBuilder bodyBuilder = new StringBuilder();
        
        // File part
        bodyBuilder.append("--").append(boundary).append(crlf);
        bodyBuilder.append("Content-Disposition: form-data; name=\"file\"; filename=\"")
                   .append(filename).append("\"").append(crlf);
        bodyBuilder.append("Content-Type: application/pdf").append(crlf);
        bodyBuilder.append(crlf);
        
        byte[] headerBytes = bodyBuilder.toString().getBytes(StandardCharsets.UTF_8);
        byte[] footerBytes = (crlf + "--" + boundary + "--" + crlf).getBytes(StandardCharsets.UTF_8);
        
        byte[] passwordBytes = new byte[0];
        if (password != null && !password.isEmpty()) {
            // TH-07: Password via form field is allowed.
            String passwordPart = crlf + "--" + boundary + crlf
                + "Content-Disposition: form-data; name=\"password\"" + crlf
                + crlf
                + password;
            passwordBytes = passwordPart.getBytes(StandardCharsets.UTF_8);
        }
        
        byte[] body = new byte[headerBytes.length + pdfBytes.length + passwordBytes.length + footerBytes.length];
        int pos = 0;
        System.arraycopy(headerBytes, 0, body, pos, headerBytes.length);
        pos += headerBytes.length;
        System.arraycopy(pdfBytes, 0, body, pos, pdfBytes.length);
        pos += pdfBytes.length;
        System.arraycopy(passwordBytes, 0, body, pos, passwordBytes.length);
        pos += passwordBytes.length;
        System.arraycopy(footerBytes, 0, body, pos, footerBytes.length);

        HttpRequest request = HttpRequest.newBuilder()
            .uri(URI.create(PDFTRACT_URL + "/extract"))
            .timeout(Duration.ofSeconds(60))
            .header("Content-Type", "multipart/form-data; boundary=" + boundary)
            .POST(HttpRequest.BodyPublishers.ofByteArray(body))
            .build();

        HttpResponse<String> response = CLIENT.send(
            request, HttpResponse.BodyHandlers.ofString()
        );

        if (response.statusCode() != 200) {
            throw new IOException(
                "pdftract HTTP " + response.statusCode() + ": " + response.body()
            );
        }

        return MAPPER.readTree(response.body());
    }

    public static String fullText(JsonNode data) {
        List<String> parts = new ArrayList<>();
        for (JsonNode page : data.get("pages")) {
            for (JsonNode block : page.get("blocks")) {
                parts.add(block.get("text").asText());
            }
        }
        return String.join("\n", parts);
    }

    public static String pageText(JsonNode data, int pageNumber) {
        for (JsonNode page : data.get("pages")) {
            if (page.get("page").asInt() == pageNumber) {
                List<String> parts = new ArrayList<>();
                for (JsonNode block : page.get("blocks")) {
                    parts.add(block.get("text").asText());
                }
                return String.join("\n", parts);
            }
        }
        throw new IllegalArgumentException("Page " + pageNumber + " not found");
    }

    public static void main(String[] args) throws Exception {
        if (args.length < 1) {
            System.err.println("Usage: PdftractHttp <file.pdf>");
            System.exit(1);
        }

        JsonNode data = extract(args[0]);
        System.out.println(fullText(data));
    }
}
```

---

## 6. Rust

> **When to prefer subprocess:** CLI tools or single-threaded batch processors — zero extra dependencies beyond `serde_json`.
> **When to prefer HTTP:** Async Tokio services — `reqwest` is non-blocking and naturally fits async Rust workloads.

### Subprocess (std::process::Command)

Add to `Cargo.toml`:
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

```rust
use serde::Deserialize;
use std::process::Command;
use std::collections::HashMap as EnvMap;

#[derive(Debug, Deserialize)]
struct Span {
    pub text: String,
    pub bbox: [f64; 4],
    pub font: String,
    pub size: f64,
    pub confidence: f64,
}

#[derive(Debug, Deserialize)]
struct Block {
    pub kind: String,
    pub text: String,
    pub bbox: [f64; 4],
}

#[derive(Debug, Deserialize)]
struct Page {
    pub page: u32,
    pub spans: Vec<Span>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Deserialize)]
struct Metadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub page_count: u32,
}

#[derive(Debug, Deserialize)]
struct PdftractResult {
    pub pages: Vec<Page>,
    pub metadata: Metadata,
}

impl PdftractResult {
    /// Concatenate all block text across every page.
    pub fn full_text(&self) -> String {
        self.pages
            .iter()
            .flat_map(|p| p.blocks.iter().map(|b| b.text.as_str()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Return concatenated block text for a single page (1-indexed).
    pub fn page_text(&self, page_number: u32) -> Option<String> {
        self.pages
            .iter()
            .find(|p| p.page == page_number)
            .map(|p| {
                p.blocks
                    .iter()
                    .map(|b| b.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
    }
}

/// Extract text from a PDF via subprocess.
/// If password is provided, it is passed via env var (TH-07 compliant).
fn extract_subprocess(pdf_path: &str, password: Option<&str>) -> Result<PdftractResult, Box<dyn std::error::Error>> {
    let mut cmd = Command::new("pdftract");
    cmd.args(["extract", pdf_path]);
    
    if let Some(pwd) = password {
        // TH-07: Pass password via env var, NOT via --password flag.
        cmd.env("PDFTRACT_PASSWORD", pwd);
    }
    
    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "pdftract failed (exit {:?}): {}",
            output.status.code(),
            stderr.trim()
        )
        .into());
    }

    let result: PdftractResult = serde_json::from_slice(&output.stdout)?;
    Ok(result)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf_path = std::env::args()
        .nth(1)
        .ok_or("usage: program <file.pdf>")?;

    let result = extract_subprocess(&pdf_path)?;

    println!("Title : {}", result.metadata.title.as_deref().unwrap_or("(none)"));
    println!("Pages : {}", result.metadata.page_count);
    println!("\n--- Full text ---");
    println!("{}", result.full_text());

    if let Some(text) = result.page_text(1) {
        println!("\n--- Page 1 ---");
        println!("{text}");
    }

    Ok(())
}
```

### HTTP (reqwest)

Add to `Cargo.toml`:
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", features = ["multipart"] }
tokio = { version = "1", features = ["full"] }
```

```rust
use reqwest::multipart;
use serde::Deserialize;
use std::path::Path;

// Re-use the same structs from the subprocess example above.
// (PdftractResult, Page, Block, Span, Metadata — copy them in)

const PDFTRACT_URL: &str = "http://localhost:8080";

/// Extract text from a PDF via HTTP.
/// If password is provided, it is sent as a multipart form field (TH-07 compliant).
async fn extract_http(pdf_path: &str, password: Option<&str>) -> Result<PdftractResult, Box<dyn std::error::Error>> {
    let bytes = tokio::fs::read(pdf_path).await?;
    let filename = Path::new(pdf_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document.pdf")
        .to_owned();

    let mut form = multipart::Form::new();
    
    let file_part = multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str("application/pdf")?;
    form = form.part("file", file_part);
    
    if let Some(pwd) = password {
        // TH-07: Password via form field is allowed.
        form = form.text("password", pwd.to_string());
    }

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{PDFTRACT_URL}/extract"))
        .multipart(form)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("pdftract HTTP {status}: {body}").into());
    }

    let result: PdftractResult = response.json().await?;
    Ok(result)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf_path = std::env::args()
        .nth(1)
        .ok_or("usage: program <file.pdf>")?;

    let result = extract_http(&pdf_path).await?;

    println!("{}", result.full_text());

    if let Some(text) = result.page_text(1) {
        println!("\n--- Page 1 ---");
        println!("{text}");
    }

    Ok(())
}
```

---

## Parsing `--progress-json` Events

When `--progress-json` is passed, pdftract emits newline-delimited JSON objects to stderr. SDKs can parse these events to show progress bars, detect errors early, or log structured diagnostics.

### Python

```python
import subprocess
import json
from typing import Any

ProgressEvent = dict[str, Any]

def extract_with_progress(pdf_path: str) -> dict:
    """Extract while parsing progress events from stderr."""
    cmd = ["pdftract", "extract", "--progress-json", pdf_path]
    
    # stderr is line-buffered; each line is either JSON or a human log.
    process = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    
    result: dict | None = None
    
    for line in process.stderr:
        line = line.rstrip("\n")
        if not line:
            continue
        
        # Try to parse as JSON; if it fails, it's a human log line.
        try:
            event: ProgressEvent = json.loads(line)
            event_type = event.get("event")
            
            if event_type == "open":
                print(f"Opening {event['path']} (fingerprint: {event['fingerprint'][:16]}...)")
            elif event_type == "page_started":
                print(f"Page {event['page']}/{event['total']}...")
            elif event_type == "page_completed":
                print(f"  → {event['span_count']} spans, {event['block_count']} blocks")
            elif event_type == "ocr_started":
                print(f"  OCR (page {event['page']}, lang={event['lang']})...")
            elif event_type == "ocr_completed":
                print(f"  OCR done in {event['duration_ms']}ms")
            elif event_type == "profile_matched":
                print(f"Profile: {event['profile']} (priority {event['priority']})")
            elif event_type == "password_received":
                # TH-07: The password value is NEVER in the event.
                print(f"Password received via {event['source']}")
            elif event_type == "completed":
                print(f"Done in {event['duration_ms']}ms, {event['page_count']} pages")
            elif event_type == "error":
                print(f"Error: {event['code']} - {event['message']}")
        except json.JSONDecodeError:
            # Human-readable log line (optional: ignore or log to file)
            print(f"[log] {line}")
    
    stdout, _ = process.communicate()
    if process.returncode != 0:
        raise RuntimeError(f"pdftract failed with exit {process.returncode}")
    
    return json.loads(stdout)
```

### Node.js

```js
import { execFile } from "node:child_process";

async function extractWithProgress(pdfPath) {
  const proc = execFile("pdftract", ["extract", "--progress-json", pdfPath]);
  
  let stdout = "";
  
  proc.stderr.on("data", (data) => {
    for (const line of data.toString().split("\n")) {
      if (!line.trim()) continue;
      
      try {
        const event = JSON.parse(line);
        switch (event.event) {
          case "open":
            console.log(`Opening ${event.path}`);
            break;
          case "page_started":
            console.log(`Page ${event.page}/${event.total}...`);
            break;
          case "page_completed":
            console.log(`  → ${event.span_count} spans, ${event.block_count} blocks`);
            break;
          case "ocr_started":
            console.log(`  OCR (page ${event.page}, lang=${event.lang})...`);
            break;
          case "ocr_completed":
            console.log(`  OCR done in ${event.duration_ms}ms`);
            break;
          case "profile_matched":
            console.log(`Profile: ${event.profile} (priority ${event.priority})`);
            break;
          case "password_received":
            console.log(`Password received via ${event.source}`);
            break;
          case "completed":
            console.log(`Done in ${event.duration_ms}ms, ${event.page_count} pages`);
            break;
          case "error":
            console.error(`Error: ${event.code} - ${event.message}`);
            break;
        }
      } catch (e) {
        // Not JSON — human log line
        console.log(`[log] ${line}`);
      }
    }
  });
  
  return new Promise((resolve, reject) => {
    proc.stdout.on("data", (d) => { stdout += d; });
    proc.on("close", (code) => {
      if (code !== 0) {
        reject(new Error(`pdftract failed with exit ${code}`));
      } else {
        resolve(JSON.parse(stdout));
      }
    });
  });
}
```

### Rust

```rust
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use serde_json::Value;

fn extract_with_progress(pdf_path: &str) -> Result<PdftractResult, Box<dyn std::error::Error>> {
    let mut child = Command::new("pdftract")
        .args(["extract", "--progress-json", pdf_path])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    let stderr = child.stderr.take().expect("stderr");
    let reader = BufReader::new(stderr);
    
    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        
        // Try to parse as JSON
        if let Ok(event) = serde_json::from_str::<Value>(&line) {
            let event_type = event.get("event").and_then(|v| v.as_str());
            
            match event_type {
                Some("open") => {
                    let path = event.get("path").and_then(|v| v.as_str()).unwrap_or("?");
                    println!("Opening {}", path);
                }
                Some("page_started") => {
                    let page = event.get("page").and_then(|v| v.as_u64()).unwrap_or(0);
                    let total = event.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
                    println!("Page {}/{}...", page, total);
                }
                Some("page_completed") => {
                    let spans = event.get("span_count").and_then(|v| v.as_u64()).unwrap_or(0);
                    let blocks = event.get("block_count").and_then(|v| v.as_u64()).unwrap_or(0);
                    println!("  → {} spans, {} blocks", spans, blocks);
                }
                Some("ocr_started") => {
                    let page = event.get("page").and_then(|v| v.as_u64()).unwrap_or(0);
                    let lang = event.get("lang").and_then(|v| v.as_str()).unwrap_or("?");
                    println!("  OCR (page {}, lang={})...", page, lang);
                }
                Some("ocr_completed") => {
                    let ms = event.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(0);
                    println!("  OCR done in {}ms", ms);
                }
                Some("profile_matched") => {
                    let profile = event.get("profile").and_then(|v| v.as_str()).unwrap_or("?");
                    let priority = event.get("priority").and_then(|v| v.as_u64()).unwrap_or(0);
                    println!("Profile: {} (priority {})", profile, priority);
                }
                Some("password_received") => {
                    let source = event.get("source").and_then(|v| v.as_str()).unwrap_or("?");
                    println!("Password received via {}", source);
                }
                Some("completed") => {
                    let ms = event.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(0);
                    let pages = event.get("page_count").and_then(|v| v.as_u64()).unwrap_or(0);
                    println!("Done in {}ms, {} pages", ms, pages);
                }
                Some("error") => {
                    let code = event.get("code").and_then(|v| v.as_str()).unwrap_or("?");
                    let msg = event.get("message").and_then(|v| v.as_str()).unwrap_or("?");
                    eprintln!("Error: {} - {}", code, msg);
                }
                _ => {
                    // Unknown event type or malformed JSON
                    println!("[log] {}", line);
                }
            }
        } else {
            // Not JSON — human log line
            println!("[log] {}", line);
        }
    }
    
    let output = child.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pdftract failed: {}", stderr).into());
    }
    
    let result: PdftractResult = serde_json::from_slice(&output.stdout)?;
    Ok(result)
}
```

---

## 7. Shell / Bash

> **When to prefer direct invocation:** shell scripts, cron jobs, CI pipelines, or any context where you have direct access to the binary.
> **When to prefer curl:** when pdftract is running as a shared service on another host, inside a container, or when you want to avoid installing the binary locally.

### Direct Invocation

```bash
#!/usr/bin/env bash
set -euo pipefail

PDF="${1:?Usage: $0 <file.pdf>}"

# --- JSON output ---
json=$(pdftract extract "$PDF")

# Full text via jq: collect all block text across all pages
full_text=$(echo "$json" | jq -r '[.pages[].blocks[].text] | join("\n")')

# Per-page text (page 1)
page1_text=$(echo "$json" | jq -r '.pages[] | select(.page == 1) | [.blocks[].text] | join("\n")')

# Metadata
title=$(echo  "$json" | jq -r '.metadata.title // "(none)"')
pages=$(echo  "$json" | jq -r '.metadata.page_count')

echo "Title : $title"
echo "Pages : $pages"
echo
echo "--- Full text ---"
echo "$full_text"
echo
echo "--- Page 1 ---"
echo "$page1_text"

# --- Plain text output (no jq needed) ---
plain=$(pdftract extract "$PDF" --text)
echo
echo "--- Plain text (--text flag) ---"
echo "$plain"

# --- Write JSON to file ---
pdftract extract "$PDF" --output "/tmp/$(basename "$PDF" .pdf).json"
echo "JSON written to /tmp/$(basename "$PDF" .pdf).json"
```

### curl (HTTP)

```bash
#!/usr/bin/env bash
set -euo pipefail

PDF="${1:?Usage: $0 <file.pdf>}"
PDFTRACT_URL="${PDFTRACT_URL:-http://localhost:8080}"

# POST the PDF and capture the response; fail fast on HTTP errors.
json=$(curl --silent --show-error --fail \
  --max-time 60 \
  -F "file=@${PDF};type=application/pdf" \
  "${PDFTRACT_URL}/extract")

# Full text via jq
full_text=$(echo "$json" | jq -r '[.pages[].blocks[].text] | join("\n")')

# Per-page text (page 1)
page1_text=$(echo "$json" | jq -r '.pages[] | select(.page == 1) | [.blocks[].text] | join("\n")')

# Metadata
title=$(echo "$json" | jq -r '.metadata.title // "(none)"')
pages=$(echo "$json" | jq -r '.metadata.page_count')

echo "Title : $title"
echo "Pages : $pages"
echo
echo "--- Full text ---"
echo "$full_text"
echo
echo "--- Page 1 ---"
echo "$page1_text"

# --- Save raw JSON ---
output_file="/tmp/$(basename "$PDF" .pdf).json"
echo "$json" > "$output_file"
echo "JSON saved to $output_file"

# --- Health check before submitting ---
# curl -sf "${PDFTRACT_URL}/health" > /dev/null \
#   || { echo "pdftract serve is not running at ${PDFTRACT_URL}"; exit 1; }
```

### Batch processing with xargs / parallel

```bash
#!/usr/bin/env bash
# Process every PDF in a directory, writing one JSON file per PDF.
# Uses GNU parallel if available, otherwise xargs -P.

PDF_DIR="${1:?Usage: $0 <dir>}"
OUT_DIR="${2:-/tmp/pdftract-out}"
mkdir -p "$OUT_DIR"

extract_one() {
  local pdf="$1"
  local out="$OUT_DIR/$(basename "$pdf" .pdf).json"
  pdftract extract "$pdf" --output "$out" && echo "OK  $pdf"  || echo "ERR $pdf"
}
export -f extract_one
export OUT_DIR

find "$PDF_DIR" -name "*.pdf" -print0 \
  | xargs -0 -P 4 -I{} bash -c 'extract_one "$@"' _ {}
```
