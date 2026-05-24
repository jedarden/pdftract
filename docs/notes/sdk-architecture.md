# SDK Architecture and Language Coverage

## Top 10 language coverage status

Based on Stack Overflow 2024 survey rankings. The existing `sdk-invocation.md` covers Python,
JavaScript, Go, Ruby, Java, Rust, and Bash. Gaps: TypeScript, C#, C++, PHP, and Kotlin.

---

## Workspace layout

The workspace is organized so that `pdftract-core` is the only crate that other consumers depend on directly. The CLI, Python bindings, and inspector UI are siblings that compose `pdftract-core` behind their respective surfaces.

```
pdftract/
├── Cargo.toml (workspace root)
├── crates/
│   ├── pdftract-core/ (library — only direct dependency for downstream consumers)
│   ├── pdftract-cli/ (binary)
│   ├── pdftract-py/ (PyO3 bindings, optional feature)
│   └── pdftract-inspector-ui/ (HTML/CSS/JS bundled via include_bytes!, Phase 7.9)
└── docs/
    ├── plan/plan.md
    ├── research/ (per-feature deep dives)
    └── notes/ (this file, sdk-invocation.md, sdk-contract.md, ocr-language-packs.md)
```

See `docs/plan/plan.md` lines 141–268 for the full file and module layout specification.

---

## Common infrastructure (required before any SDK ships)

### Binary distribution

Every SDK approach — subprocess or native — depends on platform binaries published to GitHub Releases:

| Target triple | Platform |
|---|---|
| `x86_64-unknown-linux-musl` | Linux x86_64 (production binary) |
| `aarch64-unknown-linux-musl` | Linux ARM64 |
| `x86_64-apple-darwin` | macOS Intel |
| `aarch64-apple-darwin` | macOS Apple Silicon |
| `x86_64-pc-windows-gnu` | Windows x86_64 |

The CI workflow must cross-compile for all five targets and attach the binaries to a versioned
GitHub Release tag on every release. SDKs pin to a binary version and download the appropriate
artifact at install time.

### Cross-platform test limitation (KU-12)

Per ADR-009, `iad-ci` is Linux-only. **Linux is fully CI-tested; macOS and Windows are build-tested and manually smoke-tested per release.** macOS and Windows binaries are *built* via `cross` on Linux but never *executed* in CI. This is acknowledged as Known Unknown KU-12 with the following mitigation:

- A manual smoke-test runbook in `docs/operations/manual-platform-smoke.md` is executed by the release lead before each milestone tag against at least one physical macOS machine and one Windows VM
- User bug reports for platform-specific issues acknowledged within 48 hours and addressed in the next patch release

See `docs/plan/plan.md` lines 3431–3436 and lines 608–609 for the full KU-12 specification.

### Argo CI templates

Binary and wheel builds are orchestrated by Argo WorkflowTemplates on the `iad-ci` Rackspace Spot cluster:

- `pdftract-cargo-build` — builds the Rust binary for all five target triples using `cross` (Docker-based cross-compilation)
- `pdftract-maturin-build` — builds the PyO3 wheel for all five target triples (uses `ghcr.io/rust-cross/manylinux` for Linux, `osxcross` for macOS, `cross` for Windows)

GitHub Actions is **FORBIDDEN** per ADR-009. All CI runs on `iad-ci`; secrets live in OpenBao and reach workflows via ESO-synced Kubernetes Secrets.

See `docs/plan/plan.md` ADR-009 (lines 495–502) and Phase 0.2 (lines 1015–1029) for the full CI specification.

### Release format

```
https://github.com/jedarden/pdftract/releases/download/v{VERSION}/pdftract-{TARGET}.tar.gz
```

Semantic versioning is required before any package is published to a package registry.

---

## Feature flag composition

Feature flags control the binary footprint. The default build (`cargo build`) includes only the core extraction path. Heavy optional capabilities are behind named features.

### Feature tiers

| Tier | Features | Binary size (stripped) | Use case |
|---|---|---|---|
| **slim** | `["cli", "decrypt"]` | < 3 MB | Minimal CLI without Markdown |
| **default** | `["cli", "decrypt", "markdown"]` | < 4 MB | Standard CLI with Markdown output |
| **serve** | `default + ["serve"]` | < 12 MB | HTTP server mode |
| **ocr** | `default + ["ocr"]` | < 12 MB | OCR with Tesseract |
| **full** | `default + ["serve", "ocr", "mcp", "inspect", "grep", "profiles", "cache", "receipts", "remote"]` | < 14 MB | All features except `full-render` |

### Feature dependencies

Some features implicitly enable others:

- `serve` → enables `cache` (the HTTP server is the primary cache consumer)
- `mcp` → depends on `serve` (both transports share the HTTP infrastructure)
- `inspect` → depends on `serve` (bundles a ~80 KB static HTML/CSS/JS frontend via `include_bytes!`)
- `grep` → requires `regex` crate
- `profiles` → requires `regex` crate

### Binary size budgets

Per the Primary Objectives (Weight Targets):

| Metric | Target |
|---|---|
| Binary size, default features (no OCR, no serve) | < 4 MB stripped |
| Binary size, `--features ocr,serve` | < 12 MB stripped |
| Binary size, `--features full` (everything except `full-render`) | < 14 MB stripped |
| Docker image, CLI only | < 20 MB (distroless base) |
| Docker image, with OCR (`tesseract-ocr` system pkg) | < 120 MB |
| Docker image, `pdftract:full` | < 140 MB |

See `docs/plan/plan.md` lines 46–62 for the full weight targets specification.

---

## Two SDK tracks

### Track A — Subprocess / HTTP wrappers

Each SDK ships a thin wrapper that:
1. Downloads and caches the platform binary on first use (or at install time)
2. Invokes it via subprocess for one-off extractions
3. Optionally connects to a `pdftract serve` instance over HTTP for high-throughput use

**Tradeoffs:** Fast to implement for any language, no FFI complexity, slight per-call overhead
from process spawn. Acceptable for batch and interactive workloads.

### Track B — Native bindings

The Rust core exposes a C ABI via `cbindgen`. Each language calls into the compiled shared
library directly, bypassing subprocess entirely.

**Requires:**
- `cdylib` crate type in `Cargo.toml`
- `cbindgen` generating a `pdftract.h` C header
- A `#[no_mangle] extern "C"` public API surface in the Rust core
- Per-language FFI glue

**Tradeoffs:** Zero process-spawn overhead, suitable for embedding in long-running services,
but requires per-language binding work and platform-specific shared library distribution.

**Starting recommendation:** Track A for all languages, Track B for Python first (PyO3 is
mature, Python is the highest-volume use case for RAG and LLM preprocessing pipelines).

---

## Per-language breakdown

| Language | Package Manager | Track A | Track B | Status |
|---|---|---|---|---|
| Python | PyPI | `subprocess` | PyO3 + maturin | Covered |
| JavaScript | npm | `child_process` | napi-rs | Covered |
| TypeScript | npm | Same as JS | Same + `.d.ts` types | **Gap — types only** |
| Java | Maven Central | `ProcessBuilder` | JNI via `jni` crate | Covered |
| C# | NuGet | `System.Diagnostics.Process` | P/Invoke via cbindgen | **Gap** |
| C++ | vcpkg / conan | `popen` | cbindgen → `.h` + shared lib | **Gap** |
| Go | Go modules | `os/exec` | cgo + cbindgen | Covered |
| PHP | Packagist | `proc_open` | ext-php-rs or PHP FFI | **Gap** |
| Kotlin | Maven Central | `ProcessBuilder` (JVM) | JNI (same as Java) | **Gap** |
| Rust | crates.io | `std::process::Command` | native library crate | Covered |

---

## Gap detail

### TypeScript

Minimal work on top of the existing JavaScript notes. The implementation is identical — add a
`pdftract.d.ts` type definition file and publish to npm as `@pdftract/sdk` or alongside the JS
package. Types to define:

```typescript
export interface Span {
  text: string;
  bbox: [number, number, number, number];
  font: string;
  size: number;
  confidence: number;
}

export interface Block {
  kind: 'paragraph' | 'heading' | 'table' | 'figure' | 'list';
  text: string;
  bbox: [number, number, number, number];
}

export interface Page {
  page: number;
  spans: Span[];
  blocks: Block[];
}

export interface ExtractionResult {
  pages: Page[];
  metadata: {
    title?: string;
    author?: string;
    page_count: number;
  };
}

export function extract(filePath: string): Promise<ExtractionResult>;
export function extractText(filePath: string): Promise<string>;
export function extractPage(filePath: string, page: number): Promise<Page>;
export function createClient(baseUrl: string): PdftractClient;

export class PdftractClient {
  extract(filePath: string): Promise<ExtractionResult>;
  extractText(filePath: string): Promise<string>;
}
```

---

### C# / .NET

**Track A — subprocess:**

```csharp
using System.Diagnostics;
using System.Text.Json;

public class PdftractClient
{
    private readonly string _binaryPath;

    public PdftractClient(string binaryPath = "pdftract")
    {
        _binaryPath = binaryPath;
    }

    public async Task<ExtractionResult> ExtractAsync(string pdfPath)
    {
        using var process = new Process
        {
            StartInfo = new ProcessStartInfo
            {
                FileName = _binaryPath,
                Arguments = $"extract \"{pdfPath}\"",
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
            }
        };

        process.Start();
        string stdout = await process.StandardOutput.ReadToEndAsync();
        await process.WaitForExitAsync();

        if (process.ExitCode != 0)
        {
            string stderr = await process.StandardError.ReadToEndAsync();
            throw new PdftractException($"pdftract exited {process.ExitCode}: {stderr}");
        }

        return JsonSerializer.Deserialize<ExtractionResult>(stdout)
            ?? throw new PdftractException("Empty response");
    }

    public async Task<string> ExtractTextAsync(string pdfPath)
    {
        var result = await ExtractAsync(pdfPath);
        return string.Join("\n\n", result.Pages.Select(p =>
            string.Join("\n", p.Blocks.Select(b => b.Text))));
    }
}
```

**Track A — HTTP:**

```csharp
using System.Net.Http.Headers;

public class PdftractHttpClient : IDisposable
{
    private readonly HttpClient _http;
    private readonly string _baseUrl;

    public PdftractHttpClient(string baseUrl = "http://localhost:8080")
    {
        _http = new HttpClient();
        _baseUrl = baseUrl;
    }

    public async Task<ExtractionResult> ExtractAsync(string pdfPath)
    {
        using var form = new MultipartFormDataContent();
        var fileBytes = await File.ReadAllBytesAsync(pdfPath);
        var fileContent = new ByteArrayContent(fileBytes);
        fileContent.Headers.ContentType = MediaTypeHeaderValue.Parse("application/pdf");
        form.Add(fileContent, "file", Path.GetFileName(pdfPath));

        var response = await _http.PostAsync($"{_baseUrl}/extract", form);
        response.EnsureSuccessStatusCode();

        var json = await response.Content.ReadAsStringAsync();
        return JsonSerializer.Deserialize<ExtractionResult>(json)
            ?? throw new PdftractException("Empty response");
    }

    public void Dispose() => _http.Dispose();
}
```

**Track B — native (P/Invoke):**

Requires `cbindgen` to generate `pdftract.h`, then:

```csharp
using System.Runtime.InteropServices;

internal static class NativeMethods
{
    private const string LibName = "pdftract";

    [DllImport(LibName, EntryPoint = "pdftract_extract_file")]
    internal static extern IntPtr ExtractFile(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string path);

    [DllImport(LibName, EntryPoint = "pdftract_free_result")]
    internal static extern void FreeResult(IntPtr result);
}
```

**NuGet packaging** — the `.nupkg` must embed the shared library per Runtime Identifier:

```
lib/
  net8.0/
    Pdftract.dll
runtimes/
  linux-x64/native/libpdftract.so
  linux-arm64/native/libpdftract.so
  osx-x64/native/libpdftract.dylib
  osx-arm64/native/libpdftract.dylib
  win-x64/native/pdftract.dll
```

The `.csproj` sets `RuntimeIdentifiers` and uses `<PackagePath>` to map each binary into the
correct runtime folder. This is the primary complexity in C# packaging.

---

### C++

**Track A — subprocess:**

```cpp
#include <array>
#include <cstdio>
#include <memory>
#include <stdexcept>
#include <string>

std::string pdftract_extract_json(const std::string& pdf_path) {
    std::string cmd = "pdftract extract \"" + pdf_path + "\"";
    std::array<char, 4096> buf{};
    std::string result;

    std::unique_ptr<FILE, decltype(&pclose)> pipe(popen(cmd.c_str(), "r"), pclose);
    if (!pipe) throw std::runtime_error("popen failed");

    while (fgets(buf.data(), buf.size(), pipe.get()) != nullptr)
        result += buf.data();

    return result;
}

std::string pdftract_extract_text(const std::string& pdf_path) {
    std::string cmd = "pdftract extract --text \"" + pdf_path + "\"";
    std::array<char, 4096> buf{};
    std::string result;

    std::unique_ptr<FILE, decltype(&pclose)> pipe(popen(cmd.c_str(), "r"), pclose);
    if (!pipe) throw std::runtime_error("popen failed");

    while (fgets(buf.data(), buf.size(), pipe.get()) != nullptr)
        result += buf.data();

    return result;
}
```

**Track A — HTTP (using libcurl):**

```cpp
#include <curl/curl.h>
#include <string>

static size_t write_cb(char* ptr, size_t size, size_t nmemb, std::string* data) {
    data->append(ptr, size * nmemb);
    return size * nmemb;
}

std::string pdftract_http_extract(const std::string& pdf_path,
                                   const std::string& base_url = "http://localhost:8080") {
    CURL* curl = curl_easy_init();
    if (!curl) throw std::runtime_error("curl_easy_init failed");

    std::string response;
    curl_mime* mime = curl_mime_init(curl);
    curl_mimepart* part = curl_mime_addpart(mime);
    curl_mime_name(part, "file");
    curl_mime_filedata(part, pdf_path.c_str());

    curl_easy_setopt(curl, CURLOPT_URL, (base_url + "/extract").c_str());
    curl_easy_setopt(curl, CURLOPT_MIMEPOST, mime);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, write_cb);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &response);

    CURLcode res = curl_easy_perform(curl);
    curl_mime_free(mime);
    curl_easy_cleanup(curl);

    if (res != CURLE_OK)
        throw std::runtime_error(curl_easy_strerror(res));

    return response;
}
```

**Track B — native:** `cbindgen` generates `pdftract.h`; link against `libpdftract.so` /
`pdftract.dll`. Distribute as a vcpkg port or conan recipe with the header and shared library.
No standard package manager — provide both options.

---

### PHP

**Track A — subprocess:**

```php
<?php

class Pdftract
{
    public function __construct(
        private string $binaryPath = 'pdftract'
    ) {}

    public function extract(string $pdfPath): array
    {
        $cmd = escapeshellcmd($this->binaryPath)
             . ' extract '
             . escapeshellarg($pdfPath);

        $descriptors = [
            1 => ['pipe', 'w'],
            2 => ['pipe', 'w'],
        ];

        $proc = proc_open($cmd, $descriptors, $pipes);
        if (!is_resource($proc)) {
            throw new RuntimeException('Failed to start pdftract');
        }

        $stdout = stream_get_contents($pipes[1]);
        $stderr = stream_get_contents($pipes[2]);
        fclose($pipes[1]);
        fclose($pipes[2]);
        $exit = proc_close($proc);

        if ($exit !== 0) {
            throw new RuntimeException("pdftract exited $exit: $stderr");
        }

        return json_decode($stdout, true, 512, JSON_THROW_ON_ERROR);
    }

    public function extractText(string $pdfPath): string
    {
        $result = $this->extract($pdfPath);
        $lines = [];
        foreach ($result['pages'] as $page) {
            foreach ($page['blocks'] as $block) {
                $lines[] = $block['text'];
            }
        }
        return implode("\n\n", $lines);
    }

    public function extractPage(string $pdfPath, int $page): array
    {
        $result = $this->extract($pdfPath);
        foreach ($result['pages'] as $p) {
            if ($p['page'] === $page) return $p;
        }
        throw new OutOfRangeException("Page $page not found");
    }
}
```

**Track A — HTTP:**

```php
<?php

class PdftractHttpClient
{
    public function __construct(
        private string $baseUrl = 'http://localhost:8080'
    ) {}

    public function extract(string $pdfPath): array
    {
        $ch = curl_init($this->baseUrl . '/extract');
        curl_setopt_array($ch, [
            CURLOPT_RETURNTRANSFER => true,
            CURLOPT_POST           => true,
            CURLOPT_POSTFIELDS     => ['file' => new CURLFile($pdfPath, 'application/pdf')],
        ]);

        $response = curl_exec($ch);
        $status   = curl_getinfo($ch, CURLINFO_HTTP_CODE);
        curl_close($ch);

        if ($status !== 200) {
            throw new RuntimeException("HTTP $status from pdftract serve");
        }

        return json_decode($response, true, 512, JSON_THROW_ON_ERROR);
    }

    public function extractText(string $pdfPath): string
    {
        $result = $this->extract($pdfPath);
        $lines = array_map(
            fn($page) => implode("\n", array_column($page['blocks'], 'text')),
            $result['pages']
        );
        return implode("\n\n", $lines);
    }
}
```

**Track B — native:** `ext-php-rs` compiles a PHP extension in Rust directly. Alternatively,
PHP 8+ FFI (`FFI::load`) can call into a C ABI shared library without writing a C extension.
The FFI approach is easier to distribute but has higher per-call overhead than a compiled
extension.

**Distribution:** Composer package (`packagist.org`). The package downloads the platform binary
in a post-install script. PHP extension distribution requires `pecl` and per-version compilation,
which is significant maintenance overhead — subprocess Track A is the right starting point.

---

### Kotlin

The JVM is shared with Java, so the implementation is the same `ProcessBuilder` and
`java.net.http.HttpClient` approach. The Kotlin wrapper adds idiomatic sugar: coroutines for
async, extension functions, and data classes for the JSON model.

**Subprocess:**

```kotlin
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.io.File

@Serializable
data class Span(val text: String, val bbox: List<Double>, val font: String,
                val size: Double, val confidence: Double)

@Serializable
data class Block(val kind: String, val text: String, val bbox: List<Double>)

@Serializable
data class Page(val page: Int, val spans: List<Span>, val blocks: List<Block>)

@Serializable
data class Metadata(val title: String? = null, val author: String? = null,
                    val page_count: Int)

@Serializable
data class ExtractionResult(val pages: List<Page>, val metadata: Metadata)

class Pdftract(private val binaryPath: String = "pdftract") {

    private val json = Json { ignoreUnknownKeys = true }

    suspend fun extract(pdfPath: String): ExtractionResult = withContext(Dispatchers.IO) {
        val process = ProcessBuilder(binaryPath, "extract", pdfPath)
            .redirectErrorStream(false)
            .start()

        val stdout = process.inputStream.bufferedReader().readText()
        val stderr = process.errorStream.bufferedReader().readText()
        val exit = process.waitFor()

        if (exit != 0) throw RuntimeException("pdftract exited $exit: $stderr")
        json.decodeFromString(stdout)
    }

    suspend fun extractText(pdfPath: String): String =
        extract(pdfPath).pages
            .flatMap { it.blocks }
            .joinToString("\n\n") { it.text }

    suspend fun extractPage(pdfPath: String, page: Int): Page =
        extract(pdfPath).pages.first { it.page == page }
}
```

**HTTP:**

```kotlin
import io.ktor.client.*
import io.ktor.client.request.forms.*
import io.ktor.client.statement.*
import io.ktor.http.*
import java.io.File

class PdftractHttpClient(
    private val baseUrl: String = "http://localhost:8080",
    private val client: HttpClient = HttpClient()
) {
    private val json = Json { ignoreUnknownKeys = true }

    suspend fun extract(pdfPath: String): ExtractionResult {
        val file = File(pdfPath)
        val response: HttpResponse = client.submitFormWithBinaryData(
            url = "$baseUrl/extract",
            formData = formData {
                append("file", file.readBytes(), Headers.build {
                    append(HttpHeaders.ContentType, "application/pdf")
                    append(HttpHeaders.ContentDisposition, "filename=\"${file.name}\"")
                })
            }
        )
        return json.decodeFromString(response.bodyAsText())
    }

    suspend fun extractText(pdfPath: String): String =
        extract(pdfPath).pages
            .flatMap { it.blocks }
            .joinToString("\n\n") { it.text }
}
```

**Distribution:** Maven Central, same artifact group as the Java package (`com.pdftract`).
Separate artifact ID (`pdftract-kotlin`) so Java users don't pull in Kotlin stdlib.

---

## Implementation sequencing

| Priority | Language | Effort | Rationale |
|---|---|---|---|
| 1 | TypeScript | Half a day | Type definitions on top of existing JS code |
| 2 | Kotlin | Half a day | JVM wrapper on top of existing Java code |
| 3 | C# | 1–2 days | Subprocess is straightforward; NuGet RID packaging is the complexity |
| 4 | PHP | 1 day | Composer subprocess wrapper; avoid extension track initially |
| 5 | C++ | 1–2 days | `popen` + libcurl; no package manager standard, distribute as vcpkg port |

All five are blocked on the GitHub Releases binary distribution infrastructure being in place first.

---

## Cross-references

Related documentation:

- **[`sdk-invocation.md`](sdk-invocation.md)** — Subprocess and HTTP invocation patterns for all supported languages
- **[`sdk-contract.md`](sdk-contract.md)** — The constitutional SDK specification (method surface, error mapping, versioning, conformance)
- **[`ocr-language-packs.md`](ocr-language-packs.md)** — Tesseract language pack distribution and installation
- **[`docs/plan/plan.md`](../plan/plan.md)** — The source of truth for all architectural decisions (workspace layout, cross-compile matrix, ADR-009 CI policy, KU-12 platform testing)

See also:

- **Phase 6.3** (PyO3 bindings) — Python wheel build matrix via `pdftract-maturin-build`
- **Phase 7.9** (Inspector UI) — Web debug viewer bundled via `include_bytes!`
- **ADR-009** (Argo Workflows on iad-ci) — CI/CD architecture and cross-compilation strategy
