# Repro transcript — "universal extraction failure" (parent pdftract-d0dbedce)

Bead: pdftract-b4898662 (diagnosis chain bead 1 of 4). Evidence capture only — no
verdict, no code change, no fix.

## TL;DR — the failure is NOT universal at HEAD 7b5da77f

**10 of the 11 named fixtures PASS both `extract` and `hash`** with genuine document
models (real text blocks, real fingerprints, no error keys in the JSON). Exactly one
fixture fails:

```
tests/fixtures/tagged-suspects-true.pdf
  extract: exit 1 — "No trailer in xref section"                  (cause chain present)
  hash:    exit 2 — "Failed to compute fingerprint from file"     (no cause chain)
```

Both error strings the parent bead describes as universal ("Failed to compute
fingerprint from file" from hash, an opaque payload from extract) appear ONLY on this
single fixture. Observation recorded as seen (classification is the next bead's job):
`hash` prints a fixed wrapper message for any underlying failure and drops the cause
chain, while `extract` on the same fixture names the underlying cause.

Also recorded, observation only:

- `remote_100page.pdf` extracts successfully but reports `page_count: 1`.
- `test_working_copy.pdf` and `remote_100page.pdf` extract successfully with
  0 blocks / empty text (structural success, no content).

## Capture substrate

- HEAD: `7b5da77fd471098e680a23798563bb152bbf135d`
  ("test(pdftract-8ec45a90): scaffold shared fixture-discovery module under tests/common")
- The shared working tree at /home/coding/pdftract does NOT compile: `cargo build
  -p pdftract-cli` fails with `error: could not compile pdftract-core (lib) due to 18
  previous errors` — all 18 are `error[E0308]: mismatched types`, coming from ~50
  stranded in-flight edits by other workers (dirty files in `git status`, not touched
  by this bead per its no-code-change scope). The capture therefore ran from a clean
  `git archive HEAD` extraction at `/var/tmp/pdftract-repro-head`.
- All 11 fixture files verified byte-identical (md5) between the shared workspace and
  the HEAD extraction before capture.
- Build: `timeout --kill-after=30s 600s cargo build -p pdftract-cli` — **exit 0** from
  the extraction.
- Binary: `/build/target-workers/debug/pdftract` (the global cargo `target-dir`
  override in `~/.cargo/config.toml` sends every build there), mtime
  `2026-09-22 20:24:47 -0400`, size 205607328 bytes — fresh, seconds after build
  start. `pdftract --version` → `pdftract 0.1.0`.
- Binary mtime re-checked after the full 22-run matrix: **unchanged** — no other
  worker rebuilt the binary underneath this capture, so no stale shared build was
  diagnosed.
- Working directory for every run below: `/var/tmp/pdftract-repro-head`; fixture
  paths in the transcript are repo-relative.
- Raw per-run artifacts: `/var/tmp/repro-runs/<slug>/{extract,hash}.{out,err,exit}`.

## Matrix (all 11 fixtures)

| fixture | extract exit | hash exit | extract stdout | extract stderr | hash stdout | hash stderr |
|---|---|---|---|---|---|---|
| tests/fixtures/encoding/agl-only.pdf | 0 | 0 | 1131 B | empty | fingerprint | empty |
| tests/fixtures/encoding/fingerprint-match.pdf | 0 | 0 | 1143 B | empty | fingerprint | empty |
| tests/fixtures/encoding/no-mapping.pdf | 0 | 0 | 1237 B | empty | fingerprint | empty |
| tests/fixtures/encoding/shape-match.pdf | 0 | 0 | 1143 B | empty | fingerprint | empty |
| tests/fixtures/encoding/test_working_copy.pdf | 0 | 0 | 542 B | empty | fingerprint | empty |
| tests/fixtures/encoding/unmapped-comprehensive.pdf | 0 | 0 | 1241 B | empty | fingerprint | empty |
| tests/fixtures/encoding/unmapped-glyphs.pdf | 0 | 0 | 1241 B | empty | fingerprint | empty |
| tests/fixtures/test-minimal.pdf | 0 | 0 | 1216 B | empty | fingerprint | empty |
| tests/fixtures/tagged-suspects-true.pdf | **1** | **2** | empty | 113 B | empty | 47 B |
| tests/fixtures/classify_page_simple.pdf | 0 | 0 | 1153 B | empty | fingerprint | empty |
| tests/fixtures/remote_100page.pdf | 0 | 0 | 542 B | empty | fingerprint | empty |

---

## tests/fixtures/encoding/agl-only.pdf

### Command: `pdftract extract tests/fixtures/encoding/agl-only.pdf --json -`

- exit code: 0
- stderr (full):

(empty)
- stdout (full `--json` payload):

```
{
  "attachments": [],
  "fingerprint": "pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8",
  "form_fields": [],
  "javascript_actions": [],
  "links": [],
  "metadata": {
    "block_count": 1,
    "cache_age_seconds": null,
    "cache_status": "skipped",
    "page_count": 1,
    "reading_order_algorithm": "xy_cut",
    "span_count": 1
  },
  "pages": [
    {
      "blocks": [
        {
          "bbox": [
            100.0,
            700.0,
            172.0,
            712.0
          ],
          "kind": "paragraph",
          "spans": [],
          "text": "ABCabc01 ,"
        }
      ],
      "index": 0,
      "spans": [
        {
          "bbox": [
            100.0,
            700.0,
            172.0,
            712.0
          ],
          "color": "#000000",
          "confidence": 0.30000001192092896,
          "confidence_source": "heuristic",
          "font": "Unknown",
          "rendering_mode": 0,
          "size": 12.0,
          "text": "ABCabc01 ,"
        }
      ],
      "tables": []
    }
  ],
  "schema_version": "1.0",
  "signatures": [],
  "threads": []
}
```

### Command: `pdftract hash tests/fixtures/encoding/agl-only.pdf`

- exit code: 0
- stderr (full):

(empty)
- stdout (full):

```
pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8
```

---

## tests/fixtures/encoding/fingerprint-match.pdf

### Command: `pdftract extract tests/fixtures/encoding/fingerprint-match.pdf --json -`

- exit code: 0
- stderr (full):

(empty)
- stdout (full `--json` payload):

```
{
  "attachments": [],
  "fingerprint": "pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8",
  "form_fields": [],
  "javascript_actions": [],
  "links": [],
  "metadata": {
    "block_count": 1,
    "cache_age_seconds": null,
    "cache_status": "skipped",
    "page_count": 1,
    "reading_order_algorithm": "xy_cut",
    "span_count": 1
  },
  "pages": [
    {
      "blocks": [
        {
          "bbox": [
            100.0,
            700.0,
            128.8000030517578,
            712.0
          ],
          "kind": "paragraph",
          "spans": [],
          "text": "Test"
        }
      ],
      "index": 0,
      "spans": [
        {
          "bbox": [
            100.0,
            700.0,
            128.8000030517578,
            712.0
          ],
          "color": "#000000",
          "confidence": 0.30000001192092896,
          "confidence_source": "heuristic",
          "font": "Unknown",
          "rendering_mode": 0,
          "size": 12.0,
          "text": "Test"
        }
      ],
      "tables": []
    }
  ],
  "schema_version": "1.0",
  "signatures": [],
  "threads": []
}
```

### Command: `pdftract hash tests/fixtures/encoding/fingerprint-match.pdf`

- exit code: 0
- stderr (full):

(empty)
- stdout (full):

```
pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8
```

---

## tests/fixtures/encoding/no-mapping.pdf

### Command: `pdftract extract tests/fixtures/encoding/no-mapping.pdf --json -`

- exit code: 0
- stderr (full):

(empty)
- stdout (full `--json` payload):

```
{
  "attachments": [],
  "fingerprint": "pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8",
  "form_fields": [],
  "javascript_actions": [],
  "links": [],
  "metadata": {
    "block_count": 1,
    "cache_age_seconds": null,
    "cache_status": "skipped",
    "page_count": 1,
    "reading_order_algorithm": "xy_cut",
    "span_count": 1
  },
  "pages": [
    {
      "blocks": [
        {
          "bbox": [
            50.0,
            700.0,
            78.80000305175781,
            712.0
          ],
          "kind": "paragraph",
          "spans": [],
          "text": "\u0000\u0001\u0002\u0003\u0004\u0005\u0006\u0007\b\t"
        }
      ],
      "index": 0,
      "spans": [
        {
          "bbox": [
            50.0,
            700.0,
            78.80000305175781,
            712.0
          ],
          "color": "#000000",
          "confidence": 0.30000001192092896,
          "confidence_source": "heuristic",
          "font": "Unknown",
          "rendering_mode": 0,
          "size": 12.0,
          "text": "\u0000\u0001\u0002\u0003\u0004\u0005\u0006\u0007\b\t"
        }
      ],
      "tables": []
    }
  ],
  "schema_version": "1.0",
  "signatures": [],
  "threads": []
}
```

### Command: `pdftract hash tests/fixtures/encoding/no-mapping.pdf`

- exit code: 0
- stderr (full):

(empty)
- stdout (full):

```
pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8
```

---

## tests/fixtures/encoding/shape-match.pdf

### Command: `pdftract extract tests/fixtures/encoding/shape-match.pdf --json -`

- exit code: 0
- stderr (full):

(empty)
- stdout (full `--json` payload):

```
{
  "attachments": [],
  "fingerprint": "pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8",
  "form_fields": [],
  "javascript_actions": [],
  "links": [],
  "metadata": {
    "block_count": 1,
    "cache_age_seconds": null,
    "cache_status": "skipped",
    "page_count": 1,
    "reading_order_algorithm": "xy_cut",
    "span_count": 1
  },
  "pages": [
    {
      "blocks": [
        {
          "bbox": [
            100.0,
            600.0,
            215.1999969482422,
            648.0
          ],
          "kind": "paragraph",
          "spans": [],
          "text": "Test"
        }
      ],
      "index": 0,
      "spans": [
        {
          "bbox": [
            100.0,
            600.0,
            215.1999969482422,
            648.0
          ],
          "color": "#000000",
          "confidence": 0.30000001192092896,
          "confidence_source": "heuristic",
          "font": "Unknown",
          "rendering_mode": 0,
          "size": 12.0,
          "text": "Test"
        }
      ],
      "tables": []
    }
  ],
  "schema_version": "1.0",
  "signatures": [],
  "threads": []
}
```

### Command: `pdftract hash tests/fixtures/encoding/shape-match.pdf`

- exit code: 0
- stderr (full):

(empty)
- stdout (full):

```
pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8
```

---

## tests/fixtures/encoding/test_working_copy.pdf

### Command: `pdftract extract tests/fixtures/encoding/test_working_copy.pdf --json -`

- exit code: 0
- stderr (full):

(empty)
- stdout (full `--json` payload):

```
{
  "attachments": [],
  "fingerprint": "pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8",
  "form_fields": [],
  "javascript_actions": [],
  "links": [],
  "metadata": {
    "block_count": 0,
    "cache_age_seconds": null,
    "cache_status": "skipped",
    "page_count": 1,
    "reading_order_algorithm": "xy_cut",
    "span_count": 0
  },
  "pages": [
    {
      "blocks": [],
      "index": 0,
      "spans": [],
      "tables": []
    }
  ],
  "schema_version": "1.0",
  "signatures": [],
  "threads": []
}
```

### Command: `pdftract hash tests/fixtures/encoding/test_working_copy.pdf`

- exit code: 0
- stderr (full):

(empty)
- stdout (full):

```
pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8
```

---

## tests/fixtures/encoding/unmapped-comprehensive.pdf

### Command: `pdftract extract tests/fixtures/encoding/unmapped-comprehensive.pdf --json -`

- exit code: 0
- stderr (full):

(empty)
- stdout (full `--json` payload):

```
{
  "attachments": [],
  "fingerprint": "pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8",
  "form_fields": [],
  "javascript_actions": [],
  "links": [],
  "metadata": {
    "block_count": 1,
    "cache_age_seconds": null,
    "cache_status": "skipped",
    "page_count": 1,
    "reading_order_algorithm": "xy_cut",
    "span_count": 1
  },
  "pages": [
    {
      "blocks": [
        {
          "bbox": [
            50.0,
            700.0,
            171.60000610351562,
            2052.0
          ],
          "kind": "paragraph",
          "spans": [],
          "text": "\u0000\u0001\u0002\u0003\u0004\u0005\u0006\u0007\b\t"
        }
      ],
      "index": 0,
      "spans": [
        {
          "bbox": [
            50.0,
            700.0,
            171.60000610351562,
            2052.0
          ],
          "color": "#000000",
          "confidence": 0.30000001192092896,
          "confidence_source": "heuristic",
          "font": "Unknown",
          "rendering_mode": 0,
          "size": 12.0,
          "text": "\u0000\u0001\u0002\u0003\u0004\u0005\u0006\u0007\b\t"
        }
      ],
      "tables": []
    }
  ],
  "schema_version": "1.0",
  "signatures": [],
  "threads": []
}
```

### Command: `pdftract hash tests/fixtures/encoding/unmapped-comprehensive.pdf`

- exit code: 0
- stderr (full):

(empty)
- stdout (full):

```
pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8
```

---

## tests/fixtures/encoding/unmapped-glyphs.pdf

### Command: `pdftract extract tests/fixtures/encoding/unmapped-glyphs.pdf --json -`

- exit code: 0
- stderr (full):

(empty)
- stdout (full `--json` payload):

```
{
  "attachments": [],
  "fingerprint": "pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8",
  "form_fields": [],
  "javascript_actions": [],
  "links": [],
  "metadata": {
    "block_count": 1,
    "cache_age_seconds": null,
    "cache_status": "skipped",
    "page_count": 1,
    "reading_order_algorithm": "xy_cut",
    "span_count": 1
  },
  "pages": [
    {
      "blocks": [
        {
          "bbox": [
            50.0,
            700.0,
            171.60000610351562,
            2052.0
          ],
          "kind": "paragraph",
          "spans": [],
          "text": "\u0000\u0001\u0002\u0003\u0004\u0005\u0006\u0007\b\t"
        }
      ],
      "index": 0,
      "spans": [
        {
          "bbox": [
            50.0,
            700.0,
            171.60000610351562,
            2052.0
          ],
          "color": "#000000",
          "confidence": 0.30000001192092896,
          "confidence_source": "heuristic",
          "font": "Unknown",
          "rendering_mode": 0,
          "size": 12.0,
          "text": "\u0000\u0001\u0002\u0003\u0004\u0005\u0006\u0007\b\t"
        }
      ],
      "tables": []
    }
  ],
  "schema_version": "1.0",
  "signatures": [],
  "threads": []
}
```

### Command: `pdftract hash tests/fixtures/encoding/unmapped-glyphs.pdf`

- exit code: 0
- stderr (full):

(empty)
- stdout (full):

```
pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8
```

---

## tests/fixtures/test-minimal.pdf

### Command: `pdftract extract tests/fixtures/test-minimal.pdf --json -`

- exit code: 0
- stderr (full):

(empty)
- stdout (full `--json` payload):

```
{
  "attachments": [],
  "fingerprint": "pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8",
  "form_fields": [],
  "javascript_actions": [],
  "links": [],
  "metadata": {
    "block_count": 1,
    "cache_age_seconds": null,
    "cache_status": "skipped",
    "page_count": 1,
    "reading_order_algorithm": "xy_cut",
    "span_count": 1
  },
  "pages": [
    {
      "blocks": [
        {
          "bbox": [
            56.79999923706055,
            758.0999755859375,
            186.1199951171875,
            774.2000122070312
          ],
          "kind": "paragraph",
          "spans": [],
          "text": "Dummy PDF file"
        }
      ],
      "index": 0,
      "spans": [
        {
          "bbox": [
            56.79999923706055,
            758.0999755859375,
            186.1199951171875,
            774.2000122070312
          ],
          "color": "#000000",
          "confidence": 1.0,
          "confidence_source": "native",
          "font": "Unknown",
          "rendering_mode": 0,
          "size": 12.0,
          "text": "Dummy PDF file"
        }
      ],
      "tables": []
    }
  ],
  "schema_version": "1.0",
  "signatures": [],
  "threads": []
}
```

### Command: `pdftract hash tests/fixtures/test-minimal.pdf`

- exit code: 0
- stderr (full):

(empty)
- stdout (full):

```
pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8
```

---

## tests/fixtures/tagged-suspects-true.pdf

### Command: `pdftract extract tests/fixtures/tagged-suspects-true.pdf --json -`

- exit code: 1
- stderr (full):

```
Error: Failed to extract PDF: tests/fixtures/tagged-suspects-true.pdf

Caused by:
    No trailer in xref section
```
- stdout (full `--json` payload):

(empty)

### Command: `pdftract hash tests/fixtures/tagged-suspects-true.pdf`

- exit code: 2
- stderr (full):

```
Error: Failed to compute fingerprint from file
```
- stdout (full):

(empty)

---

## tests/fixtures/classify_page_simple.pdf

### Command: `pdftract extract tests/fixtures/classify_page_simple.pdf --json -`

- exit code: 0
- stderr (full):

(empty)
- stdout (full `--json` payload):

```
{
  "attachments": [],
  "fingerprint": "pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8",
  "form_fields": [],
  "javascript_actions": [],
  "links": [],
  "metadata": {
    "block_count": 1,
    "cache_age_seconds": null,
    "cache_status": "skipped",
    "page_count": 1,
    "reading_order_algorithm": "xy_cut",
    "span_count": 1
  },
  "pages": [
    {
      "blocks": [
        {
          "bbox": [
            50.0,
            700.0,
            114.80000305175781,
            712.0
          ],
          "kind": "paragraph",
          "spans": [],
          "text": "Test Page"
        }
      ],
      "index": 0,
      "spans": [
        {
          "bbox": [
            50.0,
            700.0,
            114.80000305175781,
            712.0
          ],
          "color": "#000000",
          "confidence": 0.30000001192092896,
          "confidence_source": "heuristic",
          "font": "Unknown",
          "rendering_mode": 0,
          "size": 12.0,
          "text": "Test Page"
        }
      ],
      "tables": []
    }
  ],
  "schema_version": "1.0",
  "signatures": [],
  "threads": []
}
```

### Command: `pdftract hash tests/fixtures/classify_page_simple.pdf`

- exit code: 0
- stderr (full):

(empty)
- stdout (full):

```
pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8
```

---

## tests/fixtures/remote_100page.pdf

### Command: `pdftract extract tests/fixtures/remote_100page.pdf --json -`

- exit code: 0
- stderr (full):

(empty)
- stdout (full `--json` payload):

```
{
  "attachments": [],
  "fingerprint": "pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8",
  "form_fields": [],
  "javascript_actions": [],
  "links": [],
  "metadata": {
    "block_count": 0,
    "cache_age_seconds": null,
    "cache_status": "skipped",
    "page_count": 1,
    "reading_order_algorithm": "xy_cut",
    "span_count": 0
  },
  "pages": [
    {
      "blocks": [],
      "index": 0,
      "spans": [],
      "tables": []
    }
  ],
  "schema_version": "1.0",
  "signatures": [],
  "threads": []
}
```

### Command: `pdftract hash tests/fixtures/remote_100page.pdf`

- exit code: 0
- stderr (full):

(empty)
- stdout (full):

```
pdftract-v1:ab24a95f44ceca5d2aed4b6d056adddd8539f44c6cd6ca506534e830c82ea8a8
```

---

## Layer separation — pdftract-35505114 (diagnosis chain bead 2 of 4)

Appended 2026-09-23. Substrate: clean `git archive HEAD` extraction at
`/var/tmp/pdftract-35505114-head`, HEAD `a9f3dd3dcc420400ccbf40ed47fce71d7460b614`
("test(pdftract-5d02ea2e): pin extraction exit-code domain") — one commit newer than
the capture above (7b5da77f); the failure is unchanged. Private build:
`CARGO_TARGET_DIR=/var/tmp/pdftract-35505114-target cargo build -p pdftract-cli` →
exit 0. No code changed in this bead.

### Evidence table (fixture × probe × result)

Probe fixture: `tests/fixtures/tagged-suspects-true.pdf` (the single failing fixture
from the matrix above; controls re-run to confirm the binary is sane).

| # | probe | layer exercised | result |
|---|---|---|---|
| 1 | `pdftract extract tests/fixtures/tagged-suspects-true.pdf --json -` (fresh HEAD build) | CLI → core | **exit 1** — `Error: Failed to extract PDF: …` + `Caused by: No trailer in xref section` |
| 2 | `pdftract hash tests/fixtures/tagged-suspects-true.pdf` (same build) | CLI → core | **exit 2** — `Error: Failed to compute fingerprint from file`, cause chain dropped |
| 3 | probes 1–2 re-run with the **shared** binary `/build/target-workers/debug/pdftract` (mtime 2026-09-23 03:52) | build-configuration control | identical (exit 1 / exit 2, same messages) → **not** a stale-shared-artifact effect |
| 4 | `load_xref_with_prev_chain(&source, 1221)` — `cargo test -p pdftract-core --test test_xref_debug -- --nocapture`, run from repo root | **core parser, no CLI** | section returned with **0 entries, no trailer** (print-only test; passes vacuously but its output is the direct library-level observation) |
| 5 | `pdftract_core::extract::extract_pdf(&fixture)` — `cargo test -p pdftract-core --test struct_tree_coverage` | **library extract, no CLI** | **3/3 FAIL** with `Extraction failed: No trailer in xref section` (exit 101) |
| 6 | `cargo test -p pdftract-core --test trailer_root_contract` | core error taxonomy (synthetic docs) | **7/7 pass** — the "No trailer in xref section" contract is intact; the real fixture is what falls through it |
| 7 | byte inspection of the fixture (1444 B) | fixture data | `startxref` value = **1221**; the `xref` keyword actually sits at **1220** (`grep -abo $'\nxref'` → `1220|xref`) — the pointer lands **1 byte inside the keyword** (`ref\n0 8…`) |
| 8 | controls: extract+hash on `test-minimal.pdf`, `classify_page_simple.pdf` (fresh build) | CLI → core | 0 / 0 both → failure is fixture-specific, binary sane |

### Layer conclusion

**Parse-level, in pdftract-core's xref layer — not CLI wiring, not build
configuration.** The distinguishing observation is probes 4 + 5: with the CLI removed
entirely, the core parser returns an empty, trailer-less section for this fixture
(probe 4), and the library's own extract path fails with the identical
`No trailer in xref section` (probe 5). Per the decision rule, CLI and library fail
identically on the same input → core parsing; the fresh-HEAD private build and the
shared binary agree (probe 3) → build configuration is excluded.

Mechanism, byte-level: the fixture's `startxref` is mis-encoded — 1221 instead of
1220, landing one byte *inside* the `xref` keyword. `parse_traditional_xref`'s
recovery (xref.rs:463–464, 521–522) covers offsets landing a few bytes *short* of the
keyword and offsets landing *inside the table past* it, but not the inside-the-keyword
case; the section comes back trailer-less with the `xref keyword not found`
diagnostic, and `document::resolve_root_ref` (document.rs:396) raises the observed
message. This is the same "startxref-inside-keyword gap" fixture class already
corpus-wide (54 files, 2026-09-21 re-check) — `tagged-suspects-true.pdf` is an
instance of that known class, not a new failure mode.

Both CLI symptoms are downstream renderings of that one parse failure:

- `extract` prints the full anyhow chain and exits 1.
- `hash`'s `compute_fingerprint_from_file` (hash.rs:79) makes the *same* core calls
  (`find_startxref` → `load_xref_with_prev_chain` → `resolve_root_ref`), wraps the
  failure at hash.rs:323 with `.context("Failed to compute fingerprint from file")`,
  prints Display-only (chain dropped; main.rs:840 `eprintln!("Error: {}", e)`), and
  `map_error_to_exit_code` falls through to `EXIT_CORRUPT = 2` because the wrapper
  text matches no semantic bucket. The exit-code delta (1 vs 2) and the missing cause
  chain are CLI presentation artifacts — the piece that made the parent bead read as
  two unrelated "universal" failures — and are a defect separable from the parse
  failure itself.

Fork handed to the minimization bead: the mis-encoded `startxref` originates in the
fixture (generated by `generate_suspects_fixture`, per struct_tree_coverage.rs:69), so
the fix is either fixture/generator-side (write 1220) or parser-side (extend recovery
to the inside-keyword case, which would also cover the 54-file corpus class). Both
candidates live outside the CLI layer.

Side observations (no verdict; classification bead's input):

- All 10 passing fixtures in the matrix above produce the *identical* fingerprint
  `pdftract-v1:ab24a95f…` despite different text content — the fingerprint input
  (page count, per-page geometry/structure, catalog flags;
  fingerprint/mod.rs:140) appears insensitive to text. Cause not verified here.
- In-tree tests that "cover" this fixture are largely non-signals: sdk.rs:826/876
  *swallow* parse errors (`println!("Skipping … parse error")`), and test_xref_debug
  asserts nothing. The only in-tree failing signal is `struct_tree_coverage` (3/3).
