# pdftract SDK Invocation Guide

How to invoke the `pdftract` binary from various languages, both via subprocess and via the HTTP server mode.

## Binary Modes Reference

```
pdftract extract <file.pdf>                      # JSON to stdout
pdftract extract <file.pdf> --text               # plain text to stdout
pdftract extract <file.pdf> --output out.json    # JSON to file
pdftract serve --port 8080                       # HTTP server: POST /extract → JSON
```

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
import sys


def extract_pdf_subprocess(pdf_path: str) -> dict:
    """Extract text from a PDF via subprocess and return the parsed JSON result."""
    result = subprocess.run(
        ["pdftract", "extract", pdf_path],
        capture_output=True,
        text=True,
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
    pdf = sys.argv[1]
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


def extract_pdf_http(pdf_path: str) -> dict:
    """POST a PDF file to pdftract serve and return the parsed JSON result."""
    with open(pdf_path, "rb") as f:
        response = requests.post(
            f"{PDFTRACT_URL}/extract",
            files={"file": (pdf_path, f, "application/pdf")},
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
 * @returns {Promise<object>} Parsed pdftract JSON
 */
async function extractPdfSubprocess(pdfPath) {
  const { stdout, stderr } = await execFileAsync("pdftract", [
    "extract",
    pdfPath,
  ]).catch((err) => {
    throw new Error(`pdftract failed (exit ${err.code}): ${err.stderr}`);
  });

  return JSON.parse(stdout);
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
 * @returns {Promise<object>} Parsed pdftract JSON
 */
async function extractPdfHttp(pdfPath) {
  const bytes = await readFile(pdfPath);
  const blob = new Blob([bytes], { type: "application/pdf" });

  const form = new FormData();
  form.append("file", blob, pdfPath);

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
	"os"
	"path/filepath"
)

const pdftractURL = "http://localhost:8080"

// extractHTTP POSTs a PDF file to pdftract serve.
func extractHTTP(pdfPath string) (*PDFTractResult, error) {
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
def extract_pdf_subprocess(pdf_path)
  stdout, stderr, status = Open3.capture3("pdftract", "extract", pdf_path)

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
def extract_pdf_http(pdf_path)
  boundary = "----pdftract#{rand(0xFFFFFF).to_s(16)}"
  body = build_multipart(pdf_path, boundary)

  http = Net::HTTP.new(PDFTRACT_URL.host, PDFTRACT_URL.port)
  http.read_timeout = 60

  request = Net::HTTP::Post.new(PDFTRACT_URL.path)
  request["Content-Type"] = "multipart/form-data; boundary=#{boundary}"
  request.body = body

  response = http.request(request)
  raise "pdftract HTTP #{response.code}: #{response.body}" unless response.is_a?(Net::HTTPSuccess)

  JSON.parse(response.body)
end

def build_multipart(pdf_path, boundary)
  crlf = "\r\n"
  pdf_data = File.binread(pdf_path)
  filename = File.basename(pdf_path)

  [
    "--#{boundary}#{crlf}",
    "Content-Disposition: form-data; name=\"file\"; filename=\"#{filename}\"#{crlf}",
    "Content-Type: application/pdf#{crlf}",
    crlf,
    pdf_data,
    "#{crlf}--#{boundary}--#{crlf}",
  ].join
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

    public static JsonNode extract(String pdfPath) throws IOException, InterruptedException {
        ProcessBuilder pb = new ProcessBuilder("pdftract", "extract", pdfPath);
        pb.redirectErrorStream(false); // keep stderr separate
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

    public static JsonNode extract(String pdfPath) throws IOException, InterruptedException {
        Path path = Path.of(pdfPath);
        byte[] pdfBytes = Files.readAllBytes(path);
        String filename = path.getFileName().toString();
        String boundary = UUID.randomUUID().toString().replace("-", "");

        // Build multipart/form-data body manually (no external library needed)
        String crlf = "\r\n";
        byte[] partHeader = (
            "--" + boundary + crlf
            + "Content-Disposition: form-data; name=\"file\"; filename=\"" + filename + "\"" + crlf
            + "Content-Type: application/pdf" + crlf
            + crlf
        ).getBytes();
        byte[] partFooter = (crlf + "--" + boundary + "--" + crlf).getBytes();

        byte[] body = new byte[partHeader.length + pdfBytes.length + partFooter.length];
        System.arraycopy(partHeader, 0, body, 0, partHeader.length);
        System.arraycopy(pdfBytes, 0, body, partHeader.length, pdfBytes.length);
        System.arraycopy(partFooter, 0, body, partHeader.length + pdfBytes.length, partFooter.length);

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

fn extract_subprocess(pdf_path: &str) -> Result<PdftractResult, Box<dyn std::error::Error>> {
    let output = Command::new("pdftract")
        .args(["extract", pdf_path])
        .output()?;

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

async fn extract_http(pdf_path: &str) -> Result<PdftractResult, Box<dyn std::error::Error>> {
    let bytes = tokio::fs::read(pdf_path).await?;
    let filename = Path::new(pdf_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document.pdf")
        .to_owned();

    let part = multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str("application/pdf")?;

    let form = multipart::Form::new().part("file", part);

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
