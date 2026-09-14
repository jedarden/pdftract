# pdftract MCP dogfood pilot — plan and trial results

Bead: `bf-5jt6ra` · Date: 2026-09-14 · Status: **trial run 1 complete — pilot scoped, gated on blocking bugs**

## Purpose

pdftract ships a working `pdftract mcp` server (stdio + HTTP+SSE, Phase 6.7,
ADR-006) with zero real users ahead of v0.1.0. This pilot dogfoods the server
against real PDF-extraction work — the kind agents in this workspace do
weekly — before release, on the theory that real-world documents will surface
bugs the synthetic `tests/fixtures/` corpus does not.

That theory was correct on the first afternoon: the trial below found one
release-blocking stability bug, one release-blocking capability gap, and four
protocol-level defects.

## Non-goals

- No permanent infrastructure changes to any other repo's config (per the bead
  scope). The MCP client wiring is filed as a follow-up bead instead.
- No document content is recorded anywhere. The trial corpus includes personal
  documents (employment paperwork, resumes); only metrics, error messages, and
  structural facts appear in this file. Raw trial artifacts stay under
  `target/mcp-dogfood/` (gitignored, ephemeral).

---

## Part 1 — Pilot plan (proposed scope)

### Who calls it

| Phase | Client | What it does | Where |
|---|---|---|---|
| 2a | Claude Code sessions on this box | Ad-hoc "read this PDF" tasks (research notes, specs, signed agreements) via `.mcp.json` in **this repo only**, stdio transport, `--root` scoped to the task directory | pdftract repo + local sessions |
| 2b | NEEDLE workers | One-shot extraction during bead work, via a thin helper that speaks the same framing (workers need no long-lived server; the stdio client from this trial is the template) | NEEDLE notes, no repo config change |

### What documents (the classes that matter)

1. **Signed/legal** — Docusign output with incremental updates (N × `%%EOF`),
   form fields, signature images. Hardest real-world class; exercised first.
2. **Business reports** — multi-page, headings, tables, TOC.
3. **Hardware/software manuals** — 100+ pages, mixed columns, tables.
4. **Resumes/CVs** — dense single-page layouts, reading-order stress.
5. **Research notes** — the `~/Research` and `docs/research/` material agents
   actually read daily.

### Duration and bar

- **Window:** 2 weeks of normal work after the blocking bugs below are fixed.
- **Corpus:** ≥ 20 documents, ≥ 4 classes, every failure vs a PyMuPDF
  baseline run on the same file (the baseline discipline used in the trial).
- **Quality scorecard per document:** extraction success; U+FFFD count
  (encoding recovery); heading fidelity in `extract_markdown`; reading-order
  spot check on page 1; search recall vs `grep` on the PyMuPDF text.
- **Success =** ≥ 90% per-class extraction parity with the baseline, zero
  server wedges, MCP protocol validation clean (official inspector), and every
  defect found filed as a bead within 24h.
- **Gate:** v0.1.0 does not publish to any registry until blocking beads close.

### Prerequisites to start phase 2 (filed as beads, see Part 3)

The trial showed phase 2 is pointless until extraction works on real
documents and the server stops wedging. Blocking beads (all filed this run):

- `pdftract-c7c43f45` — F1: > 1 MiB infinite loop (server wedge).
- `pdftract-257d92c3` — F2: extraction failures on real-world + fixture PDFs.
- `pdftract-36474cfa` — F3: protocol conformance (needed for the MCP
  inspector validation step and any standard client).
- `pdftract-3b44b696` — F4+F5: advertised-vs-implemented honesty, search
  stub, fingerprint/metadata quality.

---

## Part 2 — Trial run 1 (this bead): method and results

### Setup

Two server builds, to separate committed behavior from in-flight edits:

| Build | Provenance | Purpose |
|---|---|---|
| workers release | `/build/target-workers/release/pdftract`, built 2026-09-14 00:38 from the shared checkout (includes in-flight edits) | "what a dogfooder gets today" |
| clean HEAD | built from `git archive HEAD` (commit `82c55ea6`) into `/build/target-workers/debug` | "what is committed" |

Unless noted, **every finding below reproduced on the clean-HEAD build** — the
affected sources (`crates/pdftract-core/src/parser/xref.rs`,
`crates/pdftract-cli/src/mcp/**`) are clean at HEAD.

Client: a ~120-line Python stdio client speaking JSON-RPC 2.0 with LSP
`Content-Length` framing — exactly what Claude Desktop / Claude Code speak.
Method sequence: `initialize` → `notifications/initialized` → `tools/list` →
`tools/call` per document → security probes. All reads bounded; every response
matched by id; stray messages recorded.

### Corpus (metrics only; personal content never recorded)

| Document class | Size | Independent baseline (PyMuPDF) | pdftract extract |
|---|---|---|---|
| Docusign-signed offer letter (3 incremental updates) | 290,506 B | 4 pp / 12,418 chars ✓ | **FAIL** |
| Legal agreement, 23 pp | 541,671 B | 77,733 chars ✓ | **FAIL** |
| Business report, 31 pp | 249,642 B | 39,018 chars ✓ | **FAIL** |
| Resume, 4 pp | 62,312 B | 5,958 chars ✓ | **FAIL** |
| Hardware manual, 303 pp | 4,970,830 B | extracts fine | **SERVER WEDGE** (any call) |
| Repo fixture `remote_100page.pdf` (control, >1 MiB) | 6,090,837 B | extracts fine | **SERVER WEDGE** (get_metadata) |
| Repo fixture `valid-minimal.pdf` (control, synthetic) | < 62 KiB | — | **FAIL** at HEAD: *"No /Root reference in trailer"* |

### What worked

- Handshake: `initialize` (protocolVersion 2024-11-05) + `tools/list`, ~1–3 ms.
- `get_metadata` on the four < 1 MiB documents: 1–3 ms, returns page/catalog
  data. `hash` tool likewise. (But see F5 on payload quality.)
- Security posture is real: `../` traversal → `-32602` rejection; absolute
  paths under `--root` → `-32602`; `http://` URL → `-32004` (https-only).
  Paths under `--root` must be root-relative — this rejected the first trial
  client wholesale; the error message is clear but clients must be written for it.
- CLI `hash` (structural fingerprint, different code path) works on < 1 MiB files.

### Findings

**F1 · BLOCKER — any PDF > 1 MiB hangs the MCP server forever.**
`get_metadata` on a 4.97 MB manual: no reply, single-threaded busy loop at
~100% CPU (utime 399→698 ticks over 3 s, State R), after which *every*
request — including `tools/list` — goes unanswered, and SIGTERM cannot stop
the server (the handler is only checked between loop iterations; the main
thread is blocked in `read_line`). `kill -9` required. Reproduced 3/3 on two
independent > 1 MiB files (manual + repo fixture). Root cause identified:
`crates/pdftract-core/src/parser/xref.rs:1183-1186` — the large-file chunked
path does `pos += to_read; pos = pos.saturating_sub(3)` per chunk, so after
the final partial chunk `pos` cycles `source_len-3 → source_len → source_len-3`
forever. Only files above the 1 MiB `SMALL_FILE_THRESHOLD` take this path;
smaller files use `forward_scan_memory` and are unaffected. Reachable via
`get_metadata`, `hash`, `get_form_fields` (the `open_pdf` tools). The existing
property test (`xref.rs:2913`) only asserts *no panic* on small random inputs,
so it cannot see this. **Consequence for a real client: one large PDF kills
the whole session.**

**F2 · BLOCKER — extraction fails on 4/4 real-world documents (and on the
repo's own `valid-minimal.pdf` at HEAD).**
`extract_text` / `extract_markdown` errors: *"Failed to resolve /Root:
object 1 0 R not found"* (Docusign letter — note its last trailer's `/Root`
is `1 0 R`, and it has 3 incremental updates), *"object 456 0 R not found"*
(report), *"Document contains no pages"* (agreement, resume). The independent
baseline extracted all four. At clean HEAD the same call on
`tests/fixtures/valid-minimal.pdf` fails with *"No /Root reference in
trailer"* — the parser's trailer/xref resolution layer is broken across the
board, which is consistent with the ~300 pre-existing test failures known at
HEAD. The MCP layer adds nothing here; CLI `extract` fails identically.

**F3 · MAJOR — MCP protocol conformance gaps that break standard clients.**
(a) Every response to a *notification* is a stray JSON-RPC error with
`id: null` (`-32601` after `notifications/initialized`); JSON-RPC 2.0 forbids
replying to notifications. Demonstrated effect: a pipelining client that
sends the notification and the next request back-to-back reads the stray as
the reply to its request and desynchronizes (reproduced with a naive client).
(b) `tools/call` results are returned as the bare tool payload
(`{fingerprint, metadata, outline}`) — no `content` array, which
`CallToolResult` requires; a conformant client has nothing to render.
(c) Tool *execution* failures come back as JSON-RPC errors (`-32002` etc.)
instead of in-band `isError: true` content. (d) `initialize` advertises empty
`prompts`/`resources` capabilities while `resources/list` returns `-32601`.

**F4 · MAJOR — `tools/list` advertises tools that cannot work.**
`classify` (Phase 5.6), `get_form_fields` (7.4), `get_attachments` (7.5) are
advertised and always answer `-32000 NOT_YET_IMPLEMENTED`. An agent that
discovers tools by listing gets a 30% false capability rate (3 of 10).

**F5 · MAJOR — silent stubs and empty payloads where an agent will trust them.**
`search` returns `200 OK` with `matches: []` plus an underscore-prefixed
`_note` on *every* document — an agent reads "no matches" as a real answer.
`get_metadata` returns `metadata: {}` for documents that carry rich Info
dictionaries (the Docusign letter's producer/ dates are invisible), and its
"fingerprint" is `sha256(size:page_count:pages_objnum)` — not the Phase 1.7
structural fingerprint the CLI `hash` computes; two different PDFs with equal
size and page count collide.

**F6 · MINOR — SIGTERM never produces the advertised graceful shutdown**
while a client holds stdin open: the loop is blocked in `read_line`, the
signal handler only flips an atomic checked between iterations. Every trial
run ended in `kill -9`. Supervisors that SIGTERM-then-reap will always fall
to the hard kill.

**F7 · MINOR — DX:** under `--root`, tool arguments must be root-relative;
absolute paths (even inside the root) are rejected. Document for client
authors (this exact thing invalidated the first trial client, `trial1`).

**F8 · INFO — performance where it works is excellent:** handshake 1.8 ms,
`tools/list` 0.3 ms, `get_metadata` 2.7 ms on a 31-page document.

### Method notes (reproducibility)

- Framing: `Content-Length: N\r\n\r\n{json}` both directions; responses matched
  by id; unsolicited messages recorded as strays.
- Two controls guarded every claim: a PyMuPDF baseline for the documents, and
  a clean-`git archive HEAD` build to attribute findings to committed code.
- Raw artifacts (per-call request/response JSON, audit NDJSON, server stderr,
  client scripts): `target/mcp-dogfood/{trial2,trial3,probe_seq*}` — gitignored,
  ephemeral, contain no document text (responses to extraction calls were
  errors; metadata/hash payloads are structural only).

---

## Part 3 — Follow-ups filed

| Bead | Subject | Blocks |
|---|---|---|
| `pdftract-c7c43f45` | F1 fix: bound the forward-scan chunk loop (+ >1 MiB regression fixture) | pilot phase 2 |
| `pdftract-257d92c3` | F2 fix: trailer/xref resolution so real-world + `valid-minimal.pdf` extract | pilot phase 2 |
| `pdftract-36474cfa` | F3 fix: `content` wrapper, notification silence, in-band tool errors | pilot phase 2 |
| `pdftract-3b44b696` | F4+F5 fix: unadvertise unimplemented tools; make `search` fail loudly; real fingerprint + metadata in `get_metadata` | pilot phase 2 |
| `pdftract-bb45d65b` | Pilot phase 2: wire real MCP client(s), run the 2-week scorecard | — (blocked by the four above) |

## Part 4 — Verdict

The MCP server's handshake, security sandbox, and latency are release-quality;
everything a user would actually do with it is not. The synthetic fixture
corpus hid F1/F2 entirely — every one of the six substantive findings came
from real documents or protocol-level probing a fixture-based suite cannot
express. The pilot should proceed on the planned scope **after** the blocking
beads close.
