# bf-2q1crn / pdftract-5d951c85: Investigation — pdftract extraction behavior for unmapped glyphs

**Date:** 2026-09-09
**Written by:** pdftract-4ba4e938 (terminal note bead of the chain)
**Chain root:** bf-2q1crn → umbrella pdftract-5d951c85 → capture pdftract-9a0fe1a7 → {verify pdftract-798acfe6 (CLI), verify pdftract-d6e741d5 (MCP), terminal gate pdftract-d1f2cf81}
**Companion verdicts folded in from:** provenance pdftract-a3b12545 (closed), capture pdftract-9a0fe1a7 (closed), gap analysis pdftract-304dc412 (verdict recorded on bead)
**Fixture:** `tests/fixtures/encoding/unmapped-comprehensive.pdf`
**Verification HEAD at writing:** `f037c71e` (worktree fixture blob == HEAD blob)

## Verdict (one line)

**HARD ERROR** — extraction aborts at the **page-tree enumeration** stage
(`Document::pages()` yields zero `PageDict`s → `PageExtractionError::NoPagesInDocument`,
raised at `crates/pdftract-core/src/extract.rs:755` and `:1869`; message text is
`PageError::NoPages` `Display` at `crates/pdftract-core/src/page_helper.rs:116`),
**before any content-stream tokenization or glyph/encoding work**. Zero text bytes
are emitted on every path; the U+FFFD gap is unreachable until the page-tree walk
is repaired.

## 1. Fixture provenance

| Property | Value |
|---|---|
| Path | `tests/fixtures/encoding/unmapped-comprehensive.pdf` |
| Size | 651 bytes |
| sha256 | `a7effe0945c9120e5ab33ac84557755206dd10049d64a5dbd7ba5db4597ed3d3` |
| Worktree vs HEAD | identical (verified via `git show HEAD:<path> \| sha256sum`) |
| Created in | commit `d6eb8f1d` "test(bf-5taa6): add unmapped glyph PDF fixture" |
| Companion docs | `unmapped-comprehensive.md` (spec), `.txt` (expected output), `.VERIFICATION.md` (acceptance) |
| Encrypted | No · 1 page · PDF 1.4 · 612×792 pts |

### Object layout (5 objects, xref-stream file)

| Obj | Type | Content |
|---|---|---|
| 1 | stream (Length 83) | Content stream: `BT /F1 12 Tf 50 700 Td <000102> Tj 50 680 Td <03040506> Tj 50 660 Td <070809> Tj ET` — codes emitted in document order `00 01 02`, `03 04 05 06`, `07 08 09` |
| 2 | dict | `/Type/Page`, MediaBox, `/Resources` → `/Font/F1` = Type1 `/BaseFont/UnmappedTestFont`, `/Encoding <</Type/Encoding /Differences [...]>>` (font dict is nested here, **not** a separate object), `/Contents 1 0 R`, `/Parent 3 0 R` |
| 3 | dict | `/Type/Pages /Count 1 /Kids [2 0 R]` — declares exactly one page |
| 4 | dict | `/Type/Catalog /Pages 3 0 R` |
| 5 | stream (Length 35) | XRef stream: `/Root 4 0 R /Type/XRef /Size 6 /W[1 4 2] /Index[1 5]`, uncompressed |

Trailer: `startxref 500` → `%%EOF`. No `/ToUnicode` CMap, no embedded font file, so
**AGL glyph-name lookup is the only mapping path** for the `/Differences` names.

### /Differences mapping (10 codes, base 0)

`[0 /g001 /g002 /g003 /CustomA /CustomB /NotAGlyph /glyph_0041 /A /B /space]`
→ code 0→/g001, 1→/g002, 2→/g003, 3→/CustomA, 4→/CustomB, 5→/NotAGlyph,
6→/glyph_0041 (7 unmapped names), 7→/A, 8→/B, 9→/space (3 AGL-resolvable names).

### XRef stream validity — 5/5 offsets verified

Independently re-decoded this session (W[1 4 2], Index[1 5], 35 stream bytes):

| Obj | Decoded offset | Bytes found at offset |
|---|---|---|
| 1 | 9 | `1 0 obj\n<</L` ✓ |
| 2 | 140 | `2 0 obj\n<</T` ✓ |
| 3 | 404 | `3 0 obj\n<</T` ✓ |
| 4 | 455 | `4 0 obj\n<</T` ✓ |
| 5 | 500 | `5 0 obj\n<</R` ✓ — exactly where `startxref` points |

**The fixture is spec-valid.** The failure below is a pdftract page-walk defect,
not a fixture defect. (Doc drift, minor: `unmapped-comprehensive.md` describes the
font as "5 0 obj"; in the actual bytes the font is nested in obj 2 and obj 5 is the
xref stream.)

Expected output per the fixture spec (`unmapped-comprehensive.txt`, byte-verified):
7 × U+FFFD (`EF BF BD`) arranged per text line, then `AB ` — the flat string is
`(EF BF BD)×7 + 41 42 20` = 24 bytes.

## 2. Binary provenance — what was characterized, and the HEAD vs dirty-checkout verdict

Three binaries appear in this chain's evidence; the byte-exact capture (§3) used the
**installed instrument**:

| Binary | sha256 | Provenance |
|---|---|---|
| `/home/coding/.cargo/bin/pdftract` (capture instrument) | `ac1a3d951b44bc7a70616d1597bf8fe7db30d908cb238ceaaef3943ed7dd3432`, mtime 2026-08-25 17:37:57 -0400 | Re-hashed unchanged 2026-09-09; pre-dates and is independent of the dirty-checkout question |
| A-side "dirty-tree" build, `/home/coding/pdftract/target/release/pdftract` | `f65ce4e953ba65065f3654d589efaaf89c860f937c5065bad82a9b0c9c3ff325`, mtime 2026-09-08 16:05:30 -0400 | Built from the dirty shared checkout (~71 dirty files incl. `extract.rs`, `parser/xref.rs`, `font/encoding.rs`) — no single git sha by construction |
| B-side clean-HEAD build, `~/scratch/pdftract-a3b12545-clean/target/release/pdftract` | `0bf4014841a8101a076522f695150a351d987ddd6deb8d8a70b668ff249418de`, mtime 2026-09-08 16:50:22 -0400 | Isolated git worktree at pinned `f033d52b` (detached HEAD, no source deviation); `f033d52b..HEAD` touches only docs, so it behaviorally certifies current HEAD |

**Verdict (pdftract-a3b12545, closed): HEAD regression, NOT a dirty-checkout artifact.**
The clean-HEAD B-side fails `extract` rc=1 on both the byte-verified-good control
`tests/fixtures/classifier/contract/01.pdf` (`077ee8401299b78d123f75afdd0fa4f3425def24a55942e11d6eb2aa324d7c17`)
and this fixture, while `hash` returns rc=0 with the **identical** empty-document
fingerprint for both:

```
pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8
```

2×2 (extract, rc / stderr): A×control `1 / "…: Document contains no pages"`;
A×unmapped same; B×control `1 / "Error: Failed to extract PDF"`; B×unmapped same.
Independently corroborated by the glyph-unmapped check at clean rev `a8386424`
(`~/scratch/glyph-unmapped-0ba95014`). Related finding: committed HEAD has **lower**
extract error-reporting fidelity than the dirty tree (cause chain exists in the error
value but is discarded by the `e.to_string()` printer) — tracked forward as
`pdftract-6d29239a` (open).

## 3. Byte-exact capture (CLI and MCP)

Captured 2026-09-09 06:52–06:58Z into `~/scratch/pdftract-9a0fe1a7/`
(provenance.txt: instrument `ac1a3d95…d3432`, capture-time HEAD `1264a5e4`,
fixture sha `a7effe09…ed3d3`, cwd `/home/coding/pdftract`). `MANIFEST.sha256`
re-verified this session: **12/12 OK, 0 failed**; dir holds 27 files, all mtimes
inside the capture window. Live CLI re-run (pdftract-798acfe6, closed) was
**byte-identical to the stored capture, cmp 6/6**; stored MCP transcript verified
statically (pdftract-d6e741d5, closed).

### CLI path

```
$ pdftract extract --text tests/fixtures/encoding/unmapped-comprehensive.pdf
rc=1
stdout: 0 bytes  sha256 e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 (empty)
stderr: 29 bytes sha256 0f0eb3ede0e8a762dc25da6a4a2ddddd6eb573b0754c393ce7027666a354671e
```

`od -c cli.text.stderr` (the entire failure output):

```
0000000   E   r   r   o   r   :       F   a   i   l   e   d       t   o
0000020       e   x   t   r   a   c   t       P   D   F  \n
0000035
```

`--json -` (cli.json.\*): identical shape — rc=1, 0-byte stdout, same 29-byte stderr.
**No cause chain on the CLI path**; the CLI cannot distinguish unreadable-input from
parse-failure (guard finding 3 in pdftract-a3b12545). `pdftract hash` on the same
fixture: rc=0, empty stderr, stdout = the `ab24a95f…` fingerprint above — so file
read, xref-stream parse and trailer/Catalog resolve all succeed.

### MCP path

`pdftract mcp` over JSON-RPC stdio; both extract tools return the same error shape
(exact stored bytes, 134 bytes each incl. trailing `\n`):

```
// mcp.02.extract_text.response (id=2)  sha256 dec4c86320111820f5de36a12e41ca911572c053cc590207f574644db50a3893
{"jsonrpc":"2.0","error":{"code":-32002,"message":"Extraction failed: Document contains no pages","data":{"code":"IO_ERROR"}},"id":2}

// mcp.03.extract.response (id=3)       sha256 b63f914f22b54697f6057c8a8789042e797db212bf5abb610597e02b12d0d026 — identical except "id":3
```

`response_size_bytes=0`, `duration_ms=0`. Unlike the CLI, MCP **carries the cause**
("Document contains no pages") — corroborates pdftract-6d29239a. Known quirks
recorded by the verifying bead: the file named `mcp.01.extract_text.response`
actually holds the spurious `-32601` id=null reply to `notifications/initialized`
(the real extract_text response is `mcp.02`); `initialize` id=1 (protocolVersion
2024-11-05) and `tools/list` id=4 (10 tools, `extract_text` present) succeeded.

## 4. Gap analysis vs expected `7 × U+FFFD + "AB "`

Classification (pdftract-304dc412): **hard error** — wrong character / correct are
unreachable; failure is at the **page-tree enumeration** stage.

| Code | /Differences name | Class | Expected | Actual (captured) | Verdict |
|---|---|---|---|---|---|
| 0x00 | /g001 | unmapped | U+FFFD (`EF BF BD`) | zero output bytes | UNREACHED |
| 0x01 | /g002 | unmapped | U+FFFD | zero output bytes | UNREACHED |
| 0x02 | /g003 | unmapped | U+FFFD | zero output bytes | UNREACHED |
| 0x03 | /CustomA | unmapped | U+FFFD | zero output bytes | UNREACHED |
| 0x04 | /CustomB | unmapped | U+FFFD | zero output bytes | UNREACHED |
| 0x05 | /NotAGlyph | unmapped | U+FFFD | zero output bytes | UNREACHED |
| 0x06 | /glyph_0041 | unmapped | U+FFFD | zero output bytes | UNREACHED |
| 0x07 | /A | mapped (AGL) | U+0041 `A` (41) | zero output bytes | UNREACHED |
| 0x08 | /B | mapped (AGL) | U+0042 `B` (42) | zero output bytes | UNREACHED |
| 0x09 | /space | mapped (AGL) | U+0020 ` ` (20) | zero output bytes | UNREACHED |

Expected byte string: `(EF BF BD)×7 + 41 42 20` = 24 bytes. Actual: **0 bytes on
every path**. A per-character byte-diff is therefore vacuous — no wrong-character
classification is possible from this capture.

**Stage localization.** Everything upstream of page enumeration succeeds on this
fixture (`hash` rc=0 ⇒ file read, xref-stream parse, trailer/Catalog resolve; and
§1 shows the fixture itself is spec-valid with `/Pages /Count 1`). The divergence is
the page iterator yielding nothing, so `all_pages.is_empty()` →
`NoPagesInDocument`. Zero pages from a well-formed 1-page tree is a pdftract
page-walk defect. **Earliest stage to repair:** the page-tree walk
(`Document::pages()` / parser page iteration feeding `extract.rs`). Until it returns
the page this fixture declares, the U+FFFD gap is unobservable via CLI or MCP
regardless of encoding-layer correctness.

**Diagnostics.** No diagnostic code fires for the 7 unmapped codes in this capture —
not `FONT_GLYPH_UNMAPPED`, not anything else. The only structured output is the MCP
`-32002` with `data.code "IO_ERROR"` (a transport/file class, not a glyph
diagnostic). The diagnostics layer never sees these glyphs. When the encoding stage
does become reachable, the expected code string is `FONT_GLYPH_UNMAPPED`
(`diagnostics.rs:1291`), per the bf-38mvkk / pdftract-c20e0286 family finding.

**Encoding-layer readiness.** The machinery for the expected output exists and is
merely unreachable: `DifferencesOverlay` parses `/Differences` with per-entry
diagnostics (`font/encoding.rs:129,184`), and the unmapped-glyph fallback is
`['\u{FFFD}']` / `UnicodeSource::Unknown` (`font/resolver.rs:195`). Encoding
behavior on this fixture remains **UNCHARACTERIZED** — not presumed correct — until
the page-tree gate is repaired.

## 5. Reproduction commands

```sh
cd /home/coding/pdftract
FIX=tests/fixtures/encoding/unmapped-comprehensive.pdf
sha256sum "$FIX"                                   # a7effe0945c9120e5ab33ac84557755206dd10049d64a5dbd7ba5db4597ed3d3 (651 B)

# CLI (expect rc=1, 0-byte stdout, 29-byte stderr "Error: Failed to extract PDF\n")
/home/coding/.cargo/bin/pdftract extract --text "$FIX" > /tmp/o 2> /tmp/e; echo "rc=$?"
wc -c /tmp/o /tmp/e; cat /tmp/e

# JSON mode — same failure shape, still zero text bytes
/home/coding/.cargo/bin/pdftract extract --json - "$FIX" > /tmp/oj 2> /tmp/ej; echo "rc=$?"
wc -c /tmp/oj /tmp/ej

# Upstream stages all succeed: fingerprint rc=0 (empty-document state)
/home/coding/.cargo/bin/pdftract hash "$FIX"
# pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8

# Stored capture re-verification
cd ~/scratch/pdftract-9a0fe1a7 && sha256sum -c MANIFEST.sha256   # 12/12 OK
```

MCP reproduction: drive `pdftract mcp` over stdio with `initialize` →
`tools/list` → `extract_text`; expect the `-32002` JSON-RPC error with
`data.code "IO_ERROR"` (harness: `~/scratch/pdftract-9a0fe1a7/mcp_drive2.py`,
bounded — select() 30 s deadline, kill in `stop()`, `wait(timeout=10)`).

## 6. Evidence index

| Result | Bead | Artifacts |
|---|---|---|
| Provenance verdict (HEAD regression) | pdftract-a3b12545 (closed) | `~/scratch/pdftract-a3b12545/{baseline,clean-captures,verify-ad9d50da}/` |
| Byte-exact capture (CLI + MCP) | pdftract-9a0fe1a7 (closed, terminal gate pdftract-d1f2cf81) | `~/scratch/pdftract-9a0fe1a7/` (MANIFEST 12/12) |
| CLI live re-verify | pdftract-798acfe6 (closed) | `~/scratch/pdftract-798acfe6/live.*.repo-root.*` |
| MCP transcript re-verify | pdftract-d6e741d5 (closed) | static verification, same dir |
| Gap analysis + classification | pdftract-304dc412 (verdict on bead) | derived from the stored capture |

Forward work: page-tree walk repair (the single named gate), then re-run this
fixture to characterize the encoding layer; extract error-reporting fidelity is
tracked separately as pdftract-6d29239a.
